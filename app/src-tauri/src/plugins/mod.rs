// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! The plugins: the folder `plugins` of the config folder of the app, where
//! each folder is a plugin, named by its id. A plugin comes disabled; the
//! user approves it in a question that lists what it can do, and approves it
//! again after any change of its files. The host reads the plugins, keeps
//! their settings and their states, runs their programs, serves their events
//! on the local server, and tells the control which integrations the lamp
//! can follow, their states and their alerts.

pub mod http;
pub mod manifest;
pub mod process;
pub mod rules;
pub mod scan;
pub mod states;
pub mod tokens;

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::colors::{BUILT_IN, ColorsState, Palette};
use crate::control::{self, Input, Integration, IntegrationState};
use crate::i18n::{Locale, LocaleState};
use crate::settings;
use manifest::{Capabilities, Manifest, Reason};
use process::{Notification, Program, ProgramStatus, Spec, Timing};
use rules::Action;
use states::Keys;

const EVENT_PLUGINS: &str = "plugins-changed";
const SCHEMA: u64 = 1;
const MANUAL: &str = "manual";
/// The address of the events, which the installation notes show.
const HOOKS_URL: &str = "http://127.0.0.1:47831/v1/hooks/";
/// How often the states whose time runs out leave.
const TICK: Duration = Duration::from_secs(1);

/// What the app keeps of a plugin.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Entry {
    /// The fingerprint that the user approved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    approved: Option<String>,
    #[serde(default)]
    enabled: bool,
    /// The slot that the user picked for each state.
    #[serde(default)]
    colors: BTreeMap<String, u8>,
    /// The installation note folded after a first event, until the token
    /// changes.
    #[serde(default)]
    installed: bool,
}

/// The key `integrations` of the settings.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Stored {
    /// The integration the lamp follows, none in manual mode.
    mode: Option<String>,
    plugins: BTreeMap<String, Entry>,
}

impl Stored {
    /// Reads the key field by field: a damaged entry leaves alone.
    fn from_value(value: &Value) -> Self {
        let mode = value
            .get("mode")
            .and_then(Value::as_str)
            .filter(|mode| *mode != MANUAL && manifest::valid_id(mode))
            .map(str::to_owned);
        let plugins = value
            .get("plugins")
            .and_then(Value::as_object)
            .map(|plugins| {
                plugins
                    .iter()
                    .filter(|(id, _)| manifest::valid_id(id))
                    .filter_map(|(id, entry)| {
                        Some((id.clone(), serde_json::from_value(entry.clone()).ok()?))
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self { mode, plugins }
    }

    fn to_value(&self) -> Value {
        serde_json::json!({
            "schema": SCHEMA,
            "mode": self.mode.as_deref().unwrap_or(MANUAL),
            "plugins": self.plugins,
        })
    }
}

/// The last event of a plugin, for its details.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct LastEvent {
    /// Seconds since 1970-01-01 UTC.
    pub at: u64,
    /// The label of the state it gave, none when it gave none.
    pub state: Option<String>,
}

/// A state of a plugin, with its color.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct StateView {
    pub id: String,
    pub label: String,
    /// The color that the user picked, or the default one.
    pub slot: u8,
}

/// The installation note, without its block, which `plugin_note` fills.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct NoteView {
    pub paragraphs: Vec<String>,
    pub file: Option<String>,
    pub block: bool,
}

/// A plugin as the integrations window shows it, with no token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct PluginView {
    pub id: String,
    /// The name of its folder while it is unreadable.
    pub name: String,
    pub icon: String,
    pub version: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub unreadable: Option<Reason>,
    /// The user approved it once.
    pub approved: bool,
    /// Its files changed since.
    pub modified: bool,
    pub enabled: bool,
    pub capabilities: Capabilities,
    /// The command of its program, as the approval shows it.
    pub command: Option<String>,
    /// Who sends its events, such as "Claude Code".
    pub from: Option<String>,
    pub states: Vec<StateView>,
    pub note: Option<NoteView>,
    /// A first event folded its note.
    pub installed: bool,
    pub last_event: Option<LastEvent>,
    pub program: Option<ProgramStatus>,
    /// Another program holds the port of the events.
    pub port_taken: bool,
    /// Its fingerprint now, which the approval sends back.
    pub fingerprint: Option<String>,
}

/// The block of a note, filled in, with what the window highlights.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct FilledBlock {
    pub text: String,
    pub url: String,
    pub token: Option<String>,
}

