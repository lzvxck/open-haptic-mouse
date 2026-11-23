use crate::{Micros, Profile, Sample};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Down,
    /// Down ocurrido menos de `rapid_trigger_window_us` después del Up anterior.
    RapidDown,
    Up,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Up,
    Down,
    Disarmed,
}

#[derive(Debug)]
pub struct Button {
    profile: Profile,
    state: State,
    last_up: Option<Micros>,
}

impl Button {
    pub fn new(profile: Profile) -> Self {
        Self { profile, state: State::Up, last_up: None }
    }

    /// Cambia los umbrales sin tocar el estado pulsado/suelto ni el último Up. A released button stays
    /// released even if the new actuation is already below the current position.
    pub fn set_profile(&mut self, profile: Profile) {
        self.profile = profile;
        if self.state == State::Up {
            self.state = State::Disarmed;
        }
    }

    /// Forces a held button up without a sample, e.g. when the sensor stops answering.
    pub fn release(&mut self) -> Option<Event> {
        if self.state != State::Down {
            return None;
        }
        self.state = State::Disarmed;
        self.last_up = None;
        Some(Event::Up)
    }

    fn is_rapid(&self, now: Micros) -> bool {
        let window = u64::from(self.profile.rapid_trigger_window_us);
        self.last_up.is_some_and(|up| now.0.saturating_sub(up.0) < window)
    }

