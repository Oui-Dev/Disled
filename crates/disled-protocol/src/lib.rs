// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! Command encoder and message decoder of the DISLED USB HID protocol v1.1,
//! which `protocol/README.md` specifies. The functions do no I/O; the desktop
//! app and the production tool share them.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// USB IDs of the lamp, the pid.codes test PIDs that LICENSING.md describes.
pub const USB_VENDOR_ID: u16 = 0x1209;
pub const USB_PRODUCT_ID_RELEASE: u16 = 0x0001;
pub const USB_PRODUCT_ID_DEBUG: u16 = 0x0002;
/// Vendor-defined usage page of the HID interface.
pub const HID_USAGE_PAGE: u16 = 0xFF00;

/// Tells whether a device that hidapi lists is the HID interface of a lamp.
/// hidapi reports a usage page of 0 where the platform does not give it, so 0
/// matches too.
pub fn is_lamp_interface(vendor_id: u16, product_id: u16, usage_page: u16) -> bool {
    vendor_id == USB_VENDOR_ID
        && (product_id == USB_PRODUCT_ID_RELEASE || product_id == USB_PRODUCT_ID_DEBUG)
        && (usage_page == HID_USAGE_PAGE || usage_page == 0)
}

pub const REPORT_ID_COMMAND: u8 = 1;
pub const REPORT_ID_MESSAGE: u8 = 2;
/// Payload bytes of one report, after the report ID.
pub const PAYLOAD_SIZE: usize = 63;
const HEADER_SIZE: usize = 3;
pub const PROTOCOL_MAJOR: u8 = 1;
pub const PROTOCOL_MINOR: u8 = 1;

/// Automatic brightness bias limits, percent.
pub const AUTO_BIAS_MIN: i8 = -50;
pub const AUTO_BIAS_MAX: i8 = 50;

const CMD_GET_INFO: u8 = 0x01;
const CMD_GET_STATE: u8 = 0x02;
const CMD_SET_STATE: u8 = 0x03;
const CMD_SET_TELEMETRY: u8 = 0x04;
const CMD_RESET: u8 = 0x05;
const CMD_SET_AUTO_BIAS: u8 = 0x06;
const CMD_GET_DIAGNOSTICS: u8 = 0x07;
const CMD_SELF_TEST: u8 = 0x08;
const CMD_ENTER_BOOTLOADER: u8 = 0x09;

const MSG_INFO: u8 = 0x81;
const MSG_STATE: u8 = 0x82;
const MSG_TELEMETRY: u8 = 0x83;
const MSG_DIAGNOSTICS: u8 = 0x84;
const MSG_SELF_TEST_RESULT: u8 = 0x85;
const MSG_ACK: u8 = 0x8E;
const MSG_ERROR: u8 = 0x8F;

const FIELD_POWER: u8 = 0x01;
const FIELD_COLOR: u8 = 0x02;
const FIELD_BRIGHTNESS_MODE: u8 = 0x04;
const FIELD_MANUAL_LEVEL: u8 = 0x08;

/// An enum whose discriminant is its byte on the wire.
macro_rules! wire_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident = $value:expr),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant = $value),+ }

        impl $name {
            pub fn from_wire(value: u8) -> Result<Self, DecodeError> {
                match value {
                    $($value => Ok(Self::$variant),)+
                    _ => Err(DecodeError::BadValue(stringify!($name), value)),
                }
            }
        }
    };
}

wire_enum!(LampColor { Green = 0, Orange = 1, Red = 2, WarmWhite = 3 });
wire_enum!(BrightnessMode { Auto = 0, Manual = 1 });
wire_enum!(UsbLink { Unconfigured = 0, Configured = 1, Suspended = 2 });
wire_enum!(ChangeSource { Boot = 0, Touch = 1, Host = 2, AutoBrightness = 3, Usb = 4 });
wire_enum!(ResetCause {
    Unknown = 0,
    PowerOn = 1,
    Pin = 2,
    Software = 3,
    Watchdog = 4,
    WindowWatchdog = 5,
    LowPower = 6,
});
wire_enum!(TouchState { Calibrating = 0, Idle = 1, Pressed = 2 });
wire_enum!(ProtocolError {
    UnknownCommand = 1,
    BadLength = 2,
    BadValue = 3,
    Busy = 4,
});

