// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! The lamp as the plugins with the `lamp` capability read it: whether the
//! app drives a lamp, lit or not, its color as the user sees it, its
//! brightness, what made its latest change and the integration it follows.
//! Their programs get it on their input, one JSON-RPC 2.0 notification per
//! line: `lamp` at their start and at every change, and `touch` at every
//! short press on the lid, even one that changes nothing.

use disled_protocol::{BrightnessMode, ChangeSource, State};
use serde::Serialize;

use crate::colors::{PREVIEW_SLOT, Palette, hex_of};
use crate::i18n::Locale;
use crate::tray;

/// A short press on the lid.
pub const TOUCH_LINE: &str = r#"{"jsonrpc":"2.0","method":"touch","params":{"press":"short"}}"#;

/// The color of the lamp, as the windows show it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Color {
    /// The palette slot: 0 to 3 for the built-in colors, then the custom ones.
    pub slot: u8,
    /// The name the user sees, none for a slot without one.
    pub name: Option<String>,
    /// `#RRGGBB`, the swatch of a built-in color or the code of a custom one.
    pub hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Brightness {
    /// `auto` or `manual`.
    pub mode: BrightnessMode,
    /// The level in use, percent, which follows the light in auto mode.
    pub level: u8,
}

/// What made the latest change of the lamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// A press on the lid.
    Touch,
    /// The app: the user in a window, a plugin, a call.
    App,
    /// The automatic brightness, after the light of the room.
    Light,
    /// The lamp itself: its start, the USB link, the app gone silent.
    Lamp,
}

impl From<ChangeSource> for Source {
    fn from(source: ChangeSource) -> Self {
        match source {
            ChangeSource::Touch => Source::Touch,
            ChangeSource::Host => Source::App,
            ChangeSource::AutoBrightness => Source::Light,
            ChangeSource::Boot | ChangeSource::Usb | ChangeSource::HostSilence => Source::Lamp,
        }
    }
}

/// The `params` of a `lamp` notification. Without a lamp that the app
/// drives, only `connected` and `follows`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Lamp {
    pub connected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brightness: Option<Brightness>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    /// The id of the integration the lamp follows, none in manual mode.
    pub follows: Option<String>,
}

/// The lamp of `state`, none while the app drives no lamp. While the editor
/// previews a color, the plugins read the state that the preview took the
/// lamp from, `kept`, which the lamp gets back, as the tray icon does.
pub fn view(
    state: Option<&State>,
    kept: Option<(bool, u8)>,
    palette: &Palette,
    locale: Locale,
    follows: Option<String>,
) -> Lamp {
    let Some(state) = state else {
        return Lamp {
            connected: false,
            on: None,
            color: None,
            brightness: None,
            source: None,
            follows,
        };
    };
    let (on, slot) = match kept {
        Some(kept) if state.color == PREVIEW_SLOT => kept,
        _ => (state.on, state.color),
    };
    let (rgb, name) = tray::color_of(slot, state.rgb, palette, locale);
    Lamp {
        connected: true,
        on: Some(on),
        color: Some(Color {
            slot,
            name,
            hex: hex_of(rgb),
        }),
        brightness: Some(Brightness {
            mode: state.brightness_mode,
            level: state.applied_level,
        }),
        source: Some(state.source.into()),
        follows,
    }
}

#[derive(Serialize)]
struct Notification<'a> {
    jsonrpc: &'static str,
    method: &'static str,
    params: &'a Lamp,
}

