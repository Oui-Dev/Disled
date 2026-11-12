// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! Lamp firmware updates: the firmware on offer, looked for with the app updates, then
//! on request the download and signature check of one of them, the wait for the ROM
//! bootloader that BOOT and RESET start, the erase and the flash with wchisp, and the
//! check of the new version.

use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use disled_flash::firmware::{self as files, file_name, signature_name};
use disled_flash::wchisp::{self, Probe, Stage, Wchisp};
use semver::Version;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::device::DeviceHandle;
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
/// The line of Disled v1, whose last version is offered to go back to it.
const V1_LINE: u64 = 1;

/// How often the app looks for the bootloader.
const POLL: Duration = Duration::from_secs(1);
/// wchisp failures in a row before the update gives up.
const PROBE_FAILURES: u32 = 5;
/// After the flash the lamp restarts through its bootloader, which takes
/// several seconds.
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

/// The firmware a check found.
#[derive(Debug, PartialEq, Eq)]
struct Offer {
    /// Newest first.
    releases: Vec<Release>,
    /// The one the update buttons install.
    latest: Option<Version>,
}

impl Offer {
    /// What GitHub publishes: every signed 2.x, newest first, then the last
    /// signed 1.x, to go back to Disled v1. Never a pre-release, nor a 0.x.
    fn of(releases: &[GitHubRelease]) -> Self {
        let mut signed: Vec<Release> = releases.iter().filter_map(signed).collect();
        signed.sort_by(|a, b| b.version.cmp(&a.version));
        signed.dedup_by(|a, b| a.version == b.version);
        let v1 = signed
            .iter()
            .find(|release| release.version.major == V1_LINE)
            .cloned();
        let mut releases: Vec<Release> = signed
            .into_iter()
            .filter(|release| release.version.major == LINE)
            .collect();
        let latest = releases.first().map(|release| release.version.clone());
        releases.extend(v1);
        Self { releases, latest }
    }

