use core::cell::RefCell;

use defmt::{info, warn};
use embassy_futures::select::{select, Either};
use embassy_stm32::peripherals::USB;
use embassy_stm32::uid;
use embassy_stm32::usb::Driver;
use embassy_usb::class::hid::{
    Config as HidConfig, HidBootProtocol, HidSubclass, HidWriter, ReportId as HidReportId, RequestHandler, State,
};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_usb::control::OutResponse;
use embassy_usb::{Builder, Config, Handler, UsbDevice};
use ohm_protocol::{
    MouseSync, Report, ReportId, GET_STATUS_LEN, MOUSE_REPORT_DESCRIPTOR, MOUSE_REPORT_LEN, SET_PROFILE_LEN,
    TELEMETRY_LEN, VENDOR_REPORT_DESCRIPTOR,
};
use static_cell::StaticCell;

use crate::shared::{update_status, PROFILE, STATUS, TELEMETRY};

/// pid.codes test VID/PID. Not a product identity: replace before shipping anything.
pub const TEST_VID: u16 = 0x1209;
pub const TEST_PID: u16 = 0x0001;

const MOUSE_PACKET: u16 = 8;
const VENDOR_PACKET: u16 = 64;
const POLL_MS: u8 = 1;
const LEFT_BUTTON: u8 = 0x01;

static MOUSE: Mutex<CriticalSectionRawMutex, RefCell<MouseSync>> = Mutex::new(RefCell::new(MouseSync::new()));
static MOUSE_CHANGED: Signal<CriticalSectionRawMutex, ()> = Signal::new();

pub fn update_mouse(f: impl FnOnce(&mut MouseSync)) {
    MOUSE.lock(|m| f(&mut m.borrow_mut()));
    MOUSE_CHANGED.signal(());
}

fn with_mouse<R>(f: impl FnOnce(&mut MouseSync) -> R) -> R {
    MOUSE.lock(|m| f(&mut m.borrow_mut()))
}

const MANUFACTURER: &str = "Open Haptic Mouse";
const PRODUCT: &str = "OHM prototype (TEST VID/PID)";
const SERIAL_LEN: usize = 24;

const fn max(a: usize, b: usize) -> usize {
    if a > b {
        a
    } else {
        b
    }
}

const MAX_STRING_UNITS: usize = max(MANUFACTURER.len(), max(PRODUCT.len(), SERIAL_LEN));
const _: () = assert!(2 + 2 * MAX_STRING_UNITS <= u8::MAX as usize);
const CONTROL_BUF_LEN: usize = max(2 * MAX_STRING_UNITS + 3, max(SET_PROFILE_LEN, GET_STATUS_LEN));

type UsbDriver = Driver<'static, USB>;
pub type MouseWriter = HidWriter<'static, UsbDriver, MOUSE_REPORT_LEN>;
pub type VendorWriter = HidWriter<'static, UsbDriver, TELEMETRY_LEN>;

struct VendorHandler;

impl RequestHandler for VendorHandler {
    fn get_report(&mut self, id: HidReportId, buf: &mut [u8]) -> Option<usize> {
        match id {
            HidReportId::Feature(i) if i == ReportId::GetStatus as u8 => {
                let status = STATUS.lock(|s| s.get());
                Report::GetStatus(status).encode(buf).ok()
            }
            _ => None,
        }
    }

    fn set_report(&mut self, id: HidReportId, data: &[u8]) -> OutResponse {
        match (id, Report::decode(data)) {
            (HidReportId::Feature(i), Ok(Report::SetProfile(profile))) if i == ReportId::SetProfile as u8 => {
                PROFILE.signal(profile);
                update_status(|s| s.profile = profile);
                OutResponse::Accepted
            }
            _ => {
                warn!("rejected set_report");
                OutResponse::Rejected
            }
        }
    }
}

struct BusHandler;

impl Handler for BusHandler {
    fn enabled(&mut self, enabled: bool) {
        if !enabled {
            update_mouse(|m| m.reset());
        }
    }

    fn reset(&mut self) {
        update_mouse(|m| m.reset());
    }

