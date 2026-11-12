// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! Prototype host tool of the Disled HID bootloader (firmware/bootloader/).
//! It needs no driver: the bootloader is a HID device.

mod protocol;

use std::process::ExitCode;
use std::thread::sleep;
use std::time::{Duration, Instant};

use disled_protocol as lamp;
use hidapi::{HidApi, HidDevice};
use protocol::{Info, REASON_APP_INVALID, REASON_REQUESTED, REPORT_SIZE, Session, Transport};

const USAGE: &str = "\
usage: disled-hidboot <command>

  info                        what the bootloader reports
  flash APP.bin [--tag N] [--no-boot]
                              write a firmware linked at 0x1000 (env release_bl or
                              debug_bl), then start it; asks a running lamp for
                              update mode first
  boot                        start the application
  enter                       ask a running lamp for update mode";

/// Flash operations take up to 20 ms, FINISH a CRC of 28 KB: well within.
const REPLY_TIMEOUT: Duration = Duration::from_secs(2);
/// From the reset of the application to the bootloader on the bus.
const APPEAR_TIMEOUT: Duration = Duration::from_secs(10);

struct Hid(HidDevice);

impl Transport for Hid {
    fn transact(&mut self, request: &[u8; REPORT_SIZE]) -> Result<[u8; REPORT_SIZE], String> {
        let mut report = [0u8; REPORT_SIZE + 1]; // report ID 0: the device has none
        report[1..].copy_from_slice(request);
        self.0
            .write(&report)
            .map_err(|e| format!("the bootloader is gone ({e})"))?;
        let start = Instant::now();
        let mut reply = [0u8; REPORT_SIZE];
        while start.elapsed() < REPLY_TIMEOUT {
            let length = self
                .0
                .read_timeout(&mut reply, 50)
                .map_err(|e| format!("the bootloader is gone ({e})"))?;
            if length == REPORT_SIZE && reply[..2] == request[..2] {
                return Ok(reply);
            }
        }
        Err(format!("no reply to command {:#04x}", request[0]))
    }
}

fn find_bootloader(api: &mut HidApi) -> Option<HidDevice> {
    api.reset_devices().ok()?;
    api.add_devices(protocol::VENDOR_ID, protocol::PRODUCT_ID)
        .ok()?;
    let device = api
        .device_list()
        .find(|d| d.usage_page() == protocol::USAGE_PAGE || d.usage_page() == 0)?;
    device.open_device(api).ok()
}

fn wait_for_bootloader(api: &mut HidApi) -> Result<Session<Hid>, String> {
    let start = Instant::now();
    loop {
        if let Some(device) = find_bootloader(api) {
            return Ok(Session::new(Hid(device)));
        }
        if start.elapsed() > APPEAR_TIMEOUT {
            return Err("the bootloader did not show up".into());
        }
        sleep(Duration::from_millis(200));
    }
}

/// ENTER_UPDATE of the firmware built for this bootloader (the `_bl`
/// environments). Protocol 2.0 reserves 0x09, so the codec does not know it.
const ENTER_UPDATE: u8 = 0x09;

/// Asks a running lamp for update mode: ENTER_UPDATE, which the application
/// answers with ACK before it resets into the bootloader.
fn enter(api: &mut HidApi) -> Result<(), String> {
    api.reset_devices().map_err(|e| e.to_string())?;
    api.add_devices(lamp::USB_VENDOR_ID, 0)
        .map_err(|e| e.to_string())?;
    let device = api
        .device_list()
        .find(|d| lamp::is_lamp_interface(d.vendor_id(), d.product_id(), d.usage_page()))
        .ok_or("no lamp and no bootloader on USB")?
        .open_device(api)
        .map_err(|e| e.to_string())?;

    // A frame without a body: type, seq 1, length 0.
    let mut report = [0u8; lamp::PAYLOAD_SIZE + 1];
    report[0] = lamp::REPORT_ID_COMMAND;
    report[1] = ENTER_UPDATE;
    report[2] = 1;
    device.write(&report).map_err(|e| e.to_string())?;

    let start = Instant::now();
    let mut buffer = [0u8; lamp::PAYLOAD_SIZE + 1];
    while start.elapsed() < REPLY_TIMEOUT {
        let length = device
            .read_timeout(&mut buffer, 50)
            .map_err(|e| e.to_string())?;
        if length == 0 || buffer[0] != lamp::REPORT_ID_MESSAGE {
            continue;
        }
        match lamp::decode_message(&buffer[1..length]) {
            Ok(frame) if frame.seq == 1 => {
                return match frame.message {
                    lamp::Message::Ack {
                        command: ENTER_UPDATE,
                    } => Ok(()),
                    _ => Err("the firmware of this lamp has no update mode (not a _bl \
                              build): flash the combined image once through the ROM \
                              bootloader (BOOT + RESET, wchisp)"
                        .into()),
                };
            }
            _ => continue,
        }
    }
    Err("the lamp did not answer".into())
}

