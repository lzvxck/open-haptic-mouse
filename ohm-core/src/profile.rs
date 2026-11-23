use crate::Position;

/// Parámetros ajustables por perfil.
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
    HysteresisExceedsActuation,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            actuation: Position::saturating(500),
            release_hysteresis: Position::saturating(50),
            rapid_trigger_window_us: 0,
            haptic_intensity: 128,
        }
    }
}

impl Profile {
    /// El rango de `actuation` y `release_hysteresis` lo garantiza el tipo `Position`.
    pub fn validate(&self) -> Result<(), ProfileError> {
        if self.release_hysteresis > self.actuation {
            return Err(ProfileError::HysteresisExceedsActuation);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_valid() {
        assert_eq!(Profile::default().validate(), Ok(()));
    }

    #[test]
    fn hysteresis_equal_to_actuation_is_valid() {
        let p = Profile { release_hysteresis: Position::saturating(500), ..Profile::default() };
        assert_eq!(p.validate(), Ok(()));
    }

    #[test]
    fn hysteresis_above_actuation_is_rejected() {
        let p = Profile { release_hysteresis: Position::saturating(501), ..Profile::default() };
        assert_eq!(p.validate(), Err(ProfileError::HysteresisExceedsActuation));
    }
}
