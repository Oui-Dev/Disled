// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! The catalog of plugins, github.com/Oui-Dev/Disled-plugins: plugins in
//! source form, which its maintainers review at every version, published
//! with an index that the catalog signs with a key of its own, not the
//! update key. The app reads the index only when the Catalog tab of the
//! integrations window opens, and downloads a plugin only when the user
//! installs it: its zip must have the size and the SHA-256, and its files
//! the id and the fingerprint, that the signed index gives. The usual
//! approval follows.

use std::collections::HashMap;
use std::io::Cursor;

use disled_flash::signature::verify_signed;
use parking_lot::Mutex;
use semver::Version;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager};

use super::manifest::{Capabilities, Manifest, Reason};
use super::{process, scan};
use crate::net;

/// Where the catalog publishes its index and its zip files.
const BASE_URL: &str = "https://oui-dev.github.io/Disled-plugins";
/// The page of a plugin in the catalog repository, before its id.
const SOURCE_URL: &str = "https://github.com/Oui-Dev/Disled-plugins/tree/main/plugins/";
/// The public half of the catalog key, which `tauri signer generate` wrote
/// in its `.pub` file (docs/releasing.md), minisign key 474E5F10EC4C34E3.
/// The private half signs the index in the publish workflow of the catalog.
const CATALOG_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDQ3NEU1RjEwRUM0QzM0RTMKUldUak5FenNFRjlPUndDbisrRFdldDRRdktHVTVxak4rcC9mV0hETEJlL2w3eGFMNGRnMWlmRngK";
const INDEX: &str = "index.json";
const MAX_INDEX: usize = 1024 * 1024;
/// A plugin of the catalog holds sources only.
const MAX_ZIP: usize = 10 * 1024 * 1024;
/// The version of the index that this app reads.
const SCHEMA: u64 = 1;

/// A plugin of the index.
#[derive(Debug, Clone)]
struct Published {
    manifest: Manifest,
    /// Of its zip file.
    size: u64,
    sha256: String,
    /// Of its files, as the approval computes it.
    fingerprint: String,
}

/// Why the catalog, or a plugin of it, failed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CatalogError {
    /// The network failed, with its error.
    Network { detail: String },
    /// The index is not signed with the catalog key.
    Signature,
    /// The index cannot be read, as one for a later app.
    Index,
    /// The last reading of the catalog has no such plugin.
    Missing,
    /// The zip does not hold what the index says.
    Mismatch,
    /// The plugin of the zip cannot be read, or installed.
    Unreadable { reason: Reason },
}

fn network(detail: String) -> CatalogError {
    CatalogError::Network { detail }
}

/// A plugin of the catalog, as the Catalog tab shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct CatalogView {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub capabilities: Capabilities,
    /// The program that its command needs on this system, such as `node`.
    pub needs: Option<String>,
    /// Who sends its events, such as "Claude Code".
    pub from: Option<String>,
    /// A plugin of this id is installed, readable or not.
    pub installed: bool,
    /// The plugin installed holds the files of this version.
    pub current: bool,
    /// The catalog has a later version than the one installed, or the one
    /// installed cannot be read.
    pub update: bool,
}

/// The plugins of the last reading, which an install takes its plugin from.
#[derive(Default)]
pub struct CatalogState(Mutex<Vec<Published>>);

/// Checks that `index` is the index of the catalog, signed with `key`:
/// `signature` is the content of `index.json.sig`.
fn verify(key: &str, index: &[u8], signature: &str) -> Result<(), CatalogError> {
    verify_signed(key, index, signature, &[&format!("file:{INDEX}")])
        .map_err(|_| CatalogError::Signature)
}

