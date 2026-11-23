use core::convert::Infallible;

use embassy_stm32::gpio::Output;
use embassy_time::{Duration, Instant};
use ohm_core::{HapticDriver, HapticEffect, Micros};

const PULSE: Duration = Duration::from_millis(20);

/// Stand-in for the DRV2605L IN/TRIG pin: drives the Nucleo user LED LD2 (PA5) high for 20 ms per trigger.
/// A GPIO has no amplitude, so `intensity` and `effect` are ignored. The DRV2605L driver replaces this type
/// and reads both. Call [`GpioHaptic::poll`] at least once per millisecond to end the pulse.
pub struct GpioHaptic {
    pin: Output<'static>,
    off_at: Option<Instant>,
}

impl GpioHaptic {
    pub fn new(pin: Output<'static>) -> Self {
        Self { pin, off_at: None }
    }

    pub fn poll(&mut self) {
        if self.off_at.is_some_and(|t| Instant::now() >= t) {
            self.pin.set_low();
            self.off_at = None;
        }
    }
}

impl HapticDriver for GpioHaptic {
    type Error = Infallible;

    /// Returns `at` plus the time this driver spent raising the pin.
    fn trigger(&mut self, _: HapticEffect, _: u8, at: Micros) -> Result<Micros, Infallible> {
        let start = Instant::now();
        self.pin.set_high();
        self.off_at = Some(start + PULSE);
        Ok(Micros(at.0 + start.elapsed().as_micros()))
    }
}
