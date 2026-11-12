// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! The wchisp flasher, GPL-2.0, which this code cannot link: it runs as a separate
//! program, read through its log lines. Those below are of wchisp 0.3.0, which the
//! app ships, and of the 0.2.3 nightly that PlatformIO installs.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;

use serde::Serialize;

/// Start of the name wchisp gives the lamp's chip, `CH32V203G6U6[0x3619]`.
pub const CHIP: &str = "CH32V203";

/// File name of the program.
pub const PROGRAM: &str = if cfg!(windows) {
    "wchisp.exe"
} else {
    "wchisp"
};

/// What `wchisp probe` found on USB.
#[derive(Debug, PartialEq, Eq)]
pub enum Probe {
    /// No WCH bootloader.
    Nothing,
    /// One bootloader, opened, with the name of its chip.
    Ready(String),
    /// Several bootloaders, so which one is the lamp is unknown.
    Several,
    /// On Windows, the bootloader has no WinUSB driver.
    DriverMissing,
    /// On Linux, the user may not open the bootloader, for lack of the udev
    /// rule.
    NoPermission,
}

/// Steps of `wchisp flash`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Erasing,
    Writing,
    Verifying,
    Restarting,
}

/// A wchisp program.
#[derive(Debug)]
pub struct Wchisp {
    program: PathBuf,
}

impl Wchisp {
    /// A path, or a bare name that the PATH resolves.
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
        }
    }

    pub fn program(&self) -> &Path {
        &self.program
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        // Plain log lines, without color codes.
        command
            .env("NO_COLOR", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // Without it, a console program started by a GUI app opens a
            // console window.
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        command
    }

    /// The first line of `wchisp --version`, such as `wchisp 0.3.0`. Fails
    /// when the program cannot run.
    pub fn version(&self) -> Result<String, String> {
        let output = self
            .command()
            .arg("--version")
            .output()
            .map_err(cannot_run)?;
        let text = String::from_utf8_lossy(&output.stdout);
        match text.lines().next() {
            Some(line) if output.status.success() => Ok(line.trim().to_owned()),
            _ => Err("wchisp gave no version".into()),
        }
    }

    /// Looks for WCH bootloaders and opens the one it finds.
    pub fn probe(&self) -> Result<Probe, String> {
        let output = self.command().arg("probe").output().map_err(cannot_run)?;
        let text =
            String::from_utf8_lossy(&output.stdout) + String::from_utf8_lossy(&output.stderr);
        parse_probe(&text)
    }

    /// Erases the whole code flash of the bootloader that `probe` found.
    /// `flash` erases only the sectors the new firmware covers, which would
    /// leave the end of a longer firmware behind it.
    pub fn erase(&self) -> Result<(), String> {
        let output = self.command().arg("erase").output().map_err(cannot_run)?;
        let text =
            String::from_utf8_lossy(&output.stdout) + String::from_utf8_lossy(&output.stderr);
        if output.status.success() {
            parse_erase(&text)
        } else {
            Err(failure(&text))
        }
    }

    /// Flashes a firmware file into the bootloader that `probe` found, then
    /// restarts the chip. Reports each stage as wchisp reaches it.
    pub fn flash(&self, file: &Path, mut on_stage: impl FnMut(Stage)) -> Result<(), String> {
        let mut child = self
            .command()
            .arg("flash")
            .arg(file)
            .spawn()
            .map_err(cannot_run)?;
        // Errors go to stderr. A thread reads it, so that a full pipe never
        // blocks wchisp.
        let mut stderr = child.stderr.take().ok_or("no wchisp error output")?;
        let errors = thread::spawn(move || {
            let mut text = String::new();
            let _ = stderr.read_to_string(&mut text);
            text
        });
        let mut log = String::new();
        let stdout = child.stdout.take().ok_or("no wchisp output")?;
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            if let Some(stage) = parse_stage(&line) {
                on_stage(stage);
            }
            log.push_str(&line);
            log.push('\n');
        }
        let status = child.wait().map_err(cannot_run)?;
        log.push_str(&errors.join().unwrap_or_default());
        if status.success() {
            Ok(())
        } else {
            Err(failure(&log))
        }
    }
}

