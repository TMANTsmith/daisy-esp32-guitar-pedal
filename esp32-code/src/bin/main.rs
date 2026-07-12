#![no_std]
#![no_main]
use core::{net::Ipv4Addr, str::FromStr};
use defmt::info;
use embedded_websocket::{
    WebSocketServer, WebSocketReceiveMessageType, WebSocketSendMessageType, WebSocketContext,
};
use embassy_executor::Spawner;
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
};
use esp_println::{print, println};
use esp_radio::wifi::{Config, ControllerConfig, Interface, WifiController, ap::AccessPointConfig};
esp_bootloader_esp_idf::esp_app_desc!();

// When you are okay with using a nightly compiler it's better to use https://docs.rs/static_cell/2.1.0/static_cell/macro.make_static.html
macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

const GW_IP_ADDR_ENV: Option<&'static str> = option_env!("GATEWAY_IP");

const HTML_PAGE: &[u8] = b"HTTP/1.0 200 OK\r\n\r\n\
<html>\
<body>\
<h1>ESP32 Console</h1>\
<input id=\"msg\" type=\"text\" autofocus style=\"font-size:1.5em;width:90%\">\
<div id=\"log\"></div>\
<script>\
let ws;\
function connect() {\
    ws = new WebSocket(\"ws://\" + location.host + \"/\");\
    ws.onopen = () => { document.getElementById('log').innerHTML += '<p>connected</p>'; };\
    ws.onmessage = (e) => { document.getElementById('log').innerHTML += '<p>echo: ' + e.data + '</p>'; };\
    ws.onclose = () => {\
        document.getElementById('log').innerHTML += '<p>disconnected, retrying...</p>';\
        setTimeout(connect, 500);\
    };\
}\
connect();\
document.getElementById('msg').addEventListener('keydown', function(e) {\
    if (e.key === 'Enter' && ws.readyState === WebSocket.OPEN) {\
        ws.send(this.value);\
        this.value = '';\
    }\
});\
</script>\
</body>\
</html>\r\n\
";

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    esp_alloc::heap_allocator!(#[ram(reclaimed)] size: 64 * 1024);
    esp_alloc::heap_allocator!(size: 36 * 1024);
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

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

    let mut rx_buffer = [0; 1536];
    let mut tx_buffer = [0; 1536];
    println!(
        "Connect to the AP `esp-radio` and point your browser to http://{gw_ip_addr_str}:80/"
    );
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
            let mut resp_buf = [0u8; 1024];
            let resp_len = match ws.server_accept(&ws_context.sec_websocket_key, None, &mut resp_buf) {
                Ok(len) => len,
                Err(e) => {
                    println!("ws handshake error: {:?}", e);
                    socket.close();
                    Timer::after(Duration::from_millis(100)).await;
                    socket.abort();
                    continue;
                }
            };
            if let Err(e) = socket.write_all(&resp_buf[..resp_len]).await {
                println!("write error: {:?}", e);
                socket.close();
                Timer::after(Duration::from_millis(100)).await;
                socket.abort();
                continue;
            }
            let _ = socket.flush().await;
            println!("WebSocket handshake complete");

            let mut frame_in = [0u8; 1024];
            loop {
                let n = match socket.read(&mut buffer).await {
                    Ok(0) => {
                        println!("ws connection closed");
                        break;
                    }
                    Ok(n) => n,
                    Err(e) => {
                        println!("ws read error: {:?}", e);
                        break;
                    }
                };
                match ws.read(&buffer[..n], &mut frame_in) {
                    Ok(info_) => match info_.message_type {
                        WebSocketReceiveMessageType::Text => {
                            let text = unsafe {
                                core::str::from_utf8_unchecked(&frame_in[..info_.len_to])
                            };
                            info!("Received: {}", text);
                            let out_len = ws
                                .write(
                                    WebSocketSendMessageType::Text,
                                    true,
                                    &frame_in[..info_.len_to],
                                    &mut resp_buf,
                                )
                                .unwrap();
                            let _ = socket.write_all(&resp_buf[..out_len]).await;
                            let _ = socket.flush().await;
                        }
                        WebSocketReceiveMessageType::CloseMustReply => {
                            println!("ws close requested");
                            break;
                        }
                        _ => {}
                    },
                    Err(e) => {
                        println!("ws frame error: {:?}", e);
                        break;
                    }
                }
            }
        } else {
            let r = socket.write_all(HTML_PAGE).await;
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
