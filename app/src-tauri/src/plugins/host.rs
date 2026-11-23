// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! The host of the plugins: what it keeps of each plugin, and the rules of
//! their approvals, their states and their programs. A change leaves what
//! the app must do in an outbox, so the rules run without the app, as in
//! the tests. Its fields are open to the glue and the views of the module.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::async_runtime::JoinHandle;
use tokio::sync::mpsc;

use super::catalog::Installed;
use super::manifest::{self, Manifest, Reason};
use super::process::{self, Feed, Notification, Program, ProgramStatus, Spec};
use super::rules::{self, Action};
use super::states::Keys;
use super::view::PluginView;
use super::{http, scan, tokens};
use crate::colors::{LampColor, Palette};
use crate::control::{Input, Integration, IntegrationState};
use crate::updates;

/// What the app keeps of a plugin.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Entry {
    /// The fingerprint that the user approved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) approved: Option<String>,
    #[serde(default)]
    pub(super) enabled: bool,
    /// The slot that the user picked for each state.
    #[serde(default)]
    pub(super) colors: BTreeMap<String, u8>,
    /// The installation note folded after a first event, until the token
    /// changes.
    #[serde(default)]
    pub(super) installed: bool,
}

/// The key `integrations` of the settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Stored {
    /// The integration the lamp follows, none in manual mode.
    pub(super) mode: Option<String>,
    pub(super) plugins: BTreeMap<String, Entry>,
}

impl Stored {
    /// Reads the key field by field: a damaged entry leaves alone.
    fn from_value(value: &Value) -> Self {
        let mode = value
            .get("mode")
            .and_then(Value::as_str)
            .filter(|mode| manifest::valid_id(mode))
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

    /// The key to save. Manual mode saves as null, which leaves every id to
    /// the plugins.
    fn to_value(&self) -> Value {
        serde_json::json!({
            "mode": self.mode,
            "plugins": self.plugins,
        })
    }
}

/// A plugin as the host knows it.
pub(super) struct Plugin {
    pub(super) manifest: Result<Manifest, Reason>,
    pub(super) fingerprint: Option<String>,
    pub(super) keys: Keys,
    /// The state on show, and since when.
    pub(super) shown: Option<(String, u64)>,
    /// When the last event came, and the state it gave.
    pub(super) heard: Option<(u64, Option<String>)>,
    pub(super) program: Option<ProgramStatus>,
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
pub(super) struct Outbox {
    pub(super) control: Vec<Input>,
    pub(super) stop: Vec<Program>,
    pub(super) save: Option<Value>,
    pub(super) views: Option<Vec<PluginView>>,
}

pub(super) struct Host {
    pub(super) folder: PathBuf,
    data: PathBuf,
    logs: PathBuf,
    cache: scan::Cache,
    pub(super) stored: Stored,
    pub(super) tokens: tokens::Tokens,
    pub(super) plugins: BTreeMap<String, Plugin>,
    pub(super) programs: HashMap<String, Program>,
    hooks: http::Hooks,
    events: mpsc::Sender<http::Event>,
    pub(super) server: Option<JoinHandle<()>>,
    pub(super) port_taken: Arc<AtomicBool>,
    /// The `lamp` notification that the programs last got, which a program
    /// that starts gets first.
    pub(super) lamp: String,
}

impl Host {
    /// The host of the plugins of `config/plugins`. Their data and their
    /// tokens go under `local`, their logs under `logs`, and `saved` is the
    /// key `integrations` of the settings.
    pub(super) fn new(
        config: &Path,
        local: &Path,
        logs: &Path,
        saved: &Value,
        events: mpsc::Sender<http::Event>,
        lamp: String,
    ) -> Self {
        Self {
            folder: config.join("plugins"),
            data: local.join("plugin-data"),
            logs: logs.join("plugins"),
            cache: scan::Cache::default(),
            stored: Stored::from_value(saved),
            tokens: tokens::Tokens::load(local.join("plugin-tokens.json")),
            plugins: BTreeMap::new(),
            programs: HashMap::new(),
            hooks: Arc::default(),
            events,
            server: None,
            port_taken: Arc::default(),
            lamp,
        }
    }

    pub(super) fn entry(&mut self, id: &str) -> &mut Entry {
        self.stored.plugins.entry(id.to_owned()).or_default()
    }

