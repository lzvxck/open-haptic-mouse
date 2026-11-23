/// Marca de tiempo en microsegundos.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Micros(pub u64);

/// Posición normalizada del botón: 0 = reposo, `Position::MAX` = fondo de recorrido.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position(u16);

impl Position {
    pub const MAX: u16 = 1000;

    pub fn saturating(raw: u16) -> Self {
        Self(if raw > Self::MAX { Self::MAX } else { raw })
    }

    pub fn get(self) -> u16 {
        self.0
    }
}

/// Lectura cruda del sensor, en cuentas del ADC: `u32` cubre las 24 bits del LDC1101 en modo L y las 16 de RP.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawSample {
    pub t: Micros,
    pub raw: u32,
}

/// Lectura ya calibrada: salida del pipeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sample {
    pub t: Micros,
    pub pos: Position,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saturating_clamps_to_max() {
        assert_eq!(Position::saturating(0).get(), 0);
        assert_eq!(Position::saturating(1000).get(), 1000);
        assert_eq!(Position::saturating(1001).get(), 1000);
        assert_eq!(Position::saturating(u16::MAX).get(), 1000);
    }
}
