#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use embassy_executor::Spawner;
use embassy_time::Duration;
use esp_backtrace as _;
use esp_wifi::esp_now::{EspNow, PeerInfo, BROADCAST_ADDRESS};

use hal::{clock::{
    ClockControl, CpuClock
},embassy, peripherals::Peripherals, prelude::*,
          Rng,
        };
use log::{info, error};
use esp_wifi::{initialize, EspWifiInitFor};

#[embassy_executor::task]
async fn esp_now_receiver() {
    let peripherals = unsafe { Peripherals::steal() };
    let system = peripherals.SYSTEM.split();
    // Initialize the embassy
    let timer = hal::systimer::SystemTimer::new(peripherals.SYSTIMER).alarm0;
    let rng = Rng::new(peripherals.RNG);
    let radio_clock_control = system.radio_clock_control;
    let clocks = ClockControl::configure(system.clock_control, CpuClock::Clock160MHz).freeze();

    let wifi = peripherals.WIFI;

    let init = initialize(
        EspWifiInitFor::Wifi,
        timer,
        rng,
        radio_clock_control,
        &clocks,
    )
    .unwrap();

    let mut esp_now = EspNow::new(&init, wifi).unwrap();

    // Add BROADCAST_ADDRESS as peer if needed
    let peer_info = PeerInfo {
        peer_address: BROADCAST_ADDRESS,
        lmk: None,
        channel: None,
        encrypt: false,
    };
    // info!("Adding peer");
    // match esp_now.add_peer(peer_info) {
    //     Ok(_) => info!("Peer added"),
    //     Err(e) => error!("Peer add error: {:?}", e),
    // }

    loop {
        let received_data = esp_now.receive();
        match received_data {
            Some(data) => {
                info!("Data received over ESP-NOW: {:?}", data);
            }
            _ => {
                error!("ESP-NOW receive error");
            }
        }
    }
}

#[main]
async fn main(spawner: Spawner) {
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();
    esp_println::logger::init_logger_from_env();

    info!("Starting");
    let timer = hal::timer::TimerGroup::new(peripherals.TIMG0, &clocks).timer0;

    embassy::init(&clocks, timer);
    spawner.spawn(esp_now_receiver()).unwrap();
}
