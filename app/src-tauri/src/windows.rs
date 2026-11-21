// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! The windows: the popup under the tray icon for daily use, and four that
//! open on demand: Diagnostics, the integrations and the two lamp updates.

use std::time::{Duration, Instant};

use disled_protocol::Command;
use parking_lot::Mutex;
use tauri::{
    AppHandle, LogicalSize, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
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

/// The width of the popup; its height follows its content.
const POPUP_WIDTH: f64 = 340.0;
/// The popup is never shorter, so that the lamp page gets some room and the
/// footer stays at the bottom.
const POPUP_MIN_HEIGHT: f64 = 360.0;
/// The resizable windows never get smaller.
const MIN_SIZE: (f64, f64) = (760.0, 560.0);
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
/// before the first click. On Linux, whose tray never tells its position,
/// the popup opens in the middle of the screen. The panel of macOS places
/// itself.
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
        #[cfg(target_os = "macos")]
        if panel::show(&window) {
            return;
        }
        place(&window);
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn hide_popup(app: &AppHandle) {
    if let Some(window) = popup(app) {
        let _ = window.hide();
        *app.state::<PopupState>().hidden_at.lock() = Some(Instant::now());
    }
}

/// Creates the popup, hidden. It hides whenever it loses the focus, and
/// instead of closing on Alt+F4 or Cmd+W, since nothing would create it
/// again. On macOS it becomes a panel, so that it opens over an app in full
/// screen too; should that fail, it stays a plain window, which shows
/// everywhere else.
pub fn create_popup(app: &AppHandle) -> tauri::Result<()> {
    let window = WebviewWindowBuilder::new(app, POPUP, WebviewUrl::App("index.html".into()))
        .title("Disled")
        .inner_size(POPUP_WIDTH, POPUP_MIN_HEIGHT)
        .resizable(false)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        // On Windows a page built focused takes the keyboard at once, even in
        // a hidden window, and the popup follows the focus of its page there:
        // the loss of that keyboard hid the popup that the launch had just
        // shown. Once shown, the window gets the focus and hands it to the
        // page. On macOS the page stays the first responder of the panel.
        .focused(!cfg!(target_os = "windows"))
        .build()?;
    let owner = app.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::Focused(false) => hide_popup(&owner),
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            hide_popup(&owner);
        }
        _ => {}
    });
    #[cfg(target_os = "macos")]
    panel::convert(&window);
    Ok(())
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
    #[cfg(target_os = "macos")]
    if panel::resize(&window, POPUP_WIDTH, height) {
        return height;
    }
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
        .is_some_and(|hidden_at| hidden_at.elapsed() < REOPEN_GUARD);
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

/// Opens a window on demand, and hides the popup. A window already open
/// comes to the front instead: `None`. A new one takes the title of its
/// label in the language in force, and the focus; resizable, it never gets
/// smaller than `MIN_SIZE`. Its frame takes the color mode set on the app
/// and follows it, because a theme of its own would outlast a change of
/// mode on Windows.
fn open(
    app: &AppHandle,
    label: &str,
    (width, height): (f64, f64),
    resizable: bool,
) -> Option<WebviewWindow> {
    hide_popup(app);
    if raise(app, label) {
        return None;
    }
    let mut builder = WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .title(title(label, app.state::<LocaleState>().get()))
        .inner_size(width, height)
        .resizable(resizable);
    if resizable {
        builder = builder.min_inner_size(MIN_SIZE.0, MIN_SIZE.1);
    }
    let window = builder.build().ok()?;
    let _ = window.set_focus();
    Some(window)
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
    let Some(window) = open(app, DIAGNOSTICS, (1040.0, 760.0), true) else {
        return;
    };
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
}

/// Opens the integrations window, from the mode menu of the popup.
pub fn open_integrations(app: &AppHandle) {
    open(app, INTEGRATIONS, (880.0, 680.0), true);
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
/// as soon as the user turns to the lamp or to Zadig. It opens without the
/// result of the last update. One update at a time: while the other window
/// runs one, it comes to the front instead, and when it does not, it closes,
/// so that a single window follows the updates.
fn open_update_window(app: &AppHandle, label: &str, other: &str) {
    firmware::clear_result(app);
    hide_popup(app);
    if let Some(window) = app.get_webview_window(other) {
        if firmware::running(app) {
            raise(app, other);
            return;
        }
        let _ = window.close();
    }
    // The longest step, the Windows driver in French, fits without scrolling.
    let Some(window) = open(app, label, (480.0, 640.0), false) else {
        return;
    };
    firmware::uncancel(app);
    // Otherwise the update would keep waiting for a bootloader, and flash
    // the next one with no window to show it.
    let owner = app.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::Destroyed = event {
            firmware::cancel(&owner);
        }
    });
}

/// The edges of a rectangle as AppKit counts them on macOS: in points, from
/// the bottom left corner of the main screen, upwards.
#[cfg(any(target_os = "macos", test))]
#[derive(Debug, Clone, Copy)]
struct Edges {
    left: f64,
    right: f64,
    bottom: f64,
    top: f64,
}

/// The top left corner of the popup on macOS: centred under the tray icon,
/// within the screen, and against the bottom of the icon, or of the menu bar
/// when that is lower. `screen` is the part that the menu bar and the Dock
/// leave.
#[cfg(any(target_os = "macos", test))]
fn top_left_under(icon: Edges, screen: Edges, width: f64) -> (f64, f64) {
    let centred = (icon.left + icon.right - width) / 2.0;
    let x = centred.min(screen.right - width).max(screen.left);
    (x, icon.bottom.min(screen.top))
}

