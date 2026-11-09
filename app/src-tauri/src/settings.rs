// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! Settings kept on this computer. The lamp starts in automatic brightness and forgets
//! its bias at each power-up, so the app applies the brightness ones at every
//! connection. Power and color are not settings, since the lamp always starts green.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use disled_protocol::{AUTO_BIAS_MAX, AUTO_BIAS_MIN, BrightnessMode, Command, SetState};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Theme, Wry};
use tauri_plugin_store::{Store, StoreExt};

const STORE_FILE: &str = "settings.json";
const KEY: &str = "settings";
/// The version of the app that ran last, kept next to the settings.
const VERSION_KEY: &str = "version";
/// The store writes its file once the changes stop for this long, and at
/// exit, since a slider changes the settings several times a second.
const SAVE_DELAY: Duration = Duration::from_millis(500);

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
}

impl Settings {
    /// Applies a patch, keeping every value in its range.
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
    }

    /// Commands that bring a lamp in line with these settings. The bias goes
    /// only to a lamp that announces the `auto_bias` capability.
    pub fn commands(&self, auto_bias_supported: bool) -> Vec<Command> {
        let mut commands = vec![Command::SetState(SetState {
            brightness_mode: Some(self.brightness_mode),
            manual_level: Some(self.manual_level.clamp(1, 100)),
            ..SetState::default()
        })];
        if auto_bias_supported {
            commands.push(Command::SetAutoBias {
                bias: self.auto_bias,
            });
        }
        commands
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

/// Reads the saved settings, or the defaults when there are none or they
/// cannot be read.
pub fn load(app: &AppHandle) -> Settings {
    store(app)
        .ok()
        .and_then(|store| store.get(KEY))
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
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
    fn the_bias_is_sent_only_to_lamps_that_know_it() {
        let settings = Settings {
            brightness_mode: BrightnessMode::Manual,
            manual_level: 30,
            auto_bias: 20,
            ..Settings::default()
        };
        assert_eq!(settings.commands(false).len(), 1);
        let commands = settings.commands(true);
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[1], Command::SetAutoBias { bias: 20 });
        match &commands[0] {
            Command::SetState(s) => {
                assert_eq!(s.brightness_mode, Some(BrightnessMode::Manual));
                assert_eq!(s.manual_level, Some(30));
                assert_eq!(s.on, None);
                assert_eq!(s.color, None);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn old_files_without_new_fields_still_load() {
        let settings: Settings =
            serde_json::from_str(r#"{"brightness_mode":"manual","manual_level":40}"#)
                .expect("settings");
        assert_eq!(settings.manual_level, 40);
        assert_eq!(settings.auto_bias, 0);
        assert!(!settings.autostart_initialized);
        assert!(settings.check_updates);
        assert_eq!(settings.color_mode, ColorMode::System);
        assert_eq!(settings.language, Language::System);
    }

    #[test]
    fn the_system_color_mode_forces_no_theme() {
        assert_eq!(ColorMode::System.theme(), None);
        assert_eq!(ColorMode::Dark.theme(), Some(Theme::Dark));
    }
}
