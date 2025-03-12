#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_net::{
    dns::DnsSocket,
    tcp::client::{TcpClient, TcpClientState},
    Runner, Stack, StackResources,
};
use embassy_time::{Duration, Timer};
use esp_hal::{clock::CpuClock, gpio::Output};

use esp_wifi::{
    init,
    wifi::{
        ClientConfiguration, Configuration, WifiController, WifiDevice, WifiEvent, WifiStaDevice,
        WifiState,
    },
    EspWifiController,
};
use log::info;
use reqwless::{
    client::{HttpClient, TlsConfig},
    request::RequestBuilder,
};
use static_cell;

extern crate alloc;

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(60 * 1024);
    let mut rng = esp_hal::rng::Rng::new(peripherals.RNG);

    esp_println::logger::init_logger_from_env();

    let timer0 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG1);
    esp_hal_embassy::init(timer0.timer0);

    info!("Embassy initialized!");

    let timer1 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0);

    let controller = &*mk_static!(
        EspWifiController<'static>,
        init(timer1.timer0, rng.clone(), peripherals.RADIO_CLK).unwrap()
    );

    // Create a wifi controller in station/client mode
    let (wifi_device, wifi_controller) =
        esp_wifi::wifi::new_with_mode(&controller, peripherals.WIFI, WifiStaDevice).unwrap();

    let config = embassy_net::Config::dhcpv4(Default::default());
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;
    let stack_resources = mk_static!(StackResources<3>, StackResources::<3>::new());
    // Init network stack
    let (stack, runner) = embassy_net::new(wifi_device, config, stack_resources, seed);

    spawner.spawn(connection(wifi_controller)).ok();
    spawner.spawn(net_task(runner)).ok();

    loop {
        if stack.is_link_up() {
            info!("Link is up!!");
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    info!("Waiting to get IP address...");
    loop {
        if let Some(config) = stack.config_v4() {
            info!("Got IP: {}", config.address);
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    // let tls_seed = rng.random() as u64 | ((rng.random() as u64) << 32);
    // let rsa = Rsa::new(peripherals.RSA);
    // access_time_api(stack, tls_seed).await;
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static, WifiStaDevice>>) {
    runner.run().await
}

#[embassy_executor::task]
async fn blinker(mut led: Output<'static>, interval: Duration) {
    loop {
        led.set_high();
        Timer::after(interval).await;
        led.set_low();
        Timer::after(interval).await;
    }
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    info!("start connection task");
    info!(
        "Device capabilities: {:?}",
        controller.capabilities().unwrap()
    );

    let ssid: &'static str = SSID.try_into().unwrap();
    let password: &'static str = PASSWORD.try_into().unwrap();

    info!("SSID: {:?}", ssid);

    info!("Password: {:?}", password);

    loop {
        match esp_wifi::wifi::wifi_state() {
            WifiState::StaConnected => {
                // wait until we're no longer connected
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                Timer::after(Duration::from_millis(5000)).await
            }
            _ => {}
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: ssid.try_into().unwrap(),
                password: password.try_into().unwrap(),
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            info!("Starting wifi");
            controller.start_async().await.unwrap();
            info!("Wifi started!");
        }
        info!("About to connect...");

        match controller.connect_async().await {
            Ok(_) => info!("Wifi connected!"),
            Err(e) => {
                info!("Failed to connect to wifi: {e:?}");
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}

async fn access_time_api<'d>(stack: Stack<'static>, tls_seed: u64) {
    // let mut rx_buffer = [0; 4096];
    // let mut tx_buffer = [0; 4096];

    loop {
        if stack.is_link_up() {
            info!("Link is up!!");
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }
    let dns = DnsSocket::new(stack);
    let tcp_state = TcpClientState::<1, 4096, 4096>::new();
    let tcp = TcpClient::new(stack, &tcp_state);

    // let config = TlsConfig::new(
    //     reqwless::TlsVersion::Tls1_3,
    //     reqwless::Certificates {
    //         ca_chain: reqwless::X509::pem(CERT.as_bytes()).ok(),
    //         ..Default::default()
    //     },
    //     Some(&mut rsa), // Will use hardware acceleration
    // );

    // let tls = TlsConfig::new(
    //     tls_seed,
    //     &mut rx_buffer,
    //     &mut tx_buffer,
    //     // reqwless::client::TlsVerify::None,
    // );

    let mut client = HttpClient::new(&tcp, &dns);
    // let mut client = HttpClient::new_with_tls(&tcp, &dns, tls);
    let mut buffer = [0u8; 4096];
    let http_req = client
        .request(
            reqwless::request::Method::GET,
            "https://worldtimeapi.org/api/timezone/America/New_York",
        )
        .await;

    match http_req {
        Ok(mut req) => {
            let response_result = req.send(&mut buffer).await;

            match response_result {
                Ok(_) => info!("got to read request"),
                Err(err) => info!("Ran into error after request sent: {:?}", err),
            }
        }
        Err(err) => info!("Ran into error building request: {:?}", err),
    }

    // let response = http_req.send(&mut buffer).await.unwrap();

    info!("Got response");
    // let res = response.body().read_to_end().await.unwrap();

    // let content = core::str::from_utf8(res).unwrap();
    // info!("{}", content);
}
