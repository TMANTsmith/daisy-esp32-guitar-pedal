#![no_std]
#![no_main]


extern crate alloc;

const CONSTS_JS: &str = concat!(
    "HTTP/1.0 200 OK\r\nContent-Type: application/javascript\r\n\r\n",
    include_str!("../consts.js")
);

use esp_println as _;
use serde_json::{to_string, from_str};
use pcobs::{serialize, deserialize};
use settings::*;
use alloc::boxed::Box;
use core::error::Error;
use core::marker::PhantomData;
use core::{net::Ipv4Addr, str::FromStr};
use defmt::info;
use defmt::Debug2Format;
use embassy_time::Instant;
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
    uart::{Uart, RxConfig, TxConfig, uhci, uhci::Uhci, uhci::UhciRx, uhci::UhciTx, uhci::UhciDmaRxTransfer, uhci::UhciDmaTxTransfer},
    dma::{DmaRxBuf, DmaTxBuf},
    dma_buffers
};
use esp_hal::uart::Config as UartConfig;
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
/// increases RAM use (FFT_BINS* 4 bytes for the shared buffer, plus
/// roughly the same again for the outgoing frame buffer) and network load.
/// Sample rate of the audio the spectrum was computed from. Only used here
/// for the startup log message; the actual bin->frequency mapping happens in
/// the browser.
/// How often a new spectrum frame is pushed to a connected WebSocket client.
const SPECTRUM_PUSH_INTERVAL: Duration = Duration::from_millis(50); // ~20 fps

const WS_FRAME_MARGIN: usize = 16; // header + margin for the binary WS frame
const TCP_BUF_SIZE: usize = 128 + 512;

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
static COMMAND: Signal<CriticalSectionRawMutex, CommandUart> = Signal::new();
/// FROM WEB TO UART


/// Publish a new spectrum frame. `data[i]` should be the magnitude (in dB,
/// e.g. -100.0 to 0.0) of frequency bin `i`, where bin `i` corresponds to
/// `i * (SAMPLE_RATE / 2) / FFT_BINS` Hz. If your FFT output is
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
        .with_baudrate(BAUDRATE)
        .with_rx(RxConfig::default().with_fifo_full_threshold(64)); 

    let mut uart = Uart::new(peripherals.UART0, uart_config).unwrap()
        .with_rx(rx)
        .with_tx(tx);

    // use these tx buffers
    let (rx_buffer_a, rx_descriptors_a, tx_buffer_a, tx_descriptors_a) = dma_buffers!(128);
    let (rx_buffer_b, rx_descriptors_b, tx_buffer_b, tx_descriptors_b) = dma_buffers!(128);

    let dma_rx_a = DmaRxBuf::new(rx_descriptors_a, rx_buffer_a).unwrap();
    let dma_rx_b = DmaRxBuf::new(rx_descriptors_b, rx_buffer_b).unwrap();

    // these tx buffers may be too big 
    let dma_tx_a = DmaTxBuf::new(tx_descriptors_a, tx_buffer_a).unwrap();
    let dma_tx_b = DmaTxBuf::new(tx_descriptors_b, tx_buffer_b).unwrap();

    let mut uhci = Uhci::new(uart, peripherals.UHCI0, peripherals.DMA_CH0).into_async();
    uhci.apply_rx_config(
        &uhci::RxConfig::default().with_chunk_limit(dma_rx_a.len().min(4095) as u16),
    )
        .unwrap();
    uhci.apply_tx_config(&uhci::TxConfig::default())
        .unwrap();
    uhci.set_uart_config(&uart_config).unwrap();

    let (mut uhci_rx, mut uhci_tx) = uhci.split();

    let mut uart_dma_rx = UartDmaRead::new();
    let mut uart_dma_tx = UartDmaRead::new();


    let access_point_config =
        Config::AccessPoint(AccessPointConfig::default().with_ssid("esp-radio"));
    //info!("Starting wifi");
    let (controller, interfaces) = esp_radio::wifi::new(
        peripherals.WIFI,
        ControllerConfig::default().with_initial_config(access_point_config),
    )
    .unwrap();
    //info!("Wifi started!");
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

    // Init network stack.
    // StackResources<4>: 1 UDP socket for DHCP + 2 TCP sockets (one per
    // http_worker instance below) + 1 spare.
    let (stack, runner) = embassy_net::new(
        device,
        config,
        mk_static!(StackResources<4>, StackResources::<4>::new()),
        seed,
    );
    spawner.spawn(net_task(runner).unwrap());
    spawner.spawn(connection(controller).unwrap());
    spawner.spawn(run_dhcp(stack, gw_ip_addr).unwrap());
    //spawner.spawn(uart_receiver(uart_dma_rx, uhci_rx, dma_rx_a, dma_rx_b).unwrap());
    spawner.spawn(uart_sender(uhci_tx, dma_tx_a).unwrap());

    //info!("Connect to the AP `esp-radio` and point your browser to http://{:?}:80/", gw_ip_addr_str);
    //info!("DHCP is enabled so there's no need to configure a static IP, just in case:");
    stack.wait_config_up().await;
    stack
        .config_v4();
        //.inspect(|c| info!("ipv4 config: {:?}", c));

    // Two independent HTTP workers, each with its own TcpSocket, both
    // listening on port 80. This lets one connection be torn down while
    // the other is available to accept, avoiding the "connection refused"
    // race a single-socket server hits when the browser opens a second
    // request (e.g. /consts.js) right after the first response.
    spawner.spawn(http_worker(stack).unwrap());
    spawner.spawn(http_worker(stack).unwrap());

    // main has nothing left to do.
    loop {
        Timer::after(Duration::from_secs(3600)).await;
    }
}

