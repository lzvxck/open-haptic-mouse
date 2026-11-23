use crate::report::{GET_STATUS_LEN, SET_PROFILE_LEN, TELEMETRY_LEN};

pub const MOUSE_REPORT_LEN: usize = 4;

/// Ratón estilo boot: 3 botones, X/Y relativos y rueda. Sin report ID: `[botones, x, y, rueda]`.
pub const MOUSE_REPORT_DESCRIPTOR: [u8; 52] = [
    0x05, 0x01, // Usage Page (Generic Desktop)
    0x09, 0x02, // Usage (Mouse)
    0xA1, 0x01, // Collection (Application)
    0x09, 0x01, //   Usage (Pointer)
    0xA1, 0x00, //   Collection (Physical)
    0x05, 0x09, //     Usage Page (Button)
    0x19, 0x01, //     Usage Minimum (1)
    0x29, 0x03, //     Usage Maximum (3)
    0x15, 0x00, //     Logical Minimum (0)
    0x25, 0x01, //     Logical Maximum (1)
    0x95, 0x03, //     Report Count (3)
    0x75, 0x01, //     Report Size (1)
    0x81, 0x02, //     Input (Data, Var, Abs)
    0x95, 0x01, //     Report Count (1)
    0x75, 0x05, //     Report Size (5)
    0x81, 0x03, //     Input (Const, Var, Abs)
    0x05, 0x01, //     Usage Page (Generic Desktop)
    0x09, 0x30, //     Usage (X)
    0x09, 0x31, //     Usage (Y)
    0x09, 0x38, //     Usage (Wheel)
    0x15, 0x81, //     Logical Minimum (-127)
    0x25, 0x7F, //     Logical Maximum (127)
    0x75, 0x08, //     Report Size (8)
    0x95, 0x03, //     Report Count (3)
    0x81, 0x06, //     Input (Data, Var, Rel)
    0xC0, //   End Collection
    0xC0, // End Collection
];

const fn data_len(report_len: usize) -> u8 {
    let n = report_len - 1;
    assert!(n <= u8::MAX as usize);
    n as u8
}

/// Vendor-defined (0xFF00): feature 0x01 `SetProfile`, feature 0x02 `GetStatus`, input 0x03 `Telemetry`.
/// Cada Report Count es la longitud del report sin el byte de ID.
pub const VENDOR_REPORT_DESCRIPTOR: [u8; 39] = [
    0x06, 0x00, 0xFF, // Usage Page (Vendor 0xFF00)
    0x09, 0x01, // Usage (1)
    0xA1, 0x01, // Collection (Application)
    0x15, 0x00, //   Logical Minimum (0)
    0x26, 0xFF, 0x00, //   Logical Maximum (255)
    0x75, 0x08, //   Report Size (8)
    0x85, 0x01, //   Report ID (1)
    0x09, 0x01, //   Usage (1)
    0x95, data_len(SET_PROFILE_LEN), //   Report Count
    0xB1, 0x02, //   Feature (Data, Var, Abs)
    0x85, 0x02, //   Report ID (2)
    0x09, 0x02, //   Usage (2)
    0x95, data_len(GET_STATUS_LEN), //   Report Count
    0xB1, 0x02, //   Feature (Data, Var, Abs)
    0x85, 0x03, //   Report ID (3)
    0x09, 0x03, //   Usage (3)
    0x95, data_len(TELEMETRY_LEN), //   Report Count
    0x81, 0x02, //   Input (Data, Var, Abs)
    0xC0, // End Collection
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    const INPUT: u8 = 0x8;
    const FEATURE: u8 = 0xB;
    const END_COLLECTION: u8 = 0xC;
    const COLLECTION: u8 = 0xA;

    #[derive(Debug, PartialEq)]
    struct ReportBits {
        tag: u8,
        report_id: u8,
        bits: usize,
    }

    #[derive(Default, Debug, PartialEq)]
    struct Totals {
        bits: Vec<ReportBits>,
    }

    impl Totals {
        fn add(&mut self, tag: u8, report_id: u8, bits: usize) {
            match self.bits.iter_mut().find(|e| e.tag == tag && e.report_id == report_id) {
                Some(e) => e.bits += bits,
                None => self.bits.push(ReportBits { tag, report_id, bits }),
            }
        }

        fn bytes(&self, tag: u8, report_id: u8) -> Option<usize> {
            self.bits.iter().find(|e| e.tag == tag && e.report_id == report_id).map(|e| e.bits / 8)
        }
    }

    fn walk(desc: &[u8]) -> Totals {
        let (mut i, mut depth) = (0, 0usize);
        let (mut size, mut count, mut id) = (0usize, 0usize, 0u8);
        let mut totals = Totals::default();
        while i < desc.len() {
            let prefix = desc[i];
            assert_ne!(prefix, 0xFE, "long items are not used");
            let data_len = match prefix & 0b11 {
                3 => 4,
                n => n as usize,
            };
            let (kind, tag) = ((prefix >> 2) & 0b11, prefix >> 4);
            assert!(i + 1 + data_len <= desc.len(), "truncated item at {i}");
            let mut value = 0usize;
            for (k, b) in desc[i + 1..i + 1 + data_len].iter().enumerate() {
                value |= (*b as usize) << (8 * k);
            }
            match (kind, tag) {
                (0, COLLECTION) => depth += 1,
                (0, END_COLLECTION) => {
                    assert_eq!(data_len, 0);
                    depth = depth.checked_sub(1).expect("unmatched End Collection");
                }
                (0, INPUT | FEATURE) => totals.add(tag, id, size * count),
                (1, 0x7) => size = value,
                (1, 0x8) => id = value as u8,
                (1, 0x9) => count = value,
                _ => {}
            }
            i += 1 + data_len;
        }
        assert_eq!(depth, 0, "unclosed Collection");
        totals
    }

    #[test]
    fn mouse_descriptor_is_well_formed_and_4_bytes() {
        let totals = walk(&MOUSE_REPORT_DESCRIPTOR);
        assert_eq!(totals.bits.len(), 1);
        assert_eq!(totals.bytes(INPUT, 0), Some(MOUSE_REPORT_LEN));
    }

    #[test]
    fn vendor_descriptor_is_well_formed_and_sizes_match_encode_lengths() {
        let totals = walk(&VENDOR_REPORT_DESCRIPTOR);
        assert_eq!(totals.bits.len(), 3);
        assert_eq!(totals.bytes(FEATURE, 1), Some(SET_PROFILE_LEN - 1));
        assert_eq!(totals.bytes(FEATURE, 2), Some(GET_STATUS_LEN - 1));
        assert_eq!(totals.bytes(INPUT, 3), Some(TELEMETRY_LEN - 1));
    }

    #[test]
    fn vendor_descriptor_declares_usage_page_ff00() {
        assert_eq!(VENDOR_REPORT_DESCRIPTOR[..3], [0x06, 0x00, 0xFF]);
    }

    #[test]
    fn walker_rejects_malformed_streams() {
        assert!(std::panic::catch_unwind(|| walk(&[0x05])).is_err());
        assert!(std::panic::catch_unwind(|| walk(&[0xA1, 0x01])).is_err());
        assert!(std::panic::catch_unwind(|| walk(&[0xC0])).is_err());
    }
}