    /// A local file, in development builds: the newest firmware, alone.
    fn local(release: Release) -> Self {
        Self {
            latest: Some(release.version.clone()),
            releases: vec![release],
        }
    }
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
    /// The newest firmware 2.x, which the update buttons install.
    pub latest: Option<String>,
    /// The firmware the app can install, newest first: the signed 2.x, then
    /// the last 1.x, to go back to Disled v1.
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
        self.status.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

/// Changes the status and tells every window.
fn change(app: &AppHandle, edit: impl FnOnce(&mut FirmwareStatus)) {
    let status = match app.state::<FirmwareState>().status.lock() {
        Ok(mut status) => {
            edit(&mut status);
            status.clone()
        }
        Err(_) => return,
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

/// Asks for a file; GitHub wants a user agent.
async fn fetch(app: &AppHandle, url: &str) -> Result<reqwest::Response, String> {
    // reqwest is built without a TLS provider, `rustls-no-provider`, so this
    // installs ring, the one the updater uses, unless a provider is there.
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    let client = reqwest::Client::builder()
        .user_agent(format!("Disled/{}", app.package_info().version))
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;
    client
        .get(url)
        .send()
        .await
        .and_then(|response| response.error_for_status())
        .map_err(|e| e.to_string())
}

/// Downloads a file.
async fn get(app: &AppHandle, url: &str) -> Result<Vec<u8>, String> {
    let body = fetch(app, url)
        .await?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
    Ok(body.to_vec())
}

/// The releases on GitHub, from every page: the API mixes the releases of
/// every component, so that fw-v1.0.0 leaves the first page one day.
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
        Some(release) => Ok(Offer::local(release)),
        None => github_releases(app)
            .await
            .map(|releases| Offer::of(&releases)),
    };
    let state = app.state::<FirmwareState>();
    match result {
        Ok(offer) => {
            let versions = offer
                .releases
                .iter()
                .map(|release| release.version.to_string())
                .collect();
            if let Ok(mut kept) = state.releases.lock() {
                *kept = offer.releases;
            }
            change(app, |s| {
                s.checking = false;
                s.latest = offer.latest.map(|version| version.to_string());
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
        .latest
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
        .ok()
        .and_then(|releases| {
            releases
                .iter()
                .find(|release| release.version == wanted)
                .cloned()
        })
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
    // All of it, so that nothing of the firmware before stays behind a
    // shorter one, 1.0.0 included.
    show(
        app,
        Flash::Flashing {
            stage: Stage::Erasing,
        },
    );
    flasher.erase()?;
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
/// up.
fn wait_for_bootloader(app: &AppHandle, flasher: &Wchisp) -> Result<(), Stop> {
    let state = app.state::<FirmwareState>();
    show(app, Flash::WaitingForBootloader);
    let mut previous = None;
    let mut failures = 0;
    loop {
        if state.cancel.load(Ordering::SeqCst) {
            return Err(Stop::Cancelled);
        }
        let probe = match flasher.probe() {
            Ok(probe) => {
                failures = 0;
                probe
            }
            Err(error) => {
                failures += 1;
                if failures >= PROBE_FAILURES {
                    return Err(Stop::Failed(error));
                }
                thread::sleep(POLL);
                continue;
            }
        };
        let flash = match &probe {
            Probe::Ready(chip) if chip.starts_with(wchisp::CHIP) => return Ok(()),
            Probe::Nothing => Some(Flash::WaitingForBootloader),
            // A bootloader that just appeared may not open at once, so a
            // problem shows only when two probes in a row see it.
            _ if previous.as_ref() != Some(&probe) => None,
            Probe::Ready(chip) => Some(Flash::OtherChip { chip: chip.clone() }),
            Probe::Several => Some(Flash::SeveralBootloaders),
            Probe::DriverMissing => Some(Flash::DriverMissing),
            Probe::NoPermission => Some(Flash::NoPermission),
        };
        if let Some(flash) = flash {
            show(app, flash);
        }
        previous = Some(probe);
        thread::sleep(POLL);
    }
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

    fn versions(offer: &Offer) -> Vec<String> {
        offer
            .releases
            .iter()
            .map(|release| release.version.to_string())
            .collect()
    }

    #[test]
    fn every_signed_2x_then_the_last_1x() {
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
        let offer = Offer::of(&releases);
        assert_eq!(versions(&offer), ["2.1.0", "2.0.1", "2.0.0", "1.0.0"]);
        assert_eq!(offer.latest, Some(Version::new(2, 1, 0)));
        assert_eq!(
            offer.releases[0].source,
            Source::GitHub {
                hex: "https://example.com/fw-v2.1.0/disled-fw-2.1.0.hex".into(),
                signature: "https://example.com/fw-v2.1.0/disled-fw-2.1.0.hex.sig".into(),
            }
        );
    }

    #[test]
    fn only_the_last_1x_goes_back_to_v1() {
        let releases = [signed("1.0.0"), signed("1.0.1"), signed("2.0.0")];
        assert_eq!(versions(&Offer::of(&releases)), ["2.0.0", "1.0.1"]);
        // Before the first firmware 2.x, the update buttons have nothing to
        // install.
        let offer = Offer::of(&[signed("1.0.0")]);
        assert_eq!(versions(&offer), ["1.0.0"]);
        assert_eq!(offer.latest, None);
    }

    #[test]
    fn versions_compare_as_numbers() {
        let releases = [signed("2.9.0"), signed("2.10.0"), signed("2.2.9")];
        let offer = Offer::of(&releases);
        assert_eq!(versions(&offer), ["2.10.0", "2.9.0", "2.2.9"]);
        assert_eq!(offer.latest, Some(Version::new(2, 10, 0)));
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
        assert_eq!(
            Offer::of(&releases),
            Offer {
                releases: Vec::new(),
                latest: None
            }
        );
    }

    #[test]
    fn a_local_file_stands_alone_for_the_newest() {
        for name in ["disled-fw-2.1.0.hex", "disled-fw-1.0.0.hex"] {
            let version = files::version_of(name).expect("a version");
            let offer = Offer::local(Release {
                version: version.clone(),
                source: Source::Local(name.into()),
            });
            assert_eq!(versions(&offer), [version.to_string()]);
            assert_eq!(offer.latest, Some(version));
        }
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
        assert_eq!(Offer::of(&releases).latest, Some(Version::new(2, 0, 0)));
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
