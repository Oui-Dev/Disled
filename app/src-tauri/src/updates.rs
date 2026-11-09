// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! App updates: the signed `latest.json` of the latest release, checked at start-up
//! and then daily with the lamp firmware. The popup offers the update, installed only
//! when the user asks.

use std::sync::Mutex;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::firmware::{self, FirmwareState};
use crate::settings::SettingsState;

const EVENT_STATUS: &str = "update-status";

/// The first automatic check waits for the network, which may still be
/// coming up after a login.
const FIRST_CHECK_DELAY: Duration = Duration::from_secs(30);
/// The automatic checks look at the wall clock every hour, because a plain
/// 24 h sleep would not count the hours the computer spends asleep.
const TICK: Duration = Duration::from_secs(3600);
const CHECK_INTERVAL: Duration = Duration::from_secs(24 * 3600);
/// Without them, a connection that stalls, as across a sleep, would hold
/// the check, and the automatic checks after it, for good. A read timeout
/// rather than a total one, so that a slow download still ends.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const READ_TIMEOUT: Duration = Duration::from_secs(30);

/// What the popup shows about updates.
#[derive(Debug, Clone, Default, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct UpdateStatus {
    /// A check is running.
    pub checking: bool,
    /// Version found by the last check, newer than this app.
    pub available: Option<String>,
    /// Download progress of the update being installed, percent.
    pub progress: Option<u8>,
    /// Why the last check or install failed; cleared by the next success.
    pub error: Option<String>,
    /// Last successful check, seconds since 1970-01-01 UTC.
    pub checked_at: Option<u64>,
}

#[derive(Default)]
pub struct UpdateState {
    status: Mutex<UpdateStatus>,
    /// The update found by the last check, kept for the install.
    pending: Mutex<Option<Update>>,
}

impl UpdateState {
    pub fn status(&self) -> UpdateStatus {
        self.status.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

/// Seconds since 1970-01-01 UTC.
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Tells whether no check has succeeded yet or the last one is a day old.
fn due(checked_at: Option<u64>) -> bool {
    checked_at.is_none_or(|at| now().saturating_sub(at) >= CHECK_INTERVAL.as_secs())
}

/// Changes the status and tells every window.
fn change(app: &AppHandle, edit: impl FnOnce(&mut UpdateStatus)) {
    let status = match app.state::<UpdateState>().status.lock() {
        Ok(mut status) => {
            edit(&mut status);
            status.clone()
        }
        Err(_) => return,
    };
    let _ = app.emit(EVENT_STATUS, status);
}

/// Asks GitHub for a newer version; the `update-status` events bring the
/// result.
pub async fn check(app: &AppHandle) {
    // One at a time: a click during the automatic check gets its result,
    // through the events.
    let mut running = false;
    change(app, |s| running = std::mem::replace(&mut s.checking, true));
    if running {
        return;
    }
    // The update found keeps these timeouts for its download.
    let updater = app
        .updater_builder()
        .configure_client(|client| {
            client
                .connect_timeout(CONNECT_TIMEOUT)
                .read_timeout(READ_TIMEOUT)
        })
        .build();
    let result = match updater {
        Ok(updater) => updater.check().await,
        Err(error) => Err(error),
    };
    let state = app.state::<UpdateState>();
    match result {
        Ok(update) => {
            let version = update.as_ref().map(|u| u.version.clone());
            if let Ok(mut pending) = state.pending.lock() {
                *pending = update;
            }
            change(app, |s| {
                s.checking = false;
                s.available = version;
                s.error = None;
                s.checked_at = Some(now());
            });
        }
        Err(error) => change(app, |s| {
            s.checking = false;
            s.error = Some(error.to_string());
        }),
    }
}

/// Downloads and installs the update found by the last check, then
/// restarts the app. On Windows the installer closes the app and starts it
/// again itself.
pub async fn install(app: &AppHandle) -> Result<(), String> {
    let update = app
        .state::<UpdateState>()
        .pending
        .lock()
        .ok()
        .and_then(|pending| pending.clone())
        .ok_or("no update to install")?;
    change(app, |s| {
        s.progress = Some(0);
        s.error = None;
    });
    let mut received: u64 = 0;
    let mut shown: u8 = 0;
    let result = update
        .download_and_install(
            |chunk, total| {
                received += chunk as u64;
                if let Some(total) = total.filter(|&total| total > 0) {
                    let percent = (received * 100 / total).min(100) as u8;
                    if percent != shown {
                        shown = percent;
                        change(app, |s| s.progress = Some(percent));
                    }
                }
            },
            || {},
        )
        .await;
    match result {
        Ok(()) => app.restart(),
        Err(error) => {
            let error = error.to_string();
            change(app, |s| {
                s.progress = None;
                s.error = Some(error.clone());
            });
            Err(error)
        }
    }
}

/// Checks the app and the lamp firmware at start-up, then once a day while
/// the setting allows it. The first check waits for the network, unless
/// `at_once`. Debug builds skip them, because `tauri dev` would offer to
/// install the released app.
pub fn spawn_automatic_checks(app: AppHandle, at_once: bool) {
    if cfg!(debug_assertions) {
        return;
    }
    thread::Builder::new()
        .name("disled-updates".into())
        .spawn(move || {
            if !at_once {
                thread::sleep(FIRST_CHECK_DELAY);
            }
            loop {
                if app.state::<SettingsState>().get().check_updates {
                    if due(app.state::<UpdateState>().status().checked_at) {
                        tauri::async_runtime::block_on(check(&app));
                    }
                    if due(app.state::<FirmwareState>().status().checked_at) {
                        tauri::async_runtime::block_on(firmware::check(&app));
                    }
                }
                thread::sleep(TICK);
            }
        })
        .expect("spawn the update thread");
}
