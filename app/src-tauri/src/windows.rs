// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! The windows: the popup under the tray icon for daily use, and four that
//! open on demand: Diagnostics, the integrations and the two lamp updates.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use disled_protocol::Command;
use tauri::{
    AppHandle, LogicalSize, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
    Wry,
};
use tauri_plugin_positioner::{Position, WindowExt};

use crate::device::DeviceHandle;
use crate::firmware;
use crate::i18n::{Locale, LocaleState};

const POPUP: &str = "popup";
pub const DIAGNOSTICS: &str = "diagnostics";
/// The plugins, which bring the integrations.
const INTEGRATIONS: &str = "integrations";
/// The guided update, to the newest firmware.
const LAMP_UPDATE: &str = "lamp-update";
/// The reinstall of a version of the user's choice.
const LAMP_INSTALL: &str = "lamp-install";

/// The windows opened on demand, with the message key of their title.
const TITLED: [(&str, &str); 4] = [
    (DIAGNOSTICS, "window.diagnostics"),
    (INTEGRATIONS, "window.integrations"),
    (LAMP_UPDATE, "window.lampUpdate"),
    (LAMP_INSTALL, "window.lampInstall"),
];

/// A click on the tray icon first takes the focus away from an open popup,
/// which hides it; the click itself must then not reopen it.
const REOPEN_GUARD: Duration = Duration::from_millis(300);

/// The width of the popup, as in tauri.conf.json; its height follows its
/// content.
const POPUP_WIDTH: f64 = 340.0;
/// The popup is never shorter, as in tauri.conf.json, so that the lamp page
/// gets some room and the footer stays at the bottom.
const POPUP_MIN_HEIGHT: f64 = 360.0;
/// Room kept on the screen around a popup taller than the screen, whose
/// content then scrolls.
const SCREEN_MARGIN: f64 = 48.0;

#[derive(Default)]
pub struct PopupState {
    hidden_at: Mutex<Option<Instant>>,
}

fn popup(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(POPUP)
}

/// Places the popup next to the tray icon, above it on Windows and below it
/// on macOS, or in a screen corner while the icon position is unknown,
/// before the first click or on Linux.
fn place(window: &WebviewWindow) {
    let (tray, fallback) = if cfg!(target_os = "macos") {
        (Position::TrayBottomCenter, Position::TopRight)
    } else if cfg!(target_os = "windows") {
        (Position::TrayCenter, Position::BottomRight)
    } else {
        (Position::TrayCenter, Position::Center)
    };
    if window.move_window_constrained(tray).is_err() {
        let _ = window.move_window(fallback);
    }
}

pub fn show_popup(app: &AppHandle) {
    if let Some(window) = popup(app) {
        place(&window);
        #[cfg(target_os = "macos")]
        if panel::show(&window) {
            return;
        }
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn hide_popup(app: &AppHandle) {
    if let Some(window) = popup(app) {
        let _ = window.hide();
        if let Ok(mut hidden_at) = app.state::<PopupState>().hidden_at.lock() {
            *hidden_at = Some(Instant::now());
        }
    }
}

/// Hides the popup whenever it loses the focus, and instead of closing it on
/// Alt+F4 or Cmd+W, since nothing would create it again.
pub fn hide_popup_on_blur_or_close(app: &AppHandle) {
    if let Some(window) = popup(app) {
        let owner = app.clone();
        window.on_window_event(move |event| match event {
            WindowEvent::Focused(false) => hide_popup(&owner),
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                hide_popup(&owner);
            }
            _ => {}
        });
    }
}

/// Makes the popup a panel on macOS, so that it opens over an app in full
/// screen too. Should that fail, it stays a plain window, which shows
/// everywhere else.
#[cfg(target_os = "macos")]
pub fn make_popup_a_panel(app: &AppHandle) {
    if let Some(window) = popup(app) {
        panel::convert(&window);
    }
}

/// Fits the popup to the height of its content, in logical pixels, from its
/// minimum up to the screen, and keeps it against the tray icon. Returns the
/// height it took; when it is shorter than the content, the content scrolls.
pub fn resize_popup(app: &AppHandle, height: f64) -> f64 {
    let Some(window) = popup(app) else {
        return height;
    };
    let screen = window
        .current_monitor()
        .ok()
        .flatten()
        .map(|monitor| f64::from(monitor.work_area().size.height) / monitor.scale_factor())
        .unwrap_or(f64::INFINITY);
    // On a screen too small for the minimum, the screen wins.
    let height = height
        .max(POPUP_MIN_HEIGHT)
        .min(screen - SCREEN_MARGIN)
        .max(1.0);
    let resized = window.set_size(LogicalSize::new(POPUP_WIDTH, height));
    if resized.is_ok() && window.is_visible().unwrap_or(false) {
        place(&window);
    }
    height
}

pub fn toggle_popup(app: &AppHandle) {
    let Some(window) = popup(app) else { return };
    if window.is_visible().unwrap_or(false) {
        hide_popup(app);
        return;
    }
    let just_hidden = app
        .state::<PopupState>()
        .hidden_at
        .lock()
        .map(|t| t.is_some_and(|t| t.elapsed() < REOPEN_GUARD))
        .unwrap_or(false);
    if !just_hidden {
        show_popup(app);
    }
}

/// Brings a window that is already open to the front.
fn raise(app: &AppHandle, label: &str) -> bool {
    let Some(window) = app.get_webview_window(label) else {
        return false;
    };
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
    true
}

fn title(label: &str, locale: Locale) -> String {
    TITLED
        .iter()
        .find(|(titled, _)| *titled == label)
        .map(|(_, key)| locale.t(key))
        .unwrap_or_default()
}

/// A window opened on demand, in the language in force. Its frame takes the
/// color mode set on the app and follows it, because a theme of its own
/// would outlast a change of mode on Windows.
fn builder<'a>(app: &'a AppHandle, label: &str) -> WebviewWindowBuilder<'a, Wry, AppHandle> {
    WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .title(title(label, app.state::<LocaleState>().get()))
}

