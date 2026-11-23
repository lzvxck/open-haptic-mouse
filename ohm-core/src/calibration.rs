use crate::Position;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalibrationError {
    FullNotAboveRest,
}

/// Mapea lecturas crudas del sensor a [`Position`]: `rest` es 0, `full` es `Position::MAX`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Calibration {
    rest: u32,
    full: u32,
}

impl Calibration {
    pub fn new(rest: u32, full: u32) -> Result<Self, CalibrationError> {
        if full <= rest {
            return Err(CalibrationError::FullNotAboveRest);
        }
        Ok(Self { rest, full })
    }

    pub fn normalize(&self, raw: u32) -> Position {
        if raw <= self.rest {
            return Position::saturating(0);
        }
        if raw >= self.full {
            return Position::saturating(Position::MAX);
        }
        let travel = u64::from(raw - self.rest) * u64::from(Position::MAX);
        Position::saturating((travel / u64::from(self.full - self.rest)) as u16)
    }
}

/// Promedia `N` lecturas en reposo para obtener el offset de reposo.
#[derive(Clone, Copy, Debug)]
pub struct Calibrator<const N: usize> {
    sum: u64,
    count: usize,
}

impl<const N: usize> Calibrator<N> {
    pub fn new() -> Self {
        const { assert!(N > 0) };
        Self { sum: 0, count: 0 }
    }

    /// Devuelve el promedio al completar `N` muestras y reinicia el acumulador.
    pub fn push(&mut self, raw: u32) -> Option<u32> {
        self.sum += u64::from(raw);
        self.count += 1;
        if self.count < N {
            return None;
        }
        let mean = (self.sum / N as u64) as u32;
        *self = Self::new();
        Some(mean)
    }
}

impl<const N: usize> Default for Calibrator<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const U24_MAX: u32 = 0xFF_FFFF;

    fn cal(rest: u32, full: u32) -> Calibration {
        Calibration::new(rest, full).unwrap()
    }

    #[test]
    fn full_not_above_rest_is_rejected() {
        assert_eq!(Calibration::new(100, 100), Err(CalibrationError::FullNotAboveRest));
        assert_eq!(Calibration::new(100, 99), Err(CalibrationError::FullNotAboveRest));
        assert_eq!(Calibration::new(u32::MAX, u32::MAX), Err(CalibrationError::FullNotAboveRest));
    }

    #[test]
    fn rest_and_full_map_to_range_ends() {
        let c = cal(1000, 3000);
        assert_eq!(c.normalize(1000).get(), 0);
        assert_eq!(c.normalize(3000).get(), Position::MAX);
        assert_eq!(c.normalize(2000).get(), 500);
    }

    #[test]
    fn readings_outside_range_clamp() {
        let c = cal(1000, 3000);
        assert_eq!(c.normalize(0).get(), 0);
        assert_eq!(c.normalize(999).get(), 0);
        assert_eq!(c.normalize(3001).get(), Position::MAX);
        assert_eq!(c.normalize(u32::MAX).get(), Position::MAX);
    }

    #[test]
    fn rest_zero_and_full_u32_max_do_not_overflow() {
        let c = cal(0, u32::MAX);
        assert_eq!(c.normalize(0).get(), 0);
        assert_eq!(c.normalize(u32::MAX / 2).get(), 499);
        assert_eq!(c.normalize(u32::MAX - 1).get(), Position::MAX - 1);
        assert_eq!(c.normalize(u32::MAX).get(), Position::MAX);
    }

    #[test]
    fn narrowest_range_is_a_step() {
        let c = cal(u32::MAX - 1, u32::MAX);
        assert_eq!(c.normalize(u32::MAX - 1).get(), 0);
        assert_eq!(c.normalize(u32::MAX).get(), Position::MAX);
    }

    #[test]
    fn full_24_bit_range_maps_linearly() {
        let c = cal(0, U24_MAX);
        assert_eq!(c.normalize(0).get(), 0);
        assert_eq!(c.normalize(U24_MAX / 2).get(), 499);
        assert_eq!(c.normalize(U24_MAX).get(), Position::MAX);
        assert_eq!(c.normalize(U24_MAX + 1).get(), Position::MAX);
    }

    #[test]
    fn offset_24_bit_range_maps_to_range_ends() {
        let c = cal(1_000_000, 9_000_000);
        assert_eq!(c.normalize(1_000_000).get(), 0);
        assert_eq!(c.normalize(5_000_000).get(), 500);
        assert_eq!(c.normalize(9_000_000).get(), Position::MAX);
        assert_eq!(c.normalize(999_999).get(), 0);
    }

    #[test]
    fn normalize_is_monotonic() {
        let c = cal(123, 4_000_000_000);
        let mut prev = 0;
        for raw in (0..=u32::MAX).step_by(1_000_003) {
            let p = c.normalize(raw).get();
            assert!(p >= prev);
            prev = p;
        }
    }

    #[test]
    fn calibrator_yields_mean_after_n_samples() {
        let mut c = Calibrator::<4>::new();
        assert_eq!(c.push(100), None);
        assert_eq!(c.push(102), None);
        assert_eq!(c.push(98), None);
        assert_eq!(c.push(100), Some(100));
    }

    #[test]
    fn calibrator_handles_u32_max_samples() {
        let mut c = Calibrator::<1000>::new();
        for _ in 0..999 {
            assert_eq!(c.push(u32::MAX), None);
        }
        assert_eq!(c.push(u32::MAX), Some(u32::MAX));
    }

    #[test]
    fn calibrator_means_24_bit_samples_above_u16() {
        let mut c = Calibrator::<2>::new();
        c.push(U24_MAX);
        assert_eq!(c.push(U24_MAX - 2), Some(U24_MAX - 1));
    }

    #[test]
    fn calibrator_restarts_after_yielding() {
        let mut c = Calibrator::<2>::new();
        c.push(10);
        assert_eq!(c.push(20), Some(15));
        c.push(100);
        assert_eq!(c.push(200), Some(150));
    }

    #[test]
    fn calibrator_zero_rest() {
        let mut c = Calibrator::<3>::new();
        c.push(0);
        c.push(0);
        assert_eq!(c.push(0), Some(0));
    }
}