/// Lamp settings to change; `None` leaves a setting as it is.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export, optional_fields))]
pub struct SetState {
    pub on: Option<bool>,
    pub color: Option<LampColor>,
    pub brightness_mode: Option<BrightnessMode>,
    /// Percent, 1 to 100.
    pub manual_level: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Command {
    GetInfo,
    GetState,
    SetState(SetState),
    SetTelemetry {
        period_ms: u16,
    },
    Reset,
    /// Percent, `AUTO_BIAS_MIN` to `AUTO_BIAS_MAX`, for a lamp that announces
    /// `auto_bias`.
    SetAutoBias {
        bias: i8,
    },
    GetDiagnostics,
    SelfTest,
    EnterBootloader,
}

impl Command {
    /// Encodes the command payload, which follows the report ID.
    pub fn encode(&self, seq: u8) -> [u8; PAYLOAD_SIZE] {
        let (kind, body): (u8, Vec<u8>) = match self {
            Command::GetInfo => (CMD_GET_INFO, vec![]),
            Command::GetState => (CMD_GET_STATE, vec![]),
            Command::SetState(s) => {
                let mut fields = 0;
                if s.on.is_some() {
                    fields |= FIELD_POWER;
                }
                if s.color.is_some() {
                    fields |= FIELD_COLOR;
                }
                if s.brightness_mode.is_some() {
                    fields |= FIELD_BRIGHTNESS_MODE;
                }
                if s.manual_level.is_some() {
                    fields |= FIELD_MANUAL_LEVEL;
                }
                (
                    CMD_SET_STATE,
                    vec![
                        fields,
                        u8::from(s.on.unwrap_or(false)),
                        s.color.map_or(0, |c| c as u8),
                        s.brightness_mode.map_or(0, |m| m as u8),
                        s.manual_level.unwrap_or(0),
                    ],
                )
            }
            Command::SetTelemetry { period_ms } => {
                (CMD_SET_TELEMETRY, period_ms.to_le_bytes().to_vec())
            }
            Command::Reset => (CMD_RESET, vec![]),
            Command::SetAutoBias { bias } => (CMD_SET_AUTO_BIAS, bias.to_le_bytes().to_vec()),
            Command::GetDiagnostics => (CMD_GET_DIAGNOSTICS, vec![]),
            Command::SelfTest => (CMD_SELF_TEST, vec![]),
            Command::EnterBootloader => (CMD_ENTER_BOOTLOADER, vec![]),
        };
        let mut payload = [0u8; PAYLOAD_SIZE];
        payload[0] = kind;
        payload[1] = seq;
        payload[2] = body.len() as u8;
        payload[HEADER_SIZE..HEADER_SIZE + body.len()].copy_from_slice(&body);
        payload
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Fault {
    pub mcause: u32,
    pub mepc: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Info {
    pub protocol: [u8; 2],
    pub firmware: [u8; 3],
    pub firmware_version: String,
    pub board_revision: u8,
    /// Chip unique ID in hexadecimal, as in the USB serial number.
    pub uid: String,
    pub telemetry: bool,
    pub debug_console: bool,
    // Optional commands, each announced by its capability bit.
    pub auto_bias: bool,
    pub diagnostics: bool,
    pub self_test: bool,
    pub enter_bootloader: bool,
    pub reset_cause: ResetCause,
    pub debug_build: bool,
    pub fault: Option<Fault>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export, rename = "LampState"))]
pub struct State {
    pub state_seq: u16,
    pub on: bool,
    pub color: LampColor,
    pub brightness_mode: BrightnessMode,
    pub manual_level: u8,
    /// Brightness in use, percent.
    pub applied_level: u8,
    pub link: UsbLink,
    pub source: ChangeSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Telemetry {
    pub uptime_ms: u32,
    pub touch_raw: u16,
    pub touch_value: u16,
    pub touch_baseline: u16,
    pub touch_delta_permille: i16,
    pub touch_state: TouchState,
    pub lux_centi: u32,
    pub als_raw: u16,
    pub duty: [u16; 3],
    pub led_current_ma_x10: u16,
    pub cap_ma: u16,
    pub link: UsbLink,
    pub level_permille: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Diagnostics {
    pub uptime_ms: u32,
    // Since power-up, kept across resets.
    pub boots: u16,
    pub watchdog_resets: u16,
    pub faults: u16,
    /// Last fault, `None` when no fault happened since power-up.
    pub last_fault: Option<FaultDetail>,
    /// Smallest stack margin seen since boot, bytes.
    pub stack_free_min: u16,
    // Since boot.
    pub i2c_errors: u16,
    pub light_sensor_probes: u16,
    pub touch_samples_lost: u16,
    pub touch_recalibrations: u16,
    pub usb_suspends: u16,
    /// Automatic brightness bias in force, percent.
    pub auto_bias: i8,
    pub light_sensor_present: bool,
    pub crystal_running: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct FaultDetail {
    pub mcause: u32,
    pub mepc: u32,
    pub mtval: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct SelfTestResult {
    pub light_sensor_ok: bool,
    pub touch_idle_ok: bool,
    pub touch_pressed: bool,
    pub completed: bool,
    pub lux_centi: u32,
    pub touch_baseline: u16,
    pub touch_peak_permille: i16,
}

impl SelfTestResult {
    /// Everything the lamp can check by itself passed.
    pub fn passed(&self) -> bool {
        self.light_sensor_ok && self.touch_idle_ok && self.touch_pressed && self.completed
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Message {
    Info(Info),
    State(State),
    Telemetry(Telemetry),
    Diagnostics(Diagnostics),
    SelfTestResult(SelfTestResult),
    Ack { command: u8 },
    Error { command: u8, error: ProtocolError },
}

/// A decoded message and the sequence number it answers, 0 when unsolicited.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Frame {
    pub seq: u8,
    pub message: Message,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DecodeError {
    #[error("payload too short ({0} bytes)")]
    TooShort(usize),
    #[error("body length {0} does not fit the report")]
    BadLength(usize),
    /// Hosts ignore an unsolicited message of an unknown type, as the
    /// specification asks, so callers usually skip this error.
    #[error("unknown message type 0x{0:02x}")]
    UnknownType(u8),
    #[error("message 0x{0:02x}: body of {1} bytes, expected at least {2}")]
    BodyTooShort(u8, usize, usize),
    #[error("invalid {0} value {1}")]
    BadValue(&'static str, u8),
}

/// Little-endian reader over a message body.
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn u8(&mut self) -> u8 {
        let value = self.bytes[self.pos];
        self.pos += 1;
        value
    }

    fn i8(&mut self) -> i8 {
        i8::from_le_bytes([self.u8()])
    }

    fn u16(&mut self) -> u16 {
        u16::from_le_bytes([self.u8(), self.u8()])
    }

    fn i16(&mut self) -> i16 {
        i16::from_le_bytes([self.u8(), self.u8()])
    }

    fn u32(&mut self) -> u32 {
        u32::from_le_bytes([self.u8(), self.u8(), self.u8(), self.u8()])
    }

    fn bytes(&mut self, count: usize) -> &[u8] {
        let slice = &self.bytes[self.pos..self.pos + count];
        self.pos += count;
        slice
    }
}

/// A newer lamp may add fields at the end of a body, so only a body shorter
/// than this version expects is an error.
fn expect_size(kind: u8, body: &[u8], expected: usize) -> Result<(), DecodeError> {
    if body.len() >= expected {
        Ok(())
    } else {
        Err(DecodeError::BodyTooShort(kind, body.len(), expected))
    }
}

/// Decodes a message payload, the bytes after the report ID.
pub fn decode_message(payload: &[u8]) -> Result<Frame, DecodeError> {
    if payload.len() < HEADER_SIZE {
        return Err(DecodeError::TooShort(payload.len()));
    }
    let (kind, seq, length) = (payload[0], payload[1], payload[2] as usize);
    if HEADER_SIZE + length > payload.len() || HEADER_SIZE + length > PAYLOAD_SIZE {
        return Err(DecodeError::BadLength(length));
    }
    let body = &payload[HEADER_SIZE..HEADER_SIZE + length];
    let mut r = Reader {
        bytes: body,
        pos: 0,
    };

    let message = match kind {
        MSG_INFO => {
            expect_size(kind, body, 56)?;
            let protocol = [r.u8(), r.u8()];
            let firmware = [r.u8(), r.u8(), r.u8()];
            let board_revision = r.u8();
            let uid = r.bytes(12).iter().map(|b| format!("{b:02X}")).collect();
            let capabilities = r.u32();
            let reset_cause = ResetCause::from_wire(r.u8())?;
            let flags = r.u8();
            let (mcause, mepc) = (r.u32(), r.u32());
            let text = r.bytes(24);
            let end = text.iter().position(|&b| b == 0).unwrap_or(text.len());
            Message::Info(Info {
                protocol,
                firmware,
                firmware_version: String::from_utf8_lossy(&text[..end]).into_owned(),
                board_revision,
                uid,
                telemetry: capabilities & (1 << 0) != 0,
                debug_console: capabilities & (1 << 1) != 0,
                auto_bias: capabilities & (1 << 2) != 0,
                diagnostics: capabilities & (1 << 3) != 0,
                self_test: capabilities & (1 << 4) != 0,
                enter_bootloader: capabilities & (1 << 5) != 0,
                reset_cause,
                debug_build: flags & 0x1 != 0,
                fault: (flags & 0x2 != 0).then_some(Fault { mcause, mepc }),
            })
        }
        MSG_STATE => {
            expect_size(kind, body, 9)?;
            Message::State(State {
                state_seq: r.u16(),
                on: r.u8() != 0,
                color: LampColor::from_wire(r.u8())?,
                brightness_mode: BrightnessMode::from_wire(r.u8())?,
                manual_level: r.u8(),
                applied_level: r.u8(),
                link: UsbLink::from_wire(r.u8())?,
                source: ChangeSource::from_wire(r.u8())?,
            })
        }
        MSG_TELEMETRY => {
            expect_size(kind, body, 32)?;
            Message::Telemetry(Telemetry {
                uptime_ms: r.u32(),
                touch_raw: r.u16(),
                touch_value: r.u16(),
                touch_baseline: r.u16(),
                touch_delta_permille: r.i16(),
                touch_state: TouchState::from_wire(r.u8())?,
                lux_centi: r.u32(),
                als_raw: r.u16(),
                duty: [r.u16(), r.u16(), r.u16()],
                led_current_ma_x10: r.u16(),
                cap_ma: r.u16(),
                link: UsbLink::from_wire(r.u8())?,
                level_permille: r.u16(),
            })
        }
        MSG_DIAGNOSTICS => {
            expect_size(kind, body, 36)?;
            let uptime_ms = r.u32();
            let (boots, watchdog_resets, faults) = (r.u16(), r.u16(), r.u16());
            let (mcause, mepc, mtval) = (r.u32(), r.u32(), r.u32());
            let stack_free_min = r.u16();
            let i2c_errors = r.u16();
            let light_sensor_probes = r.u16();
            let touch_samples_lost = r.u16();
            let touch_recalibrations = r.u16();
            let usb_suspends = r.u16();
            let auto_bias = r.i8();
            let flags = r.u8();
            Message::Diagnostics(Diagnostics {
                uptime_ms,
                boots,
                watchdog_resets,
                faults,
                // mcause 0 is a real exception, a misaligned fetch, so the
                // counter tells whether a fault happened.
                last_fault: (faults != 0).then_some(FaultDetail {
                    mcause,
                    mepc,
                    mtval,
                }),
                stack_free_min,
                i2c_errors,
                light_sensor_probes,
                touch_samples_lost,
                touch_recalibrations,
                usb_suspends,
                auto_bias,
                light_sensor_present: flags & 0x1 != 0,
                crystal_running: flags & 0x2 != 0,
            })
        }
        MSG_SELF_TEST_RESULT => {
            expect_size(kind, body, 9)?;
            let results = r.u8();
            Message::SelfTestResult(SelfTestResult {
                light_sensor_ok: results & 0x1 != 0,
                touch_idle_ok: results & 0x2 != 0,
                touch_pressed: results & 0x4 != 0,
                completed: results & 0x8 != 0,
                lux_centi: r.u32(),
                touch_baseline: r.u16(),
                touch_peak_permille: r.i16(),
            })
        }
        MSG_ACK => {
            expect_size(kind, body, 1)?;
            Message::Ack { command: r.u8() }
        }
        MSG_ERROR => {
            expect_size(kind, body, 2)?;
            Message::Error {
                command: r.u8(),
                error: ProtocolError::from_wire(r.u8())?,
            }
        }
        other => return Err(DecodeError::UnknownType(other)),
    };
    Ok(Frame { seq, message })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn vectors() -> Value {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../protocol/vectors/v1.json"
        );
        let text = std::fs::read_to_string(path).expect("protocol/vectors/v1.json");
        serde_json::from_str(&text).expect("valid JSON")
    }

    fn payload(hex: &str) -> [u8; PAYLOAD_SIZE] {
        let mut payload = [0u8; PAYLOAD_SIZE];
        for (i, pair) in hex.split_whitespace().enumerate() {
            payload[i] = u8::from_str_radix(pair, 16).expect("hex byte");
        }
        payload
    }

    fn u(v: &Value, key: &str) -> u64 {
        v[key].as_u64().unwrap_or_else(|| panic!("number {key}"))
    }

    fn i(v: &Value, key: &str) -> i64 {
        v[key].as_i64().unwrap_or_else(|| panic!("number {key}"))
    }

    #[test]
    fn protocol_version_matches_the_vectors() {
        let doc = vectors();
        assert_eq!(
            doc["protocol_version"][0].as_u64(),
            Some(u64::from(PROTOCOL_MAJOR))
        );
        assert_eq!(
            doc["protocol_version"][1].as_u64(),
            Some(u64::from(PROTOCOL_MINOR))
        );
    }

    #[test]
    fn encodes_every_canonical_command_vector() {
        let doc = vectors();
        let mut checked = 0;
        for v in doc["commands"].as_array().expect("commands") {
            if v["status"] != "ok" || v["canonical"] == Value::Bool(false) {
                continue;
            }
            let command = match u(v, "type") as u8 {
                CMD_GET_INFO => Command::GetInfo,
                CMD_GET_STATE => Command::GetState,
                CMD_RESET => Command::Reset,
                CMD_GET_DIAGNOSTICS => Command::GetDiagnostics,
                CMD_SELF_TEST => Command::SelfTest,
                CMD_ENTER_BOOTLOADER => Command::EnterBootloader,
                CMD_SET_AUTO_BIAS => Command::SetAutoBias {
                    bias: i(v, "auto_bias") as i8,
                },
                CMD_SET_TELEMETRY => Command::SetTelemetry {
                    period_ms: u(v, "telemetry_period_ms") as u16,
                },
                CMD_SET_STATE => {
                    let s = &v["set_state"];
                    let fields = u(s, "fields") as u8;
                    Command::SetState(SetState {
                        on: (fields & FIELD_POWER != 0).then(|| s["on"].as_bool().expect("on")),
                        color: (fields & FIELD_COLOR != 0)
                            .then(|| LampColor::from_wire(u(s, "color") as u8).expect("color")),
                        brightness_mode: (fields & FIELD_BRIGHTNESS_MODE != 0).then(|| {
                            BrightnessMode::from_wire(u(s, "brightness_mode") as u8).expect("mode")
                        }),
                        manual_level: (fields & FIELD_MANUAL_LEVEL != 0)
                            .then(|| u(s, "manual_level") as u8),
                    })
                }
                other => panic!("unexpected command type {other}"),
            };
            let name = v["name"].as_str().unwrap_or("?");
            let expected = payload(v["bytes"].as_str().expect("bytes"));
            assert_eq!(command.encode(u(v, "seq") as u8), expected, "{name}");
            checked += 1;
        }
        assert!(checked >= 10, "too few command vectors checked: {checked}");
    }

    #[test]
    fn decodes_every_message_vector() {
        let doc = vectors();
        for v in doc["messages"].as_array().expect("messages") {
            let name = v["name"].as_str().unwrap_or("?");
            let frame = decode_message(&payload(v["bytes"].as_str().expect("bytes")))
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(u64::from(frame.seq), u(v, "seq"), "{name}");
            match (v["kind"].as_str(), &frame.message) {
                (Some("state"), Message::State(s)) => {
                    let e = &v["state"];
                    assert_eq!(u64::from(s.state_seq), u(e, "state_seq"), "{name}");
                    assert_eq!(s.on, e["on"].as_bool().unwrap_or(false), "{name}");
                    assert_eq!(s.color as u64, u(e, "color"), "{name}");
                    assert_eq!(s.brightness_mode as u64, u(e, "brightness_mode"), "{name}");
                    assert_eq!(u64::from(s.manual_level), u(e, "manual_level"), "{name}");
                    assert_eq!(
                        u64::from(s.applied_level),
                        u(e, "applied_percent"),
                        "{name}"
                    );
                    assert_eq!(s.link as u64, u(e, "link"), "{name}");
                    assert_eq!(s.source as u64, u(e, "source"), "{name}");
                }
                (Some("info"), Message::Info(info)) => {
                    let e = &v["info"];
                    assert_eq!(info.protocol, [PROTOCOL_MAJOR, PROTOCOL_MINOR], "{name}");
                    assert_eq!(
                        info.firmware,
                        [
                            u(e, "fw_major") as u8,
                            u(e, "fw_minor") as u8,
                            u(e, "fw_patch") as u8
                        ]
                    );
                    assert_eq!(u64::from(info.board_revision), u(e, "board_revision"));
                    assert_eq!(info.uid, e["uid"].as_str().unwrap_or("").to_uppercase());
                    assert_eq!(
                        info.firmware_version,
                        e["fw_version"].as_str().unwrap_or("")
                    );
                    assert_eq!(info.reset_cause as u64, u(e, "reset_cause"));
                    let capabilities = u(e, "capabilities");
                    assert_eq!(info.telemetry, capabilities & 1 != 0);
                    assert_eq!(info.debug_console, capabilities & 2 != 0);
                    assert_eq!(info.auto_bias, capabilities & 4 != 0);
                    let flags = u(e, "flags");
                    assert_eq!(info.debug_build, flags & 1 != 0);
                    assert_eq!(
                        info.fault,
                        (flags & 2 != 0).then(|| Fault {
                            mcause: u(e, "fault_mcause") as u32,
                            mepc: u(e, "fault_mepc") as u32
                        })
                    );
                }
                (Some("telemetry"), Message::Telemetry(t)) => {
                    let e = &v["telemetry"];
                    assert_eq!(u64::from(t.uptime_ms), u(e, "uptime_ms"));
                    assert_eq!(u64::from(t.touch_raw), u(e, "touch_raw"));
                    assert_eq!(u64::from(t.touch_value), u(e, "touch_value"));
                    assert_eq!(u64::from(t.touch_baseline), u(e, "touch_baseline"));
                    assert_eq!(
                        i64::from(t.touch_delta_permille),
                        i(e, "touch_delta_permille")
                    );
                    assert_eq!(t.touch_state as u64, u(e, "touch_state"));
                    assert_eq!(u64::from(t.lux_centi), u(e, "lux_centi"));
                    assert_eq!(u64::from(t.als_raw), u(e, "als_raw"));
                    let duty: Vec<u64> = e["duty"]
                        .as_array()
                        .expect("duty")
                        .iter()
                        .filter_map(Value::as_u64)
                        .collect();
                    assert_eq!(
                        t.duty.iter().map(|&d| u64::from(d)).collect::<Vec<_>>(),
                        duty
                    );
                    assert_eq!(u64::from(t.led_current_ma_x10), u(e, "led_current_ma_x10"));
                    assert_eq!(u64::from(t.cap_ma), u(e, "cap_ma"));
                    assert_eq!(t.link as u64, u(e, "link"));
                    assert_eq!(u64::from(t.level_permille), u(e, "level_permille"));
                }
                (Some("diagnostics"), Message::Diagnostics(d)) => {
                    let e = &v["diagnostics"];
                    assert_eq!(u64::from(d.uptime_ms), u(e, "uptime_ms"), "{name}");
                    assert_eq!(u64::from(d.boots), u(e, "boots"), "{name}");
                    assert_eq!(u64::from(d.watchdog_resets), u(e, "watchdog_resets"));
                    assert_eq!(u64::from(d.faults), u(e, "faults"));
                    assert_eq!(
                        d.last_fault,
                        Some(FaultDetail {
                            mcause: u(e, "fault_mcause") as u32,
                            mepc: u(e, "fault_mepc") as u32,
                            mtval: u(e, "fault_mtval") as u32,
                        })
                    );
                    assert_eq!(u64::from(d.stack_free_min), u(e, "stack_free_min"));
                    assert_eq!(u64::from(d.i2c_errors), u(e, "i2c_errors"));
                    assert_eq!(
                        u64::from(d.light_sensor_probes),
                        u(e, "light_sensor_probes")
                    );
                    assert_eq!(u64::from(d.touch_samples_lost), u(e, "touch_samples_lost"));
                    assert_eq!(
                        u64::from(d.touch_recalibrations),
                        u(e, "touch_recalibrations")
                    );
                    assert_eq!(u64::from(d.usb_suspends), u(e, "usb_suspends"));
                    assert_eq!(i64::from(d.auto_bias), i(e, "auto_bias"));
                    let flags = u(e, "flags");
                    assert_eq!(d.light_sensor_present, flags & 1 != 0);
                    assert_eq!(d.crystal_running, flags & 2 != 0);
                }
                (Some("self_test_result"), Message::SelfTestResult(t)) => {
                    let e = &v["self_test_result"];
                    let flags = u(e, "flags");
                    assert_eq!(t.light_sensor_ok, flags & 1 != 0, "{name}");
                    assert_eq!(t.touch_idle_ok, flags & 2 != 0, "{name}");
                    assert_eq!(t.touch_pressed, flags & 4 != 0, "{name}");
                    assert_eq!(t.completed, flags & 8 != 0, "{name}");
                    assert_eq!(u64::from(t.lux_centi), u(e, "lux_centi"));
                    assert_eq!(u64::from(t.touch_baseline), u(e, "touch_baseline"));
                    assert_eq!(
                        i64::from(t.touch_peak_permille),
                        i(e, "touch_peak_permille")
                    );
                }
                (Some("ack"), Message::Ack { command }) => {
                    assert_eq!(u64::from(*command), u(v, "command"), "{name}");
                }
                (Some("error"), Message::Error { command, error }) => {
                    assert_eq!(u64::from(*command), u(v, "command"), "{name}");
                    assert_eq!(*error as u64, u(v, "error"), "{name}");
                }
                (kind, message) => panic!("{name}: kind {kind:?} decoded as {message:?}"),
            }
        }
    }

    #[test]
    fn rejects_malformed_messages() {
        assert_eq!(decode_message(&[0x82, 0]), Err(DecodeError::TooShort(2)));
        let mut p = [0u8; PAYLOAD_SIZE];
        p[0] = 0x82;
        p[2] = 61;
        assert_eq!(decode_message(&p), Err(DecodeError::BadLength(61)));
        p[2] = 8;
        assert_eq!(
            decode_message(&p),
            Err(DecodeError::BodyTooShort(0x82, 8, 9))
        );
        p[0] = 0x42;
        assert_eq!(decode_message(&p), Err(DecodeError::UnknownType(0x42)));
        let state = [0x82, 0, 9, 0, 0, 1, 9, 0, 50, 50, 1, 1];
        let mut p = [0u8; PAYLOAD_SIZE];
        p[..state.len()].copy_from_slice(&state);
        assert_eq!(
            decode_message(&p),
            Err(DecodeError::BadValue("LampColor", 9))
        );
    }

    #[test]
    fn self_test_passes_only_when_every_check_did() {
        let mut result = SelfTestResult {
            light_sensor_ok: true,
            touch_idle_ok: true,
            touch_pressed: true,
            completed: true,
            lux_centi: 0,
            touch_baseline: 0,
            touch_peak_permille: 0,
        };
        assert!(result.passed());
        result.touch_pressed = false;
        assert!(!result.passed());
    }

    #[test]
    fn commands_deserialize_from_the_frontend_shape() {
        let command: Command =
            serde_json::from_str(r#"{"kind":"set_state","color":"warm_white","manual_level":40}"#)
                .expect("set_state");
        assert_eq!(
            command,
            Command::SetState(SetState {
                color: Some(LampColor::WarmWhite),
                manual_level: Some(40),
                ..SetState::default()
            })
        );
        let command: Command = serde_json::from_str(r#"{"kind":"set_telemetry","period_ms":100}"#)
            .expect("set_telemetry");
        assert_eq!(command, Command::SetTelemetry { period_ms: 100 });
        let command: Command =
            serde_json::from_str(r#"{"kind":"set_auto_bias","bias":-20}"#).expect("bias");
        assert_eq!(command, Command::SetAutoBias { bias: -20 });
        let command: Command = serde_json::from_str(r#"{"kind":"self_test"}"#).expect("self_test");
        assert_eq!(command, Command::SelfTest);
    }

    #[test]
    fn only_the_lamp_hid_interface_matches() {
        assert!(is_lamp_interface(0x1209, 0x0001, 0xFF00));
        assert!(is_lamp_interface(0x1209, 0x0002, 0xFF00));
        assert!(is_lamp_interface(0x1209, 0x0001, 0));
        assert!(!is_lamp_interface(0x1209, 0x0003, 0xFF00));
        assert!(!is_lamp_interface(0x4348, 0x55E0, 0xFF00));
        assert!(!is_lamp_interface(0x1209, 0x0001, 0x0001));
    }
}
