// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! Disled desktop app: a tray icon with a popup for daily use, a
//! Diagnostics window, the USB worker behind both, the control of the lamp
//! beyond the choices of the user, the app updates and the lamp firmware
//! updates.

// Prevents an additional console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod colors;
mod control;
mod device;
mod firmware;
mod i18n;
mod settings;
mod tray;
mod updates;
mod windows;

use colors::{Around, ColorsState, Palette, PaletteChange, PreviewState};
use control::{ControlState, ControlStatus, Input};
use device::{Connection, DeviceHandle};
use disled_protocol::{Command, SetState};
use firmware::{FirmwareState, FirmwareStatus};
use i18n::{Locale, LocaleState};
use serde::Deserialize;
use settings::{Language, Settings, SettingsPatch, SettingsState};
use tauri::{AppHandle, Emitter, Manager, RunEvent, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_opener::OpenerExt;
use tray::TrayState;
use updates::{UpdateState, UpdateStatus};

/// Argument of the login item. Started with it at login, the app stays in
/// the tray.
const AUTOSTART_ARG: &str = "--autostart";

const EVENT_SETTINGS: &str = "settings-changed";
const EVENT_COLORS: &str = "colors-changed";
const EVENT_LOCALE: &str = "locale-changed";

/// Current connection, for a frontend that starts after the first event.
#[tauri::command]
fn device_connection(device: State<'_, DeviceHandle>) -> Connection {
    device.connection()
}

/// Queues a command for the lamp. Replies arrive as `device-message` events.
/// Diagnostics sends its commands this way; the popup says what the user
/// chose with `lamp_choose`.
#[tauri::command]
fn device_send(device: State<'_, DeviceHandle>, command: Command) -> Result<(), String> {
    device.send(command)
}

/// What the user chose for the lamp: off, or on, with a color slot or the
/// color it keeps.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, optional_fields))]
struct LampChoice {
    on: bool,
    slot: Option<u8>,
}

/// A choice of the user for the lamp, from a tile or the card of an
/// integration in the popup. The control hears of it: during a call, the
/// choice keeps the hand.
#[tauri::command]
fn lamp_choose(app: AppHandle, choice: LampChoice) -> Result<(), String> {
    if !app.state::<DeviceHandle>().connection().connected {
        return Err("no Disled lamp connected".into());
    }
    control::input(
        &app,
        Input::Choose {
            on: choice.on,
            slot: choice.slot,
        },
    )
}

/// What the windows show of the control, for a window that starts.
#[tauri::command]
fn control_get(control: State<'_, ControlState>) -> ControlStatus {
    control.status()
}

/// The source the lamp follows: an integration on offer, or none for the
/// user.
#[tauri::command]
fn control_set_mode(app: AppHandle, mode: Option<String>) -> Result<(), String> {
    let offered = app.state::<ControlState>().status().integrations;
    if let Some(id) = &mode
        && !offered.iter().any(|integration| &integration.id == id)
    {
        return Err(format!("no integration {id}"));
    }
    control::input(&app, Input::Follow(mode))
}

#[tauri::command]
fn colors_get(colors: State<'_, ColorsState>) -> Palette {
    colors.get()
}

/// Saves a palette that the user changed, tells every window, shows it in
/// the tray, and sends its commands to the lamp. Without a lamp, the palette
/// reaches it at its next connection.
fn apply_palette(app: &AppHandle, palette: Palette, commands: Vec<Command>) -> Result<(), String> {
    settings::save_palette(app, palette.clone())?;
    let _ = app.emit(EVENT_COLORS, palette);
    let device = app.state::<DeviceHandle>();
    for command in commands {
        let _ = device.send(command);
    }
    tray::refresh(app);
    Ok(())
}

/// Applies a deletion or an import: the palette, the call color when it
/// lost its color, and the color a call gives back.
fn apply_change(app: &AppHandle, change: PaletteChange) -> Result<(), String> {
    let mut settings = app.state::<SettingsState>().get();
    let call_color = settings.calls.color != change.call_color;
    if call_color {
        settings.calls.color = change.call_color;
        settings::save(app, settings)?;
        let _ = app.emit(EVENT_SETTINGS, settings);
    }
    apply_palette(app, change.palette, change.commands)?;
    let _ = control::input(app, Input::CallBack(change.call_then));
    if call_color {
        let _ = control::input(app, Input::CallSettings(settings.calls));
    }
    Ok(())
}

/// What else uses the colors now: the lamp, the call that shows its color,
/// and the integration the lamp follows.
fn around(app: &AppHandle) -> Around {
    let control = app.state::<ControlState>();
    Around {
        lamp: app.state::<DeviceHandle>().state().map(|state| state.color),
        call_color: app.state::<SettingsState>().get().calls.color,
        call_then: control.call_back(),
        integration: control.integration_slot(),
    }
}

/// A custom color made or changed in the editor, without `slot` for a new
/// one. A preview on the lamp ends: the lamp goes back to its state, or,
/// with `keep_lit`, stays on the new color, as from the "My colors" popover.
/// Returns the slot of the color.
#[tauri::command]
fn colors_save_custom(
    app: AppHandle,
    slot: Option<u8>,
    name: String,
    hex: String,
    keep_lit: bool,
) -> Result<u8, String> {
    let mut palette = app.state::<ColorsState>().get();
    let saved = palette.save_custom(slot, &name, &hex)?;
    let mut commands: Vec<Command> = palette
        .custom
        .iter()
        .filter(|color| color.slot == saved)
        .filter_map(|color| {
            Some(Command::SetColor {
                slot: saved,
                rgb: colors::led_of(&color.hex)?,
            })
        })
        .collect();
    commands.extend(
        app.state::<PreviewState>()
            .end((keep_lit && slot.is_none()).then_some(saved)),
    );
    apply_palette(&app, palette, commands)?;
    Ok(saved)
}

/// A new name for a built-in color, `null` for its default name.
#[tauri::command]
fn colors_rename_original(
    app: AppHandle,
    locale: State<'_, LocaleState>,
    slot: u8,
    name: Option<String>,
) -> Result<(), String> {
    let default = tray::default_name(slot, locale.get()).ok_or("not a built-in color")?;
    let mut palette = app.state::<ColorsState>().get();
    palette.rename_original(slot, name.as_deref(), &default)?;
    apply_palette(&app, palette, Vec::new())
}

#[tauri::command]
fn colors_set_cycle(app: AppHandle, slots: Vec<u8>) -> Result<(), String> {
    let mut palette = app.state::<ColorsState>().get();
    palette.set_cycle(&slots)?;
    apply_palette(&app, palette, vec![Command::SetTouchCycle { slots }])
}

/// Deletes a custom color, even in use, with its fallbacks (colors.rs).
#[tauri::command]
fn colors_delete(app: AppHandle, slot: u8) -> Result<(), String> {
    let change = app
        .state::<ColorsState>()
        .get()
        .delete(slot, &around(&app))
        .ok_or("no such custom color")?;
    apply_change(&app, change)
}

/// The share code of the colors, for the "Send" block.
#[tauri::command]
fn colors_share_code(colors: State<'_, ColorsState>) -> String {
    colors.get().share_code()
}

/// "Copy the code": the backend writes the clipboard, which no window reads.
#[tauri::command]
fn colors_copy_code(app: AppHandle) -> Result<(), String> {
    let code = app.state::<ColorsState>().get().share_code();
    app.clipboard().write_text(code).map_err(|e| e.to_string())
}

/// The colors of a pasted code, for the import preview; `null` when the
/// text holds none that reads.
#[tauri::command]
fn colors_read_code(text: String) -> Option<Palette> {
    colors::read_share_code(&text)
}

/// Replaces the colors with those of a pasted code.
#[tauri::command]
fn colors_import(app: AppHandle, text: String) -> Result<(), String> {
    let shared = colors::read_share_code(&text).ok_or("no share code")?;
    let change = app
        .state::<ColorsState>()
        .get()
        .import(&shared, &around(&app));
    apply_change(&app, change)
}

/// The preview of the editor on the lamp: a code starts or follows it, and
/// `null` ends it, the lamp back to its state. Only with the preview setting
/// on and a lamp that the app drives.
#[tauri::command]
fn colors_preview(app: AppHandle, hex: Option<String>) -> Result<(), String> {
    let device = app.state::<DeviceHandle>();
    let preview = app.state::<PreviewState>();
    let commands = match hex {
        Some(hex) => {
            let Some(lamp) = device.state() else {
                return Ok(());
            };
            if !app.state::<SettingsState>().get().color_preview {
                return Ok(());
            }
            let rgb = colors::parse_hex(&hex)
                .and_then(|hex| colors::led_of(&hex))
                .ok_or("not a color code")?;
            preview.follow(&lamp, rgb)
        }
        None => preview.end(None),
    };
    for command in commands {
        let _ = device.send(command);
    }
    Ok(())
}

/// Writes a recording chosen by the user in a save dialog. Async, so that
/// a long recording does not hold up the event loop, and every window with it.
#[tauri::command]
async fn save_csv(path: String, contents: String) -> Result<(), String> {
    if !path.to_ascii_lowercase().ends_with(".csv") {
        return Err("recordings are saved as .csv files".into());
    }
    std::fs::write(&path, contents).map_err(|e| e.to_string())
}

#[tauri::command]
fn settings_get(settings: State<'_, SettingsState>) -> Settings {
    settings.get()
}

/// Changes settings, saves them, and applies what changed to the app and to
/// the lamp.
#[tauri::command]
fn settings_set(
    app: AppHandle,
    device: State<'_, DeviceHandle>,
    patch: SettingsPatch,
) -> Result<Settings, String> {
    let mut settings = app.state::<SettingsState>().get();
    settings.apply(&patch);
    settings.fit(&app.state::<ColorsState>().get());
    settings::save(&app, settings)?;
    // Every window keeps its own copy, so all of them hear of the change.
    let _ = app.emit(EVENT_SETTINGS, settings);
    if patch.color_mode.is_some() {
        app.set_theme(settings.color_mode.theme());
    }
    if patch.language.is_some() {
        apply_language(&app, settings.language);
    }
    // Without a lamp, or with one that just left, the settings are saved
    // all the same and reach the lamp at its next connection.
    if patch.brightness_mode.is_some() || patch.manual_level.is_some() {
        let _ = device.send(Command::SetState(SetState {
            brightness_mode: Some(settings.brightness_mode),
            manual_level: Some(settings.manual_level),
            ..SetState::default()
        }));
    }
    if patch.auto_bias.is_some() {
        let _ = device.send(Command::SetAutoBias {
            bias: settings.auto_bias,
        });
    }
    if patch.touch_sensitivity.is_some() {
        let _ = device.send(Command::SetTouchSensitivity {
            percent: settings.touch_sensitivity,
        });
    }
    if patch.calls_mic.is_some() || patch.calls_color.is_some() {
        let _ = control::input(&app, Input::CallSettings(settings.calls));
    }
    Ok(settings)
}

/// Puts the tray and the window titles in the language of the setting, and
/// tells the windows when it changes.
fn apply_language(app: &AppHandle, language: Language) {
    let locale = Locale::of(language);
    if app.state::<LocaleState>().replace(locale) != locale {
        tray::relabel(app, locale);
        windows::retitle(app, locale);
        let _ = app.emit(EVENT_LOCALE, locale);
    }
}

/// The language of the app now, for a window that starts.
#[tauri::command]
fn locale_get(locale: State<'_, LocaleState>) -> Locale {
    locale.get()
}

#[tauri::command]
fn autostart_get(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
fn autostart_set(app: AppHandle, enabled: bool) -> Result<bool, String> {
    let launcher = app.autolaunch();
    let result = if enabled {
        launcher.enable()
    } else {
        launcher.disable()
    };
    result.map_err(|e| e.to_string())?;
    Ok(launcher.is_enabled().unwrap_or(enabled))
}

#[tauri::command]
fn update_status(updates: State<'_, UpdateState>) -> UpdateStatus {
    updates.status()
}

#[tauri::command]
async fn update_check(app: AppHandle) {
    updates::check(&app).await;
}

/// Returns only on failure, since on success the app restarts.
#[tauri::command]
async fn update_install(app: AppHandle) -> Result<(), String> {
    updates::install(&app).await
}

#[tauri::command]
fn firmware_status(firmware: State<'_, FirmwareState>) -> FirmwareStatus {
    firmware.status()
}

#[tauri::command]
async fn firmware_check(app: AppHandle) {
    firmware::check(&app).await;
}

/// Starts the update to the newest firmware 2.x, from the guided window;
/// `firmware-status` events follow it.
#[tauri::command]
fn firmware_install(app: AppHandle) -> Result<(), String> {
    firmware::install(&app)
}

/// Starts the update to a version of the list, from the reinstall window.
#[tauri::command]
fn firmware_install_manual(app: AppHandle, version: String) -> Result<(), String> {
    firmware::install_manual(&app, &version)
}

#[tauri::command]
fn firmware_cancel(app: AppHandle) {
    firmware::cancel(&app);
}

/// Async on purpose, because a synchronous command that creates a window
/// deadlocks WebView2 on Windows, leaving a blank window and a frozen app.
#[tauri::command]
async fn open_diagnostics(app: AppHandle) {
    windows::open_diagnostics(&app);
}

/// Async for the same reason as `open_diagnostics`.
#[tauri::command]
async fn open_lamp_update(app: AppHandle) {
    firmware::clear_result(&app);
    windows::open_lamp_update(&app);
}

/// Async for the same reason as `open_diagnostics`.
#[tauri::command]
async fn open_lamp_install(app: AppHandle) {
    firmware::clear_result(&app);
    windows::open_lamp_install(&app);
}

/// The popup measures its content and asks for that height; it gets the
/// height that fits its minimum and the screen.
#[tauri::command]
fn popup_resize(app: AppHandle, height: f64) -> f64 {
    windows::resize_popup(&app, height)
}

/// Escape pressed in the popup.
#[tauri::command]
fn popup_hide(app: AppHandle) {
    windows::hide_popup(&app);
}

/// Pages the windows may open in the browser.
#[derive(Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
enum Link {
    /// The tool that installs the WinUSB driver of the lamp's bootloader.
    Zadig,
    /// Flashing by hand, when the app cannot.
    FlashingGuide,
    /// The release of a firmware version on GitHub.
    ReleaseNotes { version: String },
}

#[tauri::command]
fn open_link(app: AppHandle, link: Link) -> Result<(), String> {
    let url = match link {
        Link::Zadig => "https://zadig.akeo.ie/".to_owned(),
        Link::FlashingGuide => {
            "https://github.com/Oui-Dev/Disled/blob/main/docs/flashing.md".to_owned()
        }
        Link::ReleaseNotes { version } => firmware::release_page(&version)?,
    };
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

/// "Quit" of the popup.
#[tauri::command]
fn quit(app: AppHandle) {
    quit_app(&app);
}

/// The one way out of the app, for "Quit" in the popup and in the tray
/// menu: the lamp first gets its own behavior back.
fn quit_app(app: &AppHandle) {
    control::release(app);
    app.exit(0);
}

/// Loads the settings and the palette, and saves them once, corrected. At
/// the very first launch of an installed app, it turns on start at login,
/// but never under `tauri dev`, which would register the debug executable.
fn init_settings(app: &AppHandle) -> Settings {
    let (mut settings, palette) = settings::load(app);
    if !settings.autostart_initialized && !cfg!(debug_assertions) {
        let _ = app.autolaunch().enable();
        settings.autostart_initialized = true;
    }
    let _ = settings::save_palette(app, palette);
    let _ = settings::save(app, settings);
    settings
}

fn main() {
    let started_at_login = std::env::args().any(|arg| arg == AUTOSTART_ARG);
    let app = tauri::Builder::default()
        // First, so that a second launch only shows the popup of the running
        // app.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            windows::show_popup(app);
        }))
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args([AUTOSTART_ARG])
                .build(),
        )
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        // From Rust only, for "Copy the code": no window has its permission.
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        // Only open_link uses it, from Rust, so no window has its permission.
        .plugin(tauri_plugin_opener::init())
        .manage(SettingsState::default())
        .manage(ColorsState::default())
        .manage(PreviewState::default())
        .manage(LocaleState::default())
        .manage(TrayState::default())
        .manage(windows::PopupState::default())
        .manage(UpdateState::default())
        .manage(FirmwareState::default())
        .setup(move |app| {
            // No Dock icon, since the app lives in the menu bar.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            // The shortcut of the developer tools, in debug builds only,
            // which a capability file would grant to every build.
            #[cfg(debug_assertions)]
            app.add_capability(
                tauri::ipc::CapabilityBuilder::new("devtools")
                    .window("*")
                    .permission("core:webview:allow-internal-toggle-devtools"),
            )?;

            let handle = app.handle().clone();
            let settings = init_settings(&handle);
            // Before the USB worker, which tells it of the lamp.
            app.manage(ControlState::new(settings.calls));
            handle.set_theme(settings.color_mode.theme());
            let locale = Locale::of(settings.language);
            handle.state::<LocaleState>().replace(locale);
            tray::create(&handle, locale)?;
            windows::hide_popup_on_blur_or_close(&handle);
            #[cfg(target_os = "macos")]
            windows::make_popup_a_panel(&handle);
            app.manage(DeviceHandle::spawn(handle.clone()));
            // A new version most often follows an update, whose restart keeps
            // the arguments, `--autostart` included. It checks again at once
            // and shows the popup, so that a lamp update shows next.
            let new_version = settings::first_launch_of_version(&handle);
            updates::spawn_automatic_checks(handle.clone(), new_version);
            if !started_at_login || new_version {
                windows::show_popup(&handle);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            device_connection,
            device_send,
            lamp_choose,
            control_get,
            control_set_mode,
            colors_get,
            colors_save_custom,
            colors_rename_original,
            colors_set_cycle,
            colors_delete,
            colors_share_code,
            colors_copy_code,
            colors_read_code,
            colors_import,
            colors_preview,
            save_csv,
            settings_get,
            settings_set,
            locale_get,
            autostart_get,
            autostart_set,
            update_status,
            update_check,
            update_install,
            firmware_status,
            firmware_check,
            firmware_install,
            firmware_install_manual,
            firmware_cancel,
            open_diagnostics,
            open_lamp_update,
            open_lamp_install,
            popup_resize,
            popup_hide,
            open_link,
            quit
        ])
        .build(tauri::generate_context!())
        .expect("error while building the Disled app");

    app.run(|_app, event| {
        // A tray app keeps running with no window open; Quit calls exit(0),
        // which comes with an exit code.
        if let RunEvent::ExitRequested {
            code: None, api, ..
        } = event
        {
            api.prevent_exit();
        }
    });
}
