#![no_std]
#![no_main]

mod haptic;
mod sensor;
mod shared;
mod usb;

use defmt::{info, warn};
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::rcc::{mux, Hsi48Config, Pll, PllMul, PllPreDiv, PllRDiv, PllSource, Sysclk};
use embassy_stm32::{bind_interrupts, peripherals, usb as stm32_usb, Config};
use embassy_time::{Duration, Ticker};
use ohm_core::{Calibration, Device, Event, Filter, Profile};
use ohm_protocol::{ButtonState, Status, Telemetry, PROTOCOL_VERSION};
use {defmt_rtt as _, panic_probe as _};

use crate::haptic::GpioHaptic;
use crate::shared::{EVENTS, PROFILE, STATUS, TELEMETRY};

bind_interrupts!(struct Irqs {
    USB_LP => stm32_usb::InterruptHandler<peripherals::USB>;
});

/// Telemetry goes out on every event and, in between, every `TELEMETRY_EVERY` ticks.
const TELEMETRY_EVERY: u8 = 10;

fn clock_config() -> Config {
    let mut config = Config::default();
    config.rcc.pll = Some(Pll {
        source: PllSource::HSI,
        prediv: PllPreDiv::DIV4,
        mul: PllMul::MUL85,
        divp: None,
        divq: None,
        divr: Some(PllRDiv::DIV2),
    });
    config.rcc.sys = Sysclk::PLL1_R;
    config.rcc.boost = true;
    config.rcc.hsi48 = Some(Hsi48Config { sync_from_usb: true });
    config.rcc.mux.clk48sel = mux::Clk48sel::HSI48;
    config
}

fn event_name(event: Event) -> &'static str {
    match event {
        Event::Down => "down",
        Event::RapidDown => "rapid-down",
        Event::Up => "up",
    }
}

#[embassy_executor::task]
async fn sensor_task(mut device: Device<sensor::Sensor, GpioHaptic>, mut profile: Profile) {
    let mut ticker = Ticker::every(Duration::from_hz(1_000));
    let mut button = ButtonState::Up;
    let mut last_event = None;
    let mut since_telemetry = 0;
    loop {
        ticker.next().await;
        if let Some(new) = PROFILE.try_take() {
            device.set_profile(new);
            profile = new;
            info!(
                "profile: actuation={} hysteresis={} rapid_us={} intensity={}",
                new.actuation.get(),
                new.release_hysteresis.get(),
                new.rapid_trigger_window_us,
                new.haptic_intensity
            );
        }
        device.haptic_mut().poll();
        let out = match device.step() {
            Ok(out) => out,
            Err(_) => {
                warn!("sensor read failed");
                continue;
            }
        };
        if let Some(event) = out.event {
            info!("event: {} latency_us={}", event_name(event), out.detect_to_trigger_us);
            button = if event == Event::Up { ButtonState::Up } else { ButtonState::Down };
            last_event = Some(event);
            if EVENTS.try_send(event).is_err() {
                warn!("event queue full, dropped {}", event_name(event));
            }
        }
        STATUS.lock(|s| {
            s.set(Some(Status { protocol_version: PROTOCOL_VERSION, profile, last_raw: out.raw, button, last_event }));
        });
        since_telemetry += 1;
        if out.event.is_some() || since_telemetry >= TELEMETRY_EVERY {
            since_telemetry = 0;
            let _ = TELEMETRY.try_send(Telemetry {
                timestamp: out.sample.t,
                position: out.sample.pos,
                event: out.event,
                latency_us: out.detect_to_trigger_us.unwrap_or(0),
            });
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(clock_config());
    info!("OHM firmware boot, sysclk 170 MHz, usb {=u16:04x}:{=u16:04x} (TEST)", usb::TEST_VID, usb::TEST_PID);

    let profile = Profile { rapid_trigger_window_us: 50_000, ..Profile::default() };
    let calibration = Calibration::new(sensor::REST, sensor::FULL).unwrap_or_else(|_| defmt::panic!("sensor REST must be below FULL"));
    let haptic = GpioHaptic::new(Output::new(p.PA5, Level::Low, Speed::Low));
    let device = Device::new(sensor::new(), haptic, calibration, Filter::new(0), profile);

    let driver = stm32_usb::Driver::new(p.USB, Irqs, p.PA12, p.PA11);
    let (usb_device, mouse, vendor) = usb::build(driver);

    spawner.spawn(usb::usb_task(usb_device).unwrap());
    spawner.spawn(usb::mouse_task(mouse).unwrap());
    spawner.spawn(usb::telemetry_task(vendor).unwrap());
    spawner.spawn(sensor_task(device, profile).unwrap());
}
