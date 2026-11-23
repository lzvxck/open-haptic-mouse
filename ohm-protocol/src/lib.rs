//! Protocolo host <-> dispositivo de OHM: feature/input reports HID de tamaño fijo.
//! Layout detallado en `PROTOCOL.md`.
#![no_std]

#[cfg(test)]
extern crate std;

mod descriptor;
mod report;

pub use descriptor::{MOUSE_REPORT_DESCRIPTOR, MOUSE_REPORT_LEN, VENDOR_REPORT_DESCRIPTOR};
pub use report::{
    ButtonState, DecodeError, EncodeError, Report, ReportId, Status, Telemetry, GET_STATUS_LEN,
    PROTOCOL_VERSION, SET_PROFILE_LEN, TELEMETRY_LEN,
};