/// The plugins of an index. A plugin that this app cannot read, such as one
/// of a later API, stays out, and so does a second plugin of the same id.
fn parse(index: &[u8]) -> Result<Vec<Published>, CatalogError> {
    let value: Value = serde_json::from_slice(index).map_err(|_| CatalogError::Index)?;
    if value.get("schema").and_then(Value::as_u64) != Some(SCHEMA) {
        return Err(CatalogError::Index);
    }
    let entries = value
        .get("plugins")
        .and_then(Value::as_array)
        .ok_or(CatalogError::Index)?;
    let digest = |entry: &Value, field: &str| {
        entry
            .get(field)
            .and_then(Value::as_str)
            .filter(|text| text.len() == 64 && text.bytes().all(|b| b.is_ascii_hexdigit()))
            .map(str::to_ascii_lowercase)
    };
    let mut published: Vec<Published> = Vec::new();
    for entry in entries {
        let Some(manifest) = entry
            .get("manifest")
            .and_then(|manifest| Manifest::from_value(manifest, None).ok())
        else {
            continue;
        };
        let (Some(size), Some(sha256), Some(fingerprint)) = (
            entry.get("size").and_then(Value::as_u64),
            digest(entry, "sha256"),
            digest(entry, "fingerprint"),
        ) else {
            continue;
        };
        if published
            .iter()
            .any(|known| known.manifest.id == manifest.id)
        {
            continue;
        }
        published.push(Published {
            manifest,
            size,
            sha256,
            fingerprint,
        });
    }
    Ok(published)
}

/// Where the zip of a plugin is: the catalog names it by id and version.
fn zip_url(manifest: &Manifest) -> String {
    format!("{BASE_URL}/zips/{}-{}.zip", manifest.id, manifest.version)
}

/// The page of a plugin in the catalog repository, with its sources.
pub fn source_url(id: &str) -> String {
    format!("{SOURCE_URL}{id}")
}

/// The plugins installed, by id: their version and their fingerprint, none
/// while the plugin cannot be read.
pub type Installed = HashMap<String, Option<(Version, String)>>;

fn view(published: &Published, installed: &Installed, language: &str) -> CatalogView {
    let manifest = &published.manifest;
    let here = installed.get(&manifest.id);
    let readable = here.and_then(Option::as_ref);
    let current = readable.is_some_and(|(_, fingerprint)| *fingerprint == published.fingerprint);
    // A program named rather than given by path, which the user installs.
    let needs = manifest
        .run
        .as_ref()
        .and_then(super::manifest::Run::command)
        .and_then(|command| command.first())
        .filter(|program| !process::is_path(program))
        .cloned();
    CatalogView {
        id: manifest.id.clone(),
        name: manifest.name.clone(),
        icon: manifest.icon.clone(),
        version: manifest.version.to_string(),
        author: manifest.author.clone(),
        description: manifest.description.get(language).clone(),
        capabilities: manifest.capabilities,
        needs,
        from: manifest
            .events
            .as_ref()
            .map(|events| events.from.get(language).clone()),
        installed: here.is_some(),
        current,
        update: !current
            && here.is_some()
            && readable.is_none_or(|(version, _)| *version < manifest.version),
    }
}

/// Reads the catalog again, for the Catalog tab.
pub async fn read(app: &AppHandle) -> Result<Vec<CatalogView>, CatalogError> {
    let index = net::get_at_most(app, &format!("{BASE_URL}/{INDEX}"), MAX_INDEX)
        .await
        .map_err(network)?;
    let signature = net::get_at_most(app, &format!("{BASE_URL}/{INDEX}.sig"), net::MAX_SIGNATURE)
        .await
        .map_err(network)?;
    verify(CATALOG_KEY, &index, &String::from_utf8_lossy(&signature))?;
    let published = parse(&index)?;
    let installed = super::installed(app);
    let language = super::language(app);
    let views = published
        .iter()
        .map(|published| view(published, &installed, language))
        .collect();
    *app.state::<CatalogState>().0.lock() = published;
    Ok(views)
}

