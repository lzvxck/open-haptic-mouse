use crate::{Micros, RawSample};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HapticEffect {
    Click,
}

pub trait SensorInterface {
    type Error;
    fn read(&mut self) -> Result<RawSample, Self::Error>;
}

pub trait HapticDriver {
    type Error;
    /// Devuelve el instante en que el efecto empieza a actuar: `at` más la latencia propia del driver.
    fn trigger(&mut self, effect: HapticEffect, intensity: u8, at: Micros) -> Result<Micros, Self::Error>;
}
