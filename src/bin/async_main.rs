#![no_std]
#![no_main]

use core::cell::OnceCell;
use embassy_executor::Spawner;
use embassy_net::{tcp::TcpSocket, Runner, StackResources};
use embassy_time::{Duration, Timer};
use esp_hal::{
    clock::CpuClock,
    gpio::{Event, Input, Io, Level, Output, Pull},
};
use esp_wifi::{
    init,
    wifi::{
        ClientConfiguration, Configuration, WifiController, WifiDeviceMode, WifiEvent, WifiMode,
        WifiStaDevice, WifiState,
    },
    EspWifiController,
};
use log::info;
use static_cell;

extern crate alloc;

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
    // let controller = init(timer1.timer0, rng, peripherals.RADIO_CLK).unwrap();

    let controller = &*mk_static!(
        EspWifiController<'static>,
        init(timer1.timer0, rng.clone(), peripherals.RADIO_CLK).unwrap()
    );

    // Create a wifi controller in station/client mode
    let (wifi_device, mut controller) =
        esp_wifi::wifi::new_with_mode(&controller, peripherals.WIFI, WifiStaDevice).unwrap();

    let config = embassy_net::Config::dhcpv4(Default::default());
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;
    let mut stackkk = StackResources::<3>::new();
    // Init network stack
    let (stack, runner) = embassy_net::new(wifi_device, config, &mut stackkk, seed);

    let led_green = Output::new(peripherals.GPIO4, Level::Low);
    // TODO: Spawn some tasks
    let res = spawner.spawn(blinker(led_green, Duration::from_millis(600)));

    // spawner.spawn(connection(controller)).ok();

    match res {
        Ok(_) => info!("was able to spawn task!"),
        Err(_) => info!("Failed to spawn  task!"),
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/v0.23.1/examples/src/bin
}

#[embassy_executor::task]
async fn blinker(mut led: Output<'static>, interval: Duration) {
    info!("Hello from blinker!");
    loop {
        led.set_high();
        Timer::after(interval).await;
        led.set_low();
        Timer::after(interval).await;
    }
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        info!("We have paniced!");
    }
}
