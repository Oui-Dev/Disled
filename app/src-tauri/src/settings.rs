// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! Settings kept on this computer, in a store file of three keys: `settings`
//! for the simple ones, `colors` for the palette (colors.rs) and
//! `integrations` for the plugins. The lamp keeps nothing, so the app sends
//! what it needs at every connection (device.rs). App 1.1.2 wrote only the
//! `settings` key: its file reads as it is, the new fields taking their
//! defaults, and it keeps the other keys if the user goes back to it.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use disled_protocol::{
    AUTO_BIAS_MAX, AUTO_BIAS_MIN, BrightnessMode, SLOT_RED, TOUCH_SENSITIVITY_MAX,
    TOUCH_SENSITIVITY_MIN,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Manager, Theme, Wry};
use tauri_plugin_store::{Store, StoreExt};

use crate::colors::{ColorsState, Palette};

const STORE_FILE: &str = "settings.json";
const KEY: &str = "settings";
const COLORS_KEY: &str = "colors";
/// The version of the app that ran last, kept next to the settings.
const VERSION_KEY: &str = "version";
/// The store writes its file once the changes stop for this long, and at
/// exit, since a slider changes the settings several times a second.
const SAVE_DELAY: Duration = Duration::from_millis(500);
/// The touch sensitivity moves by this much, percent.
pub const TOUCH_SENSITIVITY_STEP: i8 = 5;

/// Light or dark windows, or as the system.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum ColorMode {
    #[default]
    System,
    Light,
    Dark,
}

impl ColorMode {
    /// The theme to force on the windows; `None` follows the system.
    pub fn theme(self) -> Option<Theme> {
        match self {
            ColorMode::System => None,
            ColorMode::Light => Some(Theme::Light),
            ColorMode::Dark => Some(Theme::Dark),
        }
    }
}

/// The language of the app, or the system's (`i18n::Locale::of`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum Language {
    #[default]
    System,
    En,
    Fr,
}

/// The color the lamp takes while the microphone is in use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct CallSettings {
    /// Follow the microphone at all.
    pub mic: bool,
    /// A slot that holds a color; red, "Do not disturb", by default.
    pub color: u8,
}

impl Default for CallSettings {
    fn default() -> Self {
        Self {
            mic: false,
            color: SLOT_RED,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(default)]
pub struct Settings {
    pub brightness_mode: BrightnessMode,
    /// Percent, 1 to 100.
    pub manual_level: u8,
    /// Automatic brightness bias, percent.
    pub auto_bias: i8,
    /// Start at login was turned on once, at the first launch.
    pub autostart_initialized: bool,
    /// Look for a new version of the app and of the lamp firmware at
    /// start-up and once a day.
    pub check_updates: bool,
    pub color_mode: ColorMode,
    pub language: Language,
    /// Touch sensitivity correction, percent, a multiple of 5.
    pub touch_sensitivity: i8,
    /// Show a color on the lamp while the editor changes it.
    pub color_preview: bool,
    pub calls: CallSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            brightness_mode: BrightnessMode::Auto,
            manual_level: 50,
            auto_bias: 0,
            autostart_initialized: false,
            check_updates: true,
            color_mode: ColorMode::System,
            language: Language::System,
            touch_sensitivity: 0,
            color_preview: true,
            calls: CallSettings::default(),
        }
    }
}

/// Fields the front end may change; `None` keeps the current value.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export, optional_fields))]
pub struct SettingsPatch {
    pub brightness_mode: Option<BrightnessMode>,
    pub manual_level: Option<u8>,
    pub auto_bias: Option<i8>,
    pub check_updates: Option<bool>,
    pub color_mode: Option<ColorMode>,
    pub language: Option<Language>,
    pub touch_sensitivity: Option<i8>,
    pub color_preview: Option<bool>,
    pub calls_mic: Option<bool>,
    pub calls_color: Option<u8>,
}

/// One field of a saved object, if it reads as `T`.
fn field<T: DeserializeOwned>(object: &Value, key: &str) -> Option<T> {
    object.get(key).and_then(|value| T::deserialize(value).ok())
}

/// The nearest multiple of the step, within the limits.
fn touch_sensitivity(percent: i8) -> i8 {
    let step = f64::from(TOUCH_SENSITIVITY_STEP);
    let rounded = (f64::from(percent) / step).round() * step;
    rounded.clamp(
        f64::from(TOUCH_SENSITIVITY_MIN),
        f64::from(TOUCH_SENSITIVITY_MAX),
    ) as i8
}

