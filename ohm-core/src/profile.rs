use crate::Position;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Profile {
    /// Posición a partir de la cual se considera pulsado.
    pub actuation: Position,
    /// Histéresis: se libera al bajar de `actuation - release_hysteresis`.
    pub release_hysteresis: Position,
    pub rapid_trigger_window_us: u32,
    pub haptic_intensity: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileError {
    /// `actuation == 0`: the button would read as pressed at rest.
    ZeroActuation,
    /// `release_hysteresis >= actuation`: the release threshold would be 0, which no position is below.
    HysteresisNotBelowActuation,
}

impl Default for Profile {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Profile {
    pub const DEFAULT: Self = Self {
        actuation: Position::saturating(500),
        release_hysteresis: Position::saturating(50),
        rapid_trigger_window_us: 0,
        haptic_intensity: 128,
    };

    /// Accepts exactly the profiles whose button presses only above rest and always releases at rest.
    pub fn validate(&self) -> Result<(), ProfileError> {
        if self.actuation.get() == 0 {
            return Err(ProfileError::ZeroActuation);
        }
        if self.release_hysteresis >= self.actuation {
            return Err(ProfileError::HysteresisNotBelowActuation);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(actuation: u16, hysteresis: u16) -> Profile {
        Profile {
            actuation: Position::saturating(actuation),
            release_hysteresis: Position::saturating(hysteresis),
            ..Profile::default()
        }
    }

    #[test]
    fn default_is_valid() {
        assert_eq!(Profile::default().validate(), Ok(()));
    }

    #[test]
    fn hysteresis_just_below_actuation_is_valid() {
        assert_eq!(profile(500, 499).validate(), Ok(()));
        assert_eq!(profile(1, 0).validate(), Ok(()));
    }

    #[test]
    fn hysteresis_equal_to_actuation_is_rejected() {
        assert_eq!(profile(500, 500).validate(), Err(ProfileError::HysteresisNotBelowActuation));
    }

    #[test]
    fn hysteresis_above_actuation_is_rejected() {
        assert_eq!(profile(500, 501).validate(), Err(ProfileError::HysteresisNotBelowActuation));
    }

    #[test]
    fn zero_actuation_is_rejected() {
        assert_eq!(profile(0, 0).validate(), Err(ProfileError::ZeroActuation));
    }
}