#[embassy_executor::task(pool_size = 2)]
async fn http_worker(stack: Stack<'static>) {
    let mut rx_buffer = [0u8; TCP_BUF_SIZE];
    let mut tx_buffer = [0u8; TCP_BUF_SIZE];
    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    socket.set_timeout(Some(embassy_time::Duration::from_secs(10)));

    loop {
        //info!("Wait for connection...");
        let r = socket
            .accept(IpListenEndpoint {
                addr: None,
                port: 80,
            })
            .await;
        //info!("Connected...");
        if let Err(e) = r {
            //info!("connect error: {:?}", e);
            continue;
        }

        let mut buffer = [0u8; 1024];
        let mut pos = 0;
        loop {
            if pos >= buffer.len() {
                //info!("request header too large, dropping connection");
                pos = 0; // nothing usable to parse
                break;
            }
            match socket.read(&mut buffer[pos..]).await {
                Ok(0) => {
                    //info!("read EOF");
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
                    //info!("read error: {:?}", e);
                    break;
                }
            };
        }

        if pos == 0 {
            socket.close();
            Timer::after(Duration::from_millis(10)).await;
            socket.abort();
            continue;
        }

        let request = unsafe { core::str::from_utf8_unchecked(&buffer[..pos]) };

        let path = request.lines().next().and_then(|line| line.split_whitespace().nth(1)).unwrap_or("/");
        if request.contains("Upgrade: websocket") || request.contains("upgrade: websocket") {
            //info!("WebSocket upgrade requested");
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
                    //info!("not a valid websocket upgrade request");
                    socket.close();
                    Timer::after(Duration::from_millis(10)).await;
                    socket.abort();
                    continue;
                }
                Err(e) => {
                    //info!("header parse error: {:?}", Debug2Format(&e));
                    socket.close();
                    Timer::after(Duration::from_millis(10)).await;
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
                        //info!("ws handshake error: {:?}", Debug2Format(&e));
                        socket.close();
                        Timer::after(Duration::from_millis(10)).await;
                        socket.abort();
                        continue;
                    }
                };
            if let Err(e) = socket.write_all(&handshake_buf[..resp_len]).await {
                //info!("write error: {:?}", e);
                socket.close();
                Timer::after(Duration::from_millis(10)).await;
                socket.abort();
                continue;
            }
            let _ = socket.flush().await;
            //info!("WebSocket handshake complete");

            // resp_buf holds encoded outgoing WS frames (echoed text replies
            // and binary spectrum pushes both use this buffer).
            let mut resp_buf = [0u8; 1024];
            let mut frame_in = [0u8; 1024];
            // Raw bytes of the current spectrum frame, filled just before
            // each push so we're not holding the critical_section lock
            // while doing the (slower) websocket encode + TCP write.
            let mut spectrum_bytes = [0u8; 1024];

            'ws_loop: loop {
                let read_fut = socket.read(&mut buffer);
                let tick_fut = Timer::after(SPECTRUM_PUSH_INTERVAL);

                match select(read_fut, tick_fut).await {
                    // Data arrived from the client (or the connection closed/errored).
                    Either::First(res) => {
                        let n = match res {
                            Ok(0) => {
                                //info!("ws connection closed");
                                break 'ws_loop;
                            }
                            Ok(n) => n,
                            Err(e) => {
                                //info!("ws read error: {:?}", e);
                                break 'ws_loop;
                            }
                        };
                        match ws.read(&buffer[..n], &mut frame_in) {
                            Ok(info_) => match info_.message_type {
                                WebSocketReceiveMessageType::Text => {
                                    // this is where commands go
                                    //info!("command receaved from web");
                                    let text = core::str::from_utf8(&frame_in[..info_.len_to]).unwrap_or("").trim();
                                    let cmd: Result<CommandUart, serde_json::Error> = from_str(text);
                                    match cmd {
                                        Ok(cmd) => {
                                            //info!("command spliced");
                                            //info!("cmd: {}", Debug2Format(&cmd));
                                            COMMAND.signal(cmd);   // uart_sender picks this up and serializes it
                                        },
                                        Err(e) => {
                                            //info!("json from str error: {}", Debug2Format(&e));
                                        }
                                    }
                                }
                                WebSocketReceiveMessageType::CloseMustReply => {
                                    //info!("ws close requested");
                                    break 'ws_loop;
                                }
                                _ => {}
                            },
                            Err(e) => {
                                //info!("ws frame error: {:?}", Debug2Format(&e));
                                break 'ws_loop;
                            }
                        }
                    }
                    // Timer fired: push the latest spectrum as a binary frame.
                    Either::Second(_) => {
                    }
                }
            }
        } else if path == "/consts.js" {
            let r = socket.write_all(CONSTS_JS.as_bytes()).await;
            if let Err(e) = r { //info!("write error: {:?}", e); 
            }
            let _ = socket.flush().await;
        } else {
            let r = socket.write_all(HTML_PAGE.as_bytes()).await;
            if let Err(e) = r { 
                //info!("write error: {:?}", e); 
            }
            let _ = socket.flush().await;
        }

        socket.close();
        Timer::after(Duration::from_millis(10)).await;
        socket.abort();
    }
}


