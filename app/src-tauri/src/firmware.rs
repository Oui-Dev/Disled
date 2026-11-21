// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! Lamp firmware updates: the firmware on offer, looked for with the app updates, then
//! on request the download and signature check of one of them, the wait for the ROM
//! bootloader that BOOT and RESET start, the erase and the flash with wchisp, and the
//! check of the new version.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use disled_flash::firmware::{self as files, file_name, signature_name};
use disled_flash::wchisp::{self, Probe, Stage, Wchisp};
use parking_lot::Mutex;
use semver::Version;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::device::DeviceHandle;
use crate::net::{fetch, get};
use crate::updates::now;

const EVENT_STATUS: &str = "firmware-status";

/// The published releases only, since GitHub hides the drafts from
/// anonymous callers.
const RELEASES_URL: &str = "https://api.github.com/repos/Oui-Dev/Disled/releases?per_page=100";
/// The next pages come from the answers, and must stay on the API.
const API: &str = "https://api.github.com/";
/// At 100 releases a page, the releases of every component for years. The
/// newest come first, so a longer list loses only old ones.
const MAX_PAGES: usize = 20;
const TAG_PREFIX: &str = "fw-v";
/// Where GitHub shows a release, followed by its version.
const RELEASE_PAGE: &str = "https://github.com/Oui-Dev/Disled/releases/tag/fw-v";

/// The firmware line this app drives: 2.x speaks protocol 2.
const LINE: u64 = 2;

const LAMP_TIMEOUT: Duration = Duration::from_secs(60);

/// A firmware the app can install.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Release {
    version: Version,
    source: Source,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Source {
    /// The release build attached to a GitHub release, and its signature.
    GitHub { hex: String, signature: String },
    /// An unsigned local file, in development builds only.
    Local(PathBuf),
}

/// What matters here in an answer of the GitHub releases API.
#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

/// What GitHub publishes for this app: every signed 2.x, newest first,
/// never a pre-release.
fn offer(releases: &[GitHubRelease]) -> Vec<Release> {
    let mut offered: Vec<Release> = releases
        .iter()
        .filter_map(signed)
        .filter(|release| release.version.major == LINE)
        .collect();
    offered.sort_by(|a, b| b.version.cmp(&a.version));
    offered.dedup_by(|a, b| a.version == b.version);
    offered
}

/// A published firmware release whose release build is signed.
fn signed(release: &GitHubRelease) -> Option<Release> {
    if release.draft || release.prerelease {
        return None;
    }
    let version = Version::parse(release.tag_name.strip_prefix(TAG_PREFIX)?).ok()?;
    if !version.pre.is_empty() {
        return None;
    }
    let url = |name: String| {
        release
            .assets
            .iter()
            .find(|asset| asset.name == name)
            .map(|asset| asset.browser_download_url.clone())
    };
    let source = Source::GitHub {
        hex: url(file_name(&version))?,
        signature: url(signature_name(&version))?,
    };
    Some(Release { version, source })
}