fn cannot_run(error: std::io::Error) -> String {
    format!("cannot run wchisp: {error}")
}

fn parse_probe(text: &str) -> Result<Probe, String> {
    if text.contains("Failed to open USB device on Windows") {
        return Ok(Probe::DriverMissing);
    }
    if text.contains("Failed to open USB device on Linux") {
        return Ok(Probe::NoPermission);
    }
    // wchisp 0.3.0 prints "Found 1 USB device" and 0.2.3 "Found 1 devices",
    // then "Device #0: CH32V203G6U6[0x3619]".
    let found = text
        .lines()
        .filter(|line| line.contains(" device"))
        .find_map(|line| {
            line.split("Found ")
                .nth(1)?
                .split(' ')
                .next()?
                .parse::<usize>()
                .ok()
        });
    match found {
        Some(0) => Ok(Probe::Nothing),
        Some(1) => text
            .lines()
            .find_map(|line| line.split("Device #0: ").nth(1))
            .map(|chip| Probe::Ready(chip.trim().to_owned()))
            .ok_or_else(|| failure(text)),
        Some(_) => Ok(Probe::Several),
        None => Err(failure(text)),
    }
}

/// wchisp ends an erase with "Erased 32 code flash sectors", the 32 KB of
/// the CH32V203G6.
fn parse_erase(text: &str) -> Result<(), String> {
    let erased = text.lines().any(|line| {
        line.split("Erased ")
            .nth(1)
            .and_then(|rest| rest.strip_suffix(" code flash sectors"))
            .is_some_and(|sectors| sectors.parse::<u32>().is_ok())
    });
    if erased { Ok(()) } else { Err(failure(text)) }
}

fn parse_stage(line: &str) -> Option<Stage> {
    [
        ("Erasing...", Stage::Erasing),
        ("Writing to code flash...", Stage::Writing),
        ("Verifying...", Stage::Verifying),
        ("Now reset device", Stage::Restarting),
    ]
    .into_iter()
    .find(|(message, _)| line.contains(message))
    .map(|(_, stage)| stage)
}