/// Installs a plugin of the last reading, in place of the plugin of that
/// id, whose token and colors stay. Returns its id; the approval follows.
pub async fn install(app: &AppHandle, id: &str) -> Result<String, CatalogError> {
    let published = app
        .state::<CatalogState>()
        .0
        .lock()
        .iter()
        .find(|published| published.manifest.id == id)
        .cloned()
        .ok_or(CatalogError::Missing)?;
    let max = usize::try_from(published.size)
        .unwrap_or(usize::MAX)
        .min(MAX_ZIP);
    let zip = net::get_at_most(app, &zip_url(&published.manifest), max)
        .await
        .map_err(network)?;
    if u64::try_from(zip.len()).ok() != Some(published.size)
        || scan::hex(&Sha256::digest(&zip)) != published.sha256
    {
        return Err(CatalogError::Mismatch);
    }
    super::import_reviewed(app, Cursor::new(zip), id, &published.fingerprint)
}

#[cfg(test)]
mod tests {
    use super::*;
    use disled_flash::signature::Refusal;
    use serde_json::json;

    // `tauri signer sign` of TEST_INDEX, saved as index.json then as
    // other.json, with a throwaway key.
    const TEST_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDY1OEYxRjYzQjE1Nzc5QTMKUldTamVWZXhZeCtQWmVNckZMY09Hd3ArU1JNNGxsMERxbmsxWWZVR001ZVo1cXUwaDBNaExXZ0UK";
    const TEST_INDEX: &[u8] = br#"{"schema":1,"plugins":[]}"#;
    const TEST_SIGNATURE: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVTamVWZXhZeCtQWmVuSzNkMlV0enNqT2pFZHVweGw4RU1VaHREWG90bUZVRGdvaFZjam9vMmxBdklGcUNBT2JDNGozSlROa28yQ0Jjc3Vrdm5TY2Y0M1BUWDI2WW9uOVFFPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzk1MTI2MzA5CWZpbGU6aW5kZXguanNvbgpLREVVS0QwalM1UmtacXZ0d0NibVJyK1JEVm52eTFaY2RIM09PbHNVTlExZlJXNmd4bEE5WEQxeTkrbnh2Z0k1c3BsTDVwY1pyQ2hPRzgxUDlTbWZCZz09Cg==";
    const OTHER_FILE_SIGNATURE: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVTamVWZXhZeCtQWlFTd0w4c21WRkhITXd0eU1QRDRyZFpuTmJjUGJWckd0R09Mb3BPMTR2NXZza1pVQ3pDMk9KYTc0aU9LY0o0Wk40VGEwRFJudFAvdWpqVmo0aGZ5aGdNPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzk1MTI2MzU3CWZpbGU6b3RoZXIuanNvbgpVbkorMEU1UWlBR0UyWDB6bEo1cDZHVUZGRFg1dytrVU81SkxCSFlGcG5Mc1hqUnhDUU00d2FZWEFpRk5nV2VwOXBNcytLVHpxUzQ4WjJYWWJ1bHBBZz09Cg==";

    fn pomodoro(version: &str, api: u64) -> Value {
        json!({
            "api": api,
            "id": "pomodoro",
            "name": "Pomodoro",
            "version": version,
            "author": "Disled",
            "icon": "timer",
            "description": { "en": "A timer.", "fr": "Un minuteur." },
            "capabilities": ["run", "color", "lamp"],
            "states": [{ "id": "focus", "label": "Session", "color": "red" }],
            "run": { "default": ["node", "index.js"] }
        })
    }

    fn entry(manifest: Value, fingerprint: char) -> Value {
        json!({
            "manifest": manifest,
            "size": 1234,
            "sha256": "ab".repeat(32),
            "fingerprint": fingerprint.to_string().repeat(64),
        })
    }

    fn index(entries: Vec<Value>) -> Vec<u8> {
        json!({ "schema": 1, "plugins": entries })
            .to_string()
            .into_bytes()
    }

