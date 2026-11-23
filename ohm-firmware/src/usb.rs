use defmt::{info, warn};
use embassy_stm32::peripherals::USB;
use embassy_stm32::usb::Driver;
use embassy_usb::class::hid::{
    Config as HidConfig, HidBootProtocol, HidSubclass, HidWriter, ReportId as HidReportId, RequestHandler, State,
};
use embassy_usb::control::OutResponse;
use embassy_usb::{Builder, Config, UsbDevice};
use ohm_core::Event;
use ohm_protocol::{
    Report, ReportId, MOUSE_REPORT_DESCRIPTOR, MOUSE_REPORT_LEN, TELEMETRY_LEN, VENDOR_REPORT_DESCRIPTOR,
};
use static_cell::StaticCell;

use crate::shared::{EVENTS, PROFILE, STATUS, TELEMETRY};

/// pid.codes test VID/PID. Not a product identity: replace before shipping anything.
pub const TEST_VID: u16 = 0x1209;
pub const TEST_PID: u16 = 0x0001;

const MOUSE_PACKET: u16 = 8;
const VENDOR_PACKET: u16 = 64;
const POLL_MS: u8 = 1;
const LEFT_BUTTON: u8 = 0x01;

type UsbDriver = Driver<'static, USB>;
pub type MouseWriter = HidWriter<'static, UsbDriver, MOUSE_REPORT_LEN>;
pub type VendorWriter = HidWriter<'static, UsbDriver, TELEMETRY_LEN>;

struct VendorHandler;

impl RequestHandler for VendorHandler {
    fn get_report(&mut self, id: HidReportId, buf: &mut [u8]) -> Option<usize> {
        match id {
            HidReportId::Feature(i) if i == ReportId::GetStatus as u8 => {
                let status = STATUS.lock(|s| s.get())?;
                Report::GetStatus(status).encode(buf).ok()
            }
            _ => None,
        }
    }

    fn set_report(&mut self, id: HidReportId, data: &[u8]) -> OutResponse {
        match (id, Report::decode(data)) {
            (HidReportId::Feature(i), Ok(Report::SetProfile(profile))) if i == ReportId::SetProfile as u8 => {
                PROFILE.signal(profile);
                OutResponse::Accepted
            }
            _ => {
                warn!("rejected set_report");
                OutResponse::Rejected
            }
        }
    }
}

/// Composite device: interface 0 is the HID mouse, interface 1 is the vendor-defined HID (0xFF00).
/// They stay separate because Windows will not let hidapi open the mouse collection.
pub fn build(driver: UsbDriver) -> (UsbDevice<'static, UsbDriver>, MouseWriter, VendorWriter) {
    static CONFIG_DESC: StaticCell<[u8; 128]> = StaticCell::new();
    static BOS_DESC: StaticCell<[u8; 16]> = StaticCell::new();
    static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();
    static MOUSE_STATE: StaticCell<State> = StaticCell::new();
    static VENDOR_STATE: StaticCell<State> = StaticCell::new();
    static HANDLER: StaticCell<VendorHandler> = StaticCell::new();

    let mut config = Config::new(TEST_VID, TEST_PID);
    config.manufacturer = Some("Open Haptic Mouse");
    config.product = Some("OHM prototype (TEST VID/PID)");
    config.serial_number = Some("OHM0001");
    config.max_power = 100;

    let mut builder = Builder::new(
        driver,
        config,
        CONFIG_DESC.init([0; 128]),
        BOS_DESC.init([0; 16]),
        &mut [],
        CONTROL_BUF.init([0; 64]),
    );

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

#[embassy_executor::task]
pub async fn mouse_task(mut writer: MouseWriter) {
    loop {
        writer.ready().await;
        EVENTS.clear();
        info!("mouse endpoint ready");
        loop {
            let buttons = match EVENTS.receive().await {
                Event::Down | Event::RapidDown => LEFT_BUTTON,
                Event::Up => 0,
            };
            if writer.write(&[buttons, 0, 0, 0]).await.is_err() {
                break;
            }
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