/// The `lamp` notification of `lamp`, on one line.
pub fn line(lamp: &Lamp) -> String {
    serde_json::to_string(&Notification {
        jsonrpc: "2.0",
        method: "lamp",
        params: lamp,
    })
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use disled_protocol::{LinearRgb, SLOT_GREEN, SLOT_RED, TouchMode, UsbLink};
    use serde_json::{Value, json};

    fn state(on: bool, color: u8, source: ChangeSource) -> State {
        State {
            state_seq: 7,
            on,
            color,
            brightness_mode: BrightnessMode::Auto,
            manual_level: 50,
            applied_level: 40,
            link: UsbLink::Configured,
            source,
            touch_mode: TouchMode::Normal,
            rgb: LinearRgb {
                r: 65535,
                g: 0,
                b: 0,
            },
        }
    }

    fn palette() -> Palette {
        Palette::from_value(&json!({
            "schema": 1,
            "names": ["Libre", null, null, null],
            "custom": [{ "slot": 4, "name": "Musique", "hex": "#A15DFF" }]
        }))
    }

    fn params(lamp: &Lamp) -> Value {
        let line: Value = serde_json::from_str(&line(lamp)).expect("JSON");
        assert_eq!(line["jsonrpc"], "2.0");
        assert_eq!(line["method"], "lamp");
        line["params"].clone()
    }

    #[test]
    fn without_a_lamp_only_the_mode_is_known() {
        let lamp = view(None, None, &palette(), Locale::En, None);
        assert_eq!(
            params(&lamp),
            json!({ "connected": false, "follows": null })
        );
        let lamp = view(None, None, &palette(), Locale::En, Some("pomodoro".into()));
        assert_eq!(params(&lamp)["follows"], "pomodoro");
    }

    #[test]
    fn the_color_has_the_name_and_the_code_that_the_user_sees() {
        let lamp = state(true, SLOT_GREEN, ChangeSource::Touch);
        assert_eq!(
            params(&view(Some(&lamp), None, &palette(), Locale::Fr, None)),
            json!({
                "connected": true,
                "on": true,
                "color": { "slot": 0, "name": "Libre", "hex": "#00FFA2" },
                "brightness": { "mode": "auto", "level": 40 },
                "source": "touch",
                "follows": null
            })
        );
        // The default name of a built-in color follows the language.
        let red = state(true, SLOT_RED, ChangeSource::Host);
        let color = view(Some(&red), None, &palette(), Locale::Fr, None).color;
        assert_eq!(
            color.and_then(|color| color.name).as_deref(),
            Some("Ne pas déranger")
        );
        let custom = state(false, 4, ChangeSource::Host);
        let lamp = view(Some(&custom), None, &palette(), Locale::En, None);
        assert_eq!(lamp.on, Some(false));
        assert_eq!(
            lamp.color,
            Some(Color {
                slot: 4,
                name: Some("Musique".into()),
                hex: "#A15DFF".into()
            })
        );
        // A slot the palette does not know: the light of the lamp, no name.
        let unknown = state(true, 9, ChangeSource::Host);
        let color = view(Some(&unknown), None, &palette(), Locale::En, None).color;
        assert_eq!(
            color,
            Some(Color {
                slot: 9,
                name: None,
                hex: "#FF0000".into()
            })
        );
    }

    #[test]
    fn a_preview_shows_the_state_the_lamp_gets_back() {
        let preview = state(true, PREVIEW_SLOT, ChangeSource::Host);
        let lamp = view(
            Some(&preview),
            Some((false, 4)),
            &palette(),
            Locale::En,
            None,
        );
        assert_eq!(lamp.on, Some(false));
        assert_eq!(lamp.color.map(|color| color.slot), Some(4));
    }

    #[test]
    fn every_source_has_its_word() {
        let words: Vec<Value> = [
            ChangeSource::Boot,
            ChangeSource::Touch,
            ChangeSource::Host,
            ChangeSource::AutoBrightness,
            ChangeSource::Usb,
            ChangeSource::HostSilence,
        ]
        .into_iter()
        .map(|source| json!(Source::from(source)))
        .collect();
        assert_eq!(
            words,
            ["lamp", "touch", "app", "light", "lamp", "lamp"].map(Value::from)
        );
    }

    #[test]
    fn a_press_is_a_notification_without_an_answer() {
        let press: Value = serde_json::from_str(TOUCH_LINE).expect("JSON");
        assert_eq!(press["jsonrpc"], "2.0");
        assert_eq!(press["method"], "touch");
        assert_eq!(press["params"]["press"], "short");
        assert!(press.get("id").is_none());
    }
}
