#![no_std]
#![no_main]

use log::info;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::{clock::CpuClock,  gpio::{Event, Input, Io, Level, Output, Pull},};

extern crate alloc;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(72 * 1024);

    esp_println::logger::init_logger_from_env();

    let timer0 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG1);
    esp_hal_embassy::init(timer0.timer0);

    info!("Embassy initialized!");

    let timer1 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0);
    let _init = esp_wifi::init(
        timer1.timer0,
        esp_hal::rng::Rng::new(peripherals.RNG),
        peripherals.RADIO_CLK,
    )
    .unwrap();

    let led_green = Output::new(peripherals.GPIO4, Level::Low,);
    // TODO: Spawn some tasks
   let res = spawner.spawn(blinker(led_green, Duration::from_millis(600)));

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

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        info!("We have paniced!");
    }
}
