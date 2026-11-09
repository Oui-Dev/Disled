// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! The tray icon, in the menu bar on macOS and the notification area on
//! Windows: a square in the color of the lamp, a short menu, and the click
//! that opens the popup.

use std::sync::Mutex;

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

use crate::i18n::{Locale, LocaleState};
use crate::windows;

const TRAY_ID: &str = "disled";
const SIZE: u32 = 32;
/// Half the side of the square, in pixels.
const HALF: f32 = 14.0;
/// Corner radius, over a quarter of the side, because smaller corners
/// vanish once the system shows this icon at 16 px.
const CORNER: f32 = 8.0;

/// The built-in colors, by palette slot: the key of their tooltip, and their
/// color on screen, as the swatches of the popup (`app/src/utils/lamp.ts`).
const BUILT_IN_COLORS: [(&str, [u8; 3]); 4] = [
    ("green", [0x2e, 0xcc, 0x71]),
    ("orange", [0xf3, 0x9c, 0x12]),
    ("red", [0xe7, 0x4c, 0x3c]),
    ("warm_white", [0xff, 0xe2, 0xb8]),
];

/// What the icon shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Look {
    #[default]
    NoLamp,
    Off,
    /// On, with a built-in color, by palette slot.
    On(u8),
    /// A version 1 lamp, which the app does not drive.
    NeedsUpdate,
    /// A lamp of a later protocol than the app.
    Newer,
}

impl Look {
    /// A lamp the app does not drive, whose Diagnostics have nothing to show.
    fn locked(self) -> bool {
        matches!(self, Look::NeedsUpdate | Look::Newer)
    }

    fn tooltip(self, locale: Locale) -> String {
        let key = match self {
            Look::NoLamp => "noLamp",
            Look::Off => "off",
            Look::On(slot) => BUILT_IN_COLORS[usize::from(slot) % BUILT_IN_COLORS.len()].0,
            Look::NeedsUpdate => "needsUpdate",
            Look::Newer => "newer",
        };
        locale.t(&format!("tray.tooltip.{key}"))
    }
}

/// The look on show, to word the tooltip again when the language changes.
#[derive(Default)]
pub struct TrayState(Mutex<Look>);

/// Signed distance in pixels, negative inside, from a point to the edge of a
/// square centred on the origin, of half side `half` and corner radius `r`.
fn square_distance(dx: f32, dy: f32, half: f32, r: f32) -> f32 {
    let qx = dx.abs() - (half - r);
    let qy = dy.abs() - (half - r);
    qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - r
}

/// Coverage of a pixel, from 0 to 1, by the square shrunk by `inset`,
/// antialiased over one pixel.
fn square(dx: f32, dy: f32, inset: f32) -> f32 {
    let distance = square_distance(dx, dy, HALF - inset, (CORNER - inset).max(0.0));
    (0.5 - distance).clamp(0.0, 1.0)
}

/// Renders the icon: a square in the lamp color with a darker rim, which
/// keeps warm white visible on a light menu bar, a grey outline when the lamp
/// is off, and a faint outline when no lamp is connected or the app cannot
/// drive it.
fn render(look: Look) -> Image<'static> {
    let mut rgba = vec![0u8; (SIZE * SIZE * 4) as usize];
    let center = (SIZE as f32 - 1.0) / 2.0;
    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let outer = square(dx, dy, 0.0);
            if outer == 0.0 {
                continue;
            }
            let (rgb, alpha) = match look {
                Look::On(slot) => {
                    let fill = BUILT_IN_COLORS[usize::from(slot) % BUILT_IN_COLORS.len()].1;
                    let inner = square(dx, dy, 1.5);
                    // Blend the fill into a rim at 60 % brightness.
                    let rgb = fill.map(|c| {
                        let rim = f32::from(c) * 0.6;
                        (rim + (f32::from(c) - rim) * inner).round() as u8
                    });
                    (rgb, outer)
                }
                Look::Off | Look::NoLamp | Look::NeedsUpdate | Look::Newer => {
                    let hole = square(dx, dy, 2.5);
                    let ring = outer * (1.0 - hole);
                    let strength = if look == Look::Off { 1.0 } else { 0.55 };
                    ([0x9a, 0xa0, 0xa6], ring * strength)
                }
            };
            let i = ((y * SIZE + x) * 4) as usize;
            rgba[i..i + 3].copy_from_slice(&rgb);
            rgba[i + 3] = (alpha * 255.0).round() as u8;
        }
    }
    Image::new_owned(rgba, SIZE, SIZE)
}

