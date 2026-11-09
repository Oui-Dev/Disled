// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! The colors of the lamp as this computer keeps them: the names given to
//! the built-in colors, the custom colors and the touch cycle. The lamp keeps
//! none of them, so the app sends them again at every connection.

use std::sync::Mutex;

use disled_protocol::{Command, FIRST_CUSTOM_SLOT, LinearRgb, SLOT_COUNT, TOUCH_CYCLE_MAX};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The version of the `colors` key that this app writes.
pub const SCHEMA: u8 = 1;
/// The built-in colors fill the slots below this one.
pub const BUILT_IN: u8 = FIRST_CUSTOM_SLOT;
/// Most custom colors. They take slots `BUILT_IN` to `LAST_CUSTOM_SLOT`.
pub const CUSTOM_MAX: usize = 5;
pub const LAST_CUSTOM_SLOT: u8 = BUILT_IN + CUSTOM_MAX as u8 - 1;
/// Longest name, in characters.
pub const NAME_MAX: usize = 16;
/// The touch cycle of version 1: green, orange, red.
pub const DEFAULT_CYCLE: [u8; 3] = [0, 1, 2];

/// A color of the user's own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct CustomColor {
    /// `BUILT_IN` to `LAST_CUSTOM_SLOT`, the same number on the lamp.
    pub slot: u8,
    pub name: String,
    /// `#RRGGBB` in upper case, as the user typed it, never black.
    pub hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct Palette {
    pub schema: u8,
    /// The names of the built-in colors; `None` shows the default name in the
    /// language of the app.
    pub names: [Option<String>; BUILT_IN as usize],
    /// In the order the user made them.
    pub custom: Vec<CustomColor>,
    /// The slots a short press goes through, 1 to `TOUCH_CYCLE_MAX`.
    pub cycle: Vec<u8>,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            schema: SCHEMA,
            names: Default::default(),
            custom: Vec::new(),
            cycle: DEFAULT_CYCLE.to_vec(),
        }
    }
}

/// The palette in memory, shared by the commands and the USB worker.
#[derive(Default)]
pub struct ColorsState(Mutex<Palette>);

impl ColorsState {
    pub fn get(&self) -> Palette {
        self.0
            .lock()
            .map(|palette| palette.clone())
            .unwrap_or_default()
    }

    pub fn set(&self, palette: Palette) {
        if let Ok(mut shared) = self.0.lock() {
            *shared = palette;
        }
    }
}

/// A name trimmed of spaces, if it keeps 1 to `NAME_MAX` characters.
fn valid_name(name: &str) -> Option<String> {
    let name = name.trim();
    (1..=NAME_MAX)
        .contains(&name.chars().count())
        .then(|| name.to_owned())
}

/// A code of the form `#RRGGBB` in upper case, if it is one and not black.
fn valid_hex(hex: &str) -> Option<String> {
    let digits = hex.strip_prefix('#')?;
    if digits.len() != 6 || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let hex = format!("#{}", digits.to_ascii_uppercase());
    (hex != "#000000").then_some(hex)
}

impl Palette {
    /// Reads a saved palette part by part: a name, a custom color or a slot
    /// of the cycle that is not valid drops alone, past the fifth custom color
    /// the rest drops, and a cycle left empty is the default one.
    pub fn from_value(value: &Value) -> Self {
        let mut palette = Palette::default();
        if let Some(names) = value.get("names").and_then(Value::as_array) {
            for (slot, name) in names.iter().take(palette.names.len()).enumerate() {
                palette.names[slot] = name.as_str().and_then(valid_name);
            }
        }
        if let Some(custom) = value.get("custom").and_then(Value::as_array) {
            for color in custom {
                if palette.custom.len() == CUSTOM_MAX {
                    break;
                }
                let slot = color
                    .get("slot")
                    .and_then(Value::as_u64)
                    .and_then(|slot| u8::try_from(slot).ok())
                    .filter(|slot| (BUILT_IN..=LAST_CUSTOM_SLOT).contains(slot))
                    .filter(|slot| !palette.defines(*slot));
                let name = color
                    .get("name")
                    .and_then(Value::as_str)
                    .and_then(valid_name);
                let hex = color.get("hex").and_then(Value::as_str).and_then(valid_hex);
                if let (Some(slot), Some(name), Some(hex)) = (slot, name, hex) {
                    palette.custom.push(CustomColor { slot, name, hex });
                }
            }
        }
        if let Some(cycle) = value.get("cycle").and_then(Value::as_array) {
            let mut kept = Vec::new();
            for slot in cycle.iter().filter_map(Value::as_u64) {
                let Ok(slot) = u8::try_from(slot) else {
                    continue;
                };
                if kept.len() < TOUCH_CYCLE_MAX && palette.defines(slot) && !kept.contains(&slot) {
                    kept.push(slot);
                }
            }
            if !kept.is_empty() {
                palette.cycle = kept;
            }
        }
        palette
    }

    /// Tells whether a slot holds a color, built in or custom.
    pub fn defines(&self, slot: u8) -> bool {
        slot < BUILT_IN || self.custom.iter().any(|color| color.slot == slot)
    }

