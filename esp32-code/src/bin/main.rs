#![no_std]
#![no_main]


// TODO: double buffer with DMA on daisy seed?
extern crate alloc;
use alloc::boxed::Box;
use core::marker::PhantomData;
use esp32_code::Packet;
use core::{net::Ipv4Addr, str::FromStr};
use defmt::info;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex};
use embedded_websocket::{
    WebSocketServer, WebSocketReceiveMessageType, WebSocketSendMessageType,
};
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_net::{
    IpListenEndpoint,
    Ipv4Cidr,
    Runner,
    Stack,
    StackResources,
    StaticConfigV4,
    tcp::TcpSocket,
};
use embassy_time::{Duration, Timer};
use embedded_io_async::Write;
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    interrupt::software::SoftwareInterruptControl,
    ram,
    rng::Rng,
    timer::timg::TimerGroup,
    Blocking,
    Async,
    uart::{Uart, RxConfig, TxConfig, uhci, uhci::Uhci, uhci::UhciRx, uhci::UhciTx, uhci::UhciDmaRxTransfer},
    dma::{DmaRxBuf, DmaTxBuf},
    dma_buffers
};
use esp_hal::uart::Config as UartConfig;
use esp_println::{print, println};
use esp_radio::wifi::{Config, ControllerConfig, Interface, WifiController, ap::AccessPointConfig};
esp_bootloader_esp_idf::esp_app_desc!();

macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

const GW_IP_ADDR_ENV: Option<&'static str> = option_env!("GATEWAY_IP");

// ---------------------------------------------------------------------------
// Spectrum config — keep these in sync with the constants at the top of the
// <script> block in index.html and the consts in daisy.
// ---------------------------------------------------------------------------
/// Number of bins in the spectrum. Must be a power of two. Increasing this
/// increases RAM use (SPECTRUM_SIZE * 4 bytes for the shared buffer, plus
/// roughly the same again for the outgoing frame buffer) and network load.
const SPECTRUM_SIZE: usize = 512; // output of FFT
/// Sample rate of the audio the spectrum was computed from. Only used here
/// for the startup log message; the actual bin->frequency mapping happens in
/// the browser.
const SAMPLE_RATE_HZ: u32 = 48_000;
/// How often a new spectrum frame is pushed to a connected WebSocket client.
const SPECTRUM_PUSH_INTERVAL: Duration = Duration::from_millis(50); // ~20 fps

const SPECTRUM_PAYLOAD_BYTES: usize = SPECTRUM_SIZE * 4; // f32 = 4 bytes each
const WS_FRAME_MARGIN: usize = 16; // header + margin for the binary WS frame
const TCP_BUF_SIZE: usize = SPECTRUM_PAYLOAD_BYTES + 512;

const HTML_PAGE: &str = concat!("HTTP/1.0 200 OK\r\n\r\n", include_str!("../index.html"));

// ---------------------------------------------------------------------------
// Shared spectrum state.
//
// Whatever produces your FFT data (a mic + FFT task, I2S DMA callback, etc.)
// should call `set_spectrum(&data)` whenever a new frame is ready. The WS
// task below just reads whatever is currently here on its own timer — it
// doesn't care how often set_spectrum is called.
// +2 is for crc16
// ---------------------------------------------------------------------------

/// FROM UART TO WEB
static SPECTRUM_A: Signal<CriticalSectionRawMutex, Packet<f32, SPECTRUM_SIZE>> = Signal::new();
/// FROM WEB TO UART
static SPECTRUM_B: Signal<CriticalSectionRawMutex, Packet<f32, SPECTRUM_SIZE>> = Signal::new();


/// Publish a new spectrum frame. `data[i]` should be the magnitude (in dB,
/// e.g. -100.0 to 0.0) of frequency bin `i`, where bin `i` corresponds to
/// `i * (SAMPLE_RATE_HZ / 2) / SPECTRUM_SIZE` Hz. If your FFT output is
/// linear magnitude rather than dB, either convert it before calling this
/// (`20.0 * libm::log10f(mag.max(1e-6))`), or send it as-is and set
/// `INPUT_IS_DB = false` in index.html's <script>.


