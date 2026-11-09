// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! DISLED production tool (docs/production.md). For each board, it flashes the
//! release build with wchisp, checks what the lamp reports, runs its self-test
//! with the operator and appends a row to a CSV log.

mod board;
mod lamp;
mod report;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use disled_flash::firmware;
use disled_flash::wchisp::{self, Wchisp};
use hidapi::HidApi;
use semver::Version;

use board::Station;
use report::Report;

/// Flashes, checks and self-tests DISLED boards one after the other, and
/// logs each one. Stop with Ctrl+C.
#[derive(Parser)]
struct Args {
    /// Release build to flash, disled-fw-X.Y.Z.hex, with its signature
    /// (disled-fw-X.Y.Z.hex.sig) next to it: both come from the release.
    firmware: PathBuf,
    /// Accept a firmware without the update key's signature, such as a
    /// local or custom build, at your own risk. Its version is not checked.
    #[arg(long)]
    unsigned: bool,
    /// Test lamps that already run their firmware: no bootloader, no flash.
    #[arg(long)]
    no_flash: bool,
    /// CSV log, appended.
    #[arg(long, default_value = "disled-production.csv")]
    log: PathBuf,
    /// wchisp program [default: the one on the PATH, else PlatformIO's].
    #[arg(long)]
    wchisp: Option<PathBuf>,
}

/// The firmware of the session.
struct Firmware {
    path: PathBuf,
    /// The version the lamps must report; None for an unsigned firmware,
    /// which is not checked.
    version: Option<Version>,
}

/// Reads the firmware and, unless `unsigned`, checks its signature.
fn load_firmware(path: &Path, unsigned: bool) -> Result<Firmware, String> {
    let data = std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    if unsigned {
        return Ok(Firmware {
            path: path.to_owned(),
            version: None,
        });
    }
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let version = firmware::version_of(name).ok_or_else(|| {
        format!("{name} is not a release build: give a disled-fw-X.Y.Z.hex from a release")
    })?;
    let signature_path = path.with_file_name(firmware::signature_name(&version));
    let signature = std::fs::read_to_string(&signature_path).map_err(|e| {
        format!(
            "cannot read the signature {} ({e}): download it from the release, next to the firmware",
            signature_path.display()
        )
    })?;
    firmware::verify(&data, &signature, &version)?;
    Ok(Firmware {
        path: path.to_owned(),
        version: Some(version),
    })
}

/// The wchisp to use: the one given, else the one next to this program or
/// on the PATH, else the one PlatformIO installs for the firmware. Returns
/// it with its version.
fn find_wchisp(given: Option<PathBuf>) -> Result<(Wchisp, String), String> {
    let candidates = match given {
        Some(path) => vec![path],
        None => [on_path(wchisp::PROGRAM), platformio_wchisp()]
            .into_iter()
            .flatten()
            .collect(),
    };
    candidates
        .into_iter()
        .map(Wchisp::new)
        .find_map(|flasher| flasher.version().ok().map(|version| (flasher, version)))
        .ok_or_else(|| {
            "wchisp not found: install it (cargo install wchisp@0.3.0 --locked), \
             or give its path with --wchisp"
                .into()
        })
}

/// Finds a program next to this one, else on the PATH, and returns its full
/// path to show which one runs.
fn on_path(name: &str) -> Option<PathBuf> {
    let beside = std::env::current_exe()
        .ok()
        .map(|exe| exe.with_file_name(name));
    let path = std::env::var_os("PATH").unwrap_or_default();
    beside
        .into_iter()
        .chain(std::env::split_paths(&path).map(|dir| dir.join(name)))
        .find(|candidate| candidate.is_file())
}

fn platformio_wchisp() -> Option<PathBuf> {
    let core = std::env::var_os("PLATFORMIO_CORE_DIR")
        .map(PathBuf::from)
        .or_else(|| std::env::home_dir().map(|home| home.join(".platformio")))?;
    Some(core.join("packages/tool-wchisp").join(wchisp::PROGRAM))
}

fn run(args: Args) -> Result<(), String> {
    let firmware = load_firmware(&args.firmware, args.unsigned)?;
    let flasher = if args.no_flash {
        None
    } else {
        Some(find_wchisp(args.wchisp)?)
    };
    let report = Report::open(&args.log)?;
    let api = HidApi::new().map_err(|e| format!("cannot use USB HID: {e}"))?;

    println!("DISLED factory");
    match &firmware.version {
        Some(version) => println!(
            "  firmware  {} ({version}, signature checked)",
            firmware.path.display()
        ),
        None => println!(
            "  firmware  {} (UNSIGNED: not checked, at your own risk)",
            firmware.path.display()
        ),
    }
    match &flasher {
        Some((flasher, version)) => {
            println!("  wchisp    {} ({version})", flasher.program().display())
        }
        None => println!("  no flash: the lamps keep their firmware"),
    }
    println!("  log       {}", report.path().display());

    let mut station = Station {
        api,
        flasher: flasher.map(|(flasher, _)| flasher),
        firmware,
        report,
    };
    let (mut passed, mut failed) = (0, 0);
    for number in 1.. {
        if station.board(number)? {
            passed += 1;
        } else {
            failed += 1;
        }
        println!("  ({passed} passed, {failed} failed)");
    }
    Ok(())
}

fn main() -> ExitCode {
    match run(Args::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
