use ohm_simulator::{Scenario, ScenarioError, Segment, SimSensor, MAX_DURATION_US, MAX_SAMPLES};

fn json(sample_period_us: u64, hold_durations_us: &[u64]) -> String {
    let segments: Vec<String> = hold_durations_us
        .iter()
        .map(|d| format!(r#"{{ "type": "hold", "pos": 0, "duration_us": {d} }}"#))
        .collect();
    format!(
        r#"{{ "name": "t", "sample_period_us": {sample_period_us}, "seed": 1, "noise_amp": 0, "drift_per_s": 0.0, "segments": [{}] }}"#,
        segments.join(",")
    )
}

#[test]
fn zero_sample_period_is_rejected() {
    assert!(matches!(Scenario::from_json(&json(0, &[10_000])), Err(ScenarioError::ZeroSamplePeriod)));
}

#[test]
fn durations_that_overflow_u64_are_rejected() {
    assert!(matches!(Scenario::from_json(&json(1_000, &[u64::MAX, 1])), Err(ScenarioError::DurationOverflow)));
}

#[test]
fn more_samples_than_the_cap_are_rejected() {
    assert!(matches!(
        Scenario::from_json(&json(1, &[MAX_SAMPLES + 1])),
        Err(ScenarioError::TooManySamples { samples }) if samples == MAX_SAMPLES + 1
    ));
}

#[test]
fn durations_beyond_the_cap_are_rejected() {
    assert!(matches!(
        Scenario::from_json(&json(1_000_000, &[MAX_DURATION_US + 1])),
        Err(ScenarioError::DurationTooLong { .. })
    ));
}

#[test]
fn a_scenario_at_the_sample_cap_is_accepted() {
    assert!(Scenario::from_json(&json(1, &[MAX_SAMPLES])).is_ok());
}

#[test]
#[should_panic(expected = "Scenario inválido")]
fn sensor_refuses_a_zero_period_scenario_built_in_code() {
    let scenario = Scenario {
        name: "in_code".into(),
        sample_period_us: 0,
        seed: 1,
        noise_amp: 0,
        drift_per_s: 0.0,
        raw_range: Default::default(),
        segments: vec![Segment::Hold { pos: 0, duration_us: 10_000 }],
    };
    SimSensor::new(scenario);
}
