use ohm_core::{Event, HapticDriver, HapticEffect, Micros, Profile};
use ohm_simulator::{run_device, Scenario, SimHapticChain, DEFAULT_TRIGGER_DELAY_US};

const INTENSITY: u8 = 200;

fn profile() -> Profile {
    Profile { rapid_trigger_window_us: 20_000, haptic_intensity: INTENSITY, ..Profile::default() }
}

fn run(json: &str) -> (Vec<(Micros, Event, Option<u32>)>, SimHapticChain) {
    let (outs, chain) = run_device(Scenario::from_json(json).unwrap(), profile(), SimHapticChain::default());
    let events = outs.iter().filter_map(|o| o.event.map(|e| (o.sample.t, e, o.detect_to_trigger_us))).collect();
    (events, chain)
}

const SINGLE: &str = include_str!("../scenarios/single_press.json");
const DOUBLE_FAST: &str = include_str!("../scenarios/double_press_fast.json");

#[test]
fn single_press_triggers_once_at_the_down_timestamp() {
    let (events, chain) = run(SINGLE);
    assert_eq!(events.len(), 2);
    assert_eq!(chain.triggers.len(), 1);
    assert_eq!(chain.triggers[0].at, events[0].0);
    assert_eq!(chain.triggers[0].effect, HapticEffect::Click);
    assert_eq!(chain.triggers[0].intensity, INTENSITY);
}

#[test]
fn detect_to_trigger_equals_the_modelled_delay_and_up_has_none() {
    let (events, _) = run(SINGLE);
    assert_eq!(DEFAULT_TRIGGER_DELAY_US, 1500);
    assert_eq!(events[0], (Micros(20_000), Event::Down, Some(1500)));
    assert_eq!(events[1], (Micros(52_000), Event::Up, None));
}

#[test]
fn double_press_fast_triggers_twice_and_rapid_down_clicks_too() {
    let (events, chain) = run(DOUBLE_FAST);
    let downs: Vec<_> = events.iter().filter(|(_, e, _)| *e != Event::Up).collect();
    assert_eq!(downs.iter().map(|(_, e, _)| *e).collect::<Vec<_>>(), vec![Event::Down, Event::RapidDown]);
    assert_eq!(chain.triggers.len(), 2);
    for (trigger, (t, _, latency)) in chain.triggers.iter().zip(downs) {
        assert_eq!(trigger.at, *t);
        assert_eq!(*latency, Some(1500));
    }
}

#[test]
fn custom_delay_flows_into_the_report() {
    let chain = SimHapticChain { trigger_delay_us: 4_000, ..SimHapticChain::default() };
    let (outs, _) = run_device(Scenario::from_json(SINGLE).unwrap(), profile(), chain);
    let latencies: Vec<_> = outs.iter().filter_map(|o| o.detect_to_trigger_us).collect();
    assert_eq!(latencies, vec![4_000]);
}

struct FailingHaptic;

impl HapticDriver for FailingHaptic {
    type Error = &'static str;
    fn trigger(&mut self, _: HapticEffect, _: u8, _: Micros) -> Result<Micros, &'static str> {
        Err("i2c nack")
    }
}

#[test]
fn haptic_fault_does_not_drop_the_down_event() {
    let (outs, _) = run_device(Scenario::from_json(SINGLE).unwrap(), profile(), FailingHaptic);
    let down = outs.iter().find(|o| o.event == Some(Event::Down)).unwrap();
    assert_eq!(down.sample.t, Micros(20_000));
    assert_eq!(down.haptic_fault, Some("i2c nack"));
    assert_eq!(down.detect_to_trigger_us, None);
    assert!(outs.iter().any(|o| o.event == Some(Event::Up)));
}

#[test]
fn chain_onsets_feed_the_lra_model() {
    let (_, chain) = run(SINGLE);
    assert_eq!(chain.onsets(), vec![(Micros(21_500), INTENSITY)]);
}
