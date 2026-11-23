use ohm_core::{
    Button, Calibration, Device, Event, Filter, HapticDriver, HapticEffect, Micros, Position, Profile, RawSample, Sample,
    SensorInterface, StepOutput,
};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;

mod lra;
mod trace;

pub use lra::{LraModel, LraParams};
pub use trace::{trace, Trace, TraceEvent, TraceStep, TraceTrigger, WAVEFORM_RATE_HZ, WAVEFORM_TAIL_US};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Segment {
    Hold { pos: u16, duration_us: u64 },
    Ramp { from: u16, to: u16, duration_us: u64 },
}

impl Segment {
    fn duration_us(&self) -> u64 {
        match self {
            Segment::Hold { duration_us, .. } | Segment::Ramp { duration_us, .. } => *duration_us,
        }
    }

    fn pos_at(&self, local_us: u64) -> i64 {
        match *self {
            Segment::Hold { pos, .. } => pos as i64,
            Segment::Ramp { from, to, duration_us } => {
                from as i64 + (to as i64 - from as i64) * local_us as i64 / duration_us as i64
            }
        }
    }
}

/// Cuentas crudas que el sensor simulado emite en reposo y a fondo de recorrido.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawRange {
    pub rest: u32,
    pub full: u32,
}

impl Default for RawRange {
    fn default() -> Self {
        Self { rest: 30_000, full: 60_000 }
    }
}

impl RawRange {
    fn calibration(self) -> Calibration {
        Calibration::new(self.rest, self.full).expect("raw_range.full debe superar raw_range.rest")
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Scenario {
    pub name: String,
    pub sample_period_us: u64,
    pub seed: u64,
    /// Ruido uniforme en [-noise_amp, +noise_amp] unidades de posición normalizada (0..=1000).
    pub noise_amp: u16,
    /// Deriva en unidades de posición normalizada por segundo.
    pub drift_per_s: f32,
    #[serde(default)]
    pub raw_range: RawRange,
    pub segments: Vec<Segment>,
}

impl Scenario {
    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        let scenario: Self = serde_json::from_str(json)?;
        let RawRange { rest, full } = scenario.raw_range;
        Calibration::new(rest, full).map_err(|_| serde::de::Error::custom("raw_range.full debe superar raw_range.rest"))?;
        Ok(scenario)
    }

    fn total_us(&self) -> u64 {
        self.segments.iter().map(Segment::duration_us).sum()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Exhausted;

struct XorShift64(u64);

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
}

pub struct SimSensor {
    scenario: Scenario,
    rng: XorShift64,
    total_us: u64,
    n: u64,
}

impl SimSensor {
    pub fn new(scenario: Scenario) -> Self {
        Self { rng: XorShift64::new(scenario.seed), total_us: scenario.total_us(), scenario, n: 0 }
    }

    fn base_pos(&self, t_us: u64) -> i64 {
        let mut start = 0;
        for seg in &self.scenario.segments {
            let end = start + seg.duration_us();
            if t_us < end {
                return seg.pos_at(t_us - start);
            }
            start = end;
        }
        unreachable!("t is checked against total_us before base_pos")
    }
}

impl SensorInterface for SimSensor {
    type Error = Exhausted;

    fn read(&mut self) -> Result<RawSample, Exhausted> {
        let t_us = self.n * self.scenario.sample_period_us;
        if t_us >= self.total_us {
            return Err(Exhausted);
        }
        self.n += 1;
        let amp = self.scenario.noise_amp as u64;
        let noise = (self.rng.next() % (2 * amp + 1)) as i64 - amp as i64;
        let drift = (self.scenario.drift_per_s as f64 * t_us as f64 / 1e6).round() as i64;
        let pos = i128::from(self.base_pos(t_us) + noise + drift);
        let RawRange { rest, full } = self.scenario.raw_range;
        let span = i128::from(full - rest);
        // Redondeo hacia arriba: así `Calibration::normalize` devuelve exactamente `pos` para cualquier rango.
        let max = i128::from(Position::MAX);
        let raw = i128::from(rest) + (pos * span + max - 1).div_euclid(max);
        Ok(RawSample { t: Micros(t_us), raw: raw.clamp(0, i128::from(u32::MAX)) as u32 })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trigger {
    pub at: Micros,
    pub effect: HapticEffect,
    pub intensity: u8,
}

#[derive(Debug, Default)]
pub struct FakeHaptic {
    pub triggers: Vec<Trigger>,
}

impl HapticDriver for FakeHaptic {
    type Error = Infallible;

    fn trigger(&mut self, effect: HapticEffect, intensity: u8, at: Micros) -> Result<Micros, Infallible> {
        self.triggers.push(Trigger { at, effect, intensity });
        Ok(at)
    }
}

/// ASSUMPTION sin verificar: 1500 us entre la orden y el inicio del efecto, tomado de un rango
/// del datasheet del DRV2605L. Hay que medirlo en hardware antes de citarlo como latencia real.
pub const DEFAULT_TRIGGER_DELAY_US: u32 = 1500;

/// Háptico simulado con retardo fijo entre la orden (`at`) y el inicio del efecto.
#[derive(Debug)]
pub struct SimHapticChain {
    pub trigger_delay_us: u32,
    pub triggers: Vec<Trigger>,
}

impl Default for SimHapticChain {
    fn default() -> Self {
        Self { trigger_delay_us: DEFAULT_TRIGGER_DELAY_US, triggers: Vec::new() }
    }
}

impl SimHapticChain {
    /// Inicio efectivo de cada efecto, listo para [`LraModel::render`].
    pub fn onsets(&self) -> Vec<(Micros, u8)> {
        self.triggers.iter().map(|t| (Micros(t.at.0 + u64::from(self.trigger_delay_us)), t.intensity)).collect()
    }
}

impl HapticDriver for SimHapticChain {
    type Error = Infallible;

    fn trigger(&mut self, effect: HapticEffect, intensity: u8, at: Micros) -> Result<Micros, Infallible> {
        self.triggers.push(Trigger { at, effect, intensity });
        Ok(Micros(at.0 + u64::from(self.trigger_delay_us)))
    }
}

/// Ejecuta el escenario completo por un [`Device`] sin filtro y con la calibración de su `raw_range`.
pub fn run_device<H: HapticDriver>(scenario: Scenario, profile: Profile, haptic: H) -> (Vec<StepOutput<H::Error>>, H) {
    let calibration = scenario.raw_range.calibration();
    let mut device = Device::new(SimSensor::new(scenario), haptic, calibration, Filter::new(0), profile);
    let mut outputs = Vec::new();
    while let Ok(out) = device.step() {
        outputs.push(out);
    }
    (outputs, device.into_haptic())
}

pub fn replay(scenario: Scenario, profile: Profile) -> Vec<(Micros, Event)> {
    let calibration = scenario.raw_range.calibration();
    let mut sensor = SimSensor::new(scenario);
    let mut button = Button::new(profile);
    let mut events = Vec::new();
    while let Ok(raw) = sensor.read() {
        let sample = Sample { t: raw.t, pos: calibration.normalize(raw.raw) };
        if let Some(ev) = button.update(sample) {
            events.push((sample.t, ev));
        }
    }
    events
}
