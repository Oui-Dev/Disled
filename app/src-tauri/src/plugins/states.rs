// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! The states of a plugin, kept by key, such as one for each session of
//! Claude Code. Each key holds its state until its time to live runs out,
//! and the plugin shows the state of its live key of the highest priority,
//! the most recent among equals, or else its idle state, once it has heard
//! of something.

use std::time::{Duration, Instant};

use super::manifest::Manifest;

/// Live keys of a plugin; past this, the key used least recently leaves.
pub const MAX_KEYS: usize = 32;

#[derive(Debug, Clone)]
struct Key {
    key: String,
    state: String,
    /// When the key dies without a new event, if ever.
    until: Option<Instant>,
    /// When an event last touched it.
    touched: Instant,
}

#[derive(Debug, Clone, Default)]
pub struct Keys {
    keys: Vec<Key>,
    /// An event or a program set a state once.
    heard: bool,
}

impl Keys {
    /// Puts a key in a state for `ttl`. True when the key changes state,
    /// the only case in which the state may alert.
    pub fn set(&mut self, key: &str, state: &str, ttl: Option<Duration>, now: Instant) -> bool {
        self.heard = true;
        let until = ttl.map(|ttl| now + ttl);
        if let Some(known) = self.keys.iter_mut().find(|known| known.key == key) {
            let changed = known.state != state;
            known.state = state.to_owned();
            known.until = until;
            known.touched = now;
            return changed;
        }
        if self.keys.len() >= MAX_KEYS
            && let Some(oldest) = self
                .keys
                .iter()
                .enumerate()
                .min_by_key(|(_, known)| known.touched)
                .map(|(at, _)| at)
        {
            self.keys.remove(oldest);
        }
        self.keys.push(Key {
            key: key.to_owned(),
            state: state.to_owned(),
            until,
            touched: now,
        });
        true
    }

    /// Takes a key away, or every key with none.
    pub fn clear(&mut self, key: Option<&str>) {
        match key {
            Some(key) => self.keys.retain(|known| known.key != key),
            None => self.keys.clear(),
        }
    }

    /// Forgets everything, as when the plugin stops.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// The keys whose time ran out leave. True when one left.
    pub fn expire(&mut self, now: Instant) -> bool {
        let before = self.keys.len();
        self.keys
            .retain(|known| known.until.is_none_or(|until| until > now));
        self.keys.len() != before
    }

    /// The state the plugin shows: that of the live key of the highest
    /// priority, the most recent among equals, or its idle state once it
    /// heard of something. Call `expire` first.
    pub fn current<'a>(&'a self, manifest: &'a Manifest) -> Option<&'a str> {
        let priority = |state: &str| manifest.state(state).map_or(0, |state| state.priority);
        self.keys
            .iter()
            .max_by_key(|known| (priority(&known.state), known.touched))
            .map(|known| known.state.as_str())
            .or_else(|| manifest.idle.as_deref().filter(|_| self.heard))
    }

    /// The next time a key dies, for the timer of the host.
    pub fn next_expiry(&self) -> Option<Instant> {
        self.keys.iter().filter_map(|known| known.until).min()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn manifest() -> Manifest {
        Manifest::from_value(
            &json!({
                "api": 1, "id": "claude-code", "name": "Claude Code", "version": "1.0.0",
                "author": "le projet Disled", "description": "…",
                "capabilities": ["color", "events"],
                "states": [
                    { "id": "waiting", "label": "attend", "color": "orange", "priority": 3, "ttl": 1800 },
                    { "id": "working", "label": "travaille", "color": "red", "priority": 2, "ttl": 600 },
                    { "id": "done", "label": "a fini", "color": "green", "priority": 1, "ttl": 1800 }
                ],
                "idle": "done",
                "events": { "from": "Claude Code", "key": "/session_id", "rules": [] }
            }),
            Some("claude-code"),
        )
        .expect("manifest")
    }

    fn ttl(manifest: &Manifest, state: &str) -> Option<Duration> {
        manifest.state(state).and_then(|state| state.ttl)
    }

    #[test]
    fn waiting_wins_over_working_which_wins_over_done() {
        let manifest = manifest();
        let mut keys = Keys::default();
        let now = Instant::now();
        assert_eq!(keys.current(&manifest), None, "nothing heard yet");
        keys.set("a", "done", ttl(&manifest, "done"), now);
        keys.set("b", "working", ttl(&manifest, "working"), now);
        assert_eq!(keys.current(&manifest), Some("working"));
        keys.set("c", "waiting", ttl(&manifest, "waiting"), now);
        assert_eq!(keys.current(&manifest), Some("waiting"));
        keys.clear(Some("c"));
        assert_eq!(keys.current(&manifest), Some("working"));
    }

    #[test]
    fn the_most_recent_wins_among_equals() {
        let manifest = manifest();
        let mut keys = Keys::default();
        let now = Instant::now();
        keys.set("a", "working", None, now);
        keys.set("b", "working", None, now + Duration::from_secs(1));
        keys.set("a", "working", None, now + Duration::from_secs(2));
        assert_eq!(keys.current(&manifest), Some("working"));
        assert_eq!(keys.keys.len(), 2);
    }

    #[test]
    fn a_key_dies_with_its_time_to_live_and_the_plugin_goes_idle() {
        let manifest = manifest();
        let mut keys = Keys::default();
        let now = Instant::now();
        keys.set("a", "working", ttl(&manifest, "working"), now);
        assert_eq!(keys.next_expiry(), Some(now + Duration::from_secs(600)));
        assert!(!keys.expire(now + Duration::from_secs(599)));
        // A new event renews it.
        keys.set(
            "a",
            "working",
            ttl(&manifest, "working"),
            now + Duration::from_secs(599),
        );
        assert!(!keys.expire(now + Duration::from_secs(1000)));
        assert!(keys.expire(now + Duration::from_secs(1199)));
        assert_eq!(keys.current(&manifest), Some("done"), "idle");
    }

    #[test]
    fn only_a_change_of_state_may_alert() {
        let mut keys = Keys::default();
        let now = Instant::now();
        assert!(keys.set("a", "waiting", None, now));
        assert!(!keys.set("a", "waiting", None, now), "a repeated event");
        assert!(keys.set("a", "working", None, now));
        assert!(keys.set("b", "working", None, now), "a new key");
    }

    #[test]
    fn past_32_keys_the_oldest_leaves() {
        let manifest = manifest();
        let mut keys = Keys::default();
        let now = Instant::now();
        keys.set("first", "waiting", None, now);
        for at in 1..=32 {
            keys.set(
                &format!("k{at}"),
                "done",
                None,
                now + Duration::from_secs(at),
            );
        }
        assert_eq!(keys.keys.len(), MAX_KEYS);
        assert_eq!(
            keys.current(&manifest),
            Some("done"),
            "the waiting key left first"
        );
    }

    #[test]
    fn clear_without_a_key_clears_them_all() {
        let manifest = manifest();
        let mut keys = Keys::default();
        let now = Instant::now();
        keys.set("a", "working", None, now);
        keys.set("b", "waiting", None, now);
        keys.clear(None);
        assert_eq!(keys.current(&manifest), Some("done"));
        keys.reset();
        assert_eq!(keys.current(&manifest), None);
    }
}
