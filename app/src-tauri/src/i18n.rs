// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! English and French. The windows translate themselves with vue-i18n, and the backend
//! reads the same files in `app/src/locales` for the texts the system draws: the tray
//! menu and tooltip, and the window titles.

use std::sync::{LazyLock, Mutex};

use serde::Serialize;
use serde_json::Value;

use crate::settings::Language;

/// A language the app speaks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum Locale {
    #[default]
    En,
    Fr,
}

/// The message files, in the order of `Locale`, parsed at first use.
static MESSAGES: LazyLock<[Value; 2]> = LazyLock::new(|| {
    [
        include_str!("../../src/locales/en.json"),
        include_str!("../../src/locales/fr.json"),
    ]
    .map(|json| serde_json::from_str(json).expect("the message files are JSON"))
});

impl Locale {
    /// The language of a setting. "System" takes the first language of the
    /// system that the app speaks, English if there is none.
    pub fn of(language: Language) -> Self {
        match language {
            Language::En => Locale::En,
            Language::Fr => Locale::Fr,
            Language::System => sys_locale::get_locales()
                .find_map(|tag| Self::from_tag(&tag))
                .unwrap_or_default(),
        }
    }

    /// Reads a system language tag, such as "fr-FR", or "fr_FR.UTF-8" on
    /// Linux.
    fn from_tag(tag: &str) -> Option<Self> {
        let language = tag.split(['-', '_', '.']).next()?;
        match language.to_ascii_lowercase().as_str() {
            "en" => Some(Locale::En),
            "fr" => Some(Locale::Fr),
            _ => None,
        }
    }

    fn messages(self) -> &'static Value {
        &MESSAGES[self as usize]
    }

    /// The message at a dotted key such as "tray.open". It falls back to
    /// English when this language lacks it, and to the key itself when no
    /// file has it.
    pub fn t(self, key: &str) -> String {
        let find = |messages: &'static Value| {
            key.split('.')
                .try_fold(messages, |node, part| node.get(part))
                .and_then(Value::as_str)
        };
        find(self.messages())
            .or_else(|| find(Locale::En.messages()))
            .unwrap_or(key)
            .to_owned()
    }
}

/// The language in force, shared by the tray, the windows and the commands.
#[derive(Default)]
pub struct LocaleState(Mutex<Locale>);

impl LocaleState {
    pub fn get(&self) -> Locale {
        self.0.lock().map(|locale| *locale).unwrap_or_default()
    }

    /// Sets the language; returns the one it replaces.
    pub fn replace(&self, locale: Locale) -> Locale {
        self.0
            .lock()
            .map(|mut current| std::mem::replace(&mut *current, locale))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    /// The dotted keys of the messages under `node`.
    fn keys(node: &Value, prefix: &str, found: &mut BTreeSet<String>) {
        match node {
            Value::Object(children) => {
                for (name, child) in children {
                    keys(child, &format!("{prefix}{name}."), found);
                }
            }
            _ => {
                found.insert(prefix.trim_end_matches('.').to_owned());
            }
        }
    }

    #[test]
    fn both_languages_have_the_same_messages() {
        let [en, fr] = &*MESSAGES;
        let (mut english, mut french) = (BTreeSet::new(), BTreeSet::new());
        keys(en, "", &mut english);
        keys(fr, "", &mut french);
        let only_english: Vec<_> = english.difference(&french).collect();
        let only_french: Vec<_> = french.difference(&english).collect();
        assert!(
            only_english.is_empty() && only_french.is_empty(),
            "only in en.json: {only_english:?}; only in fr.json: {only_french:?}"
        );
    }

    /// The tray and the titles show the messages as they are, so what the
    /// backend reads has no vue-i18n syntax: no placeholders, links or
    /// plurals.
    #[test]
    fn the_messages_of_the_backend_are_plain_text() {
        for locale in [Locale::En, Locale::Fr] {
            let mut found = BTreeSet::new();
            for section in ["tray", "window"] {
                keys(
                    &locale.messages()[section],
                    &format!("{section}."),
                    &mut found,
                );
            }
            assert!(!found.is_empty());
            for key in found {
                let text = locale.t(&key);
                assert!(!text.contains(['{', '}', '@', '|', '$']), "{key}: {text}");
            }
        }
    }

    #[test]
    fn a_missing_message_falls_back_to_its_key() {
        assert_eq!(Locale::Fr.t("tray.open"), "Ouvrir DISLED");
        assert_eq!(Locale::Fr.t("no.such.key"), "no.such.key");
    }

    #[test]
    fn system_tags_map_to_the_languages_of_the_app() {
        assert_eq!(Locale::from_tag("fr-FR"), Some(Locale::Fr));
        assert_eq!(Locale::from_tag("fr_CA.UTF-8"), Some(Locale::Fr));
        assert_eq!(Locale::from_tag("EN-us"), Some(Locale::En));
        assert_eq!(Locale::from_tag("de-DE"), None);
        assert_eq!(Locale::of(Language::Fr), Locale::Fr);
    }
}