impl Settings {
    /// Reads saved settings field by field. A field that cannot be read
    /// keeps its default, and the others stay.
    pub fn from_value(value: &Value) -> Self {
        let calls = value.get("calls").unwrap_or(&Value::Null);
        let mut settings = Settings::default();
        settings.apply(&SettingsPatch {
            brightness_mode: field(value, "brightness_mode"),
            manual_level: field(value, "manual_level"),
            auto_bias: field(value, "auto_bias"),
            check_updates: field(value, "check_updates"),
            color_mode: field(value, "color_mode"),
            language: field(value, "language"),
            touch_sensitivity: field(value, "touch_sensitivity"),
            color_preview: field(value, "color_preview"),
            calls_mic: field(calls, "mic"),
            calls_color: field(calls, "color"),
        });
        if let Some(initialized) = field(value, "autostart_initialized") {
            settings.autostart_initialized = initialized;
        }
        settings
    }

    /// Applies a patch, keeping every value in its range. The call color
    /// still has to name a color of the palette (`fit`).
    pub fn apply(&mut self, patch: &SettingsPatch) {
        if let Some(mode) = patch.brightness_mode {
            self.brightness_mode = mode;
        }
        if let Some(level) = patch.manual_level {
            self.manual_level = level.clamp(1, 100);
        }
        if let Some(bias) = patch.auto_bias {
            self.auto_bias = bias.clamp(AUTO_BIAS_MIN, AUTO_BIAS_MAX);
        }
        if let Some(check) = patch.check_updates {
            self.check_updates = check;
        }
        if let Some(mode) = patch.color_mode {
            self.color_mode = mode;
        }
        if let Some(language) = patch.language {
            self.language = language;
        }
        if let Some(percent) = patch.touch_sensitivity {
            self.touch_sensitivity = touch_sensitivity(percent);
        }
        if let Some(preview) = patch.color_preview {
            self.color_preview = preview;
        }
        if let Some(mic) = patch.calls_mic {
            self.calls.mic = mic;
        }
        if let Some(color) = patch.calls_color {
            self.calls.color = color;
        }
    }

    /// Brings the call color back to its default when the palette does not
    /// hold it, as after the deletion of a custom color.
    pub fn fit(&mut self, palette: &Palette) {
        if !palette.defines(self.calls.color) {
            self.calls.color = CallSettings::default().color;
        }
    }
}

/// The settings in memory, shared by the commands and the USB worker.
#[derive(Default)]
pub struct SettingsState(Mutex<Settings>);

impl SettingsState {
    pub fn get(&self) -> Settings {
        self.0.lock().map(|s| *s).unwrap_or_default()
    }
}

/// Reads the saved settings and palette, each part that cannot be read
/// taking its default.
pub fn load(app: &AppHandle) -> (Settings, Palette) {
    let store = store(app).ok();
    let read = |key: &str| {
        store
            .as_ref()
            .and_then(|store| store.get(key))
            .unwrap_or(Value::Null)
    };
    read_all(&read(KEY), &read(COLORS_KEY))
}

/// The settings and the palette of two saved keys.
fn read_all(settings: &Value, colors: &Value) -> (Settings, Palette) {
    let palette = Palette::from_value(colors);
    let mut settings = Settings::from_value(settings);
    settings.fit(&palette);
    (settings, palette)
}

/// The store of the settings. Its options hold from the first call, which
/// opens it.
fn store(app: &AppHandle) -> tauri_plugin_store::Result<Arc<Store<Wry>>> {
    app.store_builder(STORE_FILE).auto_save(SAVE_DELAY).build()
}

/// Updates the shared copy and the store, which writes them to its file a
/// moment later.
pub fn save(app: &AppHandle, settings: Settings) -> Result<(), String> {
    if let Ok(mut shared) = app.state::<SettingsState>().0.lock() {
        *shared = settings;
    }
    let store = store(app).map_err(|e| e.to_string())?;
    store.set(
        KEY,
        serde_json::to_value(settings).map_err(|e| e.to_string())?,
    );
    Ok(())
}

/// Updates the shared palette and the store, as `save` does.
pub fn save_palette(app: &AppHandle, palette: Palette) -> Result<(), String> {
    let value = serde_json::to_value(&palette).map_err(|e| e.to_string())?;
    app.state::<ColorsState>().set(palette);
    store(app)
        .map_err(|e| e.to_string())?
        .set(COLORS_KEY, value);
    Ok(())
}

