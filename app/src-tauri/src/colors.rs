// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! The colors of the lamp as this computer keeps them: the names given to
//! the built-in colors, the custom colors and the touch cycle. The lamp keeps
//! none of them, so the app sends them again at every connection.

use std::sync::Mutex;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use disled_protocol::{
    Command, FIRST_CUSTOM_SLOT, LinearRgb, SLOT_COUNT, SLOT_RED, SetState, State, TOUCH_CYCLE_MAX,
};
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

/// Reads a color code as the user types it: 3 or 6 hexadecimal digits, with
/// or without "#". It gives the same color as `#RRGGBB` in upper case, and
/// refuses black, which has no light.
pub fn parse_hex(input: &str) -> Option<String> {
    let input = input.trim();
    let digits = input.strip_prefix('#').unwrap_or(input);
    let digits: String = match digits.chars().count() {
        3 => digits.chars().flat_map(|c| [c, c]).collect(),
        6 => digits.to_owned(),
        _ => return None,
    };
    valid_hex(&format!("#{digits}"))
}

/// What else uses the colors when one goes: the lamp, the calls and the
/// active integration.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Around {
    /// The slot the lamp keeps, lit or not, `None` without a version 2 lamp.
    pub lamp: Option<u8>,
    /// The call color of the settings.
    pub call_color: u8,
    /// While the app imposes the call color on the lamp: the color it gives
    /// back after the call.
    pub call_then: Option<u8>,
    /// While a plugin drives the lamp: the color it proposes for its state,
    /// which takes the place of a deleted color on the lamp.
    pub integration: Option<u8>,
}

/// A new palette, what it changes around, and the commands that bring the
/// lamp in line, in their order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteChange {
    pub palette: Palette,
    pub lamp: Option<u8>,
    pub call_color: u8,
    pub call_then: Option<u8>,
    pub commands: Vec<Command>,
}

impl Palette {
    /// The slot a new color takes: the lowest free one.
    pub fn free_slot(&self) -> Option<u8> {
        (BUILT_IN..=LAST_CUSTOM_SLOT).find(|slot| !self.defines(*slot))
    }

    /// Adds a custom color, for no `slot`, or changes one. Returns its slot.
    pub fn save_custom(&mut self, slot: Option<u8>, name: &str, hex: &str) -> Result<u8, String> {
        let name = valid_name(name).ok_or("a name has 1 to 16 characters")?;
        let hex = parse_hex(hex).ok_or("not a color code")?;
        match slot {
            None => {
                let slot = self.free_slot().ok_or("five custom colors at most")?;
                self.custom.push(CustomColor { slot, name, hex });
                Ok(slot)
            }
            Some(slot) => {
                let color = self
                    .custom
                    .iter_mut()
                    .find(|color| color.slot == slot)
                    .ok_or("no such custom color")?;
                color.name = name;
                color.hex = hex;
                Ok(slot)
            }
        }
    }

    /// Names a built-in color. A name equal to its default name in the
    /// language of the app is kept as none, so that it follows the language.
    pub fn rename_original(
        &mut self,
        slot: u8,
        name: Option<&str>,
        default_name: &str,
    ) -> Result<(), String> {
        let entry = self
            .names
            .get_mut(usize::from(slot))
            .ok_or("not a built-in color")?;
        *entry = match name {
            None => None,
            Some(name) => {
                let name = valid_name(name).ok_or("a name has 1 to 16 characters")?;
                (name != default_name).then_some(name)
            }
        };
        Ok(())
    }

    /// Sets the touch cycle: 1 to 9 slots that hold a color, none twice.
    pub fn set_cycle(&mut self, slots: &[u8]) -> Result<(), String> {
        let valid = (1..=TOUCH_CYCLE_MAX).contains(&slots.len())
            && slots.iter().all(|slot| self.defines(*slot))
            && slots
                .iter()
                .enumerate()
                .all(|(at, slot)| !slots[..at].contains(slot));
        if !valid {
            return Err("not a touch cycle".into());
        }
        self.cycle = slots.to_vec();
        Ok(())
    }

