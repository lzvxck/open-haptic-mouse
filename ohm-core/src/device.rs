use crate::{Button, Calibration, Event, Filter, HapticDriver, HapticEffect, Profile, Sample, SensorInterface};

/// Resultado de un paso. Un fallo háptico no descarta el evento: ambos viajan juntos y el
/// llamador decide qué hacer con `haptic_fault`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepOutput<HE> {
    pub raw: u32,
    /// Muestra normalizada y filtrada.
    pub sample: Sample,
    pub event: Option<Event>,
    /// Desde la muestra que cruzó el umbral hasta el instante en que el driver dice que el efecto empieza.
    pub detect_to_trigger_us: Option<u32>,
    pub haptic_fault: Option<HE>,
}

/// Consecutive failed reads after which a held button is released: 50 ms at the firmware's 1 kHz rate.
pub const FAILSAFE_READ_ERRORS: u32 = 50;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SensorFault<E> {
    pub error: E,
    /// Failed reads in a row, this one included.
    pub consecutive: u32,
    /// This read reached [`FAILSAFE_READ_ERRORS`] while the button was held, so the device released it:
    /// handle it as an [`Event::Up`].
    pub released: bool,
}

pub struct Device<S, H> {
    sensor: S,
    haptic: H,
    calibration: Calibration,
    filter: Filter,
    button: Button,
    profile: Profile,
    read_errors: u32,
}

impl<S: SensorInterface, H: HapticDriver> Device<S, H> {
    pub fn new(sensor: S, haptic: H, calibration: Calibration, filter: Filter, profile: Profile) -> Self {
        Self { sensor, haptic, calibration, filter, button: Button::new(profile), profile, read_errors: 0 }
    }

    pub fn into_haptic(self) -> H {
        self.haptic
    }

    pub fn haptic_mut(&mut self) -> &mut H {
        &mut self.haptic
    }

    /// Aplica `profile` sin reiniciar el estado pulsado/suelto del botón. El llamador lo valida antes.
    pub fn set_profile(&mut self, profile: Profile) {
        self.profile = profile;
        self.button.set_profile(profile);
    }

    pub fn step(&mut self) -> Result<StepOutput<H::Error>, SensorFault<S::Error>> {
        let raw = match self.sensor.read() {
            Ok(raw) => raw,
            Err(error) => {
                self.read_errors = self.read_errors.saturating_add(1);
                let released = self.read_errors >= FAILSAFE_READ_ERRORS && self.button.release().is_some();
                return Err(SensorFault { error, consecutive: self.read_errors, released });
            }
        };
        self.read_errors = 0;
        let normalized = self.calibration.normalize(raw.raw);
        let sample = Sample { t: raw.t, pos: self.filter.apply(normalized) };
        let event = self.button.update(sample);
        let (detect_to_trigger_us, haptic_fault) = match event {
            Some(Event::Down | Event::RapidDown) => {
                match self.haptic.trigger(HapticEffect::Click, self.profile.haptic_intensity, sample.t) {
                    Ok(started) => {
                        let us = started.0.saturating_sub(sample.t.0);
                        (Some(u32::try_from(us).unwrap_or(u32::MAX)), None)
                    }
                    Err(e) => (None, Some(e)),
                }
            }
            _ => (None, None),
        };
        Ok(StepOutput { raw: raw.raw, sample, event, detect_to_trigger_us, haptic_fault })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Micros, Position, RawSample};
    use std::vec;

    const REST: u32 = 10_000;
    const FULL: u32 = 20_000;
    const PRESSED: u32 = 16_000;