/// Titles the open windows in another language.
pub fn retitle(app: &AppHandle, locale: Locale) {
    for (label, key) in TITLED {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.set_title(&locale.t(key));
        }
    }
}

pub fn open_diagnostics(app: &AppHandle) {
    hide_popup(app);
    if raise(app, DIAGNOSTICS) {
        return;
    }
    let built = builder(app, DIAGNOSTICS)
        .inner_size(1040.0, 760.0)
        .min_inner_size(760.0, 560.0)
        .build();
    if let Ok(window) = built {
        // The window starts the telemetry, and a lamp streams until told to
        // stop, so the stream ends with the window.
        let owner = app.clone();
        window.on_window_event(move |event| {
            if let WindowEvent::Destroyed = event
                && let Some(device) = owner.try_state::<DeviceHandle>()
            {
                // Without a lamp, nothing streams.
                let _ = device.send(Command::SetTelemetry { period_ms: 0 });
            }
        });
        let _ = window.set_focus();
    }
}

/// Opens the integrations window, from the mode menu of the popup.
pub fn open_integrations(app: &AppHandle) {
    hide_popup(app);
    if raise(app, INTEGRATIONS) {
        return;
    }
    let built = builder(app, INTEGRATIONS)
        .inner_size(880.0, 680.0)
        .min_inner_size(760.0, 560.0)
        .build();
    if let Ok(window) = built {
        let _ = window.set_focus();
    }
}

/// Opens the guided lamp update.
pub fn open_lamp_update(app: &AppHandle) {
    open_update_window(app, LAMP_UPDATE, LAMP_INSTALL);
}

/// Opens the reinstall of a firmware version.
pub fn open_lamp_install(app: &AppHandle) {
    open_update_window(app, LAMP_INSTALL, LAMP_UPDATE);
}

/// Opens a lamp update window, a window rather than the popup, which hides
/// as soon as the user turns to the lamp or to Zadig. One update at a time:
/// while the other window runs one, it comes to the front instead, and when
/// it does not, it closes, so that a single window follows the updates.
fn open_update_window(app: &AppHandle, label: &str, other: &str) {
    hide_popup(app);
    if let Some(window) = app.get_webview_window(other) {
        if firmware::running(app) {
            raise(app, other);
            return;
        }
        let _ = window.close();
    }
    if raise(app, label) {
        return;
    }
    // The longest step, the Windows driver in French, fits without scrolling.
    let built = builder(app, label)
        .inner_size(480.0, 640.0)
        .resizable(false)
        .build();
    if let Ok(window) = built {
        firmware::uncancel(app);
        // Otherwise the update would keep waiting for a bootloader, and flash
        // the next one with no window to show it.
        let owner = app.clone();
        window.on_window_event(move |event| {
            if let WindowEvent::Destroyed = event {
                firmware::cancel(&owner);
            }
        });
        let _ = window.set_focus();
    }
}

/// The popup as a panel on macOS. A plain window belongs to a single Space,
/// and macOS shows none over an app in full screen; giving it the keyboard
/// also activated the app, upon which macOS can switch to another Space. A
/// panel joins every Space, full screen ones included, and takes the keyboard
/// without activating the app.
#[cfg(target_os = "macos")]
mod panel {
    use tauri::{Manager, WebviewWindow};
    use tauri_nspanel::{CollectionBehavior, Panel, StyleMask};

    tauri_nspanel::tauri_panel! {
        panel!(PopupPanel {
            config: {
                // A window without a title bar refuses the keyboard otherwise.
                can_become_key_window: true
            }
        })
    }

    /// Turns the popup into the panel, from the setup, on the main thread.
    pub fn convert(window: &WebviewWindow) {
        let Ok(panel) = PopupPanel::from_window(window) else {
            return;
        };
        // Should AppKit refuse it, the panel still shows, and a click in it
        // activates the app.
        let _ = panel.add_style_mask(StyleMask::empty().nonactivating_panel().into());
        panel.set_collection_behavior(
            CollectionBehavior::new()
                .can_join_all_spaces()
                .full_screen_auxiliary()
                .into(),
        );
        window.app_handle().manage(panel);
    }

    /// Brings the panel to the front with the keyboard, on the main thread
    /// that AppKit requires. False until the setup has made the panel, while
    /// a second launch can already ask for the popup.
    pub fn show(window: &WebviewWindow) -> bool {
        if window.try_state::<PopupPanel>().is_none() {
            return false;
        }
        let popup = window.clone();
        let _ = window.run_on_main_thread(move || {
            if let Some(panel) = popup.try_state::<PopupPanel>() {
                // In front before taking the keyboard, or it stays off the
                // Space of an app in full screen.
                panel.order_front_regardless();
                panel.make_key_window();
            }
        });
        true
    }
}
