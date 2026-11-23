// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! Command encoder and message decoder of the Disled USB HID protocol 3.0,
//! which `protocol/README.md` specifies. The functions do no I/O; the desktop
//! app and the production tool share them. INFO keeps the layout of versions
//! 1 and 2, so the decoder reads the INFO of an older lamp too: its protocol
//! major tells them apart.

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

const REPORT_ID_COMMAND: u8 = 1;
const REPORT_ID_MESSAGE: u8 = 2;
/// Payload bytes of one report, after the report ID.
pub const PAYLOAD_SIZE: usize = 63;
const HEADER_SIZE: usize = 3;
const MAX_BODY_SIZE: usize = PAYLOAD_SIZE - HEADER_SIZE;
pub const PROTOCOL_MAJOR: u8 = 3;
pub const PROTOCOL_MINOR: u8 = 0;

/// Automatic brightness bias limits, percent.
pub const AUTO_BIAS_MIN: i8 = -50;
pub const AUTO_BIAS_MAX: i8 = 50;
/// Touch sensitivity correction limits, percent: positive for a more
/// sensitive lid.
pub const TOUCH_SENSITIVITY_MIN: i8 = -50;
pub const TOUCH_SENSITIVITY_MAX: i8 = 50;

/// Palette slots. The first four hold the built-in colors, which never
/// change; the host defines the custom ones, from `FIRST_CUSTOM_SLOT` on.
pub const SLOT_COUNT: u8 = 16;
pub const SLOT_GREEN: u8 = 0;
pub const SLOT_ORANGE: u8 = 1;
pub const SLOT_RED: u8 = 2;
pub const SLOT_WARM_WHITE: u8 = 3;
pub const FIRST_CUSTOM_SLOT: u8 = 4;
/// Most slots in the touch cycle.
pub const TOUCH_CYCLE_MAX: usize = 9;

const CMD_GET_INFO: u8 = 0x01;
const CMD_GET_STATE: u8 = 0x02;
const CMD_SET_STATE: u8 = 0x03;
const CMD_SET_TELEMETRY: u8 = 0x04;
const CMD_RESET: u8 = 0x05;
const CMD_SET_AUTO_BIAS: u8 = 0x06;
const CMD_GET_DIAGNOSTICS: u8 = 0x07;
const CMD_SELF_TEST: u8 = 0x08;
// 0x09 and 0x0F are reserved.
const CMD_SET_COLOR: u8 = 0x0A;
const CMD_GET_COLOR: u8 = 0x0B;
const CMD_SET_TOUCH_CYCLE: u8 = 0x0C;
const CMD_GET_TOUCH_CYCLE: u8 = 0x0D;
const CMD_ALERT: u8 = 0x0E;
const CMD_SET_TOUCH_SENSITIVITY: u8 = 0x10;
const CMD_GET_GAMUT: u8 = 0x11;

const MSG_INFO: u8 = 0x81;
const MSG_STATE: u8 = 0x82;
const MSG_TELEMETRY: u8 = 0x83;
const MSG_DIAGNOSTICS: u8 = 0x84;
const MSG_SELF_TEST_RESULT: u8 = 0x85;
const MSG_COLOR: u8 = 0x86;
const MSG_TOUCH_CYCLE: u8 = 0x87;
const MSG_ALERT_DONE: u8 = 0x88;
const MSG_TOUCH: u8 = 0x89;
const MSG_GAMUT: u8 = 0x8A;
const MSG_ACK: u8 = 0x8E;
const MSG_ERROR: u8 = 0x8F;

const FIELD_POWER: u8 = 0x01;
const FIELD_COLOR: u8 = 0x02;
const FIELD_BRIGHTNESS_MODE: u8 = 0x04;
const FIELD_MANUAL_LEVEL: u8 = 0x08;
const FIELD_TOUCH_MODE: u8 = 0x10;