    /// Deletes a custom color, even one in use (annex C, C.6.6). The cycle
    /// closes on it first, so that no touch reaches it any more. A lamp that
    /// keeps it then takes its replacement in one step: the call color while
    /// a call imposes it, the color of the plugin state while a plugin drives
    /// the lamp, otherwise the first color of the cycle. Last, the slot is
    /// emptied, which leaves the fallback of the firmware nothing to do.
    pub fn delete(&self, slot: u8, around: &Around) -> Option<PaletteChange> {
        let mut palette = self.clone();
        let index = palette.custom.iter().position(|color| color.slot == slot)?;
        palette.custom.remove(index);
        palette.cycle.retain(|kept| *kept != slot);
        if palette.cycle.is_empty() {
            palette.cycle = DEFAULT_CYCLE.to_vec();
        }
        let first = palette.cycle[0];
        let call_color = if around.call_color == slot {
            SLOT_RED
        } else {
            around.call_color
        };
        let call_then = around
            .call_then
            .map(|then| if then == slot { first } else { then });
        let mut commands = vec![Command::SetTouchCycle {
            slots: palette.cycle.clone(),
        }];
        let mut lamp = around.lamp;
        if around.lamp == Some(slot) {
            let replacement = if around.call_then.is_some() {
                call_color
            } else {
                around.integration.unwrap_or(first)
            };
            commands.push(Command::SetState(SetState {
                color: Some(replacement),
                ..SetState::default()
            }));
            lamp = Some(replacement);
        }
        commands.push(Command::SetColor {
            slot,
            rgb: LinearRgb::default(),
        });
        Some(PaletteChange {
            palette,
            lamp,
            call_color,
            call_then,
            commands,
        })
    }

    /// Takes a shared palette in place of this one: every custom color goes
    /// first, with the fallbacks of a deletion, then the colors of the code
    /// take the slots from `BUILT_IN` on, in their order, and the names and
    /// the cycle follow.
    pub fn import(&self, shared: &Palette, around: &Around) -> PaletteChange {
        let mut change = PaletteChange {
            palette: self.clone(),
            lamp: around.lamp,
            call_color: around.call_color,
            call_then: around.call_then,
            commands: Vec::new(),
        };
        for slot in self.custom.iter().map(|color| color.slot) {
            let around = Around {
                lamp: change.lamp,
                call_color: change.call_color,
                call_then: change.call_then,
                integration: around.integration,
            };
            if let Some(deletion) = change.palette.delete(slot, &around) {
                change.commands.extend(deletion.commands);
                change.palette = deletion.palette;
                change.lamp = deletion.lamp;
                change.call_color = deletion.call_color;
                change.call_then = deletion.call_then;
            }
        }
        change
            .commands
            .extend(shared.custom.iter().filter_map(|color| {
                Some(Command::SetColor {
                    slot: color.slot,
                    rgb: led_of(&color.hex)?,
                })
            }));
        change.commands.push(Command::SetTouchCycle {
            slots: shared.cycle.clone(),
        });
        change.palette = shared.clone();
        change
    }

    /// The share code of the palette: `SHARE_PREFIX`, then its names, colors
    /// and cycle in JSON, in base64url without padding. In the code, the
    /// custom colors go by their rank, never by their slot.
    pub fn share_code(&self) -> String {
        let rank = |slot: u8| -> u8 {
            self.custom
                .iter()
                .position(|color| color.slot == slot)
                .map_or(slot, |at| BUILT_IN + at as u8)
        };
        let shared = Shared {
            v: SHARE_VERSION,
            n: self.names.clone(),
            c: self
                .custom
                .iter()
                .map(|color| {
                    (
                        color.name.clone(),
                        color.hex.trim_start_matches('#').to_owned(),
                    )
                })
                .collect(),
            t: self.cycle.iter().map(|slot| rank(*slot)).collect(),
        };
        let json = serde_json::to_vec(&shared).unwrap_or_default();
        format!("{SHARE_PREFIX}{}", URL_SAFE_NO_PAD.encode(json))
    }
}

/// The slot of the preview of the editor: the last one, which no color takes.
pub const PREVIEW_SLOT: u8 = SLOT_COUNT - 1;

