use ohm_core::{Event, HapticDriver, HapticEffect, Micros, Profile, RawSample, SensorInterface};
use ohm_simulator::{replay, Exhausted, FakeHaptic, RawRange, Scenario, Segment, SimSensor, Trigger};

fn load(json: &str) -> Scenario {
    Scenario::from_json(json).unwrap()
}

fn single_press() -> Scenario {
    load(include_str!("../scenarios/single_press.json"))
}

fn samples(scenario: Scenario) -> Vec<RawSample> {
    let mut sensor = SimSensor::new(scenario);
    std::iter::from_fn(|| sensor.read().ok()).collect()
}

fn kinds_with(scenario: Scenario, profile: Profile) -> Vec<Event> {
    replay(scenario, profile).into_iter().map(|(_, e)| e).collect()
}

fn kinds(scenario: Scenario) -> Vec<Event> {
    kinds_with(scenario, Profile::default())
}

fn rapid(window_us: u32) -> Profile {
    Profile { rapid_trigger_window_us: window_us, ..Profile::default() }
}

#[test]
fn single_press_emits_down_then_up_at_exact_times() {
    let events = replay(single_press(), Profile::default());
    assert_eq!(events, vec![(Micros(20_000), Event::Down), (Micros(52_000), Event::Up)]);
}

#[test]
fn double_press_fast_second_down_is_rapid() {
    let scenario = load(include_str!("../scenarios/double_press_fast.json"));
    assert_eq!(
        replay(scenario, rapid(20_000)),
        vec![
            (Micros(10_000), Event::Down),
            (Micros(21_000), Event::Up),
            (Micros(35_000), Event::RapidDown),
            (Micros(46_000), Event::Up),
        ]
    );
}

#[test]
fn double_press_fast_with_window_zero_has_no_rapid() {
    let scenario = load(include_str!("../scenarios/double_press_fast.json"));
    assert_eq!(kinds(scenario), vec![Event::Down, Event::Up, Event::Down, Event::Up]);
}

#[test]
fn double_press_slow_second_down_after_window_is_plain() {
    let scenario = load(include_str!("../scenarios/double_press_slow.json"));
    assert_eq!(kinds_with(scenario, rapid(20_000)), vec![Event::Down, Event::Up, Event::Down, Event::Up]);
}

#[test]
fn rapid_window_upper_bound_is_exclusive() {
    let scenario = load(include_str!("../scenarios/double_press_edge.json"));
    let events = replay(scenario.clone(), rapid(20_000));
    assert_eq!(events[2].0 .0 - events[1].0 .0, 20_000);

    let second = |window_us| kinds_with(scenario.clone(), rapid(window_us))[2];
    assert_eq!(second(19_999), Event::Down);
    assert_eq!(second(20_000), Event::Down);
    assert_eq!(second(20_001), Event::RapidDown);
}

#[test]
fn long_hold_emits_single_down_and_single_up() {
    let scenario = load(include_str!("../scenarios/long_hold.json"));
    assert_eq!(kinds_with(scenario, rapid(2_000_000)), vec![Event::Down, Event::Up]);
}

#[test]
fn noisy_hold_below_actuation_emits_nothing() {
    let scenario = load(include_str!("../scenarios/noisy_hold.json"));
    assert_eq!(replay(scenario, Profile::default()), vec![]);
}

#[test]
fn noise_inside_hysteresis_band_after_down_does_not_release() {
    let scenario = Scenario {
        name: "noisy_pressed".into(),
        sample_period_us: 1000,
        seed: 7,
        noise_amp: 20,
        drift_per_s: 0.0,
        raw_range: RawRange::default(),
        segments: vec![
            Segment::Ramp { from: 0, to: 520, duration_us: 20_000 },
            Segment::Hold { pos: 520, duration_us: 200_000 },
        ],
    };
    assert_eq!(kinds(scenario), vec![Event::Down]);
}

#[test]
fn same_seed_gives_identical_samples() {
    let mut scenario = load(include_str!("../scenarios/noisy_hold.json"));
    let a = samples(scenario.clone());
    assert_eq!(a, samples(scenario.clone()));
    scenario.seed += 1;
    assert_ne!(a, samples(scenario));
}

#[test]
fn sensor_reports_exhausted_after_scenario_ends() {
    let mut sensor = SimSensor::new(single_press());
    for _ in 0..70 {
        sensor.read().unwrap();
    }
    assert_eq!(sensor.read(), Err(Exhausted));
}

#[test]
fn raw_follows_the_range_and_noise_is_scaled_into_raw_units() {
    let mut scenario = single_press();
    let RawRange { rest, full } = scenario.raw_range;
    assert_eq!((rest, full), (30_000, 60_000));
    let raws: Vec<u32> = samples(scenario.clone()).iter().map(|s| s.raw).collect();
    assert_eq!((raws[0], raws[30]), (rest, full));
    assert_eq!(raws[20], 45_000);

    scenario.noise_amp = 300;
    let slack = 300 * (full - rest) / 1000;
    assert!(samples(scenario).iter().all(|s| (rest - slack..=full + slack).contains(&s.raw)));
}

#[test]
fn raw_saturates_at_u32_bounds() {
    let mut scenario = single_press();
    scenario.raw_range = RawRange { rest: 0, full: u32::MAX };
    scenario.noise_amp = 1000;
    let raws: Vec<u32> = samples(scenario).iter().map(|s| s.raw).collect();
    assert_eq!((raws.iter().min(), raws.iter().max()), (Some(&0), Some(&u32::MAX)));
}

#[test]
fn raw_range_is_optional_in_json_and_validated() {
    assert_eq!(single_press().raw_range, RawRange { rest: 30_000, full: 60_000 });
    let json = include_str!("../scenarios/single_press.json").replace(
        "\"noise_amp\"",
        "\"raw_range\": { \"rest\": 5, \"full\": 5 }, \"noise_amp\"",
    );
    assert!(Scenario::from_json(&json).is_err());
}

#[test]
fn events_do_not_depend_on_raw_range() {
    let wide = [
        RawRange { rest: 1_000_000, full: 9_000_000 },
        RawRange { rest: 123, full: 0xFF_FFFF },
        RawRange { rest: 0, full: u32::MAX },
    ];
    let noisy = Scenario {
        name: "noisy_pressed".into(),
        sample_period_us: 1000,
        seed: 7,
        noise_amp: 20,
        drift_per_s: 3.0,
        raw_range: RawRange::default(),
        segments: vec![
            Segment::Ramp { from: 0, to: 520, duration_us: 20_000 },
            Segment::Hold { pos: 520, duration_us: 100_000 },
            Segment::Ramp { from: 520, to: 0, duration_us: 20_000 },
        ],
    };
    let scenarios = [
        single_press(),
        load(include_str!("../scenarios/double_press_fast.json")),
        load(include_str!("../scenarios/noisy_hold.json")),
        noisy,
    ];
    for scenario in scenarios {
        let expected = replay(scenario.clone(), rapid(20_000));
        for raw_range in wide {
            let got = replay(Scenario { raw_range, ..scenario.clone() }, rapid(20_000));
            assert_eq!(got, expected, "{} with {raw_range:?}", scenario.name);
        }
    }
}

#[test]
fn fake_haptic_records_triggers() {
    let mut haptic = FakeHaptic::default();
    haptic.trigger(HapticEffect::Click, 128, Micros(20_000)).unwrap();
    assert_eq!(haptic.triggers, vec![Trigger { at: Micros(20_000), effect: HapticEffect::Click, intensity: 128 }]);
}
