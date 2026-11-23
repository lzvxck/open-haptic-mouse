#[cfg(not(feature = "sim-sensor"))]
compile_error!("no sensor selected: enable `sim-sensor` (no hardware sensor driver exists yet)");

pub const PERIOD_US: u32 = 1_000;
pub const REST: u32 = 10_000;
pub const FULL: u32 = 20_000;
const PRESSED: u32 = 16_000;

struct Segment {
    raw: u32,
    samples: usize,
}

const fn seg(raw: u32, samples: usize) -> Segment {
    Segment { raw, samples }
}

const SEGMENTS: [Segment; 7] =
    [seg(REST, 300), seg(PRESSED, 100), seg(REST, 300), seg(PRESSED, 40), seg(REST, 20), seg(PRESSED, 40), seg(REST, 300)];

const fn pattern_len() -> usize {
    let (mut i, mut n) = (0, 0);
    while i < SEGMENTS.len() {
        n += SEGMENTS[i].samples;
        i += 1;
    }
    n
}

const fn pattern<const N: usize>() -> [u32; N] {
    let (mut out, mut at, mut i) = ([0; N], 0, 0);
    while i < SEGMENTS.len() {
        let mut k = 0;
        while k < SEGMENTS[i].samples {
            out[at] = SEGMENTS[i].raw;
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
