// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! The manifest of a plugin, `plugin.json`, in version 1 of the plugin API:
//! who made it, what it can do, its states and their colors, how it talks
//! to Disled, and its installation note. Unknown fields are ignored, so that
//! version 1 reads richer manifests. A manifest that breaks a rule leaves
//! the plugin unreadable, for one reason.

use std::path::Path;
use std::time::Duration;

use semver::Version;
use serde::Serialize;
use serde_json::{Map, Value};

use super::rules::{self, Rule};

pub const FILE: &str = "plugin.json";
/// The version of the plugin API that this app reads.
pub const API: u64 = 1;

/// The icons a plugin may name. The windows bundle exactly these, since the
/// app never loads anything from the Internet.
pub const ICONS: [&str; 14] = [
    "terminal",
    "timer",
    "bot",
    "code",
    "git-branch",
    "calendar",
    "bell",
    "mail",
    "message-square",
    "music",
    "headphones",
    "coffee",
    "zap",
    "puzzle",
];
const DEFAULT_ICON: &str = "puzzle";

const MAX_STATES: usize = 8;
const MAX_DESCRIPTION: usize = 300;
const MAX_LABEL: usize = 40;
const MAX_PARAGRAPH: usize = 500;
const MAX_BLOCK: usize = 16 * 1024;
const MAX_TTL_S: u64 = 86_400;

/// Why a plugin cannot be read: in its details, and under the header of
/// the integrations window after a refused import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Reason {
    /// No `plugin.json`.
    Missing,
    /// Not JSON, from that line on.
    Json { line: usize },
    /// A field that the manifest must have.
    Field { field: String },
    /// A field that breaks its rule.
    Invalid { field: String },
    /// A later version of the plugin API.
    Api { api: u64 },
    /// The id differs from the name of the folder.
    Id,
    /// A symbolic link, which could lead out of the folder.
    Link { path: String },
    /// Too many files, or too large.
    Size,
    /// A path of a zip file that leads out of the folder.
    Path { path: String },
}

fn missing(field: &str) -> Reason {
    Reason::Field {
        field: field.into(),
    }
}

fn invalid(field: &str) -> Reason {
    Reason::Invalid {
        field: field.into(),
    }
}

/// A text in the languages of the app, or one for every language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Localized<T> {
    /// By language, an empty language for the text of every language.
    texts: Vec<(String, T)>,
}

impl<T> Localized<T> {
    /// The text in `language`, else in English, else in French, else the
    /// first one.
    pub fn get(&self, language: &str) -> &T {
        [language, "en", "fr"]
            .iter()
            .find_map(|wanted| {
                self.texts
                    .iter()
                    .find(|(lang, _)| lang == wanted)
                    .map(|(_, text)| text)
            })
            .unwrap_or(&self.texts[0].1)
    }
}

/// A localized value: one for all, or an object by language.
fn localized<T>(
    value: Option<&Value>,
    field: &str,
    read: impl Fn(&Value) -> Option<T>,
) -> Result<Localized<T>, Reason> {
    let value = value.ok_or_else(|| missing(field))?;
    let texts = match value {
        Value::Object(languages) => languages
            .iter()
            .map(|(lang, text)| read(text).map(|text| (lang.clone(), text)))
            .collect::<Option<Vec<_>>>(),
        one => read(one).map(|text| vec![(String::new(), text)]),
    };
    match texts {
        Some(texts) if !texts.is_empty() => Ok(Localized { texts }),
        _ => Err(invalid(field)),
    }
}

/// A text of 1 to `max` characters.
fn text(value: &Value, max: usize) -> Option<String> {
    let text = value.as_str()?;
    (1..=max)
        .contains(&text.chars().count())
        .then(|| text.to_owned())
}

/// What a plugin can do, from its manifest only.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct Capabilities {
    /// Disled runs its program.
    pub run: bool,
    /// It picks the color of the lamp, when the lamp follows it.
    pub color: bool,
    pub alert: bool,
    /// It gets events through the local server.
    pub events: bool,
}