/// What the lamp showed before a preview: lit or not, and its slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Kept {
    on: bool,
    slot: u8,
}

/// The preview of the editor on the lamp, and the state it took the lamp
/// from while it runs.
#[derive(Default)]
pub struct PreviewState(Mutex<Option<Kept>>);

impl PreviewState {
    /// Shows a color in the preview slot. The first one also lights the lamp
    /// on that slot, and keeps the state that `end` gives back.
    pub fn follow(&self, lamp: &State, rgb: LinearRgb) -> Vec<Command> {
        let Ok(mut kept) = self.0.lock() else {
            return Vec::new();
        };
        let mut commands = vec![Command::SetColor {
            slot: PREVIEW_SLOT,
            rgb,
        }];
        if kept.is_none() {
            *kept = Some(Kept {
                on: lamp.on,
                slot: lamp.color,
            });
            commands.push(Command::SetState(SetState {
                on: Some(true),
                color: Some(PREVIEW_SLOT),
                ..SetState::default()
            }));
        }
        commands
    }

    /// Ends the preview, if one runs: the lamp goes back to the state it
    /// had, or stays lit on `stay`, a color just saved from the popover. Then
    /// the preview slot empties.
    pub fn end(&self, stay: Option<u8>) -> Vec<Command> {
        let Some(kept) = self.0.lock().ok().and_then(|mut kept| kept.take()) else {
            return Vec::new();
        };
        let (on, slot) = stay.map_or((kept.on, kept.slot), |slot| (true, slot));
        vec![
            Command::SetState(SetState {
                on: Some(on),
                color: Some(slot),
                ..SetState::default()
            }),
            Command::SetColor {
                slot: PREVIEW_SLOT,
                rgb: LinearRgb::default(),
            },
        ]
    }

    /// Drops the preview without a command, once the user touched the lamp,
    /// which then left the preview slot, or the lamp went away.
    pub fn forget(&self) {
        if let Ok(mut kept) = self.0.lock() {
            *kept = None;
        }
    }
}

/// The prefix of a share code, which keeps its capitals (decided on
/// 2026-11-09).
pub const SHARE_PREFIX: &str = "DISLED2-";
/// The version of the content of a share code. A later one raises it only
/// if an older app would misread the code.
const SHARE_VERSION: u8 = 1;
/// A code that grows beyond this many characters while it is read is refused.
const SHARE_MAX: usize = 4096;

/// The content of a share code, with short keys: the names of the built-in
/// colors, the custom colors as name and code without "#", and the cycle,
/// where the custom colors go by `BUILT_IN` + rank.
#[derive(Debug, Serialize, Deserialize)]
struct Shared {
    v: u8,
    n: [Option<String>; BUILT_IN as usize],
    c: Vec<(String, String)>,
    t: Vec<u8>,
}

/// Reads a share code in a pasted text, which may hold other words and the
/// line breaks of a messaging app (annex C, C.7.3). From the first prefix,
/// the code runs to the first blank. As long as it does not decode, the piece
/// after a line break joins it, up to the next blank. The first code that
/// decodes wins; past `SHARE_MAX` characters, or with nothing left to join,
/// the text has no code.
pub fn read_share_code(text: &str) -> Option<Palette> {
    let mut rest = &text[text.find(SHARE_PREFIX)?..];
    let mut code = String::new();
    loop {
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        code.push_str(&rest[..end]);
        if code.len() > SHARE_MAX {
            return None;
        }
        if let Some(palette) = decode_share(&code) {
            return Some(palette);
        }
        let after = &rest[end..];
        let blank = after
            .find(|c: char| !c.is_whitespace())
            .unwrap_or(after.len());
        if blank == after.len() || !after[..blank].contains('\n') {
            return None;
        }
        rest = &after[blank..];
    }
}

