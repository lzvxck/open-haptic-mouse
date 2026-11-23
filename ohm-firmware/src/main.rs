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
use ohm_core::{Calibration, Device, Event, Filter};
use ohm_protocol::{ButtonState, Telemetry};
use {defmt_rtt as _, panic_probe as _};

use crate::haptic::GpioHaptic;
use crate::shared::{update_status, BOOT_PROFILE, PROFILE, TELEMETRY};
use crate::usb::update_mouse;

bind_interrupts!(struct Irqs {
    USB_LP => stm32_usb::InterruptHandler<peripherals::USB>;
});

const TELEMETRY_EVERY: u8 = 10;
const SENSOR_WARN_EVERY: u32 = 1_000;

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

fn publish_event(event: Event) {
    let pressed = event != Event::Up;
    update_mouse(|m| m.set_pressed(pressed));
    update_status(|s| {
        s.button = if pressed { ButtonState::Down } else { ButtonState::Up };
        s.last_event = Some(event);
    });
}

#[embassy_executor::task]
async fn sensor_task(mut device: Device<sensor::Sensor, GpioHaptic>) {
    let mut ticker = Ticker::every(Duration::from_hz(1_000));
    let mut since_telemetry = 0;
    loop {
        ticker.next().await;
        if let Some(new) = PROFILE.try_take() {
            device.set_profile(new);
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
            Err(fault) => {
                if fault.consecutive == 1 || fault.consecutive % SENSOR_WARN_EVERY == 0 {
                    warn!("sensor read failed, {} in a row", fault.consecutive);
                }
                if fault.released {
                    warn!("sensor failsafe: released the button after {} failed reads", fault.consecutive);
                    publish_event(Event::Up);
                }
                continue;
            }
        };
        if let Some(event) = out.event {
            info!("event: {} latency_us={}", event_name(event), out.detect_to_trigger_us);
            publish_event(event);
        }
        update_status(|s| s.last_raw = out.raw);
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
    #[cfg(feature = "sim-sensor")]
    warn!("sim-sensor build: sending simulated left-button clicks to the host over USB");

    let calibration = Calibration::new(sensor::REST, sensor::FULL).unwrap_or_else(|_| defmt::panic!("sensor REST must be below FULL"));
    let haptic = GpioHaptic::new(Output::new(p.PA5, Level::Low, Speed::Low));
    let device = Device::new(sensor::new(), haptic, calibration, Filter::new(0), BOOT_PROFILE);

    let driver = stm32_usb::Driver::new(p.USB, Irqs, p.PA12, p.PA11);
    let (usb_device, mouse, vendor) = usb::build(driver);

    spawner.spawn(usb::usb_task(usb_device).unwrap());
    spawner.spawn(usb::mouse_task(mouse).unwrap());
    spawner.spawn(usb::telemetry_task(vendor).unwrap());
    spawner.spawn(sensor_task(device).unwrap());
}
