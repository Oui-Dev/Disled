// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! The plugins: the folder `plugins` of the config folder of the app, where
//! each folder is a plugin, named by its id. A plugin comes disabled; the
//! user approves it in a question that lists what it can do, and approves it
//! again after any change of its files. The host reads the plugins, keeps
//! their settings and their states, runs their programs, serves their events
//! on the local server, and tells the control which integrations the lamp
//! can follow, their states and their alerts. The programs of the plugins
//! with the `lamp` capability get the state of the lamp and its presses.
//! This module ties the host to the app: its lock, the settings, the
//! control and the windows.

pub mod catalog;
mod host;
pub mod http;
pub mod lamp;
pub mod manifest;
pub mod process;
pub mod rules;
pub mod scan;
pub mod states;
pub mod tokens;
pub mod view;

use std::fs::File;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};

use crate::colors::{ColorsState, LampColor, PreviewState};
use crate::control::{self, ControlState, Input};
use crate::device::DeviceHandle;
use crate::i18n::{Locale, LocaleState};
use crate::settings;
use host::{Host, Outbox};
use manifest::Reason;
use process::{Notification, Program, ProgramStatus, Timing};
use view::{FilledBlock, PluginView};

pub(crate) const EVENT_PLUGINS: &str = "plugins-changed";
/// How often the states whose time runs out leave.
const TICK: Duration = Duration::from_secs(1);

/// The host, once the app started it.
#[derive(Default)]
pub struct PluginsState {
    host: Mutex<Option<Host>>,
    /// Taken while the state of the lamp goes to the programs, one change at
    /// a time.
    lamp: Mutex<()>,
    views: Mutex<Redraw>,
}

/// When the windows last redrew the plugins, and the views that came too
/// soon after, the last ones only.
#[derive(Default)]
struct Redraw {
    at: Option<Instant>,
    waiting: Option<Vec<PluginView>>,
}

/// Redraws the windows with `views` at the pace of the lamp, at most every
/// `control::INTEGRATION_SPACING`, so that a program that floods the app
/// with notifications costs the windows nothing more: views that come
/// sooner wait for the end of that time, in the place of those that waited.
fn redraw(app: &AppHandle, views: Vec<PluginView>) {
    let now = Instant::now();
    let state = app.state::<PluginsState>();
    let mut redraw = state.views.lock();
    let since = redraw.at.map(|at| now.saturating_duration_since(at));
    if let Some(since) = since.filter(|since| *since < control::INTEGRATION_SPACING) {
        if redraw.waiting.replace(views).is_none() {
            let owner = app.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(control::INTEGRATION_SPACING - since).await;
                let state = owner.state::<PluginsState>();
                let waiting = {
                    let mut redraw = state.views.lock();
                    redraw.at = Some(Instant::now());
                    redraw.waiting.take()
                };
                if let Some(views) = waiting {
                    let _ = owner.emit(EVENT_PLUGINS, views);
                }
            });
        }
        return;
    }
    redraw.at = Some(now);
    redraw.waiting = None;
    drop(redraw);
    let _ = app.emit(EVENT_PLUGINS, views);
}

fn language(app: &AppHandle) -> &'static str {
    match app.state::<LocaleState>().get() {
        Locale::Fr => "fr",
        Locale::En => "en",
    }
}

/// Reads the host. Unlike `with`, it neither syncs the control nor redraws
/// the windows, so a window can call it each time it redraws.
fn read<T>(app: &AppHandle, look: impl FnOnce(&Host) -> T) -> Option<T> {
    app.state::<PluginsState>().host.lock().as_ref().map(look)
}

/// Runs a change on the host, then what it leaves to do: the programs that
/// stop, the control, the settings, the windows.
fn with<T>(app: &AppHandle, change: impl FnOnce(&mut Host, &mut Outbox) -> T) -> Option<T> {
    let palette = app.state::<ColorsState>().get();
    let language = language(app);
    let mut outbox = Outbox::default();
    let result = {
        let state = app.state::<PluginsState>();
        let mut host = state.host.lock();
        let host = host.as_mut()?;
        let result = change(host, &mut outbox);
        serve(app, host);
        host.sync(&palette, language, &mut outbox);
        result
    };
    for program in outbox.stop {
        program.stop();
    }
    for input in outbox.control {
        let _ = control::input(app, input);
    }
    if let Some(value) = outbox.save {
        let _ = settings::save_integrations(app, value);
    }
    if let Some(views) = outbox.views {
        redraw(app, views);
    }
    Some(result)
}

