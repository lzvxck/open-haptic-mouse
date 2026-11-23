use ohm_core::{Event, Micros, Position, Profile, ProfileError};

pub const PROTOCOL_VERSION: u8 = 2;

pub const SET_PROFILE_LEN: usize = 10;
pub const GET_STATUS_LEN: usize = 17;
pub const TELEMETRY_LEN: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ReportId {
    SetProfile = 0x01,
    GetStatus = 0x02,
    Telemetry = 0x03,
}

impl ReportId {
    pub const fn from_byte(b: u8) -> Option<Self> {
        match b {
            0x01 => Some(Self::SetProfile),
            0x02 => Some(Self::GetStatus),
            0x03 => Some(Self::Telemetry),
            _ => None,
        }
    }

    pub const fn wire_len(self) -> usize {
        match self {
            Self::SetProfile => SET_PROFILE_LEN,
            Self::GetStatus => GET_STATUS_LEN,
            Self::Telemetry => TELEMETRY_LEN,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonState {
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Status {
    pub protocol_version: u8,
    pub profile: Profile,
    pub last_raw: u32,
    pub button: ButtonState,
    pub last_event: Option<Event>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Telemetry {
    pub timestamp: Micros,
    pub position: Position,
    pub event: Option<Event>,
    pub latency_us: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Report {
    SetProfile(Profile),
    GetStatus(Status),
    Telemetry(Telemetry),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncodeError {
    BufferTooSmall { needed: usize },
    InvalidProfile(ProfileError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    Truncated { needed: usize },
    UnknownId(u8),
    BadEnum(u8),
    PositionOutOfRange(u16),
    InvalidProfile(ProfileError),
}

impl Report {
    pub const fn id(&self) -> ReportId {
        match self {
            Self::SetProfile(_) => ReportId::SetProfile,
            Self::GetStatus(_) => ReportId::GetStatus,
            Self::Telemetry(_) => ReportId::Telemetry,
        }
    }

    /// Escribe el report en `buf[..len]` y devuelve `len`. No toca bytes posteriores.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        let id = self.id();
        let len = id.wire_len();
        if buf.len() < len {
            return Err(EncodeError::BufferTooSmall { needed: len });
        }
        buf[0] = id as u8;
        match self {
            Self::SetProfile(p) => put_profile(&mut buf[1..], p)?,
            Self::GetStatus(s) => {
                buf[1] = s.protocol_version;
                put_profile(&mut buf[2..], &s.profile)?;
                buf[11..15].copy_from_slice(&s.last_raw.to_le_bytes());
                buf[15] = s.button as u8;
                buf[16] = event_byte(s.last_event);
            }
            Self::Telemetry(t) => {
                buf[1..9].copy_from_slice(&t.timestamp.0.to_le_bytes());
                buf[9..11].copy_from_slice(&t.position.get().to_le_bytes());
                buf[11] = event_byte(t.event);
                buf[12..16].copy_from_slice(&t.latency_us.to_le_bytes());
            }
        }
        Ok(len)
    }

    /// Los bytes sobrantes tras `len` (relleno HID) se ignoran.
    pub fn decode(buf: &[u8]) -> Result<Report, DecodeError> {
        let &first = buf.first().ok_or(DecodeError::Truncated { needed: 1 })?;
        let id = ReportId::from_byte(first).ok_or(DecodeError::UnknownId(first))?;
        if buf.len() < id.wire_len() {
            return Err(DecodeError::Truncated { needed: id.wire_len() });
        }
        Ok(match id {
            ReportId::SetProfile => Self::SetProfile(get_profile(&buf[1..])?),
            ReportId::GetStatus => Self::GetStatus(Status {
                protocol_version: buf[1],
                profile: get_profile(&buf[2..])?,
                last_raw: u32_at(buf, 11),
                button: match buf[15] {
                    0 => ButtonState::Up,
                    1 => ButtonState::Down,
                    b => return Err(DecodeError::BadEnum(b)),
                },
                last_event: event_from_byte(buf[16])?,
            }),
            ReportId::Telemetry => Self::Telemetry(Telemetry {
                timestamp: Micros(u64::from_le_bytes([
                    buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7], buf[8],
                ])),
                position: position(u16_at(buf, 9))?,
                event: event_from_byte(buf[11])?,
                latency_us: u32_at(buf, 12),
            }),
        })
    }
}

fn u16_at(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([b[i], b[i + 1]])
}

fn u32_at(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}

fn position(raw: u16) -> Result<Position, DecodeError> {
    if raw > Position::MAX {
        return Err(DecodeError::PositionOutOfRange(raw));
    }
    Ok(Position::saturating(raw))
}

fn event_byte(e: Option<Event>) -> u8 {
    match e {
        None => 0,
        Some(Event::Down) => 1,
        Some(Event::Up) => 2,
        Some(Event::RapidDown) => 3,
    }
}

fn event_from_byte(b: u8) -> Result<Option<Event>, DecodeError> {
    match b {
        0 => Ok(None),
        1 => Ok(Some(Event::Down)),
        2 => Ok(Some(Event::Up)),
        3 => Ok(Some(Event::RapidDown)),
        b => Err(DecodeError::BadEnum(b)),
    }
}

/// `b` tiene 9 bytes como mínimo: lo garantiza la comprobación de longitud del report.
fn put_profile(b: &mut [u8], p: &Profile) -> Result<(), EncodeError> {
    p.validate().map_err(EncodeError::InvalidProfile)?;
    b[0..2].copy_from_slice(&p.actuation.get().to_le_bytes());
    b[2..4].copy_from_slice(&p.release_hysteresis.get().to_le_bytes());
    b[4..8].copy_from_slice(&p.rapid_trigger_window_us.to_le_bytes());
    b[8] = p.haptic_intensity;
    Ok(())
}

fn get_profile(b: &[u8]) -> Result<Profile, DecodeError> {
    let profile = Profile {
        actuation: position(u16_at(b, 0))?,
        release_hysteresis: position(u16_at(b, 2))?,
        rapid_trigger_window_us: u32_at(b, 4),
        haptic_intensity: b[8],
    };
    profile.validate().map_err(DecodeError::InvalidProfile)?;
    Ok(profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(actuation: u16, hysteresis: u16, window: u32, intensity: u8) -> Profile {
        Profile {
            actuation: Position::saturating(actuation),
            release_hysteresis: Position::saturating(hysteresis),
            rapid_trigger_window_us: window,
            haptic_intensity: intensity,
        }
    }

    fn samples() -> [Report; 11] {
        let max = Position::MAX;
        let extreme = profile(max, max - 1, u32::MAX, u8::MAX);
        [
            Report::SetProfile(Profile::default()),
            Report::SetProfile(extreme),
            Report::SetProfile(profile(1, 0, 0, 0)),
            Report::GetStatus(Status {
                protocol_version: PROTOCOL_VERSION,
                profile: Profile::default(),
                last_raw: 123,
                button: ButtonState::Down,
                last_event: Some(Event::Down),
            }),
            Report::GetStatus(Status {
                protocol_version: u8::MAX,
                profile: extreme,
                last_raw: u32::MAX,
                button: ButtonState::Up,
                last_event: Some(Event::Up),
            }),
            Report::GetStatus(Status {
                protocol_version: PROTOCOL_VERSION,
                profile: Profile::default(),
                last_raw: 0x00AB_CDEF,
                button: ButtonState::Down,
                last_event: Some(Event::RapidDown),
            }),
            Report::GetStatus(Status {
                protocol_version: 0,
                profile: profile(1, 0, 0, 0),
                last_raw: 0,
                button: ButtonState::Up,
                last_event: None,
            }),
            Report::Telemetry(Telemetry {
                timestamp: Micros(0),
                position: Position::saturating(0),
                event: None,
                latency_us: 0,
            }),
            Report::Telemetry(Telemetry {
                timestamp: Micros(u64::MAX),
                position: Position::saturating(max),
                event: Some(Event::Up),
                latency_us: u32::MAX,
            }),
            Report::Telemetry(Telemetry {
                timestamp: Micros(0x0123_4567_89AB_CDEF),
                position: Position::saturating(500),
                event: Some(Event::Down),
                latency_us: 0xDEAD_BEEF,
            }),
            Report::Telemetry(Telemetry {
                timestamp: Micros(1),
                position: Position::saturating(1),
                event: Some(Event::RapidDown),
                latency_us: 1,
            }),
        ]
    }

    #[test]
    fn round_trip_every_report() {
        for r in samples() {
            let mut buf = [0xAA; 32];
            let n = r.encode(&mut buf).unwrap();
            assert_eq!(n, r.id().wire_len());
            assert_eq!(buf[0], r.id() as u8);
            assert!(buf[n..].iter().all(|&b| b == 0xAA), "encode wrote past len");
            assert_eq!(Report::decode(&buf[..n]), Ok(r));
            assert_eq!(Report::decode(&buf), Ok(r), "trailing padding is ignored");
        }
    }

    #[test]
    fn encode_uses_little_endian_layout() {
        let mut buf = [0; TELEMETRY_LEN];
        Report::Telemetry(Telemetry {
            timestamp: Micros(0x0807_0605_0403_0201),
            position: Position::saturating(0x0102),
            event: Some(Event::Up),
            latency_us: 0x0D0C_0B0A,
        })
        .encode(&mut buf)
        .unwrap();
        assert_eq!(buf, [3, 1, 2, 3, 4, 5, 6, 7, 8, 0x02, 0x01, 2, 0x0A, 0x0B, 0x0C, 0x0D]);
    }

    #[test]
    fn status_last_raw_is_a_little_endian_u32() {
        let mut buf = [0; GET_STATUS_LEN];
        let Report::GetStatus(status) = samples()[5] else { unreachable!() };
        Report::GetStatus(Status { last_raw: 0x0403_0201, ..status }).encode(&mut buf).unwrap();
        assert_eq!(buf[11..15], [1, 2, 3, 4]);
    }

    #[test]
    fn encode_rejects_small_buffer_and_invalid_profile() {
        for r in samples() {
            let mut buf = [0; 32];
            let needed = r.id().wire_len();
            assert_eq!(r.encode(&mut buf[..needed - 1]), Err(EncodeError::BufferTooSmall { needed }));
        }
        let bad = profile(100, 101, 0, 0);
        let mut buf = [0; 32];
        assert_eq!(
            Report::SetProfile(bad).encode(&mut buf),
            Err(EncodeError::InvalidProfile(ProfileError::HysteresisNotBelowActuation))
        );
    }

    #[test]
    fn decode_rejects_truncated() {
        assert_eq!(Report::decode(&[]), Err(DecodeError::Truncated { needed: 1 }));
        for r in samples() {
            let mut buf = [0; 32];
            let n = r.encode(&mut buf).unwrap();
            for cut in 1..n {
                assert_eq!(Report::decode(&buf[..cut]), Err(DecodeError::Truncated { needed: n }));
            }
        }
    }

    #[test]
    fn decode_rejects_unknown_id() {
        for id in [0x00, 0x04, 0x7F, 0xFF] {
            assert_eq!(Report::decode(&[id; 32]), Err(DecodeError::UnknownId(id)));
        }
    }

    #[test]
    fn decode_rejects_invalid_profile() {
        let mut buf = [0; SET_PROFILE_LEN];
        Report::SetProfile(Profile::default()).encode(&mut buf).unwrap();
        buf[3..5].copy_from_slice(&501u16.to_le_bytes());
        assert_eq!(
            Report::decode(&buf),
            Err(DecodeError::InvalidProfile(ProfileError::HysteresisNotBelowActuation))
        );
        buf[1..3].copy_from_slice(&1001u16.to_le_bytes());
        assert_eq!(Report::decode(&buf), Err(DecodeError::PositionOutOfRange(1001)));
    }

    #[test]
    fn decode_rejects_profiles_whose_button_could_stick() {
        let set_profile = |actuation: u16, hysteresis: u16| {
            let mut buf = [0; SET_PROFILE_LEN];
            buf[0] = ReportId::SetProfile as u8;
            buf[1..3].copy_from_slice(&actuation.to_le_bytes());
            buf[3..5].copy_from_slice(&hysteresis.to_le_bytes());
            Report::decode(&buf)
        };
        assert_eq!(set_profile(500, 500), Err(DecodeError::InvalidProfile(ProfileError::HysteresisNotBelowActuation)));
        assert_eq!(set_profile(0, 0), Err(DecodeError::InvalidProfile(ProfileError::ZeroActuation)));
        assert_eq!(set_profile(500, 499), Ok(Report::SetProfile(profile(500, 499, 0, 0))));
    }

    #[test]
    fn decode_rejects_bad_enums_and_position() {
        let mut status = [0; GET_STATUS_LEN];
        samples()[3].encode(&mut status).unwrap();
        let mut bad_button = status;
        bad_button[15] = 2;
        assert_eq!(Report::decode(&bad_button), Err(DecodeError::BadEnum(2)));
        let mut bad_event = status;
        bad_event[16] = 0xFF;
        assert_eq!(Report::decode(&bad_event), Err(DecodeError::BadEnum(0xFF)));

        let mut tel = [0; TELEMETRY_LEN];
        samples()[9].encode(&mut tel).unwrap();
        let mut bad_event = tel;
        bad_event[11] = 4;
        assert_eq!(Report::decode(&bad_event), Err(DecodeError::BadEnum(4)));
        tel[9..11].copy_from_slice(&u16::MAX.to_le_bytes());
        assert_eq!(Report::decode(&tel), Err(DecodeError::PositionOutOfRange(u16::MAX)));
    }

    #[test]
    fn fuzz_decode_never_panics_and_accepted_input_is_canonical() {
        let mut state = 0x9E37_79B9_7F4A_7C15u64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut accepted = 0u32;
        for i in 0..200_000u32 {
            let mut buf = [0u8; 24];
            for chunk in buf.chunks_mut(8) {
                chunk.copy_from_slice(&next().to_le_bytes());
            }
            let len = (next() % 25) as usize;
            if i % 2 == 0 {
                buf[0] = 1 + (next() % 3) as u8;
                for at in [1, 2, 3, 4, 5, 9, 10, 11, 15, 16] {
                    buf[at] &= 0x03;
                }
            }
            if let Ok(r) = Report::decode(&buf[..len]) {
                accepted += 1;
                let mut out = [0u8; 24];
                let n = r.encode(&mut out).unwrap();
                assert_eq!(out[..n], buf[..n]);
            }
        }
        assert!(accepted > 1000, "fuzz never reached the happy path: {accepted}");
    }
}