/// An enum whose discriminant is its byte on the wire.
macro_rules! wire_enum {
    (
        $(#[$meta:meta])*
        $name:ident { $($(#[$variant_meta:meta])* $variant:ident = $value:expr),+ $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($(#[$variant_meta])* $variant = $value),+ }

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

wire_enum!(BrightnessMode { Auto = 0, Manual = 1 });
wire_enum!(
    /// What the touch does. Integrations drive the color in power-only mode,
    /// where a long press still turns the lamp on and off.
    TouchMode { Normal = 0, PowerOnly = 1 }
);
wire_enum!(UsbLink { Unconfigured = 0, Configured = 1, Suspended = 2 });
wire_enum!(ChangeSource {
    Boot = 0,
    Touch = 1,
    Host = 2,
    AutoBrightness = 3,
    Usb = 4,
    /// Power-only touch back to normal: no command for too long.
    HostSilence = 5,
});
wire_enum!(ResetCause {
    Unknown = 0,
    PowerOn = 1,
    Pin = 2,
    Software = 3,
    Watchdog = 4,
});
wire_enum!(TouchState { Calibrating = 0, Idle = 1, Pressed = 2 });
wire_enum!(
    /// A press on the lid that a TOUCH message tells of.
    Press { Short = 0 }
);
wire_enum!(ProtocolError {
    UnknownCommand = 1,
    BadLength = 2,
    BadValue = 3,
    Busy = 4,
    /// No alert while the lamp is off.
    LampOff = 5,
});
wire_enum!(
    /// Why an alert ended.
    AlertDoneReason {
        /// Every breath shown.
        Completed = 0,
        LampOff = 1,
        /// USB suspend; the message comes after the wake-up.
        Suspended = 2,
        /// Stopped by an ALERT with a count of 0.
        Cancelled = 3,
        /// The self-test took the LED.
        SelfTest = 4,
    }
);

/// A color: the CIE 1931 chromaticity of its light, x and y times
/// `Xy::ONE`, as Zigbee and Matter carry them. `Xy::NONE` empties a custom
/// slot.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Xy {
    pub x: u16,
    pub y: u16,
}

impl Xy {
    /// The coordinates are x and y times this.
    pub const ONE: u32 = 65536;
    /// No color.
    pub const NONE: Xy = Xy { x: 0, y: 0 };
    /// The D65 white, x 0.3127 and y 0.3290, the white of the lamp.
    pub const D65: Xy = Xy { x: 20493, y: 21561 };

    /// A chromaticity: y above 0, and x + y at most 1. The lamp refuses
    /// anything else but `NONE`.
    pub fn is_chromaticity(self) -> bool {
        self.y != 0 && u32::from(self.x) + u32::from(self.y) <= Self::ONE
    }

    /// The nearest chromaticity to the coordinates x and y, none for
    /// coordinates of anything but a chromaticity.
    pub fn from_coordinates(x: f64, y: f64) -> Option<Xy> {
        let scale = |value: f64| (value * f64::from(Self::ONE)).round();
        let (x, y) = (scale(x), scale(y));
        let range = 0.0..=f64::from(u16::MAX);
        if !range.contains(&x) || !range.contains(&y) {
            return None;
        }
        Some(Xy {
            x: x as u16,
            y: y as u16,
        })
        .filter(|xy| xy.is_chromaticity())
    }

    /// The coordinates x and y, from 0 to 1.
    pub fn coordinates(self) -> (f64, f64) {
        let one = f64::from(Self::ONE);
        (f64::from(self.x) / one, f64::from(self.y) / one)
    }
}

/// The chromaticities of the LED primaries of a lamp: it shows every color
/// of their triangle as it is, and brings the others inside.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Gamut {
    pub red: Xy,
    pub green: Xy,
    pub blue: Xy,
}

/// Lamp settings to change; `None` leaves a setting as it is.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export, optional_fields))]
pub struct SetState {
    pub on: Option<bool>,
    /// A palette slot that holds a color.
    pub color: Option<u8>,
    pub brightness_mode: Option<BrightnessMode>,
    /// Percent, 1 to 100.
    pub manual_level: Option<u8>,
    pub touch_mode: Option<TouchMode>,
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
    /// Percent, `AUTO_BIAS_MIN` to `AUTO_BIAS_MAX`.
    SetAutoBias {
        bias: i8,
    },
    GetDiagnostics,
    SelfTest,
    /// Defines a custom slot, `FIRST_CUSTOM_SLOT` to `SLOT_COUNT` - 1, with a
    /// chromaticity, or empties it with `Xy::NONE`.
    SetColor {
        slot: u8,
        xy: Xy,
    },
    /// Any slot, 0 to `SLOT_COUNT` - 1.
    GetColor {
        slot: u8,
    },
    /// 1 to `TOUCH_CYCLE_MAX` slots that hold a color, none twice.
    SetTouchCycle {
        slots: Vec<u8>,
    },
    GetTouchCycle,
    /// 1 to 10 breaths of 500 to 5000 ms; a count of 0 stops the alert and
    /// ignores the period.
    Alert {
        count: u8,
        period_ms: u16,
    },
    /// Percent, `TOUCH_SENSITIVITY_MIN` to `TOUCH_SENSITIVITY_MAX`.
    SetTouchSensitivity {
        percent: i8,
    },
    GetGamut,
}

/// The sequence number of the command after one sent with `seq`. 0 marks
/// the unsolicited messages, so a command never takes it.
pub fn next_seq(seq: u8) -> u8 {
    if seq == u8::MAX { 1 } else { seq + 1 }
}

impl Command {
    /// The code of the command, which ACK and ERROR name.
    pub fn code(&self) -> u8 {
        self.encode(0)[0]
    }

    /// The HID report that carries the command: its report ID, then its
    /// payload.
    pub fn report(&self, seq: u8) -> [u8; PAYLOAD_SIZE + 1] {
        let mut report = [0u8; PAYLOAD_SIZE + 1];
        report[0] = REPORT_ID_COMMAND;
        report[1..].copy_from_slice(&self.encode(seq));
        report
    }

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
                if s.touch_mode.is_some() {
                    fields |= FIELD_TOUCH_MODE;
                }
                (
                    CMD_SET_STATE,
                    vec![
                        fields,
                        u8::from(s.on.unwrap_or(false)),
                        s.color.unwrap_or(0),
                        s.brightness_mode.map_or(0, |m| m as u8),
                        s.manual_level.unwrap_or(0),
                        s.touch_mode.map_or(0, |m| m as u8),
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
            Command::SetColor { slot, xy } => {
                let mut body = vec![*slot];
                body.extend_from_slice(&xy.x.to_le_bytes());
                body.extend_from_slice(&xy.y.to_le_bytes());
                (CMD_SET_COLOR, body)
            }
            Command::GetColor { slot } => (CMD_GET_COLOR, vec![*slot]),
            Command::SetTouchCycle { slots } => {
                // A longer cycle goes out with its true count, which the lamp
                // refuses, but never past the report.
                let count = u8::try_from(slots.len()).unwrap_or(u8::MAX);
                let kept = slots.len().min(MAX_BODY_SIZE - 1);
                let mut body = vec![count];
                body.extend_from_slice(&slots[..kept]);
                (CMD_SET_TOUCH_CYCLE, body)
            }
            Command::GetTouchCycle => (CMD_GET_TOUCH_CYCLE, vec![]),
            Command::Alert { count, period_ms } => {
                let mut body = vec![*count];
                body.extend_from_slice(&period_ms.to_le_bytes());
                (CMD_ALERT, body)
            }
            Command::SetTouchSensitivity { percent } => {
                (CMD_SET_TOUCH_SENSITIVITY, percent.to_le_bytes().to_vec())
            }
            Command::GetGamut => (CMD_GET_GAMUT, vec![]),
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

/// INFO, whose layout is the same in versions 1 to 3. It leaves out the
/// capability bits: every version 3 lamp has all the commands of this codec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Info {
    /// Major and minor version of the protocol the lamp speaks.
    pub protocol: [u8; 2],
    pub firmware: [u8; 3],
    pub firmware_version: String,
    pub board_revision: u8,
    /// Chip unique ID in hexadecimal, as in the USB serial number.
    pub uid: String,
    pub reset_cause: ResetCause,
    pub debug_build: bool,
    pub fault: Option<Fault>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export, rename = "LampState"))]
pub struct State {
    pub state_seq: u16,
    pub on: bool,
    /// Palette slot of the color, shown while on and kept while off.
    pub color: u8,
    pub brightness_mode: BrightnessMode,
    pub manual_level: u8,
    /// Brightness in use, percent.
    pub applied_level: u8,
    pub link: UsbLink,
    pub source: ChangeSource,
    pub touch_mode: TouchMode,
    /// The chromaticity of the slot.
    pub xy: Xy,
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
    /// Touch sensitivity correction in force, percent.
    pub touch_sensitivity: i8,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Message {
    Info(Info),
    State(State),
    Telemetry(Telemetry),
    Diagnostics(Diagnostics),
    SelfTestResult(SelfTestResult),
    /// A slot and its chromaticity, `Xy::NONE` when the slot is empty.
    Color {
        slot: u8,
        xy: Xy,
    },
    Gamut(Gamut),
    TouchCycle {
        slots: Vec<u8>,
    },
    /// Carries the sequence number of the ALERT that started the alert.
    AlertDone {
        reason: AlertDoneReason,
    },
    /// A short press on the lid, whatever the lamp did with it.
    Touch {
        press: Press,
    },
    Ack {
        command: u8,
    },
    Error {
        command: u8,
        error: ProtocolError,
    },
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
    /// Also the STATE of a version 1 lamp, 9 bytes long.
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

    fn xy(&mut self) -> Xy {
        Xy {
            x: self.u16(),
            y: self.u16(),
        }
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

/// Decodes a HID report that the lamp sent, report ID first. None for an
/// empty report or one of another ID, which carries no message.
pub fn decode_report(report: &[u8]) -> Option<Result<Frame, DecodeError>> {
    let (&id, payload) = report.split_first()?;
    (id == REPORT_ID_MESSAGE).then(|| decode_message(payload))
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
            let _capabilities = r.u32();
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
                reset_cause,
                debug_build: flags & 0x1 != 0,
                fault: (flags & 0x2 != 0).then_some(Fault { mcause, mepc }),
            })
        }
        MSG_STATE => {
            expect_size(kind, body, 14)?;
            Message::State(State {
                state_seq: r.u16(),
                on: r.u8() != 0,
                color: r.u8(),
                brightness_mode: BrightnessMode::from_wire(r.u8())?,
                manual_level: r.u8(),
                applied_level: r.u8(),
                link: UsbLink::from_wire(r.u8())?,
                source: ChangeSource::from_wire(r.u8())?,
                touch_mode: TouchMode::from_wire(r.u8())?,
                xy: r.xy(),
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
            expect_size(kind, body, 37)?;
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
            let touch_sensitivity = r.i8();
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
                touch_sensitivity,
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
        MSG_COLOR => {
            expect_size(kind, body, 5)?;
            Message::Color {
                slot: r.u8(),
                xy: r.xy(),
            }
        }
        MSG_GAMUT => {
            expect_size(kind, body, 12)?;
            Message::Gamut(Gamut {
                red: r.xy(),
                green: r.xy(),
                blue: r.xy(),
            })
        }
        MSG_TOUCH_CYCLE => {
            expect_size(kind, body, 1 + TOUCH_CYCLE_MAX)?;
            let count = r.u8();
            if usize::from(count) > TOUCH_CYCLE_MAX {
                return Err(DecodeError::BadValue("touch cycle length", count));
            }
            Message::TouchCycle {
                slots: r.bytes(usize::from(count)).to_vec(),
            }
        }
        MSG_ALERT_DONE => {
            expect_size(kind, body, 1)?;
            Message::AlertDone {
                reason: AlertDoneReason::from_wire(r.u8())?,
            }
        }
        MSG_TOUCH => {
            expect_size(kind, body, 1)?;
            Message::Touch {
                press: Press::from_wire(r.u8())?,
            }
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
            "/../../protocol/vectors/v3.json"
        );
        let text = std::fs::read_to_string(path).expect("protocol/vectors/v3.json");
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

    fn xy(v: &Value) -> Xy {
        let coordinates: Vec<u16> = v
            .as_array()
            .expect("xy")
            .iter()
            .map(|c| c.as_u64().expect("coordinate") as u16)
            .collect();
        Xy {
            x: coordinates[0],
            y: coordinates[1],
        }
    }

    fn slots(cycle: &Value) -> Vec<u8> {
        let all: Vec<u8> = cycle["slots"]
            .as_array()
            .expect("slots")
            .iter()
            .map(|s| s.as_u64().expect("slot") as u8)
            .collect();
        all[..u(cycle, "count") as usize].to_vec()
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
                CMD_GET_TOUCH_CYCLE => Command::GetTouchCycle,
                CMD_GET_GAMUT => Command::GetGamut,
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
                        color: (fields & FIELD_COLOR != 0).then(|| u(s, "color") as u8),
                        brightness_mode: (fields & FIELD_BRIGHTNESS_MODE != 0).then(|| {
                            BrightnessMode::from_wire(u(s, "brightness_mode") as u8).expect("mode")
                        }),
                        manual_level: (fields & FIELD_MANUAL_LEVEL != 0)
                            .then(|| u(s, "manual_level") as u8),
                        touch_mode: (fields & FIELD_TOUCH_MODE != 0).then(|| {
                            TouchMode::from_wire(u(s, "touch_mode") as u8).expect("touch mode")
                        }),
                    })
                }
                CMD_SET_COLOR => {
                    let c = &v["set_color"];
                    Command::SetColor {
                        slot: u(c, "slot") as u8,
                        xy: xy(&c["xy"]),
                    }
                }
                CMD_GET_COLOR => Command::GetColor {
                    slot: u(v, "color_slot") as u8,
                },
                CMD_SET_TOUCH_CYCLE => Command::SetTouchCycle {
                    slots: slots(&v["touch_cycle"]),
                },
                CMD_ALERT => {
                    let a = &v["alert"];
                    Command::Alert {
                        count: u(a, "count") as u8,
                        period_ms: u(a, "period_ms") as u16,
                    }
                }
                CMD_SET_TOUCH_SENSITIVITY => Command::SetTouchSensitivity {
                    percent: i(v, "touch_sensitivity") as i8,
                },
                other => panic!("unexpected command type {other}"),
            };
            let name = v["name"].as_str().unwrap_or("?");
            let expected = payload(v["bytes"].as_str().expect("bytes"));
            assert_eq!(command.encode(u(v, "seq") as u8), expected, "{name}");
            checked += 1;
        }
        assert!(checked >= 20, "too few command vectors checked: {checked}");
    }

    #[test]
    fn decodes_every_message_vector() {
        let doc = vectors();
        for v in doc["messages"].as_array().expect("messages") {
            let name = v["name"].as_str().unwrap_or("?");
            let decoded = decode_message(&payload(v["bytes"].as_str().expect("bytes")));
            // Messages of an older lamp that this version refuses.
            if let Some(error) = v["decode_error"].as_str() {
                match (error, &decoded) {
                    ("body_too_short", Err(DecodeError::BodyTooShort(..)))
                    | ("bad_value", Err(DecodeError::BadValue(..))) => continue,
                    _ => panic!("{name}: expected {error}, got {decoded:?}"),
                }
            }
            let frame = decoded.unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(u64::from(frame.seq), u(v, "seq"), "{name}");
            match (v["kind"].as_str(), &frame.message) {
                (Some("state"), Message::State(s)) => {
                    let e = &v["state"];
                    assert_eq!(u64::from(s.state_seq), u(e, "state_seq"), "{name}");
                    assert_eq!(s.on, e["on"].as_bool().unwrap_or(false), "{name}");
                    assert_eq!(u64::from(s.color), u(e, "color"), "{name}");
                    assert_eq!(s.brightness_mode as u64, u(e, "brightness_mode"), "{name}");
                    assert_eq!(u64::from(s.manual_level), u(e, "manual_level"), "{name}");
                    assert_eq!(
                        u64::from(s.applied_level),
                        u(e, "applied_percent"),
                        "{name}"
                    );
                    assert_eq!(s.link as u64, u(e, "link"), "{name}");
                    assert_eq!(s.source as u64, u(e, "source"), "{name}");
                    assert_eq!(s.touch_mode as u64, u(e, "touch_mode"), "{name}");
                    assert_eq!(s.xy, xy(&e["xy"]), "{name}");
                }
                (Some("info"), Message::Info(info)) => {
                    let e = &v["info"];
                    let protocol: Vec<u8> = e["protocol"]
                        .as_array()
                        .expect("protocol")
                        .iter()
                        .map(|part| part.as_u64().expect("version") as u8)
                        .collect();
                    assert_eq!(info.protocol.to_vec(), protocol, "{name}");
                    assert_eq!(
                        info.firmware,
                        [
                            u(e, "fw_major") as u8,
                            u(e, "fw_minor") as u8,
                            u(e, "fw_patch") as u8
                        ],
                        "{name}"
                    );
                    assert_eq!(u64::from(info.board_revision), u(e, "board_revision"));
                    assert_eq!(info.uid, e["uid"].as_str().unwrap_or("").to_uppercase());
                    assert_eq!(
                        info.firmware_version,
                        e["fw_version"].as_str().unwrap_or("")
                    );
                    assert_eq!(info.reset_cause as u64, u(e, "reset_cause"));
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
                    assert_eq!(
                        i64::from(d.touch_sensitivity),
                        i(e, "touch_sensitivity"),
                        "{name}"
                    );
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
                (Some("color"), Message::Color { slot, xy: color }) => {
                    let e = &v["color"];
                    assert_eq!(u64::from(*slot), u(e, "slot"), "{name}");
                    assert_eq!(*color, xy(&e["xy"]), "{name}");
                }
                (Some("gamut"), Message::Gamut(gamut)) => {
                    let e = &v["gamut"];
                    let expected = Gamut {
                        red: xy(&e["red"]),
                        green: xy(&e["green"]),
                        blue: xy(&e["blue"]),
                    };
                    assert_eq!(*gamut, expected, "{name}");
                }
                (Some("touch_cycle"), Message::TouchCycle { slots: cycle }) => {
                    assert_eq!(*cycle, slots(&v["touch_cycle"]), "{name}");
                }
                (Some("alert_done"), Message::AlertDone { reason }) => {
                    assert_eq!(*reason as u64, u(v, "reason"), "{name}");
                }
                (Some("touch"), Message::Touch { press }) => {
                    assert_eq!(*press as u64, u(v, "press"), "{name}");
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
        p[2] = 9;
        assert_eq!(
            decode_message(&p),
            Err(DecodeError::BodyTooShort(0x82, 9, 14))
        );
        p[0] = 0x42;
        assert_eq!(decode_message(&p), Err(DecodeError::UnknownType(0x42)));

        // A STATE with a brightness mode of 9.
        let state = [
            0x82, 0, 14, 0, 0, 1, 4, 9, 50, 50, 1, 1, 0, 0x0D, 0x50, 0x39, 0x54,
        ];
        let mut p = [0u8; PAYLOAD_SIZE];
        p[..state.len()].copy_from_slice(&state);
        assert_eq!(
            decode_message(&p),
            Err(DecodeError::BadValue("BrightnessMode", 9))
        );

        // A TOUCH_CYCLE of 10 slots, one more than a cycle holds.
        let mut p = [0u8; PAYLOAD_SIZE];
        p[..4].copy_from_slice(&[0x87, 0, 10, 10]);
        assert_eq!(
            decode_message(&p),
            Err(DecodeError::BadValue("touch cycle length", 10))
        );
    }

    #[test]
    fn a_touch_cycle_too_long_still_fits_the_report() {
        let slots: Vec<u8> = (0..100).map(|s| s as u8).collect();
        let payload = Command::SetTouchCycle { slots }.encode(1);
        assert_eq!(payload[2] as usize, MAX_BODY_SIZE);
        assert_eq!(
            payload[HEADER_SIZE], 100,
            "the true count, which the lamp refuses"
        );
    }

    #[test]
    fn commands_leave_in_report_1_and_messages_come_in_report_2() {
        // After 255, the sequence numbers start again at 1.
        assert_eq!(next_seq(0), 1);
        assert_eq!(next_seq(u8::MAX), 1);
        assert_eq!(Command::GetInfo.report(1)[..4], [1, 0x01, 1, 0]);

        let mut report = [0u8; PAYLOAD_SIZE + 1];
        report[..5].copy_from_slice(&[2, 0x8E, 7, 1, 0x05]);
        assert_eq!(
            decode_report(&report),
            Some(Ok(Frame {
                seq: 7,
                message: Message::Ack { command: 0x05 }
            }))
        );
        report[0] = 1;
        assert_eq!(decode_report(&report), None);
        assert_eq!(decode_report(&[]), None);
    }

    #[test]
    fn commands_deserialize_from_the_frontend_shape() {
        let command: Command = serde_json::from_str(r#"{"kind":"set_telemetry","period_ms":100}"#)
            .expect("set_telemetry");
        assert_eq!(command, Command::SetTelemetry { period_ms: 100 });
        let command: Command = serde_json::from_str(r#"{"kind":"self_test"}"#).expect("self_test");
        assert_eq!(command, Command::SelfTest);
    }

    #[test]
    fn coordinates_go_to_the_nearest_chromaticity_and_back() {
        assert_eq!(Xy::from_coordinates(0.3127, 0.3290), Some(Xy::D65));
        let (x, y) = Xy::D65.coordinates();
        assert!((x - 0.3127).abs() < 1e-5 && (y - 0.3290).abs() < 1e-5);
        assert_eq!(
            Xy::from_coordinates(0.0, 1.0 - 1.0 / 65536.0),
            Some(Xy { x: 0, y: 65535 })
        );
        // No y, beyond x + y = 1, below 0, past 1, not a number.
        for (x, y) in [
            (0.3, 0.0),
            (0.6, 0.5),
            (-0.1, 0.3),
            (0.3, 1.0),
            (f64::NAN, 0.3),
        ] {
            assert_eq!(Xy::from_coordinates(x, y), None, "{x} {y}");
        }
        assert!(Xy::D65.is_chromaticity());
        assert!(!Xy::NONE.is_chromaticity());
        assert!(Xy { x: 30000, y: 35536 }.is_chromaticity());
        assert!(!Xy { x: 30000, y: 35537 }.is_chromaticity());
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
