//! Sensor selection. `sim-sensor` (default) replays a fixed press pattern, so a board running it
//! clicks the host's left button about once a second. Build with `--no-default-features` to disable.

#[cfg(not(feature = "sim-sensor"))]
compile_error!("no sensor selected: enable `sim-sensor`, or implement `ldc1101` and wire it into `sensor::new`");

#[cfg(feature = "ldc1101")]
pub mod ldc1101 {
    //! Real sensor path, not implemented yet. Implement `ohm_core::SensorInterface` over SPI for the
    //! LDC1101 (24-bit L-mode counts into `RawSample::raw`, `t` from the embassy-time clock in microseconds),
    //! then return it from `sensor::new` and set `REST`/`FULL` from a `Calibrator` run at boot.
}

pub const PERIOD_US: u32 = 1_000;
pub const REST: u32 = 10_000;
pub const FULL: u32 = 20_000;
const PRESSED: u32 = 16_000;

/// (raw counts, number of samples): rest, single press, rest, rapid double press (20 ms apart), rest.
const SEGMENTS: [(u32, usize); 7] =
    [(REST, 300), (PRESSED, 100), (REST, 300), (PRESSED, 40), (REST, 20), (PRESSED, 40), (REST, 300)];

const fn pattern_len() -> usize {
    let (mut i, mut n) = (0, 0);
    while i < SEGMENTS.len() {
        n += SEGMENTS[i].1;
        i += 1;
    }
    n
}

const fn pattern<const N: usize>() -> [u32; N] {
    let (mut out, mut at, mut i) = ([0; N], 0, 0);
    while i < SEGMENTS.len() {
        let mut k = 0;
        while k < SEGMENTS[i].1 {
            out[at] = SEGMENTS[i].0;
            at += 1;
            k += 1;
        }
        i += 1;
    }
    out
}

static PRESS_PATTERN: [u32; pattern_len()] = pattern();

#[cfg(feature = "sim-sensor")]
pub type Sensor = ohm_core::ReplaySensor<'static>;

#[cfg(feature = "sim-sensor")]
pub fn new() -> Sensor {
    ohm_core::ReplaySensor::new(&PRESS_PATTERN, PERIOD_US)
}