/// Tells whether this version of the app runs for the first time, as after
/// an update, and remembers it. Never under `tauri dev`, whose version would
/// pass for an update of the installed app.
pub fn first_launch_of_version(app: &AppHandle) -> bool {
    if cfg!(debug_assertions) {
        return false;
    }
    let Ok(store) = store(app) else {
        return false;
    };
    let version = app.package_info().version.to_string();
    if store.get(VERSION_KEY).as_ref().and_then(|v| v.as_str()) == Some(version.as_str()) {
        return false;
    }
    store.set(VERSION_KEY, version);
    let _ = store.save();
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The `settings` key as app 1.1.2 writes it.
    fn file_of_1_1_2() -> Value {
        json!({
            "brightness_mode": "manual",
            "manual_level": 40,
            "auto_bias": -10,
            "autostart_initialized": true,
            "check_updates": false,
            "color_mode": "dark",
            "language": "fr"
        })
    }

    #[test]
    fn a_patch_keeps_values_in_range() {
        let mut settings = Settings::default();
        settings.apply(&SettingsPatch {
            manual_level: Some(0),
            auto_bias: Some(-90),
            ..SettingsPatch::default()
        });
        assert_eq!(settings.manual_level, 1);
        assert_eq!(settings.auto_bias, AUTO_BIAS_MIN);
        assert_eq!(settings.brightness_mode, BrightnessMode::Auto);
    }

    #[test]
    fn a_1_1_2_file_loads_with_v2_defaults() {
        let (settings, palette) = read_all(&file_of_1_1_2(), &Value::Null);
        assert_eq!(settings.brightness_mode, BrightnessMode::Manual);
        assert_eq!(settings.manual_level, 40);
        assert_eq!(settings.auto_bias, -10);
        assert!(settings.autostart_initialized);
        assert!(!settings.check_updates);
        assert_eq!(settings.color_mode, ColorMode::Dark);
        assert_eq!(settings.language, Language::Fr);
        assert_eq!(settings.touch_sensitivity, 0);
        assert!(settings.color_preview);
        assert!(!settings.calls.mic);
        assert_eq!(settings.calls.color, SLOT_RED);
        assert_eq!(palette, Palette::default());
    }

    #[test]
    fn one_bad_field_keeps_the_others() {
        let mut file = file_of_1_1_2();
        file["manual_level"] = json!("forty");
        file["calls"] = json!({ "mic": "yes", "color": 1 });
        let settings = Settings::from_value(&file);
        assert_eq!(settings.manual_level, 50);
        assert_eq!(settings.language, Language::Fr);
        assert_eq!(settings.color_mode, ColorMode::Dark);
        assert!(!settings.calls.mic);
        assert_eq!(settings.calls.color, 1);
        assert_eq!(
            Settings::from_value(&json!("not an object")),
            Settings::default()
        );
    }

    #[test]
    fn the_sensitivity_is_rounded_and_bounded() {
        for (saved, kept) in [
            (7, 5),
            (8, 10),
            (-7, -5),
            (-8, -10),
            (2, 0),
            (90, 50),
            (-90, -50),
        ] {
            let settings = Settings::from_value(&json!({ "touch_sensitivity": saved }));
            assert_eq!(settings.touch_sensitivity, kept, "{saved}");
        }
    }

    #[test]
    fn a_file_saved_by_1_1_2_keeps_the_colors() {
        // App 1.1.2 rewrites the `settings` key without the fields it does not
        // know, and leaves the `colors` key alone.
        let colors = json!({
            "schema": 1,
            "names": [null, null, null, null],
            "custom": [{ "slot": 4, "name": "Musique", "hex": "#9B59F6" }],
            "cycle": [0, 1, 2, 4]
        });
        let (settings, palette) = read_all(&file_of_1_1_2(), &colors);
        assert_eq!(settings.touch_sensitivity, 0);
        assert_eq!(palette.custom.len(), 1);
        assert_eq!(palette.cycle, [0, 1, 2, 4]);
    }

    #[test]
    fn a_call_color_that_left_the_palette_goes_back_to_red() {
        let file = json!({ "calls": { "mic": true, "color": 6 } });
        let (settings, _) = read_all(&file, &Value::Null);
        assert!(settings.calls.mic);
        assert_eq!(settings.calls.color, SLOT_RED);
        let colors = json!({ "custom": [{ "slot": 6, "name": "Focus", "hex": "#3B82F6" }] });
        assert_eq!(read_all(&file, &colors).0.calls.color, 6);
    }

    #[test]
    fn the_system_color_mode_forces_no_theme() {
        assert_eq!(ColorMode::System.theme(), None);
        assert_eq!(ColorMode::Dark.theme(), Some(Theme::Dark));
    }
}