    fn configured(&mut self, configured: bool) {
        info!("usb configured: {}", configured);
        update_mouse(|m| m.configured(configured));
    }

    fn suspended(&mut self, suspended: bool) {
        info!("usb suspended: {}", suspended);
        update_mouse(|m| m.suspended(suspended));
    }
}

/// The vendor HID is its own interface because Windows will not let hidapi open the mouse collection.
pub fn build(driver: UsbDriver) -> (UsbDevice<'static, UsbDriver>, MouseWriter, VendorWriter) {
    static CONFIG_DESC: StaticCell<[u8; 128]> = StaticCell::new();
    static BOS_DESC: StaticCell<[u8; 16]> = StaticCell::new();
    static CONTROL_BUF: StaticCell<[u8; CONTROL_BUF_LEN]> = StaticCell::new();
    static MOUSE_STATE: StaticCell<State> = StaticCell::new();
    static VENDOR_STATE: StaticCell<State> = StaticCell::new();
    static HANDLER: StaticCell<VendorHandler> = StaticCell::new();
    static BUS_HANDLER: StaticCell<BusHandler> = StaticCell::new();

    let mut config = Config::new(TEST_VID, TEST_PID);
    let _: &[u8; SERIAL_LEN] = uid::uid_hex_bytes();
    config.manufacturer = Some(MANUFACTURER);
    config.product = Some(PRODUCT);
    config.serial_number = Some(uid::uid_hex());
    config.max_power = 100;

    let mut builder = Builder::new(
        driver,
        config,
        CONFIG_DESC.init([0; 128]),
        BOS_DESC.init([0; 16]),
        &mut [],
        CONTROL_BUF.init([0; CONTROL_BUF_LEN]),
    );
    builder.handler(BUS_HANDLER.init(BusHandler));

    let mouse = HidWriter::new(
        &mut builder,
        MOUSE_STATE.init(State::new()),
        HidConfig {
            report_descriptor: &MOUSE_REPORT_DESCRIPTOR,
            request_handler: None,
            poll_ms: POLL_MS,
            max_packet_size: MOUSE_PACKET,
            hid_subclass: HidSubclass::No,
            hid_boot_protocol: HidBootProtocol::None,
        },
    );
    let vendor = HidWriter::new(
        &mut builder,
        VENDOR_STATE.init(State::new()),
        HidConfig {
            report_descriptor: &VENDOR_REPORT_DESCRIPTOR,
            request_handler: Some(HANDLER.init(VendorHandler)),
            poll_ms: POLL_MS,
            max_packet_size: VENDOR_PACKET,
            hid_subclass: HidSubclass::No,
            hid_boot_protocol: HidBootProtocol::None,
        },
    );
    (builder.build(), mouse, vendor)
}

#[embassy_executor::task]
pub async fn usb_task(mut device: UsbDevice<'static, UsbDriver>) -> ! {
    device.run().await
}

/// A write still waiting for the host to take the previous report is dropped when the state changes;
/// the endpoint arms data only once that wait ends, so the dropped report never reaches the host.
#[embassy_executor::task]
pub async fn mouse_task(mut writer: MouseWriter) {
    loop {
        let Some(pressed) = with_mouse(|m| m.pending()) else {
            MOUSE_CHANGED.wait().await;
            continue;
        };
        let report = [if pressed { LEFT_BUTTON } else { 0 }, 0, 0, 0];
        match select(writer.write(&report), MOUSE_CHANGED.wait()).await {
            Either::First(Ok(())) => with_mouse(|m| m.written(pressed)),
            Either::First(Err(_)) => MOUSE_CHANGED.wait().await,
            Either::Second(()) => {}
        }
    }
}

#[embassy_executor::task]
pub async fn telemetry_task(mut writer: VendorWriter) {
    loop {
        writer.ready().await;
        TELEMETRY.clear();
        loop {
            let mut buf = [0; TELEMETRY_LEN];
            let report = Report::Telemetry(TELEMETRY.receive().await);
            let Ok(len) = report.encode(&mut buf) else { continue };
            if writer.write(&buf[..len]).await.is_err() {
                break;
            }
        }
    }
}