/// A state the plugin announces, and its color unless the user picks another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    pub id: String,
    pub label: Localized<String>,
    /// A built-in color, slot 0 to 3.
    pub color: u8,
    /// 1 to 9: the live key of the highest priority gives the state.
    pub priority: u8,
    /// How long a key keeps the state without a new event.
    pub ttl: Option<Duration>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Events {
    /// Who sends them, such as "Claude Code".
    pub from: Localized<String>,
    /// A JSON pointer to what tells the sources apart, such as a session.
    pub key: Option<String>,
    pub rules: Vec<Rule>,
}

/// The command of a program, for each system.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Run {
    pub windows: Option<Vec<String>>,
    pub macos: Option<Vec<String>>,
    pub linux: Option<Vec<String>>,
    pub default: Option<Vec<String>>,
}

impl Run {
    /// The command for this system.
    pub fn command(&self) -> Option<&[String]> {
        let own = if cfg!(windows) {
            &self.windows
        } else if cfg!(target_os = "macos") {
            &self.macos
        } else {
            &self.linux
        };
        own.as_deref().or(self.default.as_deref())
    }
}

/// The block of an installation note: a text, or JSON that the app shows
/// indented.
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Text(String),
    Json(Value),
}

impl Block {
    /// The block as the note shows it, before Disled fills in the address
    /// and the token.
    pub fn text(&self) -> String {
        match self {
            Block::Text(text) => text.clone(),
            Block::Json(value) => serde_json::to_string_pretty(value).unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    /// Paragraphs, where a passage between backquotes shows as code.
    pub text: Option<Localized<Vec<String>>>,
    /// The file the block goes into, such as "settings.json".
    pub file: Option<String>,
    pub block: Option<Block>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: Version,
    pub author: String,
    /// One of `ICONS`.
    pub icon: String,
    pub description: Localized<String>,
    pub capabilities: Capabilities,
    pub states: Vec<State>,
    /// The state shown when no key is alive.
    pub idle: Option<String>,
    pub events: Option<Events>,
    pub run: Option<Run>,
    pub note: Option<Note>,
}

/// An id: a lowercase letter or digit, then up to 31 of them or dashes.
pub fn valid_id(id: &str) -> bool {
    let mut chars = id.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_lowercase() || first.is_ascii_digit())
        && id.len() <= 32
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn valid_state_id(id: &str) -> bool {
    (1..=32).contains(&id.len())
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// The slot of a built-in color, by its name in a manifest.
fn built_in(name: &str) -> Option<u8> {
    ["green", "orange", "red", "warm"]
        .iter()
        .position(|known| *known == name)
        .and_then(|slot| u8::try_from(slot).ok())
}

/// Reads the manifest of the plugin in `folder`.
pub fn read(folder: &Path) -> Result<Manifest, Reason> {
    let text = match std::fs::read_to_string(folder.join(FILE)) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Err(Reason::Missing),
        Err(_) => return Err(Reason::Json { line: 1 }),
    };
    let value: Value =
        serde_json::from_str(&text).map_err(|error| Reason::Json { line: error.line() })?;
    let name = folder
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    Manifest::from_value(&value, Some(name))
}

impl Manifest {
    /// Checks a manifest. `folder` is the name of the folder of the plugin,
    /// which its id must match; none for an import, whose folder takes the id.
    pub fn from_value(value: &Value, folder: Option<&str>) -> Result<Self, Reason> {
        let object = value.as_object().ok_or(Reason::Json { line: 1 })?;
        let api = object.get("api").ok_or_else(|| missing("api"))?;
        match api.as_u64() {
            Some(API) => {}
            Some(later) if later > API => return Err(Reason::Api { api: later }),
            _ => return Err(invalid("api")),
        }
        let id = string(object, "id")?;
        if !valid_id(&id) {
            return Err(invalid("id"));
        }
        if folder.is_some_and(|folder| folder != id) {
            return Err(Reason::Id);
        }
        let name = text(object.get("name").ok_or_else(|| missing("name"))?, 32)
            .ok_or_else(|| invalid("name"))?;
        let version =
            Version::parse(&string(object, "version")?).map_err(|_| invalid("version"))?;
        let author = text(object.get("author").ok_or_else(|| missing("author"))?, 40)
            .ok_or_else(|| invalid("author"))?;
        let icon = object
            .get("icon")
            .and_then(Value::as_str)
            .filter(|icon| ICONS.contains(icon))
            .unwrap_or(DEFAULT_ICON)
            .to_owned();
        let description = localized(object.get("description"), "description", |value| {
            text(value, MAX_DESCRIPTION)
        })?;
        let capabilities = capabilities(object)?;
        let states = if capabilities.color {
            states(object.get("states").ok_or_else(|| missing("states"))?)?
        } else {
            Vec::new()
        };
        let idle = match object.get("idle") {
            None => None,
            Some(idle) => {
                let idle = idle.as_str().ok_or_else(|| invalid("idle"))?;
                if !states.iter().any(|state| state.id == idle) {
                    return Err(invalid("idle"));
                }
                Some(idle.to_owned())
            }
        };
        let events = match (capabilities.events, object.get("events")) {
            (true, Some(events)) => Some(read_events(events, &states)?),
            (true, None) => return Err(missing("events")),
            (false, Some(_)) => return Err(invalid("events")),
            (false, None) => None,
        };
        let run = match (capabilities.run, object.get("run")) {
            (true, Some(run)) => Some(read_run(run)?),
            (true, None) => return Err(missing("run")),
            (false, Some(_)) => return Err(invalid("run")),
            (false, None) => None,
        };
        let note = object.get("note").map(read_note).transpose()?;
        Ok(Self {
            id,
            name,
            version,
            author,
            icon,
            description,
            capabilities,
            states,
            idle,
            events,
            run,
            note,
        })
    }

    /// The state of this id.
    pub fn state(&self, id: &str) -> Option<&State> {
        self.states.iter().find(|state| state.id == id)
    }
}

fn string(object: &Map<String, Value>, field: &str) -> Result<String, Reason> {
    object
        .get(field)
        .ok_or_else(|| missing(field))?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| invalid(field))
}

/// The capabilities it declares, of which this API knows four; others are
/// ignored, and never granted. A plugin runs a program or gets events.
fn capabilities(object: &Map<String, Value>) -> Result<Capabilities, Reason> {
    let list = object
        .get("capabilities")
        .ok_or_else(|| missing("capabilities"))?
        .as_array()
        .ok_or_else(|| invalid("capabilities"))?;
    let mut capabilities = Capabilities::default();
    for capability in list {
        match capability.as_str() {
            Some("run") => capabilities.run = true,
            Some("color") => capabilities.color = true,
            Some("alert") => capabilities.alert = true,
            Some("events") => capabilities.events = true,
            Some(_) => {}
            None => return Err(invalid("capabilities")),
        }
    }
    if capabilities.run || capabilities.events {
        Ok(capabilities)
    } else {
        Err(invalid("capabilities"))
    }
}

fn states(value: &Value) -> Result<Vec<State>, Reason> {
    let list = value.as_array().ok_or_else(|| invalid("states"))?;
    if !(1..=MAX_STATES).contains(&list.len()) {
        return Err(invalid("states"));
    }
    let mut states: Vec<State> = Vec::new();
    for state in list {
        let state = read_state(state).ok_or_else(|| invalid("states"))?;
        if states.iter().any(|known| known.id == state.id) {
            return Err(invalid("states"));
        }
        states.push(state);
    }
    Ok(states)
}

fn read_state(value: &Value) -> Option<State> {
    let object = value.as_object()?;
    let id = object.get("id")?.as_str().filter(|id| valid_state_id(id))?;
    let label = localized(object.get("label"), "label", |value| text(value, MAX_LABEL)).ok()?;
    let color = built_in(object.get("color")?.as_str()?)?;
    let priority = match object.get("priority") {
        None => 1,
        Some(priority) => u8::try_from(priority.as_u64().filter(|p| (1..=9).contains(p))?).ok()?,
    };
    let ttl = match object.get("ttl") {
        None => None,
        Some(ttl) => Some(Duration::from_secs(
            ttl.as_u64().filter(|ttl| (1..=MAX_TTL_S).contains(ttl))?,
        )),
    };
    Some(State {
        id: id.to_owned(),
        label,
        color,
        priority,
        ttl,
    })
}

fn read_events(value: &Value, states: &[State]) -> Result<Events, Reason> {
    let object = value.as_object().ok_or_else(|| invalid("events"))?;
    let from = localized(object.get("from"), "events.from", |value| text(value, 40))?;
    let key = match object.get("key") {
        None => None,
        Some(key) => Some(
            key.as_str()
                .filter(|key| key.is_empty() || key.starts_with('/'))
                .ok_or_else(|| invalid("events.key"))?
                .to_owned(),
        ),
    };
    let rules = rules::parse(
        object.get("rules").ok_or_else(|| missing("events.rules"))?,
        |state| states.iter().any(|known| known.id == state),
    )
    .ok_or_else(|| invalid("events.rules"))?;
    Ok(Events { from, key, rules })
}

fn read_run(value: &Value) -> Result<Run, Reason> {
    let object = value.as_object().ok_or_else(|| invalid("run"))?;
    let command = |field: &str| -> Result<Option<Vec<String>>, Reason> {
        match object.get(field) {
            None => Ok(None),
            Some(Value::Array(arguments)) if !arguments.is_empty() => arguments
                .iter()
                .map(|argument| argument.as_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>()
                .filter(|arguments| !arguments[0].is_empty())
                .map(Some)
                .ok_or_else(|| invalid("run")),
            Some(_) => Err(invalid("run")),
        }
    };
    let run = Run {
        windows: command("windows")?,
        macos: command("macos")?,
        linux: command("linux")?,
        default: command("default")?,
    };
    // A program that has no command for this system cannot run here.
    if run.command().is_some() {
        Ok(run)
    } else {
        Err(invalid("run"))
    }
}

fn read_note(value: &Value) -> Result<Note, Reason> {
    let object = value.as_object().ok_or_else(|| invalid("note"))?;
    let paragraphs = match object.get("text") {
        None => None,
        Some(paragraphs) => Some(localized(Some(paragraphs), "note.text", |value| {
            value
                .as_array()?
                .iter()
                .map(|paragraph| text(paragraph, MAX_PARAGRAPH))
                .collect()
        })?),
    };
    let file = match object.get("file") {
        None => None,
        Some(file) => Some(text(file, 64).ok_or_else(|| invalid("note.file"))?),
    };
    let block = match object.get("block") {
        None => None,
        Some(Value::String(text)) => Some(Block::Text(text.clone())),
        Some(json) => Some(Block::Json(json.clone())),
    };
    if block
        .as_ref()
        .is_some_and(|block| block.text().len() > MAX_BLOCK)
    {
        return Err(invalid("note.block"));
    }
    Ok(Note {
        text: paragraphs,
        file,
        block,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The manifest of the Claude Code plugin of annex D, without its note.
    fn claude_code() -> Value {
        json!({
            "api": 1,
            "id": "claude-code",
            "name": "Claude Code",
            "version": "1.0.0",
            "author": "le projet Disled",
            "icon": "terminal",
            "description": { "fr": "Montre sur la lampe ce que fait Claude Code." },
            "capabilities": ["color", "alert", "events"],
            "states": [
                { "id": "waiting", "label": { "fr": "Claude attend votre réponse" }, "color": "orange", "priority": 3, "ttl": 1800 },
                { "id": "working", "label": { "fr": "Claude travaille" }, "color": "red", "priority": 2, "ttl": 600 },
                { "id": "done", "label": { "fr": "Claude a fini" }, "color": "green", "priority": 1, "ttl": 1800 }
            ],
            "idle": "done",
            "events": {
                "from": "Claude Code",
                "key": "/session_id",
                "rules": [
                    { "when": { "/hook_event_name": "SessionEnd" }, "clear": true },
                    { "when": { "/hook_event_name": ["PermissionRequest", "Elicitation", "StopFailure"] }, "state": "waiting", "alert": true },
                    { "when": { "/hook_event_name": "Stop" }, "state": "done" }
                ]
            }
        })
    }

    fn pomodoro() -> Value {
        json!({
            "api": 1,
            "id": "pomodoro",
            "name": "Pomodoro",
            "version": "1.2.0",
            "author": "lea-dev",
            "icon": "timer",
            "description": { "fr": "Ne pas déranger pendant vos sessions, Disponible pendant les pauses." },
            "capabilities": ["run", "color", "alert"],
            "states": [
                { "id": "work", "label": { "fr": "Session" }, "color": "red", "priority": 2 },
                { "id": "break", "label": { "fr": "Pause" }, "color": "green" }
            ],
            "run": { "default": ["node", "index.js"] }
        })
    }

    fn reason(mut value: Value, change: impl FnOnce(&mut Value)) -> Reason {
        change(&mut value);
        Manifest::from_value(&value, Some(value["id"].as_str().unwrap_or("claude-code")))
            .expect_err("refused")
    }

    #[test]
    fn the_manifests_of_annex_d_read() {
        let manifest = Manifest::from_value(&claude_code(), Some("claude-code")).expect("read");
        assert_eq!(manifest.name, "Claude Code");
        assert_eq!(manifest.icon, "terminal");
        assert!(manifest.capabilities.events && manifest.capabilities.alert);
        assert!(!manifest.capabilities.run);
        assert_eq!(manifest.state("waiting").map(|state| state.color), Some(1));
        assert_eq!(
            manifest.state("working").and_then(|state| state.ttl),
            Some(Duration::from_secs(600))
        );
        assert_eq!(manifest.idle.as_deref(), Some("done"));
        assert_eq!(
            manifest.events.as_ref().map(|events| events.rules.len()),
            Some(3)
        );

        let manifest = Manifest::from_value(&pomodoro(), Some("pomodoro")).expect("read");
        assert_eq!(manifest.state("break").map(|state| state.priority), Some(1));
        assert_eq!(
            manifest.run.as_ref().and_then(Run::command),
            Some(&["node".to_owned(), "index.js".to_owned()][..])
        );
    }

    /// The example of the repository reads as a folder of the plugins folder
    /// does, its name included.
    #[test]
    fn the_example_plugin_reads() {
        let folder =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/claude-code");
        let manifest = read(&folder).expect("examples/plugins/claude-code reads");
        assert_eq!(manifest.id, "claude-code");
        assert_eq!(manifest.states.len(), 3);
        assert!(manifest.capabilities.events && manifest.capabilities.alert);
        assert!(
            manifest
                .note
                .as_ref()
                .is_some_and(|note| matches!(note.block, Some(Block::Json(_))))
        );
    }

    #[test]
    fn a_text_comes_in_the_language_of_the_app_or_english_or_french() {
        let mut value = claude_code();
        value["description"] = json!({ "fr": "Français", "en": "English", "de": "Deutsch" });
        let manifest = Manifest::from_value(&value, Some("claude-code")).expect("read");
        assert_eq!(manifest.description.get("fr"), "Français");
        assert_eq!(manifest.description.get("es"), "English");
        value["description"] = json!({ "fr": "Français" });
        let manifest = Manifest::from_value(&value, Some("claude-code")).expect("read");
        assert_eq!(manifest.description.get("en"), "Français");
        value["description"] = json!("Partout");
        let manifest = Manifest::from_value(&value, Some("claude-code")).expect("read");
        assert_eq!(manifest.description.get("en"), "Partout");
    }

    #[test]
    fn each_refusal_has_its_reason() {
        let field = |field: &str| Reason::Field {
            field: field.into(),
        };
        assert_eq!(
            reason(claude_code(), |v| {
                v.as_object_mut().map(|o| o.remove("name"));
            }),
            field("name")
        );
        assert_eq!(
            reason(claude_code(), |v| v["api"] = json!(2)),
            Reason::Api { api: 2 }
        );
        assert_eq!(
            Manifest::from_value(&claude_code(), Some("autre")),
            Err(Reason::Id)
        );
        assert_eq!(
            reason(claude_code(), |v| v["name"] = json!("x".repeat(33))),
            invalid("name")
        );
        assert_eq!(
            reason(claude_code(), |v| v["version"] = json!("1.0")),
            invalid("version")
        );
        // Neither a program nor events.
        assert_eq!(
            reason(claude_code(), |v| v["capabilities"] = json!(["color"])),
            invalid("capabilities")
        );
        // A color without states, a state whose color is not built in.
        assert_eq!(
            reason(claude_code(), |v| {
                v.as_object_mut().map(|o| o.remove("states"));
            }),
            field("states")
        );
        assert_eq!(
            reason(claude_code(), |v| v["states"][0]["color"] = json!("pink")),
            invalid("states")
        );
        assert_eq!(
            reason(claude_code(), |v| v["idle"] = json!("asleep")),
            invalid("idle")
        );
        // A rule toward a state that does not exist.
        assert_eq!(
            reason(claude_code(), |v| v["events"]["rules"][2]["state"] =
                json!("asleep")),
            invalid("events.rules")
        );
        assert_eq!(
            reason(pomodoro(), |v| v["run"] = json!({ "default": [] })),
            invalid("run")
        );
    }

    #[test]
    fn unknown_fields_and_capabilities_are_ignored() {
        let mut value = claude_code();
        value["homepage"] = json!("https://example.com");
        value["capabilities"] = json!(["color", "alert", "events", "camera"]);
        value["icon"] = json!("skull");
        let manifest = Manifest::from_value(&value, Some("claude-code")).expect("read");
        assert_eq!(
            manifest.capabilities,
            Capabilities {
                run: false,
                color: true,
                alert: true,
                events: true
            }
        );
        // An icon outside the list falls back to the puzzle.
        assert_eq!(manifest.icon, "puzzle");
    }

    #[test]
    fn ids_are_short_and_lowercase() {
        assert!(valid_id("claude-code"));
        assert!(valid_id("a"));
        assert!(!valid_id("-claude"));
        assert!(!valid_id("Claude"));
        assert!(!valid_id(&"a".repeat(33)));
        assert!(!valid_id("../x"));
    }

    #[test]
    fn the_file_on_disk_reads_with_the_line_of_its_error() {
        let folder = tempfile::tempdir().expect("folder");
        let plugin = folder.path().join("claude-code");
        std::fs::create_dir(&plugin).expect("plugin");
        assert_eq!(read(&plugin), Err(Reason::Missing));
        std::fs::write(plugin.join(FILE), "{\n  \"api\": 1,\n  oops\n}").expect("write");
        assert_eq!(read(&plugin), Err(Reason::Json { line: 3 }));
        std::fs::write(plugin.join(FILE), claude_code().to_string()).expect("write");
        assert_eq!(
            read(&plugin).map(|manifest| manifest.id),
            Ok("claude-code".into())
        );
    }
}