/// The next page in the `Link` header of a GitHub answer, such as
/// `<https://api.github.com/repositories/1/releases?per_page=100&page=2>; rel="next"`.
fn next_page(link: &str) -> Option<String> {
    link.split(',').find_map(|part| {
        let (url, params) = part.split_once(';')?;
        let next = params
            .split(';')
            .any(|param| param.trim() == r#"rel="next""#);
        let url = url.trim().strip_prefix('<')?.strip_suffix('>')?;
        (next && url.starts_with(API)).then(|| url.to_owned())
    })
}

/// The release page of a firmware version on GitHub, built only from a
/// valid version, so that no window can open an address of its choice.
pub fn release_page(version: &str) -> Result<String, String> {
    let version =
        Version::parse(version).map_err(|_| format!("{version} is not a firmware version"))?;
    Ok(format!("{RELEASE_PAGE}{version}"))
}

/// Where a lamp update stands, shown by the update windows.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(tag = "step", rename_all = "snake_case")]
pub enum Flash {
    #[default]
    Idle,
    Downloading,
    /// Waiting for the ROM bootloader. The CH32V203 starts it only from a
    /// reset with BOOT held, so the user holds BOOT and presses RESET.
    WaitingForBootloader,
    /// On Windows, the bootloader needs the WinUSB driver, installed once
    /// with Zadig.
    DriverMissing,
    /// On Linux, the user may not open the bootloader, for lack of the udev
    /// rule.
    NoPermission,
    /// Several WCH bootloaders, so which one is the lamp is unknown.
    SeveralBootloaders,
    /// A WCH bootloader that is not a Disled lamp.
    OtherChip {
        chip: String,
    },
    /// Erasing the whole flash, then writing and checking the firmware.
    Flashing {
        stage: Stage,
    },
    /// Flashed, the lamp restarts.
    WaitingForLamp,
    Done {
        version: String,
    },
    Failed {
        error: String,
    },
}

/// What the popup and the update windows show about firmware updates.
#[derive(Debug, Clone, Default, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct FirmwareStatus {
    /// A check is running.
    pub checking: bool,
    /// The signed firmware 2.x, newest first. The update buttons install
    /// the first.
    pub versions: Vec<String>,
    /// Why the last check failed; cleared by the next success.
    pub error: Option<String>,
    /// Last successful check, seconds since 1970-01-01 UTC.
    pub checked_at: Option<u64>,
    /// The version the update installs, from its start until the window
    /// forgets its result.
    pub target: Option<String>,
    pub flash: Flash,
}

#[derive(Default)]
pub struct FirmwareState {
    status: Mutex<FirmwareStatus>,
    /// The firmware found by the last check, kept for the install.
    releases: Mutex<Vec<Release>>,
    /// An update is running.
    running: AtomicBool,
    /// The user cancelled; honored while the update waits for the
    /// bootloader.
    cancel: AtomicBool,
}

impl FirmwareState {
    pub fn status(&self) -> FirmwareStatus {
        self.status.lock().clone()
    }
}

/// Changes the status and tells every window.
fn change(app: &AppHandle, edit: impl FnOnce(&mut FirmwareStatus)) {
    let state = app.state::<FirmwareState>();
    let status = {
        let mut status = state.status.lock();
        edit(&mut status);
        status.clone()
    };
    let _ = app.emit(EVENT_STATUS, status);
}

fn show(app: &AppHandle, flash: Flash) {
    if app.state::<FirmwareState>().status().flash != flash {
        change(app, |s| s.flash = flash);
    }
}

/// In development builds, `DISLED_FIRMWARE_HEX=path/disled-fw-X.Y.Z.hex`
/// stands alone for the newest release, unsigned, to try an update with a
/// local build. Release builds ignore it.
fn local_release() -> Option<Release> {
    if !cfg!(debug_assertions) {
        return None;
    }
    let path = PathBuf::from(std::env::var_os("DISLED_FIRMWARE_HEX")?);
    let version = files::version_of(path.file_name()?.to_str()?)?;
    Some(Release {
        version,
        source: Source::Local(path),
    })
}

/// wchisp next to the app executable, where the installers put it. Debug
/// builds fall back to the one on the PATH (`cargo install wchisp`).
fn flasher() -> Result<Wchisp, String> {
    let bundled = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .with_file_name(wchisp::PROGRAM);
    if bundled.is_file() {
        Ok(Wchisp::new(bundled))
    } else if cfg!(debug_assertions) {
        Ok(Wchisp::new(wchisp::PROGRAM))
    } else {
        Err("wchisp is missing next to the app: reinstall Disled".into())
    }
}

/// The releases on GitHub, from every page: the API mixes the releases of
/// every component, so that the oldest firmware 2.x leave the first page
/// one day.
async fn github_releases(app: &AppHandle) -> Result<Vec<GitHubRelease>, String> {
    let mut releases = Vec::new();
    let mut url = RELEASES_URL.to_owned();
    for _ in 0..MAX_PAGES {
        let response = fetch(app, &url).await?;
        let next = response
            .headers()
            .get(reqwest::header::LINK)
            .and_then(|link| link.to_str().ok())
            .and_then(next_page);
        let body = response.bytes().await.map_err(|e| e.to_string())?;
        let page: Vec<GitHubRelease> = serde_json::from_slice(&body)
            .map_err(|e| format!("unexpected answer from GitHub: {e}"))?;
        releases.extend(page);
        match next {
            Some(next) => url = next,
            None => break,
        }
    }
    Ok(releases)
}