    /// Commands that bring the palette of a lamp in line with this one: the
    /// custom colors, the touch cycle once the colors it names exist, then
    /// black in every other custom slot, which clears what an earlier session
    /// left there.
    pub fn commands(&self) -> Vec<Command> {
        let mut commands: Vec<Command> = self
            .custom
            .iter()
            .filter_map(|color| {
                Some(Command::SetColor {
                    slot: color.slot,
                    rgb: led_of(&color.hex)?,
                })
            })
            .collect();
        commands.push(Command::SetTouchCycle {
            slots: self.cycle.clone(),
        });
        commands.extend(
            (BUILT_IN..SLOT_COUNT)
                .filter(|slot| !self.defines(*slot))
                .map(|slot| Command::SetColor {
                    slot,
                    rgb: LinearRgb::default(),
                }),
        );
        commands
    }
}

/// The light of the lamp for a color code: sRGB to linear light (IEC
/// 61966-2-1), scaled so that the largest channel is 65535, as the lamp
/// requires. The brightness stays apart, so a dark code and its bright
/// version give the same light. `None` for a code that is not `#RRGGBB`, or
/// black, which has no light to scale.
pub fn led_of(hex: &str) -> Option<LinearRgb> {
    let hex = valid_hex(hex)?;
    let channel = |at: usize| -> f64 {
        let value = f64::from(u8::from_str_radix(&hex[at..at + 2], 16).unwrap_or(0)) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    let (r, g, b) = (channel(1), channel(3), channel(5));
    let max = r.max(g).max(b);
    let scale = |value: f64| (value / max * 65535.0).round() as u16;
    Some(LinearRgb {
        r: scale(r),
        g: scale(g),
        b: scale(b),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_default_palette_is_the_one_of_version_1() {
        let palette = Palette::default();
        assert_eq!(palette.cycle, [0, 1, 2]);
        assert!(palette.names.iter().all(Option::is_none));
        assert!(palette.custom.is_empty());
    }

    #[test]
    fn led_of_scales_the_linear_light_to_full_scale() {
        // The table of annex C, computed on the code as typed.
        let table = [
            ("#FF0000", [65535, 0, 0]),
            ("#FFFFFF", [65535, 65535, 65535]),
            ("#808080", [65535, 65535, 65535]),
            ("#3B82F6", [3110, 15874, 65535]),
            ("#9B59F6", [23309, 7104, 65535]),
            ("#FB6FB7", [65535, 10799, 32169]),
            ("#EEF5FF", [56032, 59840, 65535]),
            ("#804020", [65535, 15565, 4385]),
        ];
        for (hex, [r, g, b]) in table {
            assert_eq!(led_of(hex), Some(LinearRgb { r, g, b }), "{hex}");
        }
        assert_eq!(led_of("#000000"), None);
        assert_eq!(led_of("#12345"), None);
        assert_eq!(led_of("#GGGGGG"), None);
    }

    #[test]
    fn a_saved_palette_keeps_every_valid_part() {
        let palette = Palette::from_value(&json!({
            "schema": 1,
            "names": [null, "  Libre  ", "", "Une phrase bien trop longue"],
            "custom": [
                { "slot": 4, "name": "Musique", "hex": "#9b59f6" },
                { "slot": 4, "name": "Doublon", "hex": "#FF0000" },
                { "slot": 9, "name": "Trop loin", "hex": "#FF0000" },
                { "slot": 5, "name": "Noir", "hex": "#000000" },
                { "slot": 6, "name": "Focus", "hex": "#3B82F6" }
            ],
            "cycle": [0, 4, 4, 7, 2, 6]
        }));
        assert_eq!(palette.names, [None, Some("Libre".into()), None, None]);
        let slots: Vec<u8> = palette.custom.iter().map(|c| c.slot).collect();
        assert_eq!(slots, [4, 6]);
        assert_eq!(palette.custom[0].hex, "#9B59F6");
        assert_eq!(palette.cycle, [0, 4, 2, 6]);
    }

    #[test]
    fn an_empty_cycle_is_the_default_one() {
        let palette = Palette::from_value(&json!({ "cycle": [5, 12] }));
        assert_eq!(palette.cycle, DEFAULT_CYCLE);
        assert_eq!(Palette::from_value(&json!(null)), Palette::default());
    }

    #[test]
    fn the_palette_goes_to_the_lamp_colors_first_then_the_cycle_then_black() {
        let palette = Palette::from_value(&json!({
            "custom": [{ "slot": 5, "name": "Musique", "hex": "#9B59F6" }],
            "cycle": [0, 5, 1]
        }));
        let commands = palette.commands();
        assert_eq!(commands.len(), 1 + 1 + 11);
        assert_eq!(
            commands[0],
            Command::SetColor {
                slot: 5,
                rgb: LinearRgb {
                    r: 23309,
                    g: 7104,
                    b: 65535
                }
            }
        );
        assert_eq!(
            commands[1],
            Command::SetTouchCycle {
                slots: vec![0, 5, 1]
            }
        );
        let cleared: Vec<u8> = commands[2..]
            .iter()
            .map(|command| match command {
                Command::SetColor { slot, rgb } if *rgb == LinearRgb::default() => *slot,
                other => panic!("unexpected {other:?}"),
            })
            .collect();
        assert_eq!(cleared, [4, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
    }
}