#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    esp_alloc::heap_allocator!(#[ram(reclaimed)] size: 64 * 1024);
    esp_alloc::heap_allocator!(size: 36 * 1024);
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);



    let rx = peripherals.GPIO22;
    let tx = peripherals.GPIO23;

    let uart_config = UartConfig::default()
        .with_baudrate(2_000_000)
        .with_rx(RxConfig::default().with_fifo_full_threshold(32)); 

    let mut uart = Uart::new(peripherals.UART0, uart_config).unwrap()
        .with_rx(rx)
        .with_tx(tx);

    let (rx_buffer_a, rx_descriptors_a, tx_buffer_a, tx_descriptors_a) = dma_buffers!(4092);
    let (rx_buffer_b, rx_descriptors_b, tx_buffer_b, tx_descriptors_b) = dma_buffers!(4092);

    let dma_rx_a = DmaRxBuf::new(rx_descriptors_a, rx_buffer_a).unwrap();
    let dma_rx_b = DmaRxBuf::new(rx_descriptors_b, rx_buffer_b).unwrap();

    let mut uhci = Uhci::new(uart, peripherals.UHCI0, peripherals.DMA_CH0).into_async();
    uhci.apply_rx_config(
        &uhci::RxConfig::default().with_chunk_limit(dma_rx_a.len() as u16),
    )
        .unwrap();
    uhci.apply_tx_config(&uhci::TxConfig::default())
        .unwrap();
    uhci.set_uart_config(&uart_config).unwrap();

    let (mut uchi_rx, mut uchi_tx) = uhci.split();

    let mut uart_dma = UartDmaRead::new();





    let access_point_config =
        Config::AccessPoint(AccessPointConfig::default().with_ssid("esp-radio"));
    println!("Starting wifi");
    let (controller, interfaces) = esp_radio::wifi::new(
        peripherals.WIFI,
        ControllerConfig::default().with_initial_config(access_point_config),
    )
    .unwrap();
    println!("Wifi started!");
    let device = interfaces.access_point;

    let gw_ip_addr_str = GW_IP_ADDR_ENV.unwrap_or("192.168.2.1");
    let gw_ip_addr = Ipv4Addr::from_str(gw_ip_addr_str).expect("failed to parse gateway ip");
    let config = embassy_net::Config::ipv4_static(StaticConfigV4 {
        address: Ipv4Cidr::new(gw_ip_addr, 24),
        gateway: Some(gw_ip_addr),
        dns_servers: Default::default(),
    });
    let rng = Rng::new();
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    // Init network stack
    let (stack, runner) = embassy_net::new(
        device,
        config,
        mk_static!(StackResources<3>, StackResources::<3>::new()),
        seed,
    );
    spawner.spawn(net_task(runner).unwrap());
    spawner.spawn(connection(controller).unwrap());
    spawner.spawn(run_dhcp(stack, gw_ip_addr).unwrap());
    spawner.spawn(uart_runner(uart_dma, uchi_rx, dma_rx_a, dma_rx_b).unwrap());
    // Remove this once you're feeding set_spectrum() from a real FFT source.
    //spawner.spawn(spectrum_demo_task().unwrap());

    let mut rx_buffer = [0; TCP_BUF_SIZE];
    let mut tx_buffer = [0; TCP_BUF_SIZE];
    println!(
        "Connect to the AP `esp-radio` and point your browser to http://{gw_ip_addr_str}:80/"
    );
    println!("Spectrum: {} bins @ {} Hz sample rate", SPECTRUM_SIZE, SAMPLE_RATE_HZ);
    println!("DHCP is enabled so there's no need to configure a static IP, just in case:");
    stack.wait_config_up().await;
    stack
        .config_v4()
        .inspect(|c| println!("ipv4 config: {c:?}"));

    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    socket.set_timeout(Some(embassy_time::Duration::from_secs(10)));

    loop {
        println!("Wait for connection...");
        let r = socket
            .accept(IpListenEndpoint {
                addr: None,
                port: 80,
            })
            .await;
        println!("Connected...");
        if let Err(e) = r {
            println!("connect error: {:?}", e);
            continue;
        }

        let mut buffer = [0u8; 1024];
        let mut pos = 0;
        loop {
            if pos >= buffer.len() {
                println!("request header too large, dropping connection");
                pos = 0; // nothing usable to parse
                break;
            }
            match socket.read(&mut buffer[pos..]).await {
                Ok(0) => {
                    println!("read EOF");
                    break;
                }
                Ok(len) => {
                    pos += len;
                    let to_print = unsafe { core::str::from_utf8_unchecked(&buffer[..pos]) };
                    if to_print.contains("\r\n\r\n") {
                        break;
                    }
                }
                Err(e) => {
                    println!("read error: {:?}", e);
                    break;
                }
            };
        }

        if pos == 0 {
            socket.close();
            Timer::after(Duration::from_millis(100)).await;
            socket.abort();
            continue;
        }

        let request = unsafe { core::str::from_utf8_unchecked(&buffer[..pos]) };
        print!("{}", request);
        println!();

        if request.contains("Upgrade: websocket") || request.contains("upgrade: websocket") {
            println!("WebSocket upgrade requested");
            let header_iter = request
                .lines()
                .skip(1) // skip the request line, e.g. "GET / HTTP/1.1"
                .filter_map(|line| {
                    let mut parts = line.splitn(2, ':');
                    let name = parts.next()?.trim();
                    let value = parts.next()?.trim();
                    Some((name, value.as_bytes()))
                });
            let ws_context = match embedded_websocket::read_http_header(header_iter) {
                Ok(Some(ctx)) => ctx,
                Ok(None) => {
                    println!("not a valid websocket upgrade request");
                    socket.close();
                    Timer::after(Duration::from_millis(100)).await;
                    socket.abort();
                    continue;
                }
                Err(e) => {
                    println!("header parse error: {:?}", e);
                    socket.close();
                    Timer::after(Duration::from_millis(100)).await;
                    socket.abort();
                    continue;
                }
            };
            let mut ws = WebSocketServer::new_server();
            let mut handshake_buf = [0u8; 1024];
            let resp_len =
                match ws.server_accept(&ws_context.sec_websocket_key, None, &mut handshake_buf) {
                    Ok(len) => len,
                    Err(e) => {
                        println!("ws handshake error: {:?}", e);
                        socket.close();
                        Timer::after(Duration::from_millis(100)).await;
                        socket.abort();
                        continue;
                    }
                };
            if let Err(e) = socket.write_all(&handshake_buf[..resp_len]).await {
                println!("write error: {:?}", e);
                socket.close();
                Timer::after(Duration::from_millis(100)).await;
                socket.abort();
                continue;
            }
            let _ = socket.flush().await;
            println!("WebSocket handshake complete");

            // resp_buf holds encoded outgoing WS frames (echoed text replies
            // and binary spectrum pushes both use this buffer).
            let mut resp_buf = [0u8; SPECTRUM_PAYLOAD_BYTES + WS_FRAME_MARGIN];
            let mut frame_in = [0u8; 1024];
            // Raw bytes of the current spectrum frame, filled just before
            // each push so we're not holding the critical_section lock
            // while doing the (slower) websocket encode + TCP write.
            let mut spectrum_bytes = [0u8; SPECTRUM_PAYLOAD_BYTES];

            'ws_loop: loop {
                let read_fut = socket.read(&mut buffer);
                let tick_fut = Timer::after(SPECTRUM_PUSH_INTERVAL);

                match select(read_fut, tick_fut).await {
                    // Data arrived from the client (or the connection closed/errored).
                    Either::First(res) => {
                        let n = match res {
                            Ok(0) => {
                                println!("ws connection closed");
                                break 'ws_loop;
                            }
                            Ok(n) => n,
                            Err(e) => {
                                println!("ws read error: {:?}", e);
                                break 'ws_loop;
                            }
                        };
                        match ws.read(&buffer[..n], &mut frame_in) {
                            Ok(info_) => match info_.message_type {
                                WebSocketReceiveMessageType::Text => {
                                    // Not used by the spectrum page today, but
                                    // handled in case you add client->device
                                    // commands later (e.g. "gain:12").
                                    let text = unsafe {
                                        core::str::from_utf8_unchecked(&frame_in[..info_.len_to])
                                    };
                                    info!("Received text: {}", text);
                                }
                                WebSocketReceiveMessageType::CloseMustReply => {
                                    println!("ws close requested");
                                    break 'ws_loop;
                                }
                                _ => {}
                            },
                            Err(e) => {
                                println!("ws frame error: {:?}", e);
                                break 'ws_loop;
                            }
                        }
                    }
                    // Timer fired: push the latest spectrum as a binary frame.
                    Either::Second(_) => {
                        let spectrum = SPECTRUM_A.wait().await;
                        let spectrum_bytes: &[u8] = bytemuck::cast_slice(spectrum.info());
                        let out_len = match ws.write(
                            WebSocketSendMessageType::Binary,
                            true,
                            &spectrum_bytes,
                            &mut resp_buf,
                        ) {
                            Ok(len) => len,
                            Err(e) => {
                                println!("ws encode error: {:?}", e);
                                break 'ws_loop;
                            }
                        };
                        if let Err(e) = socket.write_all(&resp_buf[..out_len]).await {
                            println!("ws write error: {:?}", e);
                            break 'ws_loop;
                        }
                        SPECTRUM_B.signal(spectrum);
                        let _ = socket.flush().await;
                    }
                }
            }
        } else {
            let r = socket.write_all(HTML_PAGE.as_bytes()).await;
            if let Err(e) = r {
                println!("write error: {:?}", e);
            }
            let r = socket.flush().await;
            if let Err(e) = r {
                println!("flush error: {:?}", e);
            }
        }

        socket.close();
        Timer::after(Duration::from_millis(100)).await;
        socket.abort();
    }
}