    struct Script<'a>(core::slice::Iter<'a, u32>, u64);

    impl SensorInterface for Script<'_> {
        type Error = ();
        fn read(&mut self) -> Result<RawSample, ()> {
            let raw = *self.0.next().ok_or(())?;
            self.1 += 1_000;
            Ok(RawSample { t: Micros(self.1), raw })
        }
    }

    struct Haptic {
        fail: bool,
        last: Option<(u8, Micros)>,
    }

    impl HapticDriver for Haptic {
        type Error = &'static str;
        fn trigger(&mut self, _: HapticEffect, intensity: u8, at: Micros) -> Result<Micros, &'static str> {
            if self.fail {
                return Err("i2c nack");
            }
            self.last = Some((intensity, at));
            Ok(Micros(at.0 + 700))
        }
    }

    fn device<'a>(script: &'a [u32], fail: bool) -> Device<Script<'a>, Haptic> {
        let profile = Profile { haptic_intensity: 77, ..Profile::default() };
        let cal = Calibration::new(REST, FULL).unwrap();
        Device::new(Script(script.iter(), 0), Haptic { fail, last: None }, cal, Filter::new(0), profile)
    }

    #[test]
    fn down_triggers_with_profile_intensity_and_reports_latency() {
        let mut d = device(&[REST, PRESSED, PRESSED, REST], false);
        assert_eq!(d.step().unwrap().event, None);
        let out = d.step().unwrap();
        assert_eq!(out.event, Some(Event::Down));
        assert_eq!(out.sample.pos.get(), 600);
        assert_eq!(out.detect_to_trigger_us, Some(700));
        assert_eq!(out.haptic_fault, None);
        assert_eq!(d.into_haptic().last, Some((77, Micros(2_000))));
    }

    #[test]
    fn up_does_not_trigger() {
        let mut d = device(&[PRESSED, REST], false);
        d.step().unwrap();
        let out = d.step().unwrap();
        assert_eq!(out.event, Some(Event::Up));
        assert_eq!(out.detect_to_trigger_us, None);
    }

    #[test]
    fn haptic_fault_is_reported_beside_the_event_not_instead_of_it() {
        let mut d = device(&[PRESSED], true);
        let out = d.step().unwrap();
        assert_eq!(out.event, Some(Event::Down));
        assert_eq!(out.haptic_fault, Some("i2c nack"));
        assert_eq!(out.detect_to_trigger_us, None);
    }

    #[test]
    fn set_profile_keeps_a_held_button_pressed() {
        let mut d = device(&[PRESSED, PRESSED, PRESSED], false);
        assert_eq!(d.step().unwrap().event, Some(Event::Down));
        d.set_profile(Profile { actuation: Position::saturating(550), ..Profile::default() });
        assert_eq!(d.step().unwrap().event, None, "no second Down");
        d.set_profile(Profile { actuation: Position::saturating(800), ..Profile::default() });
        assert_eq!(d.step().unwrap().event, Some(Event::Up), "release threshold is now 750");
    }

    #[test]
    fn set_profile_applies_new_actuation_and_intensity_to_next_press() {
        let mut d = device(&[REST, PRESSED, REST, PRESSED], false);
        d.set_profile(Profile { actuation: Position::saturating(700), haptic_intensity: 9, ..Profile::default() });
        assert_eq!(d.step().unwrap().event, None);
        assert_eq!(d.step().unwrap().event, None, "600 < new actuation 700");
        d.set_profile(Profile { haptic_intensity: 33, ..Profile::default() });
        d.step().unwrap();
        assert_eq!(d.step().unwrap().event, Some(Event::Down));
        assert_eq!(d.into_haptic().last.map(|l| l.0), Some(33));
    }

    #[test]
    fn step_reports_raw_counts() {
        assert_eq!(device(&[PRESSED], false).step().unwrap().raw, PRESSED);
    }

    #[test]
    fn sensor_error_propagates() {
        assert_eq!(device(&[], false).step().err(), Some(SensorFault { error: (), consecutive: 1, released: false }));
    }

    struct Flaky<'a>(core::slice::Iter<'a, Option<u32>>, u64);

    impl SensorInterface for Flaky<'_> {
        type Error = ();
        fn read(&mut self) -> Result<RawSample, ()> {
            self.1 += 1_000;
            let raw = self.0.next().copied().flatten().ok_or(())?;
            Ok(RawSample { t: Micros(self.1), raw })
        }
    }

    fn flaky(script: &[Option<u32>]) -> Device<Flaky<'_>, Haptic> {
        let cal = Calibration::new(REST, FULL).unwrap();
        Device::new(Flaky(script.iter(), 0), Haptic { fail: false, last: None }, cal, Filter::new(0), Profile::default())
    }

    fn released_at(d: &mut Device<Flaky<'_>, Haptic>, reads: u32) -> Option<u32> {
        (1..=reads).find(|_| d.step().is_err_and(|f| f.released))
    }

    #[test]
    fn sensor_failing_while_held_releases_once_after_the_failsafe_count() {
        let n = FAILSAFE_READ_ERRORS as usize;
        let mut script = vec![Some(PRESSED)];
        script.extend(core::iter::repeat_n(None, 3 * n));
        let mut d = flaky(&script);
        assert_eq!(d.step().unwrap().event, Some(Event::Down));
        assert_eq!(released_at(&mut d, FAILSAFE_READ_ERRORS), Some(FAILSAFE_READ_ERRORS));
        assert_eq!(released_at(&mut d, 2 * FAILSAFE_READ_ERRORS), None, "released once, stays up");
    }

    #[test]
    fn a_good_sample_resets_the_failsafe_count() {
        let n = FAILSAFE_READ_ERRORS as usize;
        let mut script = vec![Some(PRESSED)];
        script.extend(core::iter::repeat_n(None, n - 1));
        script.push(Some(PRESSED));
        script.extend(core::iter::repeat_n(None, n - 1));
        let mut d = flaky(&script);
        assert_eq!(d.step().unwrap().event, Some(Event::Down));
        assert_eq!(released_at(&mut d, FAILSAFE_READ_ERRORS - 1), None);
        assert_eq!(d.step().unwrap().event, None, "still held");
        assert_eq!(released_at(&mut d, FAILSAFE_READ_ERRORS - 1), None);
    }

    #[test]
    fn sensor_failing_while_up_never_releases() {
        let script = vec![None; 3 * FAILSAFE_READ_ERRORS as usize];
        assert_eq!(released_at(&mut flaky(&script), 3 * FAILSAFE_READ_ERRORS), None);
    }

    #[test]
    fn after_a_failsafe_release_a_still_held_button_needs_a_new_press() {
        let mut script = vec![Some(PRESSED)];
        script.extend(core::iter::repeat_n(None, FAILSAFE_READ_ERRORS as usize));
        script.extend([Some(PRESSED), Some(REST), Some(PRESSED)]);
        let mut d = flaky(&script);
        d.step().unwrap();
        assert_eq!(released_at(&mut d, FAILSAFE_READ_ERRORS), Some(FAILSAFE_READ_ERRORS));
        assert_eq!(d.step().unwrap().event, None, "recovered while held: no phantom Down");
        assert_eq!(d.step().unwrap().event, None);
        assert_eq!(d.step().unwrap().event, Some(Event::Down));
    }
}