fn open(api: &mut HidApi, enter_if_needed: bool) -> Result<Session<Hid>, String> {
    if let Some(device) = find_bootloader(api) {
        return Ok(Session::new(Hid(device)));
    }
    if !enter_if_needed {
        return Err("no bootloader on USB (try `enter`)".into());
    }
    enter(api)?;
    println!("Lamp restarting in update mode...");
    wait_for_bootloader(api)
}

fn print_info(info: &Info) {
    println!(
        "Bootloader protocol {}.{}",
        info.protocol[0], info.protocol[1]
    );
    let mut why = Vec::new();
    if info.reasons & REASON_REQUESTED != 0 {
        why.push("requested by the application");
    }
    if info.reasons & REASON_APP_INVALID != 0 {
        why.push("no valid application");
    }
    println!(
        "Update mode: {}",
        if why.is_empty() {
            "-".into()
        } else {
            why.join(", ")
        }
    );
    println!(
        "Application: {:#x}, up to {} bytes, pages of {} bytes",
        info.app_offset, info.app_max, info.page_size
    );
    match info.record {
        Some(r) => println!(
            "Installed: {} bytes, CRC-32 {:08X}, tag {:#x}",
            r.size, r.crc, r.tag
        ),
        None => println!("Installed: none"),
    }
}

fn flash(api: &mut HidApi, args: &[String]) -> Result<(), String> {
    let mut path = None;
    let mut tag = 0u32;
    let mut boot = true;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--no-boot" => boot = false,
            "--tag" => {
                let value = rest.next().ok_or("--tag needs a value")?;
                tag = match value.strip_prefix("0x") {
                    Some(hex) => u32::from_str_radix(hex, 16),
                    None => value.parse(),
                }
                .map_err(|_| format!("bad tag {value}"))?;
            }
            _ if path.is_none() => path = Some(arg.clone()),
            _ => return Err(USAGE.into()),
        }
    }
    let path = path.ok_or(USAGE)?;
    let image = std::fs::read(&path).map_err(|e| format!("{path}: {e}"))?;

    let mut session = open(api, true)?;
    let start = Instant::now();
    session
        .update(&image, tag, |done, total| {
            print!("\rWriting {done} / {total} bytes");
            use std::io::Write;
            let _ = std::io::stdout().flush();
        })
        .map_err(|e| e.to_string())?;
    println!(
        "\nWritten and checked (CRC-32 {:08X}) in {:.1} s",
        protocol::crc32(&image),
        start.elapsed().as_secs_f32()
    );
    if boot {
        session.boot().map_err(|e| e.to_string())?;
        println!("Application starting");
    }
    Ok(())
}

fn run(args: &[String]) -> Result<(), String> {
    let mut api = HidApi::new().map_err(|e| e.to_string())?;
    match args.first().map(String::as_str) {
        Some("info") => {
            let info = open(&mut api, false)?.info().map_err(|e| e.to_string())?;
            print_info(&info);
            Ok(())
        }
        Some("flash") => flash(&mut api, &args[1..]),
        Some("boot") => open(&mut api, false)?.boot().map_err(|e| e.to_string()),
        Some("enter") => {
            enter(&mut api)?;
            wait_for_bootloader(&mut api)?;
            println!("Update mode");
            Ok(())
        }
        _ => Err(USAGE.into()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
