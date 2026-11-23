use core::cell::Cell;

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use ohm_core::{Event, Profile};
use ohm_protocol::{Status, Telemetry};

/// Latest-wins: a profile that arrives before the previous one is applied replaces it.
pub static PROFILE: Signal<CriticalSectionRawMutex, Profile> = Signal::new();
pub static EVENTS: Channel<CriticalSectionRawMutex, Event, 16> = Channel::new();
pub static TELEMETRY: Channel<CriticalSectionRawMutex, Telemetry, 8> = Channel::new();
/// `None` until the sensor task completes its first step.
pub static STATUS: Mutex<CriticalSectionRawMutex, Cell<Option<Status>>> = Mutex::new(Cell::new(None));
