use ohm_core::Micros;
use std::f64::consts::TAU;

/// Parámetros MODELADOS de un LRA. Todos son suposiciones de datasheet sin verificar:
/// ninguno se ha medido en un actuador real y deben sustituirse por medidas de hardware.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LraParams {
    pub freq_hz: f32,
    pub rise_tau_s: f32,
    pub decay_tau_s: f32,
}

impl LraParams {
    pub const MODELLED: Self = Self { freq_hz: 180.0, rise_tau_s: 0.050, decay_tau_s: 0.080 };
}

impl Default for LraParams {
    fn default() -> Self {
        Self::MODELLED
    }
}

/// Convierte disparos `(instante de inicio, intensidad)` en una forma de onda determinista:
/// senoide de `freq_hz` con envolvente `e^(-t/decay) - e^(-t/rise)` normalizada a pico 1
/// y escalada por `intensidad / 255`. Los disparos solapados se suman.
#[derive(Clone, Copy, Debug, Default)]
pub struct LraModel {
    pub params: LraParams,
}

impl LraModel {
    pub fn render(&self, onsets: &[(Micros, u8)], sample_rate_hz: u32, duration_us: u64) -> Vec<f32> {
        let LraParams { freq_hz, rise_tau_s, decay_tau_s } = self.params;
        let (rise, decay, freq) = (f64::from(rise_tau_s), f64::from(decay_tau_s), f64::from(freq_hz));
        let envelope = |t: f64| (-t / decay).exp() - (-t / rise).exp();
        let t_peak = rise * decay / (decay - rise) * (decay / rise).ln();
        let peak = envelope(t_peak);
        let n = duration_us * u64::from(sample_rate_hz) / 1_000_000;
        (0..n)
            .map(|i| {
                let t_us = i * 1_000_000 / u64::from(sample_rate_hz);
                onsets
                    .iter()
                    .filter(|(t0, _)| t_us >= t0.0)
                    .map(|&(t0, intensity)| {
                        let dt = (t_us - t0.0) as f64 / 1e6;
                        f64::from(intensity) / 255.0 * envelope(dt) / peak * (TAU * freq * dt).sin()
                    })
                    .sum::<f64>() as f32
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 48_000;
    const ONSET_US: u64 = 10_000;

    fn one(intensity: u8) -> Vec<f32> {
        LraModel::default().render(&[(Micros(ONSET_US), intensity)], RATE, 400_000)
    }

    fn onset_index() -> usize {
        (ONSET_US * u64::from(RATE) / 1_000_000) as usize
    }

    fn peak(w: &[f32]) -> f32 {
        w.iter().fold(0.0, |m, x| m.max(x.abs()))
    }

    #[test]
    fn peak_frequency_is_about_180_hz() {
        let w = one(255);
        let signs: Vec<bool> = w[onset_index()..].iter().filter(|x| **x != 0.0).map(|x| *x > 0.0).collect();
        let crossings = signs.windows(2).filter(|p| p[0] != p[1]).count();
        let freq = crossings as f64 / 2.0 / 0.39;
        assert!((freq - 180.0).abs() < 3.0, "{freq}");
    }

    #[test]
    fn amplitude_scales_with_intensity() {
        let ratio = peak(&one(128)) / peak(&one(255));
        assert!((ratio - 128.0 / 255.0).abs() < 1e-3, "{ratio}");
        assert!((peak(&one(255)) - 1.0).abs() < 0.05);
        assert_eq!(peak(&one(0)), 0.0);
    }

    #[test]
    fn no_output_before_the_trigger() {
        assert!(one(255)[..onset_index()].iter().all(|x| *x == 0.0));
        assert!(one(255)[onset_index() + 100..][..2000].iter().any(|x| *x != 0.0));
    }

    #[test]
    fn rise_then_decay_envelope_peaks_between_the_two_time_constants() {
        let w = one(255);
        let i = w.iter().position(|x| x.abs() == peak(&w)).unwrap();
        let t = (i - onset_index()) as f64 / f64::from(RATE);
        assert!((0.02..0.12).contains(&t), "{t}");
    }
}