#[embassy_executor::task]
async fn uart_runner(mut uart_dma: UartDmaRead<Off>, uhci_rx: UhciRx<'static, Async>, mut dma_rx_a: DmaRxBuf, mut dma_rx_b: DmaRxBuf) {

    #[derive(defmt::Format)]
    enum State {
        Scanning { matched: usize },
        Collecting { filled: usize },
    }
    


    let mut state = State::Scanning { matched: 0 };

    let mut scratch = [0u8; 128];
    let mut packet = Packet::new(&[0f32; SPECTRUM_SIZE]);


    const HEADER: [u8; 4] = [0xAA, 0x55, 0xAA, 0x55];
    const PACKET_LEN: usize = 4 + SPECTRUM_PAYLOAD_BYTES + 2;

    let mut uart_dma = uart_dma.on(uhci_rx, dma_rx_a);
    let mut next_dma = dma_rx_b;



    loop {

        let filled = uart_dma.read(next_dma).await.unwrap();


        for chunk in filled.received_data() {
            for &byte in chunk {
                match &mut state {
                    State::Scanning { matched } => {
                        if byte == HEADER[*matched] {
                            *matched += 1;
                            if *matched == HEADER.len() {
                                packet.as_bytes_mut()[..4].copy_from_slice(&HEADER);
                                state = State::Collecting { filled: 4 };
                            }
                        } else {
                            *matched = if byte == HEADER[0] { 1 } else { 0 };
                        }
                    }
                    State::Collecting { filled } => {
                        packet.as_bytes_mut()[*filled] = byte;
                        *filled += 1;
                        if *filled == PACKET_LEN {
                            if packet.verify() {
                                //info!("CRC passed");
                                SPECTRUM_A.signal(packet);
                                packet = SPECTRUM_B.wait().await;
                            } else {
                                info!("CRC failed");
                                //info!("packet: {:?}", &packet);
                            }
                            state = State::Scanning { matched: 0 };
                        }
                    }
                }
            }
        }
        next_dma = filled;
    }
}

