//! Lógica común de OHM: independiente del HAL, usable en firmware y simulador.
#![no_std]

#[cfg(test)]
extern crate std;

mod button;
mod calibration;
mod device;
mod filter;
mod profile;
mod replay;
mod traits;
mod types;

pub use button::{Button, Event};
pub use calibration::{Calibration, CalibrationError, Calibrator};
pub use device::{Device, SensorFault, StepOutput, FAILSAFE_READ_ERRORS};
pub use filter::Filter;
pub use profile::{Profile, ProfileError};
pub use replay::ReplaySensor;
pub use traits::{HapticDriver, HapticEffect, SensorInterface};
pub use types::{Micros, Position, RawSample, Sample};
