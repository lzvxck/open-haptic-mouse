use core::cell::Cell;

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use ohm_core::Profile;
use ohm_protocol::{ButtonState, Status, Telemetry, PROTOCOL_VERSION};

pub static PROFILE: Signal<CriticalSectionRawMutex, Profile> = Signal::new();
pub static TELEMETRY: Channel<CriticalSectionRawMutex, Telemetry, 8> = Channel::new();

pub const BOOT_PROFILE: Profile = Profile { rapid_trigger_window_us: 50_000, ..Profile::DEFAULT };

pub static STATUS: Mutex<CriticalSectionRawMutex, Cell<Status>> = Mutex::new(Cell::new(Status {
    protocol_version: PROTOCOL_VERSION,
    profile: BOOT_PROFILE,
    last_raw: 0,
    button: ButtonState::Up,
    last_event: None,
}));

pub fn update_status(f: impl FnOnce(&mut Status)) {
    STATUS.lock(|s| {
        let mut status = s.get();
        f(&mut status);
        s.set(status);
    });
}
