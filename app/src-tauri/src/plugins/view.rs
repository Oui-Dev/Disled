// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! The plugins as the integrations window shows them, with no token, and
//! the block of a note, which only that window asks for.

use std::sync::atomic::Ordering;

use serde::Serialize;

use super::host::Host;
use super::http;
use super::manifest::{self, Capabilities, Reason};
use super::process::ProgramStatus;
use crate::colors::{LampColor, Palette};

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
    /// The color it shows: the one the user picked, or that of the plugin.
    pub color: LampColor,
    /// The color that the plugin gives it, which the user can pick back.
    pub default: LampColor,
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
    /// The port of the events, while another program holds it.
    pub port_taken: Option<u16>,
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

impl Host {
    pub(super) fn views(&self, palette: &Palette, language: &str) -> Vec<PluginView> {
        let port_taken = self.port_taken.load(Ordering::SeqCst);
        self.plugins
            .iter()
            .map(|(id, plugin)| {
                let entry = self.stored.plugins.get(id);
                let manifest = plugin.manifest.as_ref().ok();
                PluginView {
                    id: id.clone(),
                    name: manifest.map_or_else(|| id.clone(), |manifest| manifest.name.clone()),
                    icon: manifest.map_or_else(
                        || manifest::DEFAULT_ICON.into(),
                        |manifest| manifest.icon.clone(),
                    ),
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
                                    color: self
                                        .color(id, &state.id, palette)
                                        .unwrap_or(state.color),
                                    default: state.color,
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
                    port_taken: (port_taken
                        && manifest.is_some_and(|manifest| manifest.capabilities.events))
                    .then_some(http::PORT),
                    fingerprint: plugin.fingerprint.clone(),
                }
            })
            .collect()
    }

    /// The block of the note of a plugin, with its address and its token.
    pub(super) fn note(&self, id: &str) -> Option<FilledBlock> {
        let block = self.manifest(id)?.note.as_ref()?.block.as_ref()?.text();
        let url = http::hook_url(id);
        let token = self.tokens.get(id).map(str::to_owned);
        let text = block
            .replace("{{url}}", &url)
            .replace("{{token}}", token.as_deref().unwrap_or("{{token}}"));
        Some(FilledBlock { text, url, token })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