    #[test]
    fn only_the_index_signed_with_the_catalog_key_reads() {
        assert_eq!(verify(TEST_KEY, TEST_INDEX, TEST_SIGNATURE), Ok(()));
        // The catalog key reads, and refuses what the throwaway key signed.
        assert_eq!(
            verify_signed(CATALOG_KEY, TEST_INDEX, TEST_SIGNATURE, &[]),
            Err(Refusal::NotSigned)
        );
        // The same bytes, signed under another name.
        assert_eq!(
            verify(TEST_KEY, TEST_INDEX, OTHER_FILE_SIGNATURE),
            Err(CatalogError::Signature)
        );
    }

    #[test]
    fn an_index_keeps_the_plugins_that_this_app_reads() {
        let mut broken = entry(pomodoro("1.0.0", 1), 'c');
        broken["sha256"] = json!("not hex");
        let later = entry(pomodoro("3.0.0", 2), 'd');
        let twice = entry(pomodoro("9.0.0", 1), 'e');
        let published = parse(&index(vec![
            later,
            broken,
            entry(pomodoro("1.2.0", 1), 'f'),
            twice,
        ]))
        .expect("index");
        assert_eq!(published.len(), 1);
        assert_eq!(published[0].manifest.version.to_string(), "1.2.0");
        assert_eq!(published[0].fingerprint, "f".repeat(64));
        // An index of a later schema needs a later app.
        let later_schema = json!({ "schema": 2, "plugins": [] }).to_string();
        assert!(matches!(
            parse(later_schema.as_bytes()),
            Err(CatalogError::Index)
        ));
        assert!(matches!(parse(b"<html>"), Err(CatalogError::Index)));
    }

    #[test]
    fn a_plugin_shows_whether_it_is_installed_and_current() {
        let published = parse(&index(vec![entry(pomodoro("1.2.0", 1), 'f')])).expect("index");
        let published = &published[0];
        let shown = |installed: Installed| view(published, &installed, "fr");

        let absent = shown(Installed::new());
        assert!(!absent.installed && !absent.current && !absent.update);
        assert_eq!(absent.description, "Un minuteur.");
        assert_eq!(absent.needs.as_deref(), Some("node"));

        let at = |version: &str, fingerprint: char| {
            Installed::from([(
                "pomodoro".to_owned(),
                Some((
                    Version::parse(version).expect("version"),
                    fingerprint.to_string().repeat(64),
                )),
            )])
        };
        let current = shown(at("1.2.0", 'f'));
        assert!(current.installed && current.current && !current.update);
        let older = shown(at("1.1.0", 'a'));
        assert!(older.installed && !older.current && older.update);
        // Changed by hand, or newer: nothing to offer.
        let changed = shown(at("1.2.0", 'a'));
        assert!(!changed.current && !changed.update);
        let newer = shown(at("2.0.0", 'a'));
        assert!(!newer.update);
        // A plugin of that id that does not read: the catalog replaces it.
        let unreadable = shown(Installed::from([("pomodoro".to_owned(), None)]));
        assert!(unreadable.installed && unreadable.update);
    }

    #[test]
    fn a_plugin_has_its_zip_and_its_sources_in_the_catalog() {
        let published = parse(&index(vec![entry(pomodoro("1.2.0", 1), 'f')])).expect("index");
        assert_eq!(
            zip_url(&published[0].manifest),
            "https://oui-dev.github.io/Disled-plugins/zips/pomodoro-1.2.0.zip"
        );
        assert_eq!(
            source_url("pomodoro"),
            "https://github.com/Oui-Dev/Disled-plugins/tree/main/plugins/pomodoro"
        );
    }

    #[test]
    fn a_plugin_given_by_path_needs_nothing_named() {
        let mut manifest = pomodoro("1.0.0", 1);
        manifest["run"] = json!({ "default": ["bin/timer"] });
        let published = parse(&index(vec![entry(manifest, 'f')])).expect("index");
        assert_eq!(view(&published[0], &Installed::new(), "en").needs, None);
    }
}
