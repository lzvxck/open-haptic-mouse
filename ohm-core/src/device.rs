use crate::{Button, Calibration, Event, Filter, HapticDriver, HapticEffect, Profile, Sample, SensorInterface};

/// Resultado de un paso. Un fallo háptico no descarta el evento: ambos viajan juntos y el
/// llamador decide qué hacer con `haptic_fault`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepOutput<HE> {
    /// Cuentas crudas del sensor de esta muestra.
    pub raw: u32,
    /// Muestra normalizada y filtrada.
    pub sample: Sample,
    pub event: Option<Event>,
    /// Desde la muestra que cruzó el umbral hasta el instante en que el driver dice que el efecto empieza.
    pub detect_to_trigger_us: Option<u32>,
    pub haptic_fault: Option<HE>,
}

/// Cadena completa sensor → calibración → filtro → botón → háptico.
pub struct Device<S, H> {
    sensor: S,
    haptic: H,
    calibration: Calibration,
    filter: Filter,
    button: Button,
    profile: Profile,
}

impl<S: SensorInterface, H: HapticDriver> Device<S, H> {
    pub fn new(sensor: S, haptic: H, calibration: Calibration, filter: Filter, profile: Profile) -> Self {
        Self { sensor, haptic, calibration, filter, button: Button::new(profile), profile }
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

    pub fn step(&mut self) -> Result<StepOutput<H::Error>, S::Error> {
        let raw = self.sensor.read()?;
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
        assert_eq!(device(&[], false).step().err(), Some(()));
    }
}