/// Looks for the firmware on offer on GitHub; the `firmware-status` events
/// bring the result.
pub async fn check(app: &AppHandle) {
    // One at a time, as for the app updates.
    let mut running = false;
    change(app, |s| running = std::mem::replace(&mut s.checking, true));
    if running {
        return;
    }
    let result = match local_release() {
        Some(release) => Ok(vec![release]),
        None => github_releases(app).await.map(|releases| offer(&releases)),
    };
    let state = app.state::<FirmwareState>();
    match result {
        Ok(releases) => {
            let versions = releases
                .iter()
                .map(|release| release.version.to_string())
                .collect();
            *state.releases.lock() = releases;
            change(app, |s| {
                s.checking = false;
                s.versions = versions;
                s.error = None;
                s.checked_at = Some(now());
            });
        }
        Err(error) => change(app, |s| {
            s.checking = false;
            s.error = Some(error);
        }),
    }
}

/// Downloads the release build, checks it and writes it to a file for
/// wchisp.
async fn download(app: &AppHandle, release: &Release) -> Result<PathBuf, String> {
    let (hex, signature) = match &release.source {
        Source::Local(path) => return Ok(path.clone()),
        Source::GitHub { hex, signature } => (hex, signature),
    };
    let data = get(app, hex).await?;
    let signature = String::from_utf8(get(app, signature).await?)
        .map_err(|_| "the firmware signature is unreadable")?;
    files::verify(&data, &signature, &release.version)?;
    let folder = app
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?
        .join("firmware");
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let file = folder.join(file_name(&release.version));
    std::fs::write(&file, &data).map_err(|e| e.to_string())?;
    Ok(file)
}

/// Why an update ended early.
enum Stop {
    Cancelled,
    Failed(String),
}

impl From<String> for Stop {
    fn from(error: String) -> Self {
        Stop::Failed(error)
    }
}

/// Updates the lamp with the newest firmware 2.x, for the guided window.
pub fn install(app: &AppHandle) -> Result<(), String> {
    let latest = app
        .state::<FirmwareState>()
        .status()
        .versions
        .into_iter()
        .next()
        .ok_or("no firmware to install: check for updates first")?;
    install_manual(app, &latest)
}

/// Updates the lamp with a version of the list, on its own thread; the
/// update windows follow the status.
pub fn install_manual(app: &AppHandle, version: &str) -> Result<(), String> {
    let state = app.state::<FirmwareState>();
    let wanted =
        Version::parse(version).map_err(|_| format!("{version} is not a firmware version"))?;
    let release = state
        .releases
        .lock()
        .iter()
        .find(|release| release.version == wanted)
        .cloned()
        .ok_or_else(|| format!("firmware {wanted} is not on offer: check for updates first"))?;
    // A second click goes to the update already running, which the window
    // follows; an error would show it as failed.
    if state.running.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    state.cancel.store(false, Ordering::SeqCst);
    change(app, |s| s.target = Some(wanted.to_string()));
    let owner = app.clone();
    let spawned = thread::Builder::new()
        .name("disled-firmware".into())
        .spawn(move || {
            let flash = match update(&owner, &release) {
                Ok(()) => Flash::Done {
                    version: release.version.to_string(),
                },
                Err(Stop::Cancelled) => Flash::Idle,
                Err(Stop::Failed(error)) => Flash::Failed { error },
            };
            // Freed first, because the window offers to try again as soon as
            // it shows the result.
            owner
                .state::<FirmwareState>()
                .running
                .store(false, Ordering::SeqCst);
            let cancelled = flash == Flash::Idle;
            change(&owner, |s| {
                s.flash = flash;
                if cancelled {
                    s.target = None;
                }
            });
        });
    if let Err(error) = spawned {
        state.running.store(false, Ordering::SeqCst);
        change(app, |s| s.target = None);
        return Err(error.to_string());
    }
    Ok(())
}

/// An update is running.
pub fn running(app: &AppHandle) -> bool {
    app.state::<FirmwareState>().running.load(Ordering::SeqCst)
}