/// Serves the events of the approved plugins. The windows redraw when
/// another program takes the port, or leaves it. Not in `with` itself,
/// where the closure would make each instance of `with` need another one,
/// without end.
fn serve(app: &AppHandle, host: &mut Host) {
    let owner = app.clone();
    host.serve(move || {
        with(&owner, |_, _| {});
    });
}

/// A change asked by a window, which fails while the host has not started.
fn change<T>(
    app: &AppHandle,
    edit: impl FnOnce(&mut Host, &mut Outbox) -> Result<T, String>,
) -> Result<T, String> {
    with(app, edit).unwrap_or_else(|| Err("the plugins are not ready".into()))
}

/// Starts the programs that the active plugins want, with the home folder
/// and the language of the app, and its callbacks.
fn run_programs(app: &AppHandle, host: &mut Host) {
    let home = app.path().home_dir().ok();
    host.run_programs(home, language(app), |spec, feed| {
        let (notes, statuses) = (app.clone(), app.clone());
        let (note_id, status_id) = (spec.id.clone(), spec.id.clone());
        Program::start(
            spec,
            Timing::default(),
            feed,
            move |note| on_note(&notes, &note_id, note),
            move |status| on_program(&statuses, &status_id, status),
        )
    });
}

/// Reads the folder again, then starts the programs that it allows.
fn rescan(app: &AppHandle, host: &mut Host, outbox: &mut Outbox) {
    host.rescan(outbox);
    run_programs(app, host);
}

/// Starts the host: reads the plugins, runs the programs of the active
/// ones, serves the events, and gives the control the integrations on offer
/// and the one the lamp followed.
pub fn start(app: &AppHandle) {
    let paths = app.path();
    let (Ok(config), Ok(local)) = (paths.app_config_dir(), paths.app_local_data_dir()) else {
        return;
    };
    let logs = paths.app_log_dir().unwrap_or_else(|_| local.join("logs"));
    let (events, mut received) = tokio::sync::mpsc::channel(http::QUEUE);
    let lamp = lamp::line(&lamp_now(app));
    let saved = settings::load_integrations(app);
    let host = Host::new(&config, &local, &logs, &saved, events, lamp);
    let mode = host.stored.mode.clone();
    *app.state::<PluginsState>().host.lock() = Some(host);
    with(app, |host, outbox| rescan(app, host, outbox));
    // The lamp follows again the integration it followed, if still on offer.
    if mode.is_some() {
        let _ = control::input(app, Input::Follow(mode));
    }
    let owner = app.clone();
    tauri::async_runtime::spawn(async move {
        while let Some((id, event)) = received.recv().await {
            on_event(&owner, &id, &event);
        }
    });
    let owner = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut ticks = tokio::time::interval(TICK);
        loop {
            ticks.tick().await;
            tick(&owner);
        }
    });
}

/// The states whose time ran out leave.
fn tick(app: &AppHandle) {
    let now = Instant::now();
    if read(app, |host| host.due(now)) == Some(true) {
        with(app, |host, _| host.expire(now));
    }
}

/// An event of the local server, for a plugin that may take it.
fn on_event(app: &AppHandle, id: &str, event: &Value) {
    with(app, |host, outbox| host.on_event(id, event, outbox));
}

/// A notification of a program.
fn on_note(app: &AppHandle, id: &str, note: Notification) {
    with(app, |host, outbox| host.on_note(id, note, outbox));
}

/// How a program fares.
fn on_program(app: &AppHandle, id: &str, status: ProgramStatus) {
    with(app, |host, _| host.on_program(id, status));
}

/// The plugins, read again, for the integrations window.
pub fn list(app: &AppHandle) -> Vec<PluginView> {
    let palette = app.state::<ColorsState>().get();
    let language = language(app);
    with(app, |host, outbox| {
        rescan(app, host, outbox);
        host.views(&palette, language)
    })
    .unwrap_or_default()
}

