use core::cell::{Cell, RefCell};

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use ohm_core::Profile;
use ohm_protocol::{MouseSync, Status, Telemetry};

/// Latest-wins: a profile that arrives before the previous one is applied replaces it.
pub static PROFILE: Signal<CriticalSectionRawMutex, Profile> = Signal::new();
pub static TELEMETRY: Channel<CriticalSectionRawMutex, Telemetry, 8> = Channel::new();
/// Set by `main` before any task runs, so it is never `None` while USB is up.
pub static STATUS: Mutex<CriticalSectionRawMutex, Cell<Option<Status>>> = Mutex::new(Cell::new(None));

/// Button level and bus state for the mouse endpoint.
static MOUSE: Mutex<CriticalSectionRawMutex, RefCell<MouseSync>> = Mutex::new(RefCell::new(MouseSync::new()));
/// Raised by [`update_mouse`]; `mouse_task` waits on it.
pub static MOUSE_CHANGED: Signal<CriticalSectionRawMutex, ()> = Signal::new();

/// For the button level and bus events: changes the mouse state and wakes `mouse_task`.
pub fn update_mouse(f: impl FnOnce(&mut MouseSync)) {
    MOUSE.lock(|m| f(&mut m.borrow_mut()));
    MOUSE_CHANGED.signal(());
}

/// For `mouse_task` only: reads or records its own writes without waking itself.
pub fn with_mouse<R>(f: impl FnOnce(&mut MouseSync) -> R) -> R {
    MOUSE.lock(|m| f(&mut m.borrow_mut()))
}

pub fn update_status(f: impl FnOnce(&mut Status)) {
    STATUS.lock(|s| {
        if let Some(mut status) = s.get() {
            f(&mut status);
            s.set(Some(status));
        }
    });
}