/// The popup as a panel on macOS. A plain window belongs to a single Space,
/// and macOS shows none over an app in full screen; giving it the keyboard
/// also activated the app, upon which macOS can switch to another Space. A
/// panel joins every Space, full screen ones included, and takes the keyboard
/// without activating the app. AppKit places and sizes it at once, in points
/// under the tray icon: tao would move it only once shown, at first from the
/// middle of the screen, and the positioner mixes the scales of two displays
/// and gives up on a window that a display took away with it.
#[cfg(target_os = "macos")]
mod panel {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSScreen, NSWindow};
    use objc2_foundation::{NSPoint, NSRect, NSSize};
    use tauri::{Manager, WebviewWindow};
    use tauri_nspanel::{CollectionBehavior, Panel, StyleMask};

    use self::class::PopupPanel;
    use super::{Edges, top_left_under};

    /// The class of the panel, in a module of its own: its macro imports
    /// AppKit names too.
    mod class {
        tauri_nspanel::tauri_panel! {
            panel!(PopupPanel {
                config: {
                    // A window without a title bar refuses the keyboard otherwise.
                    can_become_key_window: true
                }
            })
        }
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

    /// Shows the panel under the tray icon, in front and with the keyboard,
    /// on the main thread that AppKit requires. False until the setup has
    /// made the panel, while a second launch can already ask for the popup.
    pub fn show(window: &WebviewWindow) -> bool {
        if window.try_state::<PopupPanel>().is_none() {
            return false;
        }
        let popup = window.clone();
        let _ = window.run_on_main_thread(move || {
            let (Some(panel), Some(mtm)) =
                (popup.try_state::<PopupPanel>(), MainThreadMarker::new())
            else {
                return;
            };
            // A hidden app shows none of its windows, after Cmd+H or the Hide
            // Others of another app. Activating it, as for a plain window,
            // would take the keyboard from the app in front.
            let app = NSApplication::sharedApplication(mtm);
            if app.isHidden() {
                app.unhideWithoutActivation();
            }
            place(panel.as_panel(), &popup, mtm);
            // In front before taking the keyboard, or it stays off the Space
            // of an app in full screen.
            panel.order_front_regardless();
            panel.make_key_window();
        });
        true
    }

    /// Gives the panel a new size, its top edge where it was, under the tray
    /// icon. False until the setup has made the panel.
    pub fn resize(window: &WebviewWindow, width: f64, height: f64) -> bool {
        if window.try_state::<PopupPanel>().is_none() {
            return false;
        }
        let popup = window.clone();
        let _ = window.run_on_main_thread(move || {
            if let Some(panel) = popup.try_state::<PopupPanel>() {
                let panel = panel.as_panel();
                let frame = panel.frame();
                let origin = NSPoint::new(frame.origin.x, frame.max().y - height);
                panel.setFrame_display(NSRect::new(origin, NSSize::new(width, height)), true);
            }
        });
        true
    }

    /// Puts the panel under the tray icon, or in the top right corner of the
    /// screen in use while the icon has no place in a menu bar.
    fn place(panel: &NSWindow, popup: &WebviewWindow, mtm: MainThreadMarker) {
        let width = panel.frame().size.width;
        let in_use = NSScreen::mainScreen(mtm).map(|screen| edges(screen.visibleFrame()));
        let top_left = match crate::tray::icon_frame(popup.app_handle()) {
            Some((icon, screen)) => screen
                .map(edges)
                .or(in_use)
                .map(|screen| top_left_under(edges(icon), screen, width)),
            None => in_use.map(|screen| (screen.right - width, screen.top)),
        };
        if let Some((x, y)) = top_left {
            panel.setFrameTopLeftPoint(NSPoint::new(x, y));
        }
    }

    fn edges(rect: NSRect) -> Edges {
        Edges {
            left: rect.min().x,
            right: rect.max().x,
            bottom: rect.min().y,
            top: rect.max().y,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A screen of 1512 × 982 points under a menu bar of 37 points.
    const SCREEN: Edges = Edges {
        left: 0.0,
        right: 1512.0,
        bottom: 0.0,
        top: 945.0,
    };

    fn icon(left: f64) -> Edges {
        Edges {
            left,
            right: left + 30.0,
            bottom: 945.0,
            top: 982.0,
        }
    }

    #[test]
    fn the_popup_opens_under_the_tray_icon_within_its_screen() {
        assert_eq!(
            top_left_under(icon(1000.0), SCREEN, POPUP_WIDTH),
            (845.0, 945.0)
        );
        // Near an edge, it stays on the screen: the right edge of the main
        // display, the left edge of a display on its left.
        assert_eq!(
            top_left_under(icon(1470.0), SCREEN, POPUP_WIDTH),
            (1172.0, 945.0)
        );
        let on_the_left = Edges {
            left: -1512.0,
            right: 0.0,
            ..SCREEN
        };
        assert_eq!(
            top_left_under(icon(-1505.0), on_the_left, POPUP_WIDTH),
            (-1512.0, 945.0)
        );
    }

    #[test]
    fn the_popup_opens_under_the_menu_bar_of_an_app_in_full_screen() {
        // A menu bar that hides itself leaves the whole screen; shown over
        // the app, it holds the icon.
        let whole = Edges {
            top: 982.0,
            ..SCREEN
        };
        assert_eq!(
            top_left_under(icon(1000.0), whole, POPUP_WIDTH),
            (845.0, 945.0)
        );
        // Hidden above the screen, the icon leaves the popup under the place
        // of the menu bar.
        let hidden = Edges {
            bottom: 982.0,
            top: 1019.0,
            ..icon(1000.0)
        };
        assert_eq!(top_left_under(hidden, SCREEN, POPUP_WIDTH), (845.0, 945.0));
    }
}