fn now_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// A command as the approval shows it: its arguments with spaces between
/// them, in straight quotes when they hold one.
fn shown_command(arguments: &[String]) -> String {
    arguments
        .iter()
        .map(|argument| {
            if argument.contains(' ') {
                format!("\"{argument}\"")
            } else {
                argument.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A plugin as the host knows it.
struct Plugin {
    manifest: Result<Manifest, Reason>,
    fingerprint: Option<String>,
    keys: Keys,
    /// The state on show, and since when.
    shown: Option<(String, u64)>,
    /// When the last event came, and the state it gave.
    heard: Option<(u64, Option<String>)>,
    program: Option<ProgramStatus>,
}

impl Plugin {
    fn read(folder: &Path, cache: &mut scan::Cache) -> Self {
        let manifest = manifest::read(folder);
        let fingerprint = match &manifest {
            Ok(_) => scan::fingerprint(folder, cache),
            Err(reason) => Err(reason.clone()),
        };
        let (manifest, fingerprint) = match fingerprint {
            Ok(fingerprint) => (manifest, Some(fingerprint)),
            Err(reason) => (Err(reason), None),
        };
        Self {
            manifest,
            fingerprint,
            keys: Keys::default(),
            shown: None,
            heard: None,
            program: None,
        }
    }
}

/// What a change leaves to do once the host is free again.
#[derive(Default)]
struct Outbox {
    control: Vec<Input>,
    stop: Vec<Program>,
    save: Option<Value>,
    views: Option<Vec<PluginView>>,
}

pub struct Host {
    folder: PathBuf,
    data: PathBuf,
    logs: PathBuf,
    cache: scan::Cache,
    stored: Stored,
    tokens: tokens::Tokens,
    plugins: BTreeMap<String, Plugin>,
    programs: HashMap<String, Program>,
    hooks: http::Hooks,
    events: tokio::sync::mpsc::Sender<http::Event>,
    server: Option<tauri::async_runtime::JoinHandle<()>>,
    port_taken: Arc<std::sync::atomic::AtomicBool>,
    /// What the control last heard.
    announced: Vec<Integration>,
    announced_states: HashMap<String, Option<IntegrationState>>,
}

/// The host, once the app started it.
#[derive(Default)]
pub struct PluginsState(Mutex<Option<Host>>);

fn language(app: &AppHandle) -> &'static str {
    match app.state::<LocaleState>().get() {
        Locale::Fr => "fr",
        Locale::En => "en",
    }
}

impl Host {
    fn entry(&mut self, id: &str) -> &mut Entry {
        self.stored.plugins.entry(id.to_owned()).or_default()
    }

    /// A plugin the user approved as it is now.
    fn approved(&self, id: &str) -> bool {
        let fingerprint = self
            .plugins
            .get(id)
            .and_then(|plugin| plugin.fingerprint.as_ref());
        fingerprint.is_some()
            && self
                .stored
                .plugins
                .get(id)
                .and_then(|entry| entry.approved.as_ref())
                == fingerprint
    }

    fn active(&self, id: &str) -> bool {
        self.approved(id)
            && self
                .stored
                .plugins
                .get(id)
                .is_some_and(|entry| entry.enabled)
    }

    fn manifest(&self, id: &str) -> Option<&Manifest> {
        self.plugins
            .get(id)
            .and_then(|plugin| plugin.manifest.as_ref().ok())
    }

    /// The color of a state: the one the user picked while it exists, the
    /// default one otherwise.
    fn slot(&self, id: &str, state: &str, palette: &Palette) -> Option<u8> {
        let default = self.manifest(id)?.state(state)?.color;
        let picked = self
            .stored
            .plugins
            .get(id)
            .and_then(|entry| entry.colors.get(state))
            .copied()
            .filter(|slot| *slot < BUILT_IN || palette.defines(*slot));
        Some(picked.unwrap_or(default))
    }

    /// Reads the folder again: new plugins, changed ones, gone ones.
    fn rescan(&mut self, app: &AppHandle, outbox: &mut Outbox) {
        let _ = std::fs::create_dir_all(&self.folder);
        let mut found = BTreeMap::new();
        if let Ok(entries) = std::fs::read_dir(&self.folder) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                // Imports under way start with a dot.
                if name.starts_with('.') || !entry.path().is_dir() {
                    continue;
                }
                found.insert(name, entry.path());
            }
        }
        let gone: Vec<String> = self
            .plugins
            .keys()
            .filter(|id| !found.contains_key(*id))
            .cloned()
            .collect();
        for id in gone {
            self.forget(&id, outbox);
        }
        for (id, folder) in found {
            let mut plugin = Plugin::read(&folder, &mut self.cache);
            if let Some(known) = self.plugins.remove(&id)
                && known.fingerprint == plugin.fingerprint
            {
                plugin.keys = known.keys;
                plugin.shown = known.shown;
                plugin.heard = known.heard;
                // A program not found is looked for again, as it may be
                // installed since.
                plugin.program = known
                    .program
                    .filter(|status| !matches!(status, ProgramStatus::NotFound { .. }));
            } else if let Some(program) = self.programs.remove(&id) {
                // Changed files: the program of the old files stops.
                outbox.stop.push(program);
            }
            self.plugins.insert(id.clone(), plugin);
            // An approval was for other files: the plugin waits for a new one.
            if !self.approved(&id)
                && self
                    .stored
                    .plugins
                    .get(&id)
                    .is_some_and(|entry| entry.enabled)
            {
                self.entry(&id).enabled = false;
                self.save(outbox);
            }
        }
        self.run_programs(app);
    }

    /// A plugin that left: its program stops, its states go.
    fn forget(&mut self, id: &str, outbox: &mut Outbox) {
        self.plugins.remove(id);
        if let Some(program) = self.programs.remove(id) {
            outbox.stop.push(program);
        }
    }

    /// Starts the programs of the active plugins that run one, and stops
    /// the others.
    fn run_programs(&mut self, app: &AppHandle) {
        let wanted: Vec<String> = self
            .plugins
            .keys()
            .filter(|id| self.active(id))
            .filter(|id| {
                self.manifest(id)
                    .is_some_and(|manifest| manifest.capabilities.run)
            })
            .cloned()
            .collect();
        for id in wanted {
            if self.programs.contains_key(&id)
                || matches!(
                    self.plugins
                        .get(&id)
                        .and_then(|plugin| plugin.program.as_ref()),
                    Some(ProgramStatus::Failed | ProgramStatus::NotFound { .. })
                )
            {
                continue;
            }
            let Some(command) = self
                .manifest(&id)
                .and_then(|manifest| manifest.run.as_ref())
                .and_then(manifest::Run::command)
                .map(<[String]>::to_vec)
            else {
                continue;
            };
            let folder = self.folder.join(&id);
            let home = app.path().home_dir().ok();
            let Some(program) =
                process::resolve(&command[0], &folder, std::env::var_os("PATH"), home)
            else {
                if let Some(plugin) = self.plugins.get_mut(&id) {
                    plugin.program = Some(ProgramStatus::NotFound {
                        program: command[0].clone(),
                    });
                }
                continue;
            };
            let spec = Spec {
                id: id.clone(),
                program,
                arguments: command[1..].to_vec(),
                folder,
                data: self.data.join(&id),
                language: language(app).into(),
                log: self.logs.join(format!("{id}.log")),
            };
            let (notes, statuses) = (app.clone(), app.clone());
            let (note_id, status_id) = (id.clone(), id.clone());
            let program = Program::start(
                spec,
                Timing::default(),
                move |note| on_note(&notes, &note_id, note),
                move |status| on_program(&statuses, &status_id, status),
            );
            self.programs.insert(id, program);
        }
    }

    /// The approved plugins with events, for the local server, which runs
    /// only while there is one.
    fn serve(&mut self, app: &AppHandle) {
        let hooks: HashMap<String, http::Hook> = self
            .plugins
            .iter()
            .filter(|(id, plugin)| {
                self.approved(id)
                    && plugin
                        .manifest
                        .as_ref()
                        .is_ok_and(|manifest| manifest.capabilities.events)
            })
            .filter_map(|(id, _)| {
                Some((
                    id.clone(),
                    http::Hook {
                        token: self.tokens.get(id)?.to_owned(),
                        enabled: self.active(id),
                    },
                ))
            })
            .collect();
        let wanted = !hooks.is_empty();
        if let Ok(mut shared) = self.hooks.write() {
            *shared = hooks;
        }
        match (&self.server, wanted) {
            (None, true) => {
                let router = http::router(Arc::clone(&self.hooks), self.events.clone());
                let taken = Arc::clone(&self.port_taken);
                let owner = app.clone();
                self.server = Some(tauri::async_runtime::spawn(http::serve(
                    router,
                    move |now| {
                        // The windows hear of a change, which no change of
                        // the plugins brings.
                        if taken.swap(now, std::sync::atomic::Ordering::SeqCst) != now {
                            with(&owner, |_, _| {});
                        }
                    },
                )));
            }
            (Some(_), false) => {
                if let Some(server) = self.server.take() {
                    server.abort();
                }
                self.port_taken
                    .store(false, std::sync::atomic::Ordering::SeqCst);
            }
            _ => {}
        }
    }

    fn save(&self, outbox: &mut Outbox) {
        outbox.save = Some(self.stored.to_value());
    }

    /// Brings the control and the windows in line with the plugins.
    fn sync(&mut self, app: &AppHandle, palette: &Palette, language: &str, outbox: &mut Outbox) {
        self.serve(app);
        let integrations: Vec<Integration> = self
            .plugins
            .iter()
            .filter(|(id, _)| self.active(id))
            .filter_map(|(id, plugin)| {
                let manifest = plugin.manifest.as_ref().ok()?;
                manifest.capabilities.color.then(|| Integration {
                    id: id.clone(),
                    name: manifest.name.clone(),
                    icon: manifest.icon.clone(),
                })
            })
            .collect();
        if integrations != self.announced {
            outbox
                .control
                .push(Input::Integrations(integrations.clone()));
            self.announced = integrations.clone();
        }
        // The lamp cannot follow a plugin that left.
        if self
            .stored
            .mode
            .as_ref()
            .is_some_and(|mode| !integrations.iter().any(|known| &known.id == mode))
        {
            self.stored.mode = None;
            self.save(outbox);
        }
        for integration in &integrations {
            let state = self.integration_state(&integration.id, palette, language);
            if self.announced_states.get(&integration.id) != Some(&state) {
                outbox.control.push(Input::Announce {
                    id: integration.id.clone(),
                    state: state.clone(),
                });
                self.announced_states.insert(integration.id.clone(), state);
            }
        }
        self.announced_states
            .retain(|id, _| integrations.iter().any(|known| &known.id == id));
        outbox.views = Some(self.views(palette, language));
    }

    fn integration_state(
        &self,
        id: &str,
        palette: &Palette,
        language: &str,
    ) -> Option<IntegrationState> {
        let plugin = self.plugins.get(id)?;
        let (state, since) = plugin.shown.as_ref()?;
        let manifest = plugin.manifest.as_ref().ok()?;
        Some(IntegrationState {
            label: manifest.state(state)?.label.get(language).clone(),
            slot: self.slot(id, state, palette)?,
            since: *since,
        })
    }

    /// Updates the state on show of a plugin, after its keys changed.
    fn refresh(&mut self, id: &str) {
        let Some(plugin) = self.plugins.get_mut(id) else {
            return;
        };
        let Ok(manifest) = plugin.manifest.as_ref() else {
            return;
        };
        let current = plugin.keys.current(manifest).map(str::to_owned);
        let same = plugin.shown.as_ref().map(|(state, _)| state) == current.as_ref();
        if !same {
            plugin.shown = current.map(|state| (state, now_s()));
        }
    }

    fn views(&self, palette: &Palette, language: &str) -> Vec<PluginView> {
        let port_taken = self.port_taken.load(std::sync::atomic::Ordering::SeqCst);
        self.plugins
            .iter()
            .map(|(id, plugin)| {
                let entry = self.stored.plugins.get(id);
                let manifest = plugin.manifest.as_ref().ok();
                PluginView {
                    id: id.clone(),
                    name: manifest.map_or_else(|| id.clone(), |manifest| manifest.name.clone()),
                    icon: manifest
                        .map_or_else(|| "puzzle".into(), |manifest| manifest.icon.clone()),
                    version: manifest.map(|manifest| manifest.version.to_string()),
                    author: manifest.map(|manifest| manifest.author.clone()),
                    description: manifest
                        .map(|manifest| manifest.description.get(language).clone()),
                    unreadable: plugin.manifest.as_ref().err().cloned(),
                    approved: entry.is_some_and(|entry| entry.approved.is_some()),
                    modified: entry.is_some_and(|entry| entry.approved.is_some())
                        && !self.approved(id),
                    enabled: self.active(id),
                    capabilities: manifest
                        .map(|manifest| manifest.capabilities)
                        .unwrap_or_default(),
                    command: manifest
                        .and_then(|manifest| manifest.run.as_ref())
                        .and_then(manifest::Run::command)
                        .map(shown_command),
                    from: manifest
                        .and_then(|manifest| manifest.events.as_ref())
                        .map(|events| events.from.get(language).clone()),
                    states: manifest
                        .map(|manifest| {
                            manifest
                                .states
                                .iter()
                                .map(|state| StateView {
                                    id: state.id.clone(),
                                    label: state.label.get(language).clone(),
                                    slot: self.slot(id, &state.id, palette).unwrap_or(state.color),
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                    note: manifest
                        .and_then(|manifest| manifest.note.as_ref())
                        .map(|note| NoteView {
                            paragraphs: note
                                .text
                                .as_ref()
                                .map(|text| text.get(language).clone())
                                .unwrap_or_default(),
                            file: note.file.clone(),
                            block: note.block.is_some(),
                        }),
                    installed: entry.is_some_and(|entry| entry.installed),
                    last_event: plugin.heard.as_ref().map(|(at, state)| LastEvent {
                        at: *at,
                        state: state.as_ref().and_then(|state| {
                            Some(manifest?.state(state)?.label.get(language).clone())
                        }),
                    }),
                    program: plugin.program.clone(),
                    port_taken: port_taken
                        && manifest.is_some_and(|manifest| manifest.capabilities.events),
                    fingerprint: plugin.fingerprint.clone(),
                }
            })
            .collect()
    }

    /// An action of a rule or of a program, on a key.
    fn act(&mut self, id: &str, key: &str, action: &Action, outbox: &mut Outbox) {
        let now = Instant::now();
        let Some(manifest) = self.manifest(id).cloned() else {
            return;
        };
        let Some(plugin) = self.plugins.get_mut(id) else {
            return;
        };
        let mut alert = false;
        let mut given = None;
        match action {
            Action::State {
                state,
                alert: wanted,
            } => {
                let ttl = manifest.state(state).and_then(|state| state.ttl);
                let changed = plugin.keys.set(key, state, ttl, now);
                alert = *wanted && changed;
                given = Some(state.clone());
            }
            Action::Alert => alert = true,
            Action::Clear => plugin.keys.clear(Some(key)),
            Action::Ignore => return,
        }
        plugin.heard = Some((now_s(), given));
        self.refresh(id);
        if alert && manifest.capabilities.alert {
            outbox.control.push(Input::Alert {
                plugin: id.to_owned(),
            });
        }
    }
}

/// Runs a change on the host, then what it leaves to do: the programs that
/// stop, the control, the settings, the windows.
fn with<T>(app: &AppHandle, change: impl FnOnce(&mut Host, &mut Outbox) -> T) -> Option<T> {
    let palette = app.state::<ColorsState>().get();
    let language = language(app);
    let mut outbox = Outbox::default();
    let result = {
        let state = app.state::<PluginsState>();
        let mut host = state.0.lock().ok()?;
        let host = host.as_mut()?;
        let result = change(host, &mut outbox);
        host.sync(app, &palette, language, &mut outbox);
        result
    };
    for program in outbox.stop {
        program.stop();
    }
    for input in outbox.control {
        let _ = control::input(app, input);
    }
    if let Some(value) = outbox.save {
        let _ = settings::save_integrations(app, value);
    }
    if let Some(views) = outbox.views {
        let _ = app.emit(EVENT_PLUGINS, views);
    }
    Some(result)
}

/// Starts the host: reads the plugins, runs the programs of the active
/// ones, serves the events, and gives the control the integrations on offer
/// and the one the lamp followed.
pub fn start(app: &AppHandle) {
    let paths = app.path();
    let (Ok(config), Ok(local)) = (paths.app_config_dir(), paths.app_local_data_dir()) else {
        return;
    };
    let logs = paths.app_log_dir().unwrap_or_else(|_| local.join("logs"));
    let (events, mut received) = tokio::sync::mpsc::channel(http::QUEUE);
    let host = Host {
        folder: config.join("plugins"),
        data: local.join("plugin-data"),
        logs: logs.join("plugins"),
        cache: scan::Cache::default(),
        stored: Stored::from_value(&settings::load_integrations(app)),
        tokens: tokens::Tokens::load(local.join("plugin-tokens.json")),
        plugins: BTreeMap::new(),
        programs: HashMap::new(),
        hooks: Arc::default(),
        events,
        server: None,
        port_taken: Arc::default(),
        announced: Vec::new(),
        announced_states: HashMap::new(),
    };
    let mode = host.stored.mode.clone();
    if let Ok(mut state) = app.state::<PluginsState>().0.lock() {
        *state = Some(host);
    }
    with(app, |host, outbox| host.rescan(app, outbox));
    // The lamp follows again the integration it followed, if still on offer.
    if mode.is_some() {
        let _ = control::input(app, Input::Follow(mode));
    }
    let owner = app.clone();
    tauri::async_runtime::spawn(async move {
        while let Some((id, event)) = received.recv().await {
            on_event(&owner, &id, &event);
        }
    });
    let owner = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut ticks = tokio::time::interval(TICK);
        loop {
            ticks.tick().await;
            tick(&owner);
        }
    });
}

/// The states whose time ran out leave.
fn tick(app: &AppHandle) {
    let now = Instant::now();
    let due = app
        .state::<PluginsState>()
        .0
        .lock()
        .ok()
        .and_then(|host| {
            host.as_ref().map(|host| {
                host.plugins
                    .values()
                    .any(|plugin| plugin.keys.next_expiry().is_some_and(|until| until <= now))
            })
        })
        .unwrap_or(false);
    if due {
        with(app, |host, _| {
            let ids: Vec<String> = host.plugins.keys().cloned().collect();
            for id in ids {
                let expired = host
                    .plugins
                    .get_mut(&id)
                    .is_some_and(|plugin| plugin.keys.expire(now));
                if expired {
                    host.refresh(&id);
                }
            }
        });
    }
}

/// An event of the local server, for a plugin that may take it.
fn on_event(app: &AppHandle, id: &str, event: &Value) {
    with(app, |host, outbox| {
        if !host.active(id) {
            return;
        }
        let Some(events) = host
            .manifest(id)
            .and_then(|manifest| manifest.events.clone())
        else {
            return;
        };
        // The first event folds the installation note.
        if !host.entry(id).installed {
            host.entry(id).installed = true;
            host.save(outbox);
        }
        let key = rules::key_of(event, events.key.as_deref());
        match rules::action_for(&events.rules, event) {
            Some(action) => host.act(id, &key, action, outbox),
            None => {
                if let Some(plugin) = host.plugins.get_mut(id) {
                    plugin.heard = Some((now_s(), None));
                }
            }
        }
    });
}

/// A notification of a program.
fn on_note(app: &AppHandle, id: &str, note: Notification) {
    with(app, |host, outbox| {
        if !host.active(id) {
            return;
        }
        let (key, action) = match note {
            Notification::State { state, key } => {
                if host
                    .manifest(id)
                    .and_then(|manifest| manifest.state(&state))
                    .is_none()
                {
                    return;
                }
                (
                    key,
                    Action::State {
                        state,
                        alert: false,
                    },
                )
            }
            Notification::Clear { key: None } => {
                if let Some(plugin) = host.plugins.get_mut(id) {
                    plugin.keys.clear(None);
                }
                host.refresh(id);
                return;
            }
            Notification::Clear { key } => (key, Action::Clear),
            Notification::Alert => (None, Action::Alert),
        };
        let key = key.unwrap_or_else(|| rules::DEFAULT_KEY.into());
        host.act(id, &key, &action, outbox);
    });
}

/// How a program fares. One that stopped loses its states.
fn on_program(app: &AppHandle, id: &str, status: ProgramStatus) {
    with(app, |host, _| {
        if let Some(plugin) = host.plugins.get_mut(id) {
            if status != ProgramStatus::Running {
                plugin.keys.reset();
            }
            plugin.program = Some(status);
        }
        host.refresh(id);
    });
}

/// The plugins, read again, for the integrations window.
pub fn list(app: &AppHandle) -> Vec<PluginView> {
    let palette = app.state::<ColorsState>().get();
    let language = language(app);
    with(app, |host, outbox| {
        host.rescan(app, outbox);
        host.views(&palette, language)
    })
    .unwrap_or_default()
}

/// The block of a note, with the address and the token of the plugin.
pub fn note(app: &AppHandle, id: &str) -> Option<FilledBlock> {
    with(app, |host, _| {
        let block = host.manifest(id)?.note.as_ref()?.block.as_ref()?.text();
        let url = format!("{HOOKS_URL}{id}");
        let token = host.tokens.get(id).map(str::to_owned);
        let text = block
            .replace("{{url}}", &url)
            .replace("{{token}}", token.as_deref().unwrap_or("{{token}}"));
        Some(FilledBlock { text, url, token })
    })
    .flatten()
}

/// "Copy": the backend writes the block in the clipboard.
pub fn copy_note(app: &AppHandle, id: &str) -> Result<(), String> {
    let block = note(app, id).ok_or("no note to copy")?;
    app.clipboard()
        .write_text(block.text)
        .map_err(|e| e.to_string())
}

/// The approval, for the fingerprint that the question showed: the plugin
/// turns on, with its token and its program.
pub fn approve(app: &AppHandle, id: &str, fingerprint: &str) -> Result<(), String> {
    with(app, |host, outbox| {
        host.rescan(app, outbox);
        let current = host
            .plugins
            .get(id)
            .and_then(|plugin| plugin.fingerprint.clone());
        if current.as_deref() != Some(fingerprint) {
            return Err("the files of the plugin changed: read the question again".into());
        }
        if host
            .manifest(id)
            .is_some_and(|manifest| manifest.capabilities.events)
        {
            host.tokens.ensure(id).map_err(|e| e.to_string())?;
        }
        let entry = host.entry(id);
        entry.approved = Some(fingerprint.to_owned());
        entry.enabled = true;
        if let Some(plugin) = host.plugins.get_mut(id) {
            plugin.program = None;
        }
        host.run_programs(app);
        host.save(outbox);
        Ok(())
    })
    .unwrap_or_else(|| Err("the plugins are not ready".into()))
}

/// Turns a plugin on or off. On wants the approval of its files as they
/// are; off stops its program and forgets its states, and the lamp it
/// drove goes back to the user.
pub fn set_enabled(app: &AppHandle, id: &str, enabled: bool) -> Result<(), String> {
    with(app, |host, outbox| {
        host.rescan(app, outbox);
        if enabled && !host.approved(id) {
            return Err("the plugin needs an approval".into());
        }
        host.entry(id).enabled = enabled;
        if let Some(plugin) = host.plugins.get_mut(id) {
            plugin.keys.reset();
            plugin.shown = None;
            plugin.program = None;
        }
        if !enabled && let Some(program) = host.programs.remove(id) {
            outbox.stop.push(program);
        }
        host.run_programs(app);
        host.save(outbox);
        Ok(())
    })
    .unwrap_or_else(|| Err("the plugins are not ready".into()))
}

/// The color of a state of a plugin.
pub fn set_color(app: &AppHandle, id: &str, state: &str, slot: u8) -> Result<(), String> {
    let palette = app.state::<ColorsState>().get();
    if slot >= BUILT_IN && !palette.defines(slot) {
        return Err("no such color".into());
    }
    with(app, |host, outbox| {
        if host
            .manifest(id)
            .and_then(|manifest| manifest.state(state))
            .is_none()
        {
            return Err("no such state".into());
        }
        host.entry(id).colors.insert(state.to_owned(), slot);
        host.save(outbox);
        Ok(())
    })
    .unwrap_or_else(|| Err("the plugins are not ready".into()))
}

/// A new token: the old one stops working, and the note unfolds again.
pub fn regenerate_token(app: &AppHandle, id: &str) -> Result<(), String> {
    with(app, |host, outbox| {
        host.tokens.regenerate(id).map_err(|e| e.to_string())?;
        host.entry(id).installed = false;
        host.save(outbox);
        Ok(())
    })
    .unwrap_or_else(|| Err("the plugins are not ready".into()))
}

/// Removes a plugin: its program, its folder, its token, its data, and what
/// the app kept of it.
pub fn remove(app: &AppHandle, id: &str) -> Result<(), String> {
    with(app, |host, outbox| {
        // A folder the scan found, unreadable ones too: a plain name, never
        // a path, since the scan takes the names of the folder's entries.
        if !host.plugins.contains_key(id) {
            return Err("no such plugin".into());
        }
        host.forget(id, outbox);
        host.stored.plugins.remove(id);
        host.tokens.remove(id).map_err(|e| e.to_string())?;
        host.save(outbox);
        Ok((host.folder.join(id), host.data.join(id)))
    })
    .unwrap_or_else(|| Err("the plugins are not ready".into()))
    .and_then(|(folder, data)| {
        // Once the program stopped, its files can go.
        std::fs::remove_dir_all(&folder).map_err(|e| e.to_string())?;
        let _ = std::fs::remove_dir_all(data);
        with(app, |host, outbox| host.rescan(app, outbox));
        Ok(())
    })
}

/// Imports the plugin of a zip file, in place of the plugin of the same id,
/// whose token and colors stay. Returns its id; the approval follows.
pub fn import(app: &AppHandle, zip: &Path) -> Result<String, Reason> {
    let folder = with(app, |host, _| host.folder.clone()).ok_or(Reason::Missing)?;
    let _ = std::fs::create_dir_all(&folder);
    let unpacked = scan::unpack(zip, &folder)?;
    // The program of the plugin it replaces stops first, since its files go.
    let id = unpacked.id().to_owned();
    with(app, |host, outbox| {
        if let Some(program) = host.programs.remove(&id) {
            outbox.stop.push(program);
        }
    });
    let result = unpacked.install(&folder);
    with(app, |host, outbox| host.rescan(app, outbox));
    result
}

/// The folder of the plugins, which "Open the plugins folder" shows.
pub fn folder(app: &AppHandle) -> Option<PathBuf> {
    with(app, |host, _| host.folder.clone())
}

/// Remembers the integration the lamp follows, none in manual mode.
pub fn remember_mode(app: &AppHandle, mode: Option<String>) {
    with(app, |host, outbox| {
        host.stored.mode = mode;
        host.save(outbox);
    });
}

/// The color that takes the place of a deleted one on a lamp that follows a
/// plugin: the default color of its state.
pub fn fallback_color(app: &AppHandle) -> Option<u8> {
    let mode = app.state::<crate::control::ControlState>().status().mode?;
    with(app, |host, _| {
        let state = host.plugins.get(&mode)?.shown.as_ref()?.0.clone();
        Some(host.manifest(&mode)?.state(&state)?.color)
    })
    .flatten()
}

/// The palette changed: a state whose color left takes its default one.
pub fn colors_changed(app: &AppHandle) {
    let palette = app.state::<ColorsState>().get();
    with(app, |host, outbox| {
        let mut changed = false;
        for entry in host.stored.plugins.values_mut() {
            let before = entry.colors.len();
            entry
                .colors
                .retain(|_, slot| *slot < BUILT_IN || palette.defines(*slot));
            changed |= entry.colors.len() != before;
        }
        if changed {
            host.save(outbox);
        }
    });
}

/// The labels of the states follow a new language, on the lamp page too.
pub fn relabel(app: &AppHandle) {
    with(app, |_, _| {});
}

/// The programs stop with the app, all at once, and the server closes.
pub fn shutdown(app: &AppHandle) {
    let programs: Vec<Program> = app
        .state::<PluginsState>()
        .0
        .lock()
        .ok()
        .and_then(|mut host| {
            host.as_mut().map(|host| {
                if let Some(server) = host.server.take() {
                    server.abort();
                }
                host.programs.drain().map(|(_, program)| program).collect()
            })
        })
        .unwrap_or_default();
    let stoppers: Vec<_> = programs
        .into_iter()
        .map(|program| thread::spawn(move || program.stop()))
        .collect();
    for stopper in stoppers {
        let _ = stopper.join();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_saved_key_reads_field_by_field() {
        let stored = Stored::from_value(&json!({
            "schema": 1,
            "mode": "claude-code",
            "plugins": {
                "claude-code": { "approved": "9f2c", "enabled": true, "colors": { "working": 2 }, "installed": true },
                "broken": { "enabled": "yes" },
                "../evil": { "enabled": true }
            }
        }));
        assert_eq!(stored.mode.as_deref(), Some("claude-code"));
        assert_eq!(stored.plugins.len(), 1);
        let entry = &stored.plugins["claude-code"];
        assert_eq!(entry.approved.as_deref(), Some("9f2c"));
        assert_eq!(entry.colors.get("working"), Some(&2));
        // Written back, it reads the same.
        assert_eq!(Stored::from_value(&stored.to_value()), stored);
        // As after app 1.1.2: manual mode, no plugin.
        let fresh = Stored::from_value(&Value::Null);
        assert_eq!(fresh.mode, None);
        assert!(fresh.plugins.is_empty());
        assert_eq!(fresh.to_value()["mode"], "manual");
    }

    #[test]
    fn a_command_shows_as_the_approval_writes_it() {
        assert_eq!(
            shown_command(&["node".into(), "index.js".into()]),
            "node index.js"
        );
        assert_eq!(
            shown_command(&["python".into(), "my script.py".into()]),
            "python \"my script.py\""
        );
    }
}
