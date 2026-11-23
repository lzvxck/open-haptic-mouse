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
}

/// Máquina de estados del botón. Se alimenta con una lectura por muestra.
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

    /// Cambia los umbrales sin tocar el estado pulsado/suelto ni el último Up.
    pub fn set_profile(&mut self, profile: Profile) {
        self.profile = profile;
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
