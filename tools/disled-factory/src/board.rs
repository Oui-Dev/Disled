// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! One board on the station: its bootloader, the flash, what the lamp then
//! reports, the self-test with the operator, and its row in the log.

use std::io::{self, BufRead, Write};
use std::thread;
use std::time::{Duration, Instant};

use disled_flash::wchisp::{CHIP, Probe, Stage, Wchisp};
use disled_protocol::{Command, Info, Message, PROTOCOL_MAJOR, SelfTestResult};
use hidapi::HidApi;
use semver::Version;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::Firmware;
use crate::lamp::{self, Lamp};
use crate::report::{Check, Report, Row};

const POLL: Duration = Duration::from_secs(1);
/// wchisp failures in a row before the board fails.
const PROBE_FAILURES: u32 = 5;
/// After the flash the lamp restarts in about 2 s.
const LAMP_TIMEOUT: Duration = Duration::from_secs(30);
const REPLY_TIMEOUT: Duration = Duration::from_secs(2);

/// The self-test sequence (protocol/README.md): four colors for 1.5 s
/// each, then up to 10 s for a touch on the lid while the lamp glows blue.
const COLORS: [&str; 4] = ["red", "green", "blue", "warm white"];
const COLOR_STEP: Duration = Duration::from_millis(1500);
const TOUCH_WAIT: Duration = Duration::from_secs(10);

const DRIVER_HELP: &str = "Windows needs the WinUSB driver for the bootloader, once: run Zadig \
     (https://zadig.akeo.ie), Options > List All Devices, select USB ID 4348 55E0 \
     (or 1A86 55E0), choose WinUSB, then Install Driver.";

pub struct Station {
    pub api: HidApi,
    /// None when the lamps already run their firmware, with `--no-flash`.
    pub flasher: Option<Wchisp>,
    pub firmware: Firmware,
    pub report: Report,
}

impl Station {
    /// Runs one board, logs it, and waits until it is unplugged. Returns
    /// whether it passed; fails only when the log cannot be written.
    pub fn board(&mut self, number: u32) -> Result<bool, String> {
        println!("\nBoard {number}");
        let mut row = Row {
            signed: self.firmware.version.is_some(),
            ..Row::default()
        };
        if let Err(error) = self.check(&mut row) {
            println!("  [FAIL] {error}");
            row.error = Some(error);
        }
        row.date = now();
        self.report.append(&row)?;
        let passed = row.passed();
        println!("  {}", if passed { "PASS" } else { "FAIL" });
        println!("  Unplug the board.");
        self.wait_for_removal();
        Ok(passed)
    }

    fn check(&mut self, row: &mut Row) -> Result<(), String> {
        let mut lamp = match &self.flasher {
            Some(flasher) => {
                println!("  Plug the board in with BOOT held.");
                wait_for_bootloader(&mut self.api, flasher)?;
                flasher.flash(&self.firmware.path, show_stage)?;
                println!();
                wait_for_lamp(&mut self.api, Some(LAMP_TIMEOUT))?
            }
            None => {
                println!("  Plug the lamp in.");
                wait_for_lamp(&mut self.api, None)?
            }
        };
        let info = match lamp.request(&Command::GetInfo, REPLY_TIMEOUT)? {
            Message::Info(info) => info,
            other => return Err(format!("unexpected reply to GET_INFO: {other:?}")),
        };
        check_info(&info, &self.firmware, row)?;
        let result = self_test(&mut lamp)?;
        record(&result, row);
        if !result.completed {
            return Err("the self-test did not finish".into());
        }
        let wrong = ask_colors()?;
        row.colors = Check::from(wrong.is_empty());
        row.wrong_colors = wrong;
        Ok(())
    }

    /// Waits until neither a lamp nor a bootloader is connected, so that the
    /// next board is a new one.
    fn wait_for_removal(&mut self) {
        loop {
            let bootloader = self
                .flasher
                .as_ref()
                .is_some_and(|flasher| !matches!(flasher.probe(), Ok(Probe::Nothing)));
            if !bootloader && !lamp::present(&mut self.api) {
                return;
            }
            thread::sleep(POLL);
        }
    }
}

fn now() -> String {
    let now = OffsetDateTime::now_utc();
    now.replace_nanosecond(0)
        .unwrap_or(now)
        .format(&Rfc3339)
        .unwrap_or_default()
}

