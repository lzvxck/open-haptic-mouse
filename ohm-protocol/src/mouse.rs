/// Decides which left-button level the mouse endpoint should report. It carries only the current
/// level, never a history of edges, so a slow, suspended or reset host is told where the button is
/// now and cannot replay stale clicks or miss a final release.
///
/// The firmware feeds it the button level and USB bus events, asks [`MouseSync::pending`] for the
/// report to write, and calls [`MouseSync::written`] once the endpoint has accepted that report.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MouseSync {
    pressed: bool,
    configured: bool,
    suspended: bool,
    /// Level of the last report handed to the endpoint in this session. `None`: the host must be told.
    reported: Option<bool>,
}

impl MouseSync {
    pub const fn new() -> Self {
        Self { pressed: false, configured: false, suspended: false, reported: None }
    }

    pub fn set_pressed(&mut self, pressed: bool) {
        self.pressed = pressed;
    }

    pub fn configured(&mut self, configured: bool) {
        self.configured = configured;
        self.reported = None;
    }

    pub fn suspended(&mut self, suspended: bool) {
        self.suspended = suspended;
        if !suspended {
            self.reported = None;
        }
    }

    /// A bus reset ends the session: the host reconfigures and must be told the level again.
    pub fn reset(&mut self) {
        *self = Self { pressed: self.pressed, ..Self::new() };
    }

    /// Left-button level to write now, if the host is listening and has not been told it yet.
    pub fn pending(&self) -> Option<bool> {
        (self.configured && !self.suspended && self.reported != Some(self.pressed)).then_some(self.pressed)
    }

    pub fn written(&mut self, pressed: bool) {
        self.reported = Some(pressed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn listening() -> MouseSync {
        let mut m = MouseSync::new();
        m.configured(true);
        m
    }

    /// Writes every pending report the way the mouse task does and returns what the host received.
    fn drain(m: &mut MouseSync) -> Vec<bool> {
        let mut sent = Vec::new();
        while let Some(level) = m.pending() {
            m.written(level);
            sent.push(level);
        }
        sent
    }

    #[test]
    fn first_report_after_configuration_is_the_current_level() {
        let mut m = MouseSync::new();
        m.set_pressed(true);
        assert_eq!(m.pending(), None, "not configured");
        m.configured(true);
        assert_eq!(drain(&mut m), [true]);
    }

    #[test]
    fn each_level_change_is_sent_once() {
        let mut m = listening();
        assert_eq!(drain(&mut m), [false]);
        m.set_pressed(true);
        assert_eq!(drain(&mut m), [true]);
        m.set_pressed(true);
        assert_eq!(drain(&mut m), []);
        m.set_pressed(false);
        assert_eq!(drain(&mut m), [false]);
    }

    #[test]
    fn a_burst_of_edges_while_the_host_is_slow_never_loses_the_final_level() {
        for edges in [1, 2, 15, 16, 17, 1000, 1001] {
            let mut m = listening();
            assert_eq!(drain(&mut m), [false]);
            for i in 0..edges {
                m.set_pressed(i % 2 == 0);
            }
            let pressed_at_end = edges % 2 == 1;
            let expected: &[bool] = if pressed_at_end { &[true] } else { &[] };
            assert_eq!(drain(&mut m), expected, "{edges} edges");
        }
    }

    #[test]
    fn nothing_is_written_while_suspended() {
        let mut m = listening();
        drain(&mut m);
        m.suspended(true);
        m.set_pressed(true);
        assert_eq!(m.pending(), None);
    }

    #[test]
    fn resume_sends_exactly_the_current_level() {
        for (before, during) in [(false, false), (false, true), (true, false), (true, true)] {
            let mut m = listening();
            m.set_pressed(before);
            drain(&mut m);
            m.suspended(true);
            m.set_pressed(during);
            m.suspended(false);
            assert_eq!(drain(&mut m), [during], "pressed {before} before suspend, {during} at resume");
        }
    }

    #[test]
    fn a_click_made_and_released_during_suspend_sends_no_press() {
        let mut m = listening();
        drain(&mut m);
        m.suspended(true);
        m.set_pressed(true);
        m.set_pressed(false);
        m.suspended(false);
        assert_eq!(drain(&mut m), [false]);
    }

    #[test]
    fn a_press_held_before_suspend_and_released_during_it_is_released_on_resume() {
        let mut m = listening();
        m.set_pressed(true);
        drain(&mut m);
        m.suspended(true);
        m.set_pressed(false);
        m.suspended(false);
        assert_eq!(drain(&mut m), [false]);
    }

    #[test]
    fn reset_waits_for_configuration_then_sends_the_current_level() {
        let mut m = listening();
        m.set_pressed(true);
        drain(&mut m);
        m.suspended(true);
        m.reset();
        assert_eq!(m.pending(), None, "reset leaves the device unconfigured");
        m.set_pressed(false);
        m.configured(true);
        assert_eq!(drain(&mut m), [false]);
    }

    #[test]
    fn deconfiguring_stops_reports_until_reconfigured() {
        let mut m = listening();
        drain(&mut m);
        m.configured(false);
        m.set_pressed(true);
        assert_eq!(m.pending(), None);
        m.configured(true);
        assert_eq!(drain(&mut m), [true]);
    }
}