#[embassy_executor::task]
async fn uart_sender(mut uhci_tx: UhciTx<'static, Async>, mut dma_tx_a: DmaTxBuf) {

    loop {

        let mut buf = [0u8; 512];
        let command = COMMAND.wait().await;

        let len = match serialize(&command, buf.as_mut_slice()) {
            Ok(l) => {
                buf[l] = FRAME_DELIM;
                l + 1
            },
            Err(err) => {
                //info!("error: {}", Debug2Format(&err));
                continue;
            }
        };
        //info!("sending: {}", &buf[0..len]);
        dma_tx_a.as_mut_slice()[0..len].copy_from_slice(&buf[0..len]);
        dma_tx_a.set_length(len);
        let mut transfer = uhci_tx
            .write(dma_tx_a)
            .unwrap_or_else(|x| panic!("Something went horribly wrong: {:?}", x.0));
        transfer.wait_for_done().await;
        let (err, uhci, dma_tx) = transfer.wait();
        uhci_tx = uhci;
        dma_tx_a = dma_tx;
        //info!("uart sent");
        if let Err(err) = err {
            //info!("error: {}", Debug2Format(&err));
        }
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
        defmt::error!("DHCP server error: {:?}", Debug2Format(&e));
    }
}

#[embassy_executor::task]
async fn connection(controller: WifiController<'static>) {
    //info!("start connection task");
    loop {
        let ev = controller
            .wait_for_access_point_connected_event_async()
            .await;
        match ev {
            Ok(esp_radio::wifi::AccessPointStationEventInfo::Connected(
                access_point_station_connected_info,
            )) => {
                //info!("Station connected: {:?}", access_point_station_connected_info);
            }
            Ok(esp_radio::wifi::AccessPointStationEventInfo::Disconnected(
                access_point_station_disconnected_info,
            )) => {
                //info!("Station disconnected: {:?}", access_point_station_disconnected_info);
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
    async fn read(&mut self, dma: DmaRxBuf) -> (Result<(), uhci::Error>, DmaRxBuf) {
        if let Some(mut transfer) = self.transfer.take() {
            transfer.wait_for_done().await;
            let (err, uhci_rx, dma_rx) = transfer.wait();
            self.transfer = Some(uhci_rx.read(dma).unwrap_or_else(|x| panic!("Something went horribly wrong: {:?}", x.0)));
            (err, dma_rx) 
        }
        else {
            panic!("uh oh");
        }
    }
}