    pub fn update(&mut self, sample: Sample) -> Option<Event> {
        let pos = sample.pos;
        let release_below = self.profile.actuation.get().saturating_sub(self.profile.release_hysteresis.get());
        match self.state {
            State::Up if pos >= self.profile.actuation => {
                let rapid = self.is_rapid(sample.t);
                self.state = State::Down;
                Some(if rapid { Event::RapidDown } else { Event::Down })
            }
            State::Down if pos.get() < release_below => {
                self.state = State::Up;
                self.last_up = Some(sample.t);
                Some(Event::Up)
            }
            State::Disarmed if pos < self.profile.actuation => {
                self.state = State::Up;
                None
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Micros, Position};

    fn s(t: u64, pos: u16) -> Sample {
        Sample { t: Micros(t), pos: Position::saturating(pos) }
    }

    #[test]
    fn down_then_up_with_hysteresis() {
        let mut b = Button::new(Profile::default());
        assert_eq!(b.update(s(0, 100)), None);
        assert_eq!(b.update(s(1, 500)), Some(Event::Down));
        assert_eq!(b.update(s(2, 480)), None);
        assert_eq!(b.update(s(3, 449)), Some(Event::Up));
    }

    #[test]
    fn no_double_down() {
        let mut b = Button::new(Profile::default());
        assert_eq!(b.update(s(0, 600)), Some(Event::Down));
        assert_eq!(b.update(s(1, 900)), None);
    }

    #[test]
    fn out_of_range_sample_saturates_into_down() {
        let mut b = Button::new(Profile::default());
        assert_eq!(b.update(s(0, u16::MAX)), Some(Event::Down));
    }

    #[test]
    fn release_boundary_is_exclusive() {
        let mut b = Button::new(Profile::default());
        b.update(s(0, 600));
        assert_eq!(b.update(s(1, 450)), None);
        assert_eq!(b.update(s(2, 449)), Some(Event::Up));
    }

    fn rapid(window_us: u32) -> Profile {
        Profile { rapid_trigger_window_us: window_us, ..Profile::default() }
    }

    fn press_release_press(b: &mut Button, gap_us: u64) -> Option<Event> {
        b.update(s(0, 600));
        b.update(s(1_000, 0));
        b.update(s(1_000 + gap_us, 600))
    }

    #[test]
    fn second_down_inside_window_is_rapid() {
        let mut b = Button::new(rapid(10_000));
        assert_eq!(press_release_press(&mut b, 5_000), Some(Event::RapidDown));
    }

    #[test]
    fn second_down_after_window_is_plain_down() {
        let mut b = Button::new(rapid(10_000));
        assert_eq!(press_release_press(&mut b, 10_001), Some(Event::Down));
    }

    #[test]
    fn rapid_window_upper_bound_is_exclusive() {
        let mut b = Button::new(rapid(10_000));
        assert_eq!(press_release_press(&mut b, 9_999), Some(Event::RapidDown));
        let mut b = Button::new(rapid(10_000));
        assert_eq!(press_release_press(&mut b, 10_000), Some(Event::Down));
    }

    #[test]
    fn window_zero_never_rapid() {
        let mut b = Button::new(rapid(0));
        assert_eq!(press_release_press(&mut b, 0), Some(Event::Down));
        assert_eq!(press_release_press(&mut b, 1), Some(Event::Down));
    }

    #[test]
    fn first_down_is_never_rapid() {
        let mut b = Button::new(rapid(u32::MAX));
        assert_eq!(b.update(s(0, 600)), Some(Event::Down));
    }

    #[test]
    fn rapid_is_measured_from_the_latest_up() {
        let mut b = Button::new(rapid(10_000));
        b.update(s(0, 600));
        b.update(s(1_000, 0));
        assert_eq!(b.update(s(50_000, 600)), Some(Event::Down));
        assert_eq!(b.update(s(51_000, 0)), Some(Event::Up));
        assert_eq!(b.update(s(55_000, 600)), Some(Event::RapidDown));
    }

    #[test]
    fn set_profile_keeps_pressed_state_and_uses_new_release_threshold() {
        let mut b = Button::new(Profile::default());
        assert_eq!(b.update(s(0, 600)), Some(Event::Down));
        b.set_profile(Profile { actuation: Position::saturating(800), ..Profile::default() });
        assert_eq!(b.update(s(1, 750)), None);
        assert_eq!(b.update(s(2, 749)), Some(Event::Up));
    }

    #[test]
    fn every_valid_profile_stays_up_at_rest_and_releases_after_a_full_press() {
        let mut checked = 0u32;
        for actuation in 0..=Position::MAX {
            for hysteresis in 0..=Position::MAX {
                let profile = Profile {
                    actuation: Position::saturating(actuation),
                    release_hysteresis: Position::saturating(hysteresis),
                    ..Profile::default()
                };
                if profile.validate().is_err() {
                    continue;
                }
                checked += 1;
                let mut b = Button::new(profile);
                let cycle = [b.update(s(0, 0)), b.update(s(1, Position::MAX)), b.update(s(2, 0))];
                assert_eq!(cycle, [None, Some(Event::Down), Some(Event::Up)], "{actuation}/{hysteresis}");
            }
        }
        assert_eq!(checked, 1000 * 1001 / 2);
    }

    #[test]
    fn lowering_actuation_under_a_released_button_does_not_press_it() {
        let mut b = Button::new(Profile::default());
        assert_eq!(b.update(s(0, 400)), None);
        b.set_profile(Profile { actuation: Position::saturating(300), ..Profile::default() });
        assert_eq!(b.update(s(1, 400)), None);
        assert_eq!(b.update(s(2, 450)), None);
        assert_eq!(b.update(s(3, 299)), None);
        assert_eq!(b.update(s(4, 300)), Some(Event::Down));
    }

    #[test]
    fn release_lifts_a_held_button_once_and_needs_a_new_crossing() {
        let mut b = Button::new(Profile::default());
        assert_eq!(b.release(), None, "nothing to release");
        assert_eq!(b.update(s(0, 600)), Some(Event::Down));
        assert_eq!(b.release(), Some(Event::Up));
        assert_eq!(b.release(), None);
        assert_eq!(b.update(s(1, 600)), None, "still held after recovery: no phantom Down");
        assert_eq!(b.update(s(2, 499)), None);
        assert_eq!(b.update(s(3, 500)), Some(Event::Down));
    }

    #[test]
    fn long_hold_emits_nothing_more() {
        let mut b = Button::new(rapid(10_000));
        assert_eq!(b.update(s(0, 600)), Some(Event::Down));
        for t in 1..1_000 {
            assert_eq!(b.update(s(t * 1_000, 600)), None);
        }
    }

    #[test]
    fn chatter_inside_hysteresis_band_emits_nothing() {
        let mut b = Button::new(rapid(10_000));
        assert_eq!(b.update(s(0, 500)), Some(Event::Down));
        for t in 1..100 {
            let pos = if t % 2 == 0 { 451 } else { 520 };
            assert_eq!(b.update(s(t * 10, pos)), None);
        }
    }

    #[test]
    fn chatter_below_actuation_emits_nothing() {
        let mut b = Button::new(rapid(10_000));
        for t in 0..100 {
            let pos = if t % 2 == 0 { 0 } else { 499 };
            assert_eq!(b.update(s(t * 10, pos)), None);
        }
    }
}
