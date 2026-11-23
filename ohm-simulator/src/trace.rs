use crate::{run_device, LraModel, Scenario, SimHapticChain};
use ohm_core::{Event, Profile};
use serde::Serialize;

pub const WAVEFORM_RATE_HZ: u32 = 10_000;
pub const WAVEFORM_TAIL_US: u64 = 300_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceEvent {
    Down,
    RapidDown,
    Up,
}

impl From<Event> for TraceEvent {
    fn from(ev: Event) -> Self {
        match ev {
            Event::Down => Self::Down,
            Event::RapidDown => Self::RapidDown,
            Event::Up => Self::Up,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct TraceStep {
    pub t_us: u64,
    pub raw: u32,
    /// Posición normalizada (0..=1000) tras calibración y filtro.
    pub pos: u16,
    pub event: Option<TraceEvent>,
    pub detect_to_trigger_us: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct TraceTrigger {
    pub at_us: u64,
    /// Inicio efectivo del efecto: `at_us` más el retardo MODELADO de la cadena háptica.
    pub onset_us: u64,
    pub intensity: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Trace {
    pub scenario: String,
    pub duration_us: u64,
    pub actuation: u16,
    /// Posición bajo la cual se libera: `actuation - release_hysteresis`.
    pub release: u16,
    pub trigger_delay_us: u32,
    pub steps: Vec<TraceStep>,
    pub triggers: Vec<TraceTrigger>,
    pub waveform_rate_hz: u32,
    /// Forma de onda del LRA MODELADO, amplitud normalizada a ±1.
    pub waveform: Vec<f32>,
}

pub fn trace(scenario: Scenario, profile: Profile) -> Trace {
    let name = scenario.name.clone();
    let duration_us = scenario.total_us() + WAVEFORM_TAIL_US;
    let (outputs, chain) = run_device(scenario, profile, SimHapticChain::default());
    let steps = outputs
        .into_iter()
        .map(|out| TraceStep {
            t_us: out.sample.t.0,
            raw: out.raw,
            pos: out.sample.pos.get(),
            event: out.event.map(TraceEvent::from),
            detect_to_trigger_us: out.detect_to_trigger_us,
        })
        .collect();
    let triggers = chain
        .triggers
        .iter()
        .zip(chain.onsets())
        .map(|(t, (onset, intensity))| TraceTrigger { at_us: t.at.0, onset_us: onset.0, intensity })
        .collect();
    Trace {
        scenario: name,
        duration_us,
        actuation: profile.actuation.get(),
        release: profile.actuation.get().saturating_sub(profile.release_hysteresis.get()),
        trigger_delay_us: chain.trigger_delay_us,
        steps,
        triggers,
        waveform_rate_hz: WAVEFORM_RATE_HZ,
        waveform: LraModel::default().render(&chain.onsets(), WAVEFORM_RATE_HZ, duration_us),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ohm_core::Profile;

    fn load(json: &str) -> Scenario {
        Scenario::from_json(json).unwrap()
    }

    fn single_press() -> Trace {
        trace(load(include_str!("../scenarios/single_press.json")), Profile::default())
    }

    fn events(t: &Trace) -> Vec<(u64, TraceEvent)> {
        t.steps.iter().filter_map(|s| s.event.map(|e| (s.t_us, e))).collect()
    }

    #[test]
    fn single_press_traces_every_step_with_raw_and_position() {
        let t = single_press();
        assert_eq!(t.steps.len(), 70);
        assert_eq!((t.steps[0].raw, t.steps[0].pos), (30_000, 0));
        let top = t.steps.iter().find(|s| s.pos == 1000).unwrap();
        assert_eq!(top.raw, 60_000);
        assert_eq!(events(&t), vec![(20_000, TraceEvent::Down), (52_000, TraceEvent::Up)]);
    }

    #[test]
    fn thresholds_follow_the_profile() {
        let t = single_press();
        assert_eq!((t.actuation, t.release), (500, 450));
    }

    #[test]
    fn down_carries_modelled_latency_and_one_trigger() {
        let t = single_press();
        let down = t.steps.iter().find(|s| s.event == Some(TraceEvent::Down)).unwrap();
        assert_eq!(down.detect_to_trigger_us, Some(t.trigger_delay_us));
        assert_eq!(t.triggers, vec![TraceTrigger { at_us: 20_000, onset_us: 20_000 + u64::from(t.trigger_delay_us), intensity: 128 }]);
    }

    #[test]
    fn waveform_is_silent_until_the_onset_then_rings() {
        let t = single_press();
        let rate = u64::from(t.waveform_rate_hz);
        assert_eq!(t.waveform.len() as u64, t.duration_us * rate / 1_000_000);
        let onset = (t.triggers[0].onset_us * rate / 1_000_000) as usize;
        assert!(t.waveform[..=onset].iter().all(|x| *x == 0.0));
        assert!(t.waveform[onset..].iter().any(|x| x.abs() > 0.1));
    }

    #[test]
    fn rapid_window_marks_the_second_down_and_triggers_twice() {
        let profile = Profile { rapid_trigger_window_us: 20_000, ..Profile::default() };
        let t = trace(load(include_str!("../scenarios/double_press_fast.json")), profile);
        let kinds: Vec<_> = events(&t).into_iter().map(|(_, e)| e).collect();
        assert_eq!(kinds, vec![TraceEvent::Down, TraceEvent::Up, TraceEvent::RapidDown, TraceEvent::Up]);
        assert_eq!(t.triggers.len(), 2);
    }

    #[test]
    fn zero_intensity_still_triggers_but_renders_silence() {
        let profile = Profile { haptic_intensity: 0, ..Profile::default() };
        let t = trace(load(include_str!("../scenarios/single_press.json")), profile);
        assert_eq!(t.triggers.len(), 1);
        assert!(t.waveform.iter().all(|x| *x == 0.0));
    }

    #[test]
    fn serializes_events_as_snake_case() {
        let json = serde_json::to_string(&single_press().steps[20]).unwrap();
        assert!(json.contains(r#""event":"down""#), "{json}");
    }
}