fn show_stage(stage: Stage) {
    let text = match stage {
        Stage::Erasing => "  Flashing: erase",
        Stage::Writing => ", write",
        Stage::Verifying => ", verify",
        Stage::Restarting => ", restart",
    };
    print!("{text}");
    let _ = io::stdout().flush();
}

/// Waits for the bootloader of a DISLED board, telling the operator what
/// holds it up. A bootloader that just appeared may not open at once, so a
/// problem shows only when two probes in a row see it.
fn wait_for_bootloader(api: &mut HidApi, flasher: &Wchisp) -> Result<(), String> {
    let mut previous = None;
    let mut shown: Option<String> = None;
    let mut failures = 0;
    loop {
        let probe = match flasher.probe() {
            Ok(probe) => {
                failures = 0;
                probe
            }
            Err(error) => {
                failures += 1;
                if failures >= PROBE_FAILURES {
                    return Err(error);
                }
                thread::sleep(POLL);
                continue;
            }
        };
        let hint = match &probe {
            Probe::Ready(chip) if chip.starts_with(CHIP) => {
                println!("  [ok]   bootloader {chip}");
                return Ok(());
            }
            Probe::Nothing if lamp::present(api) => Some(
                "This board started its firmware: unplug it, then plug it in again with BOOT held."
                    .to_owned(),
            ),
            Probe::Nothing => None,
            _ if previous.as_ref() != Some(&probe) => None,
            Probe::Ready(chip) => Some(format!("{chip} is not a DISLED board: unplug it.")),
            Probe::Several => {
                Some("Several boards are in their bootloader: keep only one plugged in.".to_owned())
            }
            Probe::DriverMissing => Some(DRIVER_HELP.to_owned()),
            Probe::NoPermission => Some(
                "No access to the bootloader: install the udev rule of the app \
                 (app/src-tauri/linux/70-disled.rules)."
                    .to_owned(),
            ),
        };
        if let Some(hint) = hint
            && shown.as_ref() != Some(&hint)
        {
            println!("  {hint}");
            shown = Some(hint);
        }
        previous = Some(probe);
        thread::sleep(POLL);
    }
}

/// Waits for the lamp to show up on USB.
fn wait_for_lamp(api: &mut HidApi, timeout: Option<Duration>) -> Result<Lamp, String> {
    let start = Instant::now();
    loop {
        if let Some(lamp) = Lamp::open(api) {
            return Ok(lamp);
        }
        if timeout.is_some_and(|timeout| start.elapsed() > timeout) {
            return Err(
                "the lamp did not start: press RESET, or unplug it and plug it back in".into(),
            );
        }
        thread::sleep(Duration::from_millis(250));
    }
}

fn check_info(info: &Info, firmware: &Firmware, row: &mut Row) -> Result<(), String> {
    row.uid = info.uid.clone();
    row.firmware = info.firmware_version.clone();
    row.board_revision = Some(info.board_revision);
    println!(
        "  [ok]   lamp {}: firmware {}{}, board revision {}",
        info.uid,
        info.firmware_version,
        if info.debug_build {
            " (debug build)"
        } else {
            ""
        },
        info.board_revision
    );
    if let Some(fault) = &info.fault {
        let remark = format!(
            "fault before the last reset: mcause 0x{:X}, mepc 0x{:08X}",
            fault.mcause, fault.mepc
        );
        println!("  note:  {remark}");
        row.remarks.push(remark);
    }
    if info.protocol[0] != PROTOCOL_MAJOR {
        return Err(format!(
            "the lamp speaks protocol {}.{}, this tool {PROTOCOL_MAJOR}.x",
            info.protocol[0], info.protocol[1]
        ));
    }
    if let Some(expected) = &firmware.version {
        let [major, minor, patch] = info.firmware.map(u64::from);
        if Version::new(major, minor, patch) != *expected || info.debug_build {
            return Err(format!(
                "the lamp runs firmware {}, not the release build {expected}",
                info.firmware_version
            ));
        }
    }
    if !info.self_test {
        return Err("this firmware has no self-test".into());
    }
    Ok(())
}

