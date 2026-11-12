// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! A lamp on USB, reached through its HID interface with blocking requests
//! and replies.

use std::time::{Duration, Instant};

use disled_protocol::{
    self as protocol, Command, DecodeError, Message, PAYLOAD_SIZE, REPORT_ID_COMMAND,
    REPORT_ID_MESSAGE, USB_VENDOR_ID,
};
use hidapi::{DeviceInfo, HidApi, HidDevice};

const READ_SLICE_MS: i32 = 20;

pub struct Lamp {
    device: HidDevice,
    seq: u8,
}

/// Lists the HID devices of the lamp's vendor only. hidapi then skips the
/// other devices before reading their strings, so a slow one cannot stall the
/// scan.
fn scan(api: &mut HidApi) -> bool {
    api.reset_devices().is_ok() && api.add_devices(USB_VENDOR_ID, 0).is_ok()
}

/// The HID interface of a lamp among the devices of the last scan.
fn lamp_interface(api: &HidApi) -> Option<&DeviceInfo> {
    api.device_list()
        .find(|d| protocol::is_lamp_interface(d.vendor_id(), d.product_id(), d.usage_page()))
}

/// Tells whether a lamp is connected.
pub fn present(api: &mut HidApi) -> bool {
    scan(api) && lamp_interface(api).is_some()
}

impl Lamp {
    /// Opens the connected lamp, if there is one.
    pub fn open(api: &mut HidApi) -> Option<Self> {
        if !scan(api) {
            return None;
        }
        let device = lamp_interface(api)?.open_device(api).ok()?;
        Some(Self { device, seq: 0 })
    }

    /// Sends a command; returns the sequence number its replies carry.
    pub fn send(&mut self, command: &Command) -> Result<u8, String> {
        // 0 marks unsolicited messages, so a request never uses it.
        self.seq = if self.seq == u8::MAX { 1 } else { self.seq + 1 };
        let mut report = [0u8; PAYLOAD_SIZE + 1];
        report[0] = REPORT_ID_COMMAND;
        report[1..].copy_from_slice(&command.encode(self.seq));
        self.device
            .write(&report)
            .map_err(|e| format!("the lamp is gone ({e})"))?;
        Ok(self.seq)
    }

    /// Waits for the next message that answers `seq`, skipping the others.
    /// `tick` runs between reads, with the time spent waiting.
    pub fn reply(
        &mut self,
        seq: u8,
        timeout: Duration,
        mut tick: impl FnMut(Duration),
    ) -> Result<Message, String> {
        let start = Instant::now();
        let mut buffer = [0u8; PAYLOAD_SIZE + 1];
        while start.elapsed() < timeout {
            tick(start.elapsed());
            let length = self
                .device
                .read_timeout(&mut buffer, READ_SLICE_MS)
                .map_err(|e| format!("the lamp is gone ({e})"))?;
            if length == 0 || buffer[0] != REPORT_ID_MESSAGE {
                continue;
            }
            match protocol::decode_message(&buffer[1..length]) {
                Ok(frame) if frame.seq == seq => return Ok(frame.message),
                Ok(_) | Err(DecodeError::UnknownType(_)) => {}
                Err(error) => return Err(format!("unreadable message: {error}")),
            }
        }
        Err("the lamp did not answer".into())
    }

    /// Sends a command and waits for its reply.
    pub fn request(&mut self, command: &Command, timeout: Duration) -> Result<Message, String> {
        let seq = self.send(command)?;
        self.reply(seq, timeout, |_| {})
    }
}