fn menu(app: &AppHandle, locale: Locale, look: Look) -> tauri::Result<Menu<Wry>> {
    let item = |id: &str, enabled: bool| {
        MenuItem::with_id(
            app,
            id,
            locale.t(&format!("tray.{id}")),
            enabled,
            None::<&str>,
        )
    };
    Menu::with_items(
        app,
        &[
            &item("open", true)?,
            &item("diagnostics", !look.locked())?,
            &PredefinedMenuItem::separator(app)?,
            &item("quit", true)?,
        ],
    )
}

/// Creates the icon and its menu. Linux has no click events, so the menu
/// gives access to the popup.
pub fn create(app: &AppHandle, locale: Locale) -> tauri::Result<()> {
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(render(Look::NoLamp))
        .tooltip(Look::NoLamp.tooltip(locale))
        .menu(&menu(app, locale, Look::NoLamp)?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => windows::show_popup(app),
            // Off the event loop, where creating a window deadlocks WebView2
            // on Windows, as for the `open_diagnostics` command.
            "diagnostics" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move { windows::open_diagnostics(&app) });
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                windows::toggle_popup(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

/// Shows the lamp state in the icon and its tooltip. An unchanged look is
/// skipped, since in automatic brightness the lamp reports its level up to
/// twice a second.
pub fn show(app: &AppHandle, look: Look) {
    let mut was_locked = false;
    if let Ok(mut shown) = app.state::<TrayState>().0.lock() {
        if *shown == look {
            return;
        }
        was_locked = shown.locked();
        *shown = look;
    }
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let locale = app.state::<LocaleState>().get();
        let _ = tray.set_icon(Some(render(look)));
        let _ = tray.set_tooltip(Some(look.tooltip(locale)));
        if look.locked() != was_locked
            && let Ok(menu) = menu(app, locale, look)
        {
            let _ = tray.set_menu(Some(menu));
        }
    }
}

/// Words the menu and the tooltip in another language.
pub fn relabel(app: &AppHandle, locale: Locale) {
    let look = app
        .state::<TrayState>()
        .0
        .lock()
        .map(|look| *look)
        .unwrap_or_default();
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        if let Ok(menu) = menu(app, locale, look) {
            let _ = tray.set_menu(Some(menu));
        }
        let _ = tray.set_tooltip(Some(look.tooltip(locale)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(image: &Image<'_>, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * SIZE + x) * 4) as usize;
        let rgba = image.rgba();
        [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
    }

    #[test]
    fn a_lit_lamp_is_a_full_square_in_its_color() {
        let image = render(Look::On(2));
        assert_eq!(pixel(&image, 16, 16), [0xe7, 0x4c, 0x3c, 255]);
        assert_eq!(pixel(&image, 0, 0)[3], 0, "the margin stays transparent");
        assert_eq!(pixel(&image, 5, 5)[3], 255, "a square, not a dot");
        let corner = pixel(&image, 4, 4)[3];
        assert!(corner > 0 && corner < 255, "the corners are rounded");
    }

    #[test]
    fn an_unlit_lamp_is_an_outline() {
        let image = render(Look::Off);
        assert_eq!(pixel(&image, 16, 16)[3], 0, "the middle is empty");
        assert!(pixel(&image, 16, 3)[3] > 200, "the outline is drawn");
        let faint = render(Look::NoLamp);
        assert!(pixel(&faint, 16, 3)[3] < pixel(&image, 16, 3)[3]);
    }

    #[test]
    fn every_look_has_a_tooltip_in_every_language() {
        let looks = [
            Look::NoLamp,
            Look::Off,
            Look::On(0),
            Look::On(1),
            Look::On(2),
            Look::On(3),
            Look::NeedsUpdate,
            Look::Newer,
        ];
        for locale in [Locale::En, Locale::Fr] {
            for look in looks {
                assert!(
                    look.tooltip(locale).starts_with("DISLED"),
                    "{look:?} in {locale:?}"
                );
            }
        }
        assert_eq!(Look::On(3).tooltip(Locale::En), "DISLED: warm white");
    }
}