/// The palette of a share code, if every part of it holds.
fn decode_share(code: &str) -> Option<Palette> {
    let bytes = URL_SAFE_NO_PAD
        .decode(code.strip_prefix(SHARE_PREFIX)?)
        .ok()?;
    let shared: Shared = serde_json::from_slice(&bytes).ok()?;
    if shared.v != SHARE_VERSION || shared.c.len() > CUSTOM_MAX {
        return None;
    }
    let mut palette = Palette::default();
    for (entry, name) in palette.names.iter_mut().zip(&shared.n) {
        *entry = match name {
            None => None,
            Some(name) => Some(valid_name(name)?),
        };
    }
    for (rank, (name, hex)) in shared.c.iter().enumerate() {
        if hex.len() != 6 {
            return None;
        }
        palette.custom.push(CustomColor {
            slot: BUILT_IN + rank as u8,
            name: valid_name(name)?,
            hex: valid_hex(&format!("#{hex}"))?,
        });
    }
    palette.set_cycle(&shared.t).ok()?;
    Some(palette)
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

    /// Two custom colors, Musique in slot 4 and Focus in slot 5, the second
    /// in the cycle.
    fn two_colors() -> Palette {
        Palette::from_value(&json!({
            "names": [null, "Libre", null, null],
            "custom": [
                { "slot": 4, "name": "Musique", "hex": "#9B59F6" },
                { "slot": 5, "name": "Focus", "hex": "#3B82F6" }
            ],
            "cycle": [0, 5, 1, 2]
        }))
    }

    fn black(slot: u8) -> Command {
        Command::SetColor {
            slot,
            rgb: LinearRgb::default(),
        }
    }

    fn cycle(slots: &[u8]) -> Command {
        Command::SetTouchCycle {
            slots: slots.to_vec(),
        }
    }

    fn lamp_to(slot: u8) -> Command {
        Command::SetState(SetState {
            color: Some(slot),
            ..SetState::default()
        })
    }

    #[test]
    fn a_typed_code_is_read_as_it_is_typed() {
        assert_eq!(parse_hex("#3B82F6").as_deref(), Some("#3B82F6"));
        assert_eq!(parse_hex("3b82f6").as_deref(), Some("#3B82F6"));
        assert_eq!(parse_hex("F0A").as_deref(), Some("#FF00AA"));
        assert_eq!(parse_hex("#f0a").as_deref(), Some("#FF00AA"));
        // A grey stays grey, though the lamp shows it at full light.
        assert_eq!(parse_hex("#808080").as_deref(), Some("#808080"));
        for refused in ["#12345", "#GGGGGG", "#000000", "000", "", "#1234567"] {
            assert_eq!(parse_hex(refused), None, "{refused}");
        }
    }

    #[test]
    fn a_new_color_takes_the_lowest_free_slot() {
        let mut palette = two_colors();
        assert_eq!(palette.save_custom(None, "Lecture", "#F6D32D"), Ok(6));
        let deleted = palette.delete(4, &Around::default()).expect("deleted");
        let mut palette = deleted.palette;
        assert_eq!(palette.save_custom(None, "Sport", "#22D3EE"), Ok(4));
        let order: Vec<&str> = palette.custom.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            order,
            ["Focus", "Lecture", "Sport"],
            "the order they were made"
        );
    }

    #[test]
    fn five_custom_colors_at_most_with_valid_names() {
        let mut palette = Palette::default();
        for name in ["Un", "Deux", "Trois", "Quatre", "Cinq"] {
            assert!(palette.save_custom(None, name, "#FF0000").is_ok());
        }
        assert!(palette.save_custom(None, "Six", "#FF0000").is_err());
        let mut palette = Palette::default();
        assert!(palette.save_custom(None, "   ", "#FF0000").is_err());
        assert!(
            palette
                .save_custom(None, "Dix-sept lettres!", "#FF0000")
                .is_err()
        );
        assert_eq!(
            palette.save_custom(None, "  Seize lettres!  ", "#FF0000"),
            Ok(4)
        );
        assert_eq!(palette.custom[0].name, "Seize lettres!");
        assert!(palette.save_custom(Some(5), "Absente", "#FF0000").is_err());
    }

    #[test]
    fn an_original_name_equal_to_its_default_follows_the_language() {
        let mut palette = Palette::default();
        palette
            .rename_original(0, Some("Libre"), "Disponible")
            .expect("renamed");
        assert_eq!(palette.names[0].as_deref(), Some("Libre"));
        palette
            .rename_original(0, Some("Disponible"), "Disponible")
            .expect("renamed");
        assert_eq!(palette.names[0], None);
        assert!(palette.rename_original(4, Some("Libre"), "").is_err());
        assert!(palette.rename_original(1, Some(""), "Occupé").is_err());
    }

    #[test]
    fn a_cycle_holds_one_to_nine_defined_colors_once() {
        let mut palette = two_colors();
        assert!(palette.set_cycle(&[3, 4]).is_ok());
        assert!(palette.set_cycle(&[]).is_err());
        assert!(palette.set_cycle(&[0, 6]).is_err(), "slot 6 holds no color");
        assert!(palette.set_cycle(&[0, 1, 0]).is_err());
        assert_eq!(palette.cycle, [3, 4]);
    }

    #[test]
    fn a_deletion_closes_the_cycle_before_it_empties_the_slot() {
        let palette = two_colors();
        let around = Around {
            call_color: SLOT_RED,
            ..Around::default()
        };

        // In the middle of the cycle, the lamp elsewhere.
        let change = palette.delete(5, &around).expect("deleted");
        assert_eq!(change.palette.cycle, [0, 1, 2]);
        assert_eq!(change.commands, [cycle(&[0, 1, 2]), black(5)]);
        assert_eq!(change.palette.custom.len(), 1);

        // The only color of the cycle: the default cycle comes back.
        let mut alone = two_colors();
        alone.set_cycle(&[4]).expect("cycle");
        let change = alone.delete(4, &around).expect("deleted");
        assert_eq!(change.palette.cycle, DEFAULT_CYCLE);
        assert_eq!(change.commands, [cycle(&[0, 1, 2]), black(4)]);
    }

    #[test]
    fn a_lamp_on_the_deleted_color_takes_the_first_of_the_cycle() {
        // Lit or not: the lamp keeps a slot either way, and the replacement
        // comes in one step, before the slot empties.
        let around = Around {
            lamp: Some(5),
            call_color: SLOT_RED,
            ..Around::default()
        };
        let change = two_colors().delete(5, &around).expect("deleted");
        assert_eq!(change.commands, [cycle(&[0, 1, 2]), lamp_to(0), black(5)]);
        assert_eq!(change.lamp, Some(0));
    }

    #[test]
    fn the_calls_lose_the_deleted_color_too() {
        // The call color goes back to "Do not disturb".
        let around = Around {
            call_color: 5,
            ..Around::default()
        };
        let change = two_colors().delete(5, &around).expect("deleted");
        assert_eq!(change.call_color, SLOT_RED);

        // During an imposed call on it, the lamp takes "Do not disturb".
        let around = Around {
            lamp: Some(5),
            call_color: 5,
            call_then: Some(1),
            ..Around::default()
        };
        let change = two_colors().delete(5, &around).expect("deleted");
        assert_eq!(change.commands[1], lamp_to(SLOT_RED));
        assert_eq!(change.call_then, Some(1));

        // "then S" becomes the first color of the new cycle.
        let around = Around {
            lamp: Some(SLOT_RED),
            call_color: SLOT_RED,
            call_then: Some(4),
            ..Around::default()
        };
        let change = two_colors().delete(4, &around).expect("deleted");
        assert_eq!(change.call_then, Some(0));
        assert_eq!(change.commands, [cycle(&[0, 5, 1, 2]), black(4)]);
    }

    #[test]
    fn a_plugin_showing_the_deleted_color_gives_its_own_color() {
        let around = Around {
            lamp: Some(4),
            call_color: SLOT_RED,
            integration: Some(3),
            ..Around::default()
        };
        let change = two_colors().delete(4, &around).expect("deleted");
        assert_eq!(
            change.commands,
            [cycle(&[0, 5, 1, 2]), lamp_to(3), black(4)]
        );
    }

    #[test]
    fn only_a_custom_color_can_be_deleted() {
        assert!(two_colors().delete(2, &Around::default()).is_none());
        assert!(two_colors().delete(6, &Around::default()).is_none());
    }

    #[test]
    fn a_share_code_holds_names_colors_and_cycle() {
        let mut palette = two_colors();
        // Slots with a gap: the code goes by rank.
        palette = palette
            .delete(4, &Around::default())
            .expect("deleted")
            .palette;
        palette
            .save_custom(None, "Lecture", "#F6D32D")
            .expect("saved");
        palette.set_cycle(&[0, 5, 4]).expect("cycle");
        let code = palette.share_code();
        assert!(code.starts_with("DISLED2-"));
        let read = read_share_code(&code).expect("read");
        assert_eq!(read.names, palette.names);
        let colors: Vec<(&str, &str, u8)> = read
            .custom
            .iter()
            .map(|c| (c.name.as_str(), c.hex.as_str(), c.slot))
            .collect();
        assert_eq!(colors, [("Focus", "#3B82F6", 4), ("Lecture", "#F6D32D", 5)]);
        assert_eq!(read.cycle, [0, 4, 5]);
    }

    /// A code of this content, as another app would write it.
    fn code_of(content: Value) -> String {
        format!(
            "{SHARE_PREFIX}{}",
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&content).expect("JSON"))
        )
    }

    #[test]
    fn a_code_reads_through_the_words_and_breaks_around_it() {
        let code = two_colors().share_code();
        let read = |text: &str| read_share_code(text).map(|p| p.cycle);
        let expected = Some(vec![0, 5, 1, 2]);
        assert_eq!(read(&format!("Voici : {code} à plus")), expected);
        assert_eq!(read(&format!("{code}\nMerci")), expected);
        let (head, tail) = code.split_at(30);
        let (middle, end) = tail.split_at(25);
        assert_eq!(
            read(&format!("Voici\n{head}\n{middle}\r\n{end}\nMerci")),
            expected
        );
    }

    #[test]
    fn every_unreadable_code_is_refused_alike() {
        let good = json!({
            "v": 1,
            "n": [null, "Libre", null, null],
            "c": [["Musique", "9B59F6"]],
            "t": [0, 4]
        });
        assert!(read_share_code(&code_of(good.clone())).is_some());
        let mut unreadable = vec![
            "pas de code ici".to_owned(),
            "DISLED2-%%%%".to_owned(),
            format!("{SHARE_PREFIX}{}", URL_SAFE_NO_PAD.encode(b"{not json")),
        ];
        let changes: [(&str, Value); 7] = [
            ("v", json!(2)),
            ("c", json!(vec![["Couleur", "FF0000"]; 6])),
            ("n", json!([null, "Dix-sept lettres!", null, null])),
            ("t", json!([0, 5])),
            ("t", json!([])),
            ("c", json!([["Noir", "000000"]])),
            ("c", json!([["Court", "FFF"]])),
        ];
        for (key, value) in changes {
            let mut content = good.clone();
            content[key] = value;
            unreadable.push(code_of(content));
        }
        // A code joined with what follows it on the same line.
        unreadable.push(format!("{}merci", code_of(good.clone())));
        // Past 4,096 characters, even when what follows would fix it.
        let long = code_of(
            json!({ "v": 1, "n": [null, null, null, null], "c": [], "t": [0], "x": "a".repeat(4200) }),
        );
        unreadable.push(long);
        for text in unreadable {
            assert_eq!(read_share_code(&text), None, "{text:.60}");
        }
    }

    #[test]
    fn an_import_replaces_everything_with_the_fallbacks() {
        let mut shared = Palette::default();
        shared.save_custom(None, "Sport", "#22D3EE").expect("saved");
        shared.set_cycle(&[4, 0]).expect("cycle");
        // The lamp shows Focus, also the call color.
        let around = Around {
            lamp: Some(5),
            call_color: 5,
            ..Around::default()
        };
        let change = two_colors().import(&shared, &around);
        assert_eq!(change.palette, shared);
        assert_eq!(change.call_color, SLOT_RED);
        assert_eq!(change.lamp, Some(0));
        let sport = led_of("#22D3EE").expect("light");
        assert_eq!(
            change.commands,
            [
                cycle(&[0, 5, 1, 2]),
                black(4),
                cycle(&[0, 1, 2]),
                lamp_to(0),
                black(5),
                Command::SetColor {
                    slot: 4,
                    rgb: sport
                },
                cycle(&[4, 0]),
            ]
        );
    }
}
