use core::convert::Infallible;

use crate::{Micros, RawSample, SensorInterface};

/// Reproduce en bucle cuentas crudas con un periodo fijo, para correr el pipeline sin ADC.
/// El tiempo es virtual: la primera lectura es `t = 0` y cada una suma `period_us`.
#[derive(Clone, Debug)]
pub struct ReplaySensor<'a> {
    samples: &'a [u32],
    period_us: u64,
    next: usize,
    t: u64,
}

impl<'a> ReplaySensor<'a> {
    /// # Panics
    /// Si `samples` está vacío.
    pub fn new(samples: &'a [u32], period_us: u32) -> Self {
        assert!(!samples.is_empty(), "ReplaySensor needs at least one sample");
        Self { samples, period_us: u64::from(period_us), next: 0, t: 0 }
    }
}

impl SensorInterface for ReplaySensor<'_> {
    type Error = Infallible;

    fn read(&mut self) -> Result<RawSample, Infallible> {
        let sample = RawSample { t: Micros(self.t), raw: self.samples[self.next] };
        self.next = (self.next + 1) % self.samples.len();
        self.t += self.period_us;
        Ok(sample)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loops_the_slice_and_advances_time_by_the_period() {
        let mut s = ReplaySensor::new(&[10, 20, 30], 1_000);
        let got: [(u64, u32); 7] = core::array::from_fn(|_| {
            let r = s.read().unwrap();
            (r.t.0, r.raw)
        });
        assert_eq!(
            got,
            [(0, 10), (1_000, 20), (2_000, 30), (3_000, 10), (4_000, 20), (5_000, 30), (6_000, 10)]
        );
    }

    #[test]
    #[should_panic(expected = "at least one sample")]
    fn empty_slice_is_rejected_up_front() {
        ReplaySensor::new(&[], 1_000);
    }
}