/// The block of a note, with the address and the token of the plugin.
pub fn note(app: &AppHandle, id: &str) -> Option<FilledBlock> {
    read(app, |host| host.note(id)).flatten()
}

/// The approval, for the fingerprint that the question showed: the plugin
/// turns on, with its token and its program.
pub fn approve(app: &AppHandle, id: &str, fingerprint: &str) -> Result<(), String> {
    change(app, |host, outbox| {
        let approved = host.approve(id, fingerprint, outbox);
        run_programs(app, host);
        approved
    })
}

/// Turns a plugin on or off.
pub fn set_enabled(app: &AppHandle, id: &str, enabled: bool) -> Result<(), String> {
    change(app, |host, outbox| {
        let set = host.set_enabled(id, enabled, outbox);
        run_programs(app, host);
        set
    })
}

/// The color of a state of a plugin, or the one of the plugin again, for
/// no `slot`.
pub fn set_color(app: &AppHandle, id: &str, state: &str, slot: Option<u8>) -> Result<(), String> {
    let palette = app.state::<ColorsState>().get();
    change(app, |host, outbox| {
        host.set_color(id, state, slot, &palette, outbox)
    })
}

/// A new token: the old one stops working, and the note unfolds again.
pub fn regenerate_token(app: &AppHandle, id: &str) -> Result<(), String> {
    change(app, |host, outbox| {
        host.tokens.regenerate(id).map_err(|e| e.to_string())?;
        host.entry(id).installed = false;
        host.save(outbox);
        Ok(())
    })
}

/// Removes a plugin: its program, its folder, its token, its data, and what
/// the app kept of it.
pub fn remove(app: &AppHandle, id: &str) -> Result<(), String> {
    change(app, |host, outbox| host.remove(id, outbox)).and_then(|(folder, data)| {
        // Once the program stopped, its files can go.
        std::fs::remove_dir_all(&folder).map_err(|e| e.to_string())?;
        let _ = std::fs::remove_dir_all(data);
        with(app, |host, outbox| rescan(app, host, outbox));
        Ok(())
    })
}

/// Imports the plugin of a zip file, in place of the plugin of the same id,
/// whose token and colors stay. Returns its id; the approval follows.
pub fn import(app: &AppHandle, zip: &Path) -> Result<String, Reason> {
    let zip = File::open(zip).map_err(|_| Reason::Io)?;
    put_in_place(app, unpack(app, zip)?)
}

/// Imports a plugin of the catalog, as `import` does, once its zip proved to
/// hold the plugin `id` with the files of `fingerprint`, those of the index.
fn import_reviewed(
    app: &AppHandle,
    zip: impl Read + Seek,
    id: &str,
    fingerprint: &str,
) -> Result<String, catalog::CatalogError> {
    let unreadable = |reason| catalog::CatalogError::Unreadable { reason };
    let unpacked = unpack(app, zip).map_err(unreadable)?;
    if !unpacked.matches(id, fingerprint) {
        unpacked.discard();
        return Err(catalog::CatalogError::Mismatch);
    }
    put_in_place(app, unpacked).map_err(unreadable)
}

/// Unpacks a zip file in the folder of the plugins, under a name that no
/// plugin takes until it is put in place.
fn unpack(app: &AppHandle, zip: impl Read + Seek) -> Result<scan::Unpacked, Reason> {
    let folder = read(app, |host| host.folder.clone()).ok_or(Reason::Io)?;
    let _ = std::fs::create_dir_all(&folder);
    scan::unpack(zip, &folder)
}

/// Puts an unpacked plugin in place of the plugin of its id, whose program
/// stops first, since its files go.
fn put_in_place(app: &AppHandle, unpacked: scan::Unpacked) -> Result<String, Reason> {
    let id = unpacked.id().to_owned();
    with(app, |host, outbox| {
        if let Some(program) = host.programs.remove(&id) {
            outbox.stop.push(program);
        }
    });
    let result = unpacked.install();
    with(app, |host, outbox| rescan(app, host, outbox));
    result
}