/// Small, dependency-free PRNG — good enough for fake demo noise, not for
/// anything security-sensitive.
fn xorshift32(state: &mut u32) -> u32 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    x
}

use esp_hal_dhcp_server::{server::DhcpServer, structs::DhcpServerConfig, simple_leaser::SingleDhcpLeaser};
use embassy_net::udp::UdpSocket;
use embassy_net::udp::PacketMetadata;

#[embassy_executor::task]
async fn run_dhcp(stack: Stack<'static>, gw_ip_addr: Ipv4Addr) {
    let config = DhcpServerConfig {
        ip: gw_ip_addr,
        lease_time: Duration::from_secs(3600),
        gateways: &[gw_ip_addr],
        subnet: None,
        dns: &[gw_ip_addr],
        use_captive_portal: false, // set true only if you want the captive-portal redirect behavior
    };
    let mut leaser = SingleDhcpLeaser::new(Ipv4Addr::new(192, 168, 2, 69)); // any free address on your subnet
    let res = esp_hal_dhcp_server::run_dhcp_server(stack, config, &mut leaser).await;
    if let Err(e) = res {
        defmt::error!("DHCP server error: {:?}", defmt::Debug2Format(&e));
    }
}

#[embassy_executor::task]
async fn connection(controller: WifiController<'static>) {
    println!("start connection task");
    loop {
        let ev = controller
            .wait_for_access_point_connected_event_async()
            .await;
        match ev {
            Ok(esp_radio::wifi::AccessPointStationEventInfo::Connected(
                access_point_station_connected_info,
            )) => {
                println!(
                    "Station connected: {:?}",
                    access_point_station_connected_info
                );
            }
            Ok(esp_radio::wifi::AccessPointStationEventInfo::Disconnected(
                access_point_station_disconnected_info,
            )) => {
                println!(
                    "Station disconnected: {:?}",
                    access_point_station_disconnected_info
                );
            }
            _ => (),
        }
        Timer::after(Duration::from_millis(5000)).await
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, esp_radio::wifi::Interface<'static>>) {
    runner.run().await
}


// must be made options because the transfer and dma functions consume the caller
// and output the caller when done

struct On;
struct Off;

struct UartDmaRead<State> {
    transfer: Option<UhciDmaRxTransfer<'static, Async, DmaRxBuf>>,
    _phantom: PhantomData<State>
}

// phudo code 
// make function that retuns a rx buf 
// the function when one rx buf is returned

impl UartDmaRead<Off> { 
    pub fn new() -> Self {
        Self { 
            transfer: None,
            _phantom: PhantomData,
        }
    }
    pub fn on(self, uhci_rx: UhciRx<'static, Async>, dma: DmaRxBuf) -> UartDmaRead<On>{
        let transfer = Some(uhci_rx.read(dma).unwrap_or_else(|x| panic!("Something went horribly wrong: {:?}", x.0)));
        UartDmaRead::<On> { transfer, _phantom: PhantomData }
    }
}

impl UartDmaRead<On> { 
    async fn read(&mut self, dma: DmaRxBuf) -> Result<DmaRxBuf, uhci::Error> {
        if let Some(mut transfer) = self.transfer.take() {
            transfer.wait_for_done().await;
            let (err, uhci_rx, dma_rx) = transfer.wait();
            self.transfer = Some(uhci_rx.read(dma).unwrap_or_else(|x| panic!("Something went horribly wrong: {:?}", x.0)));
            if let Err(e) = err {
                Err(e)
            }
            else { 
                Ok(dma_rx)
            }
        }
        else {
            panic!("uh oh");
        }
    }
}


 

    /*
    let transfer = uhci_rx.read(dma_rx).unwrap_or_else(|x| panic!("Something went horribly wrong: {:?}", x.0));
    transfer.wait_for_done().await;
    let (err, uhci_rx, dma_rx) = transfer.wait();
    err.unwrap();
    dma_rx.received_data();
    */