/// The `Error: …` line wchisp ended with, or its last line.
fn failure(text: &str) -> String {
    let reason = text
        .lines()
        .find_map(|line| line.strip_prefix("Error: "))
        .or_else(|| text.lines().rev().find(|line| !line.trim().is_empty()))
        .unwrap_or("no output");
    format!("wchisp: {}", reason.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Log lines on stdout, errors on stderr.

    #[test]
    fn no_bootloader() {
        let text = "03:29:57 [INFO] Found 0 USB devices\n\
                    03:29:57 [INFO] hint: use `wchisp info` to check chip info\n";
        assert_eq!(parse_probe(text), Ok(Probe::Nothing));
        // PlatformIO's wchisp 0.2.3.
        let text = "04:21:57 [INFO] Found 0 devices\n\
                    04:21:57 [INFO] hint: use `wchisp info` to check chip info\n";
        assert_eq!(parse_probe(text), Ok(Probe::Nothing));
    }

    #[test]
    fn the_lamp_in_its_bootloader() {
        let text = "03:29:57 [INFO] Found 1 USB device\n\
                    03:29:57 [INFO] Opening USB device #0\n\
                    03:29:57 [INFO] \tDevice #0: CH32V203G6U6[0x3619]\n\
                    03:29:57 [INFO] hint: use `wchisp info` to check chip info\n";
        assert_eq!(
            parse_probe(text),
            Ok(Probe::Ready("CH32V203G6U6[0x3619]".into()))
        );
        // PlatformIO's wchisp 0.2.3 prints the device without a log prefix.
        let text = "04:21:57 [INFO] Found 1 devices\n\
                    04:21:57 [INFO] hint: use `wchisp info` to check chip info\n\
                    04:21:57 [INFO] Opening USB device #0\n\
                    Device #0: CH32V203G6U6[0x3619]\n";
        assert_eq!(
            parse_probe(text),
            Ok(Probe::Ready("CH32V203G6U6[0x3619]".into()))
        );
    }

    #[test]
    fn several_bootloaders() {
        let text = "03:29:57 [INFO] Found 2 USB devices\n\
                    03:29:57 [INFO] Opening USB device #0\n";
        assert_eq!(parse_probe(text), Ok(Probe::Several));
    }

    #[test]
    fn windows_without_the_winusb_driver() {
        let text = "03:29:57 [INFO] Found 1 USB device\n\
                    03:29:57 [INFO] Opening USB device #0\n\
                    03:29:57 [WARN] It's likely no WinUSB/LibUSB drivers installed. \
                    Please install it from Zadig. See also: https://zadig.akeo.ie\n\
                    03:29:57 [ERROR] Failed to open USB device: Bus 001 Device 005: ID 4348:55e0\n\
                    Error: Failed to open USB device on Windows\n";
        assert_eq!(parse_probe(text), Ok(Probe::DriverMissing));
    }

    #[test]
    fn linux_without_the_udev_rule() {
        let text = "03:29:57 [INFO] Found 1 USB device\n\
                    Error: Failed to open USB device on Linux due to no enough permission\n";
        assert_eq!(parse_probe(text), Ok(Probe::NoPermission));
    }

    #[test]
    fn other_failures_carry_the_wchisp_error() {
        let text = "03:29:57 [INFO] Found 1 USB device\n\
                    03:29:57 [INFO] Opening USB device #0\n\
                    Error: Operation timed out\n";
        assert_eq!(parse_probe(text), Err("wchisp: Operation timed out".into()));
        assert_eq!(
            failure("03:29:57 [INFO] Opening USB device #0\n"),
            "wchisp: 03:29:57 [INFO] Opening USB device #0"
        );
    }

    // The lines of `wchisp erase`, read in the source of wchisp 0.3.0, until
    // the test lamp confirms them.

    #[test]
    fn a_full_erase() {
        let text = "03:30:00 [INFO] Opening USB device #0\n\
                    03:30:01 [INFO] Erased 32 code flash sectors\n";
        assert_eq!(parse_erase(text), Ok(()));
    }

    #[test]
    fn a_failed_erase_carries_the_wchisp_error() {
        let text = "03:30:00 [INFO] Opening USB device #0\n\
                    Error: erase failed\n";
        assert_eq!(parse_erase(text), Err("wchisp: erase failed".into()));
        // An erase that wchisp does not report did not happen.
        assert_eq!(
            parse_erase("03:30:00 [INFO] Opening USB device #0\n"),
            Err("wchisp: 03:30:00 [INFO] Opening USB device #0".into())
        );
    }

    #[test]
    fn flash_stages() {
        let stages: Vec<_> = [
            "03:30:01 [INFO] Opening USB device #0",
            "03:30:01 [INFO] Firmware size: 21504",
            "03:30:01 [INFO] Erasing...",
            "03:30:02 [INFO] Erase done",
            "03:30:02 [INFO] Writing to code flash...",
            "03:30:04 [INFO] Code flash 21504 bytes written",
            "03:30:04 [INFO] Verifying...",
            "03:30:05 [INFO] Verify OK",
            "03:30:05 [INFO] Now reset device and skip any communication errors",
            "03:30:05 [INFO] Device reset",
        ]
        .into_iter()
        .filter_map(parse_stage)
        .collect();
        assert_eq!(
            stages,
            [
                Stage::Erasing,
                Stage::Writing,
                Stage::Verifying,
                Stage::Restarting
            ]
        );
    }
}