/// Stops an update that waits for the bootloader.
pub fn cancel(app: &AppHandle) {
    app.state::<FirmwareState>()
        .cancel
        .store(true, Ordering::SeqCst);
}

/// Takes back the cancel of a closed update window, when a new one opens
/// and follows the update still running.
pub fn uncancel(app: &AppHandle) {
    app.state::<FirmwareState>()
        .cancel
        .store(false, Ordering::SeqCst);
}

/// Forgets the result of the last update, when an update window opens.
pub fn clear_result(app: &AppHandle) {
    let state = app.state::<FirmwareState>();
    let finished = matches!(
        state.status().flash,
        Flash::Done { .. } | Flash::Failed { .. }
    );
    if finished && !state.running.load(Ordering::SeqCst) {
        change(app, |s| {
            s.flash = Flash::Idle;
            s.target = None;
        });
    }
}

/// The update itself; returns once the lamp runs the new firmware.
fn update(app: &AppHandle, release: &Release) -> Result<(), Stop> {
    show(app, Flash::Downloading);
    let flasher = flasher()?;
    let file = tauri::async_runtime::block_on(download(app, release))?;

    wait_for_bootloader(app, &flasher)?;
    flasher.flash(&file, |stage| show(app, Flash::Flashing { stage }))?;

    show(app, Flash::WaitingForLamp);
    let device = app.state::<DeviceHandle>();
    let since = Instant::now();
    loop {
        if let Some(info) = device.info() {
            let [major, minor, patch] = info.firmware.map(u64::from);
            return if Version::new(major, minor, patch) == release.version {
                Ok(())
            } else {
                Err(Stop::Failed(format!(
                    "after the update the lamp reports firmware {}",
                    info.firmware_version
                )))
            };
        }
        if since.elapsed() > LAMP_TIMEOUT {
            return Err(Stop::Failed(
                "the lamp did not start again: press RESET, or unplug it and plug it back in"
                    .into(),
            ));
        }
        thread::sleep(Duration::from_millis(250));
    }
}

/// Waits until the lamp's bootloader can be flashed, showing what holds it
/// up, unless the user cancels.
fn wait_for_bootloader(app: &AppHandle, flasher: &Wchisp) -> Result<(), Stop> {
    let state = app.state::<FirmwareState>();
    show(app, Flash::WaitingForBootloader);
    flasher.wait_for_bootloader(|probe, settled| {
        if state.cancel.load(Ordering::SeqCst) {
            return Err(Stop::Cancelled);
        }
        if let Some(flash) = probe.and_then(|probe| flash_of(probe, settled)) {
            show(app, flash);
        }
        Ok(())
    })?;
    Ok(())
}