    /// A plugin the user approved as it is now.
    pub(super) fn approved(&self, id: &str) -> bool {
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

    pub(super) fn active(&self, id: &str) -> bool {
        self.approved(id)
            && self
                .stored
                .plugins
                .get(id)
                .is_some_and(|entry| entry.enabled)
    }

    pub(super) fn manifest(&self, id: &str) -> Option<&Manifest> {
        self.plugins
            .get(id)
            .and_then(|plugin| plugin.manifest.as_ref().ok())
    }

    /// The color of a state: the one the user picked while it exists, the
    /// one of the plugin otherwise.
    pub(super) fn color(&self, id: &str, state: &str, palette: &Palette) -> Option<LampColor> {
        let default = self.manifest(id)?.state(state)?.color;
        let picked = self
            .stored
            .plugins
            .get(id)
            .and_then(|entry| entry.colors.get(state))
            .copied()
            .filter(|slot| palette.defines(*slot));
        Some(picked.map_or(default, LampColor::Slot))
    }

    /// Reads the folder again: new plugins, changed ones, gone ones. The
    /// programs of the files that changed or left stop, and `run_programs`
    /// starts the new ones.
    pub(super) fn rescan(&mut self, outbox: &mut Outbox) {
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
            self.plugins.insert(id, plugin);
        }
    }

    /// A plugin that left: its program stops, its states go.
    fn forget(&mut self, id: &str, outbox: &mut Outbox) {
        self.plugins.remove(id);
        if let Some(program) = self.programs.remove(id) {
            outbox.stop.push(program);
        }
    }

    /// Starts the programs of the active plugins that run one and are not
    /// running yet; the callers stop the others. `start` runs a program
    /// with the callbacks of the app.
    pub(super) fn run_programs(
        &mut self,
        home: Option<PathBuf>,
        language: &str,
        start: impl Fn(Spec, Option<Arc<Feed>>) -> Program,
    ) {
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
            let Some(program) =
                process::resolve(&command[0], &folder, std::env::var_os("PATH"), home.clone())
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
                language: language.into(),
                log: self.logs.join(format!("{id}.log")),
            };
            let feed = self
                .manifest(&id)
                .is_some_and(|manifest| manifest.capabilities.lamp)
                .then(|| Feed::new(self.lamp.clone()));
            self.programs.insert(id, start(spec, feed));
        }
    }

    /// The approved plugins with events, for the local server, which runs
    /// only while there is one. `redraw` tells the windows that another
    /// program took the port, or left it.
    pub(super) fn serve(&mut self, redraw: impl Fn() + Send + Sync + 'static) {
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
        *self.hooks.write() = hooks;
        match (&self.server, wanted) {
            (None, true) => {
                let router = http::router(Arc::clone(&self.hooks), self.events.clone());
                let taken = Arc::clone(&self.port_taken);
                self.server = Some(tauri::async_runtime::spawn(http::serve(
                    router,
                    move |now| {
                        // The windows hear of a change, which no change of
                        // the plugins brings.
                        if taken.swap(now, Ordering::SeqCst) != now {
                            redraw();
                        }
                    },
                )));
            }
            (Some(_), false) => {
                if let Some(server) = self.server.take() {
                    server.abort();
                }
                self.port_taken.store(false, Ordering::SeqCst);
            }
            _ => {}
        }
    }

    pub(super) fn save(&self, outbox: &mut Outbox) {
        outbox.save = Some(self.stored.to_value());
    }