/// Runs the self-test, naming each color as the lamp shows it.
fn self_test(lamp: &mut Lamp) -> Result<SelfTestResult, String> {
    println!("  Self-test: watch the lamp, then touch the lid when it glows blue.");
    let seq = lamp.send(&Command::SelfTest)?;
    match lamp.reply(seq, REPLY_TIMEOUT, |_| {})? {
        Message::Ack { .. } => {}
        Message::Error { error, .. } => {
            return Err(format!("the lamp refused the self-test ({error:?})"));
        }
        other => return Err(format!("unexpected reply to SELF_TEST: {other:?}")),
    }
    let mut announced = 0;
    let longest = COLOR_STEP * COLORS.len() as u32 + TOUCH_WAIT + Duration::from_secs(4);
    let reply = lamp.reply(seq, longest, |elapsed| {
        let step = (elapsed.as_millis() / COLOR_STEP.as_millis()) as usize;
        while announced <= step.min(COLORS.len()) {
            match COLORS.get(announced) {
                Some(color) => print!("{}{color}", if announced == 0 { "    " } else { ", " }),
                None => print!("\n    Touch the lid now."),
            }
            let _ = io::stdout().flush();
            announced += 1;
        }
    });
    println!();
    match reply? {
        Message::SelfTestResult(result) => Ok(result),
        other => Err(format!("unexpected reply to SELF_TEST: {other:?}")),
    }
}

fn record(result: &SelfTestResult, row: &mut Row) {
    row.light_sensor = Check::from(result.light_sensor_ok);
    row.lux_centi = Some(result.lux_centi);
    row.touch_idle = Check::from(result.touch_idle_ok);
    row.touch_baseline = Some(result.touch_baseline);
    row.touch = Check::from(result.touch_pressed);
    row.touch_peak_permille = Some(result.touch_peak_permille);
    let mark = |passed: bool| if passed { "[ok]  " } else { "[FAIL]" };
    println!(
        "  {} light sensor: {}.{:02} lx",
        mark(result.light_sensor_ok),
        result.lux_centi / 100,
        result.lux_centi % 100
    );
    println!(
        "  {} touch at rest: baseline {}",
        mark(result.touch_idle_ok),
        result.touch_baseline
    );
    println!(
        "  {} touch: largest drop {} per mille",
        mark(result.touch_pressed),
        result.touch_peak_permille
    );
}

/// Asks the operator about the colors; returns the wrong ones.
fn ask_colors() -> Result<Vec<&'static str>, String> {
    loop {
        if let Some(right) = yes_no(&ask(
            "Did the lamp show red, green, blue and warm white? [y/n]",
        )?) {
            if right {
                return Ok(Vec::new());
            }
            break;
        }
    }
    loop {
        let wrong = wrong_colors(&ask("Which ones were wrong? Letters r, g, b, w:")?);
        if !wrong.is_empty() {
            return Ok(wrong);
        }
    }
}

fn ask(question: &str) -> Result<String, String> {
    print!("  {question} ");
    let _ = io::stdout().flush();
    let mut line = String::new();
    match io::stdin().lock().read_line(&mut line) {
        Ok(0) => Err("no answer from the operator".into()),
        Ok(_) => Ok(line),
        Err(error) => Err(format!("no answer from the operator ({error})")),
    }
}

fn yes_no(answer: &str) -> Option<bool> {
    match answer.trim().to_ascii_lowercase().as_str() {
        "y" | "yes" => Some(true),
        "n" | "no" => Some(false),
        _ => None,
    }
}

/// Reads letters such as `r b` or `rb`, or words such as `red, warm white`.
fn wrong_colors(answer: &str) -> Vec<&'static str> {
    let mut letters = Vec::new();
    for word in answer
        .to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphabetic())
        .filter(|word| !word.is_empty())
    {
        if word.chars().all(|c| "rgbw".contains(c)) {
            letters.extend(word.chars());
        } else {
            letters.extend(word.chars().next());
        }
    }
    [('r', "red"), ('g', "green"), ('b', "blue"), ('w', "white")]
        .into_iter()
        .filter(|(letter, _)| letters.contains(letter))
        .map(|(_, color)| color)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers() {
        assert_eq!(yes_no("y\n"), Some(true));
        assert_eq!(yes_no(" Yes \r\n"), Some(true));
        assert_eq!(yes_no("n\n"), Some(false));
        assert_eq!(yes_no("maybe\n"), None);
        assert_eq!(yes_no("\n"), None);
        assert_eq!(wrong_colors("r, B\n"), ["red", "blue"]);
        assert_eq!(wrong_colors("rb\n"), ["red", "blue"]);
        assert_eq!(wrong_colors("green\n"), ["green"]);
        assert_eq!(wrong_colors("Red and warm white\n"), ["red", "white"]);
        assert!(wrong_colors("x\n").is_empty());
    }
}