/// What the update windows show of a probe that did not find the lamp's
/// bootloader, once it counts.
fn flash_of(probe: &Probe, settled: bool) -> Option<Flash> {
    if !settled {
        return None;
    }
    Some(match probe {
        Probe::Nothing => Flash::WaitingForBootloader,
        Probe::Ready(chip) => Flash::OtherChip { chip: chip.clone() },
        Probe::Several => Flash::SeveralBootloaders,
        Probe::DriverMissing => Flash::DriverMissing,
        Probe::NoPermission => Flash::NoPermission,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, assets: &[&str]) -> GitHubRelease {
        GitHubRelease {
            tag_name: tag.into(),
            draft: false,
            prerelease: false,
            assets: assets
                .iter()
                .map(|name| GitHubAsset {
                    name: (*name).into(),
                    browser_download_url: format!("https://example.com/{tag}/{name}"),
                })
                .collect(),
        }
    }

    fn signed(version: &str) -> GitHubRelease {
        let hex = format!("disled-fw-{version}.hex");
        release(
            &format!("fw-v{version}"),
            &[&hex, &format!("{hex}.sig"), "SHA256SUMS"],
        )
    }

    fn versions(offered: &[Release]) -> Vec<String> {
        offered
            .iter()
            .map(|release| release.version.to_string())
            .collect()
    }

    #[test]
    fn every_signed_2x_newest_first() {
        let releases = [
            release("app-v2.0.0", &["Disled_2.0.0_x64-setup.exe", "latest.json"]),
            signed("2.0.0"),
            signed("0.2.2"),
            signed("1.0.0"),
            signed("2.1.0"),
            signed("0.9.0"),
            signed("2.0.1"),
            signed("2.2.0-rc.1"),
            GitHubRelease {
                prerelease: true,
                ..signed("2.3.0")
            },
            GitHubRelease {
                draft: true,
                ..signed("2.4.0")
            },
            // A firmware this app cannot drive.
            signed("3.0.0"),
        ];
        let offered = offer(&releases);
        assert_eq!(versions(&offered), ["2.1.0", "2.0.1", "2.0.0"]);
        assert_eq!(
            offered[0].source,
            Source::GitHub {
                hex: "https://example.com/fw-v2.1.0/disled-fw-2.1.0.hex".into(),
                signature: "https://example.com/fw-v2.1.0/disled-fw-2.1.0.hex.sig".into(),
            }
        );
        // Before the first firmware 2.x, the update buttons have nothing to
        // install.
        assert!(offer(&[signed("1.0.0")]).is_empty());
    }

    #[test]
    fn versions_compare_as_numbers() {
        let releases = [signed("2.9.0"), signed("2.10.0"), signed("2.2.9")];
        assert_eq!(versions(&offer(&releases)), ["2.10.0", "2.9.0", "2.2.9"]);
    }

    #[test]
    fn unsigned_firmware_is_never_offered() {
        let releases = [release(
            "fw-v2.0.1",
            &[
                "disled-fw-2.0.1.hex",
                "disled-fw-2.0.1-debug.hex",
                "SHA256SUMS",
            ],
        )];
        assert!(offer(&releases).is_empty());
    }

    #[test]
    fn a_github_answer_reads() {
        let answer = r#"[{
            "tag_name": "fw-v2.0.0", "name": "fw-v2.0.0", "draft": false,
            "prerelease": false, "body": "Protocol 2.0",
            "assets": [
                {"name": "disled-fw-2.0.0.hex", "size": 58359,
                 "browser_download_url": "https://github.com/Oui-Dev/Disled/releases/download/fw-v2.0.0/disled-fw-2.0.0.hex"},
                {"name": "disled-fw-2.0.0.hex.sig", "size": 412,
                 "browser_download_url": "https://github.com/Oui-Dev/Disled/releases/download/fw-v2.0.0/disled-fw-2.0.0.hex.sig"}
            ]
        }]"#;
        let releases: Vec<GitHubRelease> = serde_json::from_str(answer).expect("releases");
        assert_eq!(versions(&offer(&releases)), ["2.0.0"]);
    }

    #[test]
    fn the_next_page_comes_from_the_link_header() {
        let first = r#"<https://api.github.com/repositories/42/releases?per_page=100&page=2>; rel="next", <https://api.github.com/repositories/42/releases?per_page=100&page=3>; rel="last""#;
        assert_eq!(
            next_page(first).as_deref(),
            Some("https://api.github.com/repositories/42/releases?per_page=100&page=2")
        );
        let last = r#"<https://api.github.com/repositories/42/releases?per_page=100&page=2>; rel="prev", <https://api.github.com/repositories/42/releases?per_page=100&page=1>; rel="first""#;
        assert_eq!(next_page(last), None);
        // Never off the API.
        assert_eq!(
            next_page(r#"<https://example.com/releases?page=2>; rel="next""#),
            None
        );
    }

    #[test]
    fn a_probe_shows_once_it_counts() {
        assert_eq!(flash_of(&Probe::DriverMissing, false), None);
        assert_eq!(
            flash_of(&Probe::DriverMissing, true),
            Some(Flash::DriverMissing)
        );
        assert_eq!(
            flash_of(&Probe::Ready("CH32V003".into()), true),
            Some(Flash::OtherChip {
                chip: "CH32V003".into()
            })
        );
    }

    #[test]
    fn release_pages_take_only_a_version() {
        assert_eq!(
            release_page("2.1.0").as_deref(),
            Ok("https://github.com/Oui-Dev/Disled/releases/tag/fw-v2.1.0")
        );
        for address in ["2.1.0/../../../evil", "https://example.com", "", "2.1"] {
            assert!(release_page(address).is_err(), "{address}");
        }
    }
}