    /// Brings the control and the windows in line with the plugins. The
    /// control gets every input at each sync: one that changes nothing does
    /// nothing.
    pub(super) fn sync(&mut self, palette: &Palette, language: &str, outbox: &mut Outbox) {
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
        let announces: Vec<Input> = integrations
            .iter()
            .map(|integration| Input::Announce {
                id: integration.id.clone(),
                state: self.integration_state(&integration.id, palette, language),
            })
            .collect();
        outbox.control.push(Input::Integrations(integrations));
        outbox.control.extend(announces);
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
            color: self.color(id, state, palette)?,
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
            plugin.shown = current.map(|state| (state, updates::now()));
        }
    }

    /// An action of a rule or of a program, on a key.
    fn act(&mut self, id: &str, key: &str, action: &Action, outbox: &mut Outbox) {
        let now = Instant::now();
        let Some(plugin) = self.plugins.get_mut(id) else {
            return;
        };
        let Ok(manifest) = plugin.manifest.as_ref() else {
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
        let alert = alert && manifest.capabilities.alert;
        plugin.heard = Some((updates::now(), given));
        self.refresh(id);
        if alert {
            outbox.control.push(Input::Alert {
                plugin: id.to_owned(),
            });
        }
    }

    /// The approval of the files as they are now, for the fingerprint that
    /// the question showed: the plugin turns on, with its token. Its program
    /// waits for `run_programs`.
    pub(super) fn approve(
        &mut self,
        id: &str,
        fingerprint: &str,
        outbox: &mut Outbox,
    ) -> Result<(), String> {
        self.rescan(outbox);
        let current = self
            .plugins
            .get(id)
            .and_then(|plugin| plugin.fingerprint.as_deref());
        if current != Some(fingerprint) {
            return Err("the files of the plugin changed: read the question again".into());
        }
        if self
            .manifest(id)
            .is_some_and(|manifest| manifest.capabilities.events)
        {
            self.tokens.ensure(id).map_err(|e| e.to_string())?;
        }
        let entry = self.entry(id);
        entry.approved = Some(fingerprint.to_owned());
        entry.enabled = true;
        if let Some(plugin) = self.plugins.get_mut(id) {
            plugin.program = None;
        }
        self.save(outbox);
        Ok(())
    }

    /// Turns a plugin on or off. On wants the approval of its files as they
    /// are, and its program waits for `run_programs`. Off stops its program
    /// and forgets its states, and the lamp it drove goes back to the user.
    pub(super) fn set_enabled(
        &mut self,
        id: &str,
        enabled: bool,
        outbox: &mut Outbox,
    ) -> Result<(), String> {
        self.rescan(outbox);
        if enabled && !self.approved(id) {
            return Err("the plugin needs an approval".into());
        }
        self.entry(id).enabled = enabled;
        if let Some(plugin) = self.plugins.get_mut(id) {
            plugin.keys.reset();
            plugin.shown = None;
            plugin.program = None;
        }
        if !enabled && let Some(program) = self.programs.remove(id) {
            outbox.stop.push(program);
        }
        self.save(outbox);
        Ok(())
    }

    /// The color of a state of a plugin, built in or in the palette, or the
    /// one of the plugin again, for no `slot`.
    pub(super) fn set_color(
        &mut self,
        id: &str,
        state: &str,
        slot: Option<u8>,
        palette: &Palette,
        outbox: &mut Outbox,
    ) -> Result<(), String> {
        if slot.is_some_and(|slot| !palette.defines(slot)) {
            return Err("no such color".into());
        }
        if self
            .manifest(id)
            .and_then(|manifest| manifest.state(state))
            .is_none()
        {
            return Err("no such state".into());
        }
        let colors = &mut self.entry(id).colors;
        match slot {
            Some(slot) => colors.insert(state.to_owned(), slot),
            None => colors.remove(state),
        };
        self.save(outbox);
        Ok(())
    }

    /// Forgets a plugin: its program, its token and what the app kept of it.
    /// Returns its folder and the folder of its data, which the caller
    /// deletes once the program stopped.
    pub(super) fn remove(
        &mut self,
        id: &str,
        outbox: &mut Outbox,
    ) -> Result<(PathBuf, PathBuf), String> {
        // A folder the scan found, unreadable ones too: a plain name, never
        // a path, since the scan takes the names of the folder's entries.
        if !self.plugins.contains_key(id) {
            return Err("no such plugin".into());
        }
        self.forget(id, outbox);
        self.stored.plugins.remove(id);
        self.tokens.remove(id).map_err(|e| e.to_string())?;
        self.save(outbox);
        Ok((self.folder.join(id), self.data.join(id)))
    }

    /// An event of the local server, for a plugin that may take it.
    pub(super) fn on_event(&mut self, id: &str, event: &Value, outbox: &mut Outbox) {
        if !self.active(id) {
            return;
        }
        let Some((key, action)) = self.manifest(id).and_then(|manifest| {
            let events = manifest.events.as_ref()?;
            let key = rules::key_of(event, events.key.as_deref());
            Some((key, rules::action_for(&events.rules, event).cloned()))
        }) else {
            return;
        };
        // The first event folds the installation note.
        if !self.entry(id).installed {
            self.entry(id).installed = true;
            self.save(outbox);
        }
        match action {
            Some(action) => self.act(id, &key, &action, outbox),
            None => {
                if let Some(plugin) = self.plugins.get_mut(id) {
                    plugin.heard = Some((updates::now(), None));
                }
            }
        }
    }

    /// A notification of a program.
    pub(super) fn on_note(&mut self, id: &str, note: Notification, outbox: &mut Outbox) {
        if !self.active(id) {
            return;
        }
        let (key, action) = match note {
            Notification::State { state, key } => {
                if self
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
                if let Some(plugin) = self.plugins.get_mut(id) {
                    plugin.keys.clear(None);
                }
                self.refresh(id);
                return;
            }
            Notification::Clear { key } => (key, Action::Clear),
            Notification::Alert => (None, Action::Alert),
        };
        let key = key.unwrap_or_else(|| rules::DEFAULT_KEY.into());
        self.act(id, &key, &action, outbox);
    }

    /// How a program fares. One that stopped loses its states.
    pub(super) fn on_program(&mut self, id: &str, status: ProgramStatus) {
        if let Some(plugin) = self.plugins.get_mut(id) {
            if status != ProgramStatus::Running {
                plugin.keys.reset();
            }
            plugin.program = Some(status);
        }
        self.refresh(id);
    }

    /// A key whose time runs out by `now`, which `expire` takes away.
    pub(super) fn due(&self, now: Instant) -> bool {
        self.plugins
            .values()
            .any(|plugin| plugin.keys.next_expiry().is_some_and(|until| until <= now))
    }

    /// The keys whose time ran out leave, and their plugins show what is
    /// left.
    pub(super) fn expire(&mut self, now: Instant) {
        let ids: Vec<String> = self.plugins.keys().cloned().collect();
        for id in ids {
            let expired = self
                .plugins
                .get_mut(&id)
                .is_some_and(|plugin| plugin.keys.expire(now));
            if expired {
                self.refresh(&id);
            }
        }
    }

    /// The plugins of the folder, for the catalog.
    pub(super) fn installed(&self) -> Installed {
        self.plugins
            .iter()
            .map(|(id, plugin)| {
                let version = plugin
                    .manifest
                    .as_ref()
                    .ok()
                    .map(|manifest| manifest.version.clone());
                (id.clone(), version.zip(plugin.fingerprint.clone()))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::scan::tests::write;
    use serde_json::json;

    fn host(folder: &Path) -> Host {
        let (events, _) = mpsc::channel(1);
        Host::new(folder, folder, folder, &Value::Null, events, String::new())
    }

    /// A plugin with a program, whose command these tests never start.
    fn add(host: &Host, id: &str, capabilities: &[&str]) {
        let manifest = json!({
            "api": 2, "id": id, "name": id, "version": "1.0.0", "author": "test",
            "description": "A test plugin.", "capabilities": capabilities,
            "states": [{ "id": "work", "label": "Work", "color": "red" }],
            "run": { "default": ["bin/run"] }
        });
        write(
            &host.folder.join(id).join(manifest::FILE),
            &manifest.to_string(),
        );
    }

    fn fingerprint(host: &Host, id: &str) -> String {
        host.plugins[id].fingerprint.clone().expect("fingerprint")
    }

    /// Adds a plugin and approves its files.
    fn add_approved(host: &mut Host, id: &str, capabilities: &[&str]) {
        add(host, id, capabilities);
        let mut outbox = Outbox::default();
        host.rescan(&mut outbox);
        let files = fingerprint(host, id);
        host.approve(id, &files, &mut outbox).expect("approval");
    }

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
        // Written back, it reads the same, even for a plugin named "manual".
        assert_eq!(Stored::from_value(&stored.to_value()), stored);
        let manual = Stored {
            mode: Some("manual".into()),
            plugins: BTreeMap::new(),
        };
        assert_eq!(Stored::from_value(&manual.to_value()), manual);
        // As after app 1.1.2: manual mode, no plugin.
        let fresh = Stored::from_value(&Value::Null);
        assert_eq!(fresh.mode, None);
        assert!(fresh.plugins.is_empty());
        assert_eq!(fresh.to_value()["mode"], Value::Null);
    }

    #[test]
    fn an_approval_is_for_the_files_that_the_question_showed() {
        let folder = tempfile::tempdir().expect("folder");
        let mut host = host(folder.path());
        add(&host, "timer", &["run", "color"]);
        let mut outbox = Outbox::default();
        host.rescan(&mut outbox);
        let shown = fingerprint(&host, "timer");
        // A file changes while the question is open.
        write(&host.folder.join("timer/index.js"), "changed");
        assert!(host.approve("timer", &shown, &mut outbox).is_err());
        assert!(!host.active("timer"));
        let files = fingerprint(&host, "timer");
        assert_eq!(host.approve("timer", &files, &mut outbox), Ok(()));
        assert!(host.active("timer"));
    }

    #[test]
    fn a_changed_file_turns_the_plugin_off_until_it_is_undone() {
        let folder = tempfile::tempdir().expect("folder");
        let mut host = host(folder.path());
        add_approved(&mut host, "timer", &["run", "color"]);
        let added = host.folder.join("timer/index.js");
        write(&added, "added");
        let mut outbox = Outbox::default();
        host.rescan(&mut outbox);
        assert!(!host.active("timer"));
        std::fs::remove_file(&added).expect("undo");
        host.rescan(&mut outbox);
        assert!(host.active("timer"));
    }

    #[test]
    fn only_an_approved_plugin_turns_on() {
        let folder = tempfile::tempdir().expect("folder");
        let mut host = host(folder.path());
        add(&host, "timer", &["run", "color"]);
        let mut outbox = Outbox::default();
        assert!(host.set_enabled("timer", true, &mut outbox).is_err());
        assert!(!host.active("timer"));
        add_approved(&mut host, "timer", &["run", "color"]);
        assert_eq!(host.set_enabled("timer", false, &mut outbox), Ok(()));
        assert!(!host.active("timer"));
        assert_eq!(host.set_enabled("timer", true, &mut outbox), Ok(()));
        assert!(host.active("timer"));
    }

    #[test]
    fn an_alert_needs_the_alert_capability() {
        let folder = tempfile::tempdir().expect("folder");
        let mut host = host(folder.path());
        add_approved(&mut host, "timer", &["run", "color"]);
        add_approved(&mut host, "alarm", &["run", "color", "alert"]);
        let mut outbox = Outbox::default();
        host.on_note("timer", Notification::Alert, &mut outbox);
        host.on_note("alarm", Notification::Alert, &mut outbox);
        assert_eq!(
            outbox.control,
            [Input::Alert {
                plugin: "alarm".into()
            }]
        );
    }

    #[test]
    fn a_program_gives_only_the_states_of_its_manifest() {
        let folder = tempfile::tempdir().expect("folder");
        let mut host = host(folder.path());
        add_approved(&mut host, "timer", &["run", "color"]);
        let state = |state: &str| Notification::State {
            state: state.into(),
            key: None,
        };
        let mut outbox = Outbox::default();
        host.on_note("timer", state("asleep"), &mut outbox);
        assert_eq!(host.plugins["timer"].heard, None);
        host.on_note("timer", state("work"), &mut outbox);
        let shown = host.plugins["timer"].shown.as_ref();
        assert_eq!(shown.map(|(state, _)| state.as_str()), Some("work"));
    }

    #[test]
    fn a_color_picked_for_a_state_replaces_that_of_the_plugin() {
        let folder = tempfile::tempdir().expect("folder");
        let mut host = host(folder.path());
        add_approved(&mut host, "timer", &["run", "color"]);
        let palette = Palette::from_value(&json!({
            "custom": [{ "slot": 4, "name": "Focus", "xy": { "x": 12452, "y": 11796 } }]
        }));
        let mut outbox = Outbox::default();
        let red = Some(LampColor::Slot(2));
        assert_eq!(
            host.set_color("timer", "work", Some(4), &palette, &mut outbox),
            Ok(())
        );
        assert_eq!(
            host.color("timer", "work", &palette),
            Some(LampColor::Slot(4))
        );
        // Once that color is deleted, the plugin's shows again.
        assert_eq!(host.color("timer", "work", &Palette::default()), red);
        assert_eq!(
            host.set_color("timer", "work", None, &palette, &mut outbox),
            Ok(())
        );
        assert_eq!(host.color("timer", "work", &palette), red);
        assert!(
            host.set_color("timer", "work", Some(9), &palette, &mut outbox)
                .is_err()
        );
        assert!(
            host.set_color("timer", "nope", Some(1), &palette, &mut outbox)
                .is_err()
        );
    }
}