/// The plugins of the folder, read again, for the catalog.
fn installed(app: &AppHandle) -> catalog::Installed {
    with(app, |host, outbox| {
        rescan(app, host, outbox);
        host.installed()
    })
    .unwrap_or_default()
}

/// The folder of the plugins, which "Open the plugins folder" shows.
pub fn folder(app: &AppHandle) -> Option<PathBuf> {
    read(app, |host| host.folder.clone())
}

/// Remembers the integration the lamp follows, none in manual mode.
pub fn remember_mode(app: &AppHandle, mode: Option<String>) {
    with(app, |host, outbox| {
        host.stored.mode = mode;
        host.save(outbox);
    });
}

/// The color that takes the place of a deleted one on a lamp that follows a
/// plugin: the color that the plugin gives its state.
pub fn fallback_color(app: &AppHandle) -> Option<LampColor> {
    let mode = app.state::<ControlState>().status().mode?;
    read(app, |host| {
        let (state, _) = host.plugins.get(&mode)?.shown.as_ref()?;
        Some(host.manifest(&mode)?.state(state)?.color)
    })
    .flatten()
}

/// The palette changed: a state whose color left takes its default one, and
/// the programs that read the lamp get the new names of the colors.
pub fn colors_changed(app: &AppHandle) {
    let palette = app.state::<ColorsState>().get();
    with(app, |host, outbox| {
        let mut changed = false;
        for entry in host.stored.plugins.values_mut() {
            let before = entry.colors.len();
            entry.colors.retain(|_, slot| palette.defines(*slot));
            changed |= entry.colors.len() != before;
        }
        if changed {
            host.save(outbox);
        }
    });
    lamp_changed(app);
}

/// The labels of the states follow a new language, on the lamp page too,
/// and so do the default names of the colors that the programs read.
pub fn relabel(app: &AppHandle) {
    with(app, |_, _| {});
    lamp_changed(app);
}

/// The lamp as the plugins read it now.
fn lamp_now(app: &AppHandle) -> lamp::Lamp {
    let state = app
        .try_state::<DeviceHandle>()
        .and_then(|device| device.state());
    let kept = app
        .try_state::<PreviewState>()
        .and_then(|preview| preview.kept());
    let status = app
        .try_state::<ControlState>()
        .map(|control| control.status());
    let follows = status.as_ref().and_then(|status| status.mode.clone());
    let label = status.and_then(|status| status.state.map(|state| state.label));
    let palette = app.state::<ColorsState>().get();
    let locale = app.state::<LocaleState>().get();
    lamp::view(
        state.as_ref(),
        kept,
        &palette,
        locale,
        follows,
        label.as_deref(),
    )
}

/// Something that the plugins read of the lamp may have changed: its state,
/// the palette, the language or the integration it follows. The programs
/// that read the lamp get its new state, once. Never under the lock of the
/// host or of the control, which it takes.
pub fn lamp_changed(app: &AppHandle) {
    let Some(state) = app.try_state::<PluginsState>() else {
        return;
    };
    // A state read before another never reaches the programs after it.
    let _turn = state.lamp.lock();
    let line = lamp::line(&lamp_now(app));
    let mut host = state.host.lock();
    let Some(host) = host.as_mut().filter(|host| host.lamp != line) else {
        return;
    };
    for feed in host.programs.values().filter_map(Program::feed) {
        feed.state(line.clone());
    }
    host.lamp = line;
}

/// A short press on the lid, for the programs that read the lamp.
pub fn pressed(app: &AppHandle) {
    read(app, |host| {
        for feed in host.programs.values().filter_map(Program::feed) {
            feed.press(lamp::TOUCH_LINE);
        }
    });
}

/// The programs stop with the app, all at once, and the server closes.
pub fn shutdown(app: &AppHandle) {
    let programs: Vec<Program> = app
        .state::<PluginsState>()
        .host
        .lock()
        .as_mut()
        .map(|host| {
            if let Some(server) = host.server.take() {
                server.abort();
            }
            host.programs.drain().map(|(_, program)| program).collect()
        })
        .unwrap_or_default();
    let stoppers: Vec<_> = programs
        .into_iter()
        .map(|program| thread::spawn(move || program.stop()))
        .collect();
    for stopper in stoppers {
        let _ = stopper.join();
    }
}
