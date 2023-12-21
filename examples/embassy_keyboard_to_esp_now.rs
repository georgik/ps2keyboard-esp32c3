#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::pipe::Pipe;
use embedded_hal_async::digital::Wait;
use esp_backtrace as _;
use esp_wifi::esp_now::{EspNow, PeerInfo, BROADCAST_ADDRESS};
use esp_wifi::{initialize, EspWifiInitFor};

use hal::peripheral;
use hal::{
    clock::{ClockControl, CpuClock},
    embassy,
    gpio::{Gpio1, Gpio2, Input, PullDown},
    IO,
    peripherals::Peripherals,
    prelude::*,
    Rng,
    timer::TimerGroup,
};
use log::{error, info};

const PIPE_BUF_SIZE: usize = 5;
static PIPE: Pipe<CriticalSectionRawMutex, PIPE_BUF_SIZE> = Pipe::new();

#[embassy_executor::task]
async fn esp_now_writer(
) {
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

    loop {
        let mut byte = [0u8];
        if PIPE.read(&mut byte).await > 0 {
            let status = esp_now.send_async(&BROADCAST_ADDRESS, &byte).await;
            match status {
                Ok(_) => info!("Data sent over ESP-NOW: {:?}", byte),
                Err(e) => error!("ESP-NOW send error: {:?}", e),
            }
        }
    }
}

#[embassy_executor::task]
async fn ps2_reader(mut data: Gpio1<Input<PullDown>>, mut clk: Gpio2<Input<PullDown>>) {
    let mut bit_count: usize = 0;
    let mut current_byte: u8 = 0;
    loop {
        clk.wait_for_falling_edge().await.unwrap();
        let bit = if data.is_high().unwrap() { 1 } else { 0 };
        if bit_count > 0 && bit_count < 9 {
            current_byte >>= 1;
            current_byte |= bit << 7;
        }
        bit_count += 1;
        if bit_count == 11 {
            PIPE.write(&[current_byte]).await;
            bit_count = 0;
            current_byte = 0;
        }
    }
}

#[main]
async fn main(spawner: Spawner) {
    let peripherals = Peripherals::take();

  
    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let data = io.pins.gpio1.into_pull_down_input();
    let clk = io.pins.gpio2.into_pull_down_input();

    // Spawn the tasks
    spawner.spawn(ps2_reader(data, clk)).unwrap();
    spawner.spawn(esp_now_writer()).unwrap();
}
