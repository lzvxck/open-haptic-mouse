use ohm_core::{Position, Profile, ProfileError};
use ohm_simulator::{trace, Scenario, Trace};
use serde::{Deserialize, Serialize};

const SCENARIOS: [(&str, &str); 6] = [
    ("single_press", include_str!("../../../ohm-simulator/scenarios/single_press.json")),
    ("double_press_fast", include_str!("../../../ohm-simulator/scenarios/double_press_fast.json")),
    ("double_press_slow", include_str!("../../../ohm-simulator/scenarios/double_press_slow.json")),
    ("double_press_edge", include_str!("../../../ohm-simulator/scenarios/double_press_edge.json")),
    ("noisy_hold", include_str!("../../../ohm-simulator/scenarios/noisy_hold.json")),
    ("long_hold", include_str!("../../../ohm-simulator/scenarios/long_hold.json")),
];

pub type RunResult = Trace;

const UI_RAPID_TRIGGER_WINDOW_MS: u32 = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileDto {
    pub actuation: u16,
    pub release_hysteresis: u16,
    pub rapid_trigger_window_ms: u32,
    pub haptic_intensity: u8,
}

impl From<Profile> for ProfileDto {
    fn from(p: Profile) -> Self {
        Self {
            actuation: p.actuation.get(),
            release_hysteresis: p.release_hysteresis.get(),
            rapid_trigger_window_ms: p.rapid_trigger_window_us / 1000,
            haptic_intensity: p.haptic_intensity,
        }
    }
}

fn position(name: &str, value: u16) -> Result<Position, String> {
    if value > Position::MAX {
        return Err(format!("{name} must be between 0 and {}, got {value}", Position::MAX));
    }
    Ok(Position::saturating(value))
}

impl TryFrom<ProfileDto> for Profile {
    type Error = String;

    fn try_from(dto: ProfileDto) -> Result<Self, String> {
        let rapid_trigger_window_us = dto
            .rapid_trigger_window_ms
            .checked_mul(1000)
            .ok_or_else(|| format!("rapid trigger window of {} ms is too large", dto.rapid_trigger_window_ms))?;
        let profile = Profile {
            actuation: position("actuation", dto.actuation)?,
            release_hysteresis: position("release hysteresis", dto.release_hysteresis)?,
            rapid_trigger_window_us,
            haptic_intensity: dto.haptic_intensity,
        };
        profile.validate().map_err(|e| match e {
            ProfileError::ZeroActuation => "actuation must be at least 1".to_owned(),
            ProfileError::HysteresisNotBelowActuation => {
                format!("release hysteresis ({}) must be below actuation ({})", dto.release_hysteresis, dto.actuation)
            }
        })?;
        Ok(profile)
    }
}

pub fn list_scenarios() -> Vec<String> {
    SCENARIOS.iter().map(|(name, _)| (*name).to_owned()).collect()
}

pub fn default_profile() -> ProfileDto {
    ProfileDto { rapid_trigger_window_ms: UI_RAPID_TRIGGER_WINDOW_MS, ..Profile::default().into() }
}

pub fn run_scenario(name: &str, profile: ProfileDto) -> Result<RunResult, String> {
    let json = SCENARIOS
        .iter()
        .find_map(|(n, json)| (*n == name).then_some(*json))
        .ok_or_else(|| format!("unknown scenario \"{name}\""))?;
    let scenario = Scenario::from_json(json).map_err(|e| format!("scenario \"{name}\" is invalid: {e}"))?;
    Ok(trace(scenario, Profile::try_from(profile)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ohm_simulator::TraceEvent;

    fn dto() -> ProfileDto {
        default_profile()
    }

    #[test]
    fn lists_every_embedded_scenario_and_each_one_runs() {
        let names = list_scenarios();
        assert_eq!(names.len(), 6);
        for name in &names {
            let result = run_scenario(name, dto()).unwrap();
            assert_eq!(&result.scenario, name);
            assert!(!result.steps.is_empty());
        }
    }

    #[test]
    fn default_profile_is_core_default_with_the_ui_rapid_window() {
        assert_eq!(
            dto(),
            ProfileDto { actuation: 500, release_hysteresis: 50, rapid_trigger_window_ms: 20, haptic_intensity: 128 }
        );
    }

    #[test]
    fn run_applies_the_profile_it_was_given() {
        let profile = ProfileDto { actuation: 300, release_hysteresis: 100, haptic_intensity: 200, ..dto() };
        let result = run_scenario("single_press", profile).unwrap();
        assert_eq!((result.actuation, result.release), (300, 200));
        assert_eq!(result.triggers[0].intensity, 200);
        let down = result.steps.iter().find(|s| s.event == Some(TraceEvent::Down)).unwrap();
        assert!(down.pos >= 300);
    }

    #[test]
    fn window_in_milliseconds_reaches_rapid_trigger() {
        let result = run_scenario("double_press_fast", dto()).unwrap();
        assert!(result.steps.iter().any(|s| s.event == Some(TraceEvent::RapidDown)));
        let none = run_scenario("double_press_fast", ProfileDto { rapid_trigger_window_ms: 0, ..dto() }).unwrap();
        assert!(none.steps.iter().all(|s| s.event != Some(TraceEvent::RapidDown)));
    }

    #[test]
    fn hysteresis_at_or_above_actuation_is_a_readable_error() {
        let err =
            run_scenario("single_press", ProfileDto { actuation: 100, release_hysteresis: 150, ..dto() }).unwrap_err();
        assert_eq!(err, "release hysteresis (150) must be below actuation (100)");
        let err =
            run_scenario("single_press", ProfileDto { actuation: 100, release_hysteresis: 100, ..dto() }).unwrap_err();
        assert_eq!(err, "release hysteresis (100) must be below actuation (100)");
    }

    #[test]
    fn zero_actuation_is_a_readable_error() {
        let err = run_scenario("single_press", ProfileDto { actuation: 0, release_hysteresis: 0, ..dto() }).unwrap_err();
        assert_eq!(err, "actuation must be at least 1");
    }

    #[test]
    fn out_of_range_values_are_rejected_not_clamped() {
        let err = run_scenario("single_press", ProfileDto { actuation: 1001, ..dto() }).unwrap_err();
        assert_eq!(err, "actuation must be between 0 and 1000, got 1001");
        let err =
            run_scenario("single_press", ProfileDto { rapid_trigger_window_ms: u32::MAX, ..dto() }).unwrap_err();
        assert!(err.contains("too large"), "{err}");
    }

    #[test]
    fn unknown_scenario_is_an_error() {
        assert_eq!(run_scenario("nope", dto()).unwrap_err(), "unknown scenario \"nope\"");
    }

    #[test]
    fn dto_and_result_cross_the_json_boundary_with_snake_case_fields() {
        let profile: ProfileDto = serde_json::from_str(
            r#"{"actuation":400,"release_hysteresis":40,"rapid_trigger_window_ms":5,"haptic_intensity":255}"#,
        )
        .unwrap();
        let json = serde_json::to_value(run_scenario("single_press", profile).unwrap()).unwrap();
        assert_eq!(json["actuation"], 400);
        assert_eq!(json["steps"][0]["t_us"], 0);
        assert!(json["waveform"].is_array());
    }
}
