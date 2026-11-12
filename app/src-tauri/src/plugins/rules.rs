// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! The rules of a plugin with events, which turn each event into an action:
//! the first rule whose conditions all hold wins, and an event that none
//! matches does nothing. The conditions read the event through JSON
//! pointers (RFC 6901); the app keeps nothing else of it.

use serde_json::{Map, Value};

const MAX_RULES: usize = 64;
const MAX_CONDITIONS: usize = 8;
/// The key of the events whose key is missing.
pub const DEFAULT_KEY: &str = "default";

/// What a value of the event must be.
#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    /// A text, a number or a boolean.
    Equals(Value),
    /// One of them.
    OneOf(Vec<Value>),
    /// A text or a list with something in it, or not.
    NonEmpty(bool),
    /// Present, or not.
    Exists(bool),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// The key takes this state, with an alert if it changes state.
    State {
        state: String,
        alert: bool,
    },
    Alert,
    /// The key leaves.
    Clear,
    Ignore,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    /// JSON pointers and their conditions, which must all hold.
    pub when: Vec<(String, Condition)>,
    pub action: Action,
}

fn scalar(value: &Value) -> bool {
    matches!(value, Value::String(_) | Value::Number(_) | Value::Bool(_))
}

fn condition(value: &Value) -> Option<Condition> {
    match value {
        Value::Array(values) if !values.is_empty() && values.iter().all(scalar) => {
            Some(Condition::OneOf(values.clone()))
        }
        Value::Object(test) if test.len() == 1 => match test.iter().next()? {
            (name, Value::Bool(wanted)) if name == "nonEmpty" => Some(Condition::NonEmpty(*wanted)),
            (name, Value::Bool(wanted)) if name == "exists" => Some(Condition::Exists(*wanted)),
            _ => None,
        },
        value if scalar(value) => Some(Condition::Equals(value.clone())),
        _ => None,
    }
}

fn flag(rule: &Map<String, Value>, name: &str) -> bool {
    rule.get(name).and_then(Value::as_bool).unwrap_or(false)
}

fn action(rule: &Map<String, Value>, known_state: &impl Fn(&str) -> bool) -> Option<Action> {
    if let Some(state) = rule.get("state") {
        let state = state.as_str().filter(|state| known_state(state))?;
        return Some(Action::State {
            state: state.to_owned(),
            alert: flag(rule, "alert"),
        });
    }
    if flag(rule, "alert") {
        Some(Action::Alert)
    } else if flag(rule, "clear") {
        Some(Action::Clear)
    } else if flag(rule, "ignore") {
        Some(Action::Ignore)
    } else {
        None
    }
}

/// Reads the rules of a manifest, whose states `known_state` tells apart;
/// none when one breaks a rule.
pub fn parse(value: &Value, known_state: impl Fn(&str) -> bool) -> Option<Vec<Rule>> {
    let list = value.as_array()?;
    if list.len() > MAX_RULES {
        return None;
    }
    list.iter()
        .map(|rule| {
            let rule = rule.as_object()?;
            let when = rule.get("when")?.as_object()?;
            if when.len() > MAX_CONDITIONS {
                return None;
            }
            let when = when
                .iter()
                .map(|(pointer, test)| {
                    (pointer.is_empty() || pointer.starts_with('/'))
                        .then_some(())
                        .and_then(|()| Some((pointer.clone(), condition(test)?)))
                })
                .collect::<Option<Vec<_>>>()?;
            Some(Rule {
                when,
                action: action(rule, &known_state)?,
            })
        })
        .collect()
}

fn holds(event: &Value, pointer: &str, condition: &Condition) -> bool {
    let value = event.pointer(pointer);
    match condition {
        Condition::Equals(wanted) => value == Some(wanted),
        Condition::OneOf(wanted) => value.is_some_and(|value| wanted.contains(value)),
        Condition::NonEmpty(wanted) => {
            let full = match value {
                Some(Value::String(text)) => !text.is_empty(),
                Some(Value::Array(items)) => !items.is_empty(),
                _ => false,
            };
            full == *wanted
        }
        Condition::Exists(wanted) => value.is_some() == *wanted,
    }
}

/// The action of the first rule that the event matches.
pub fn action_for<'a>(rules: &'a [Rule], event: &Value) -> Option<&'a Action> {
    rules
        .iter()
        .find(|rule| {
            rule.when
                .iter()
                .all(|(pointer, condition)| holds(event, pointer, condition))
        })
        .map(|rule| &rule.action)
}

/// The key of an event: the value at the pointer, `default` without one.
pub fn key_of(event: &Value, pointer: Option<&str>) -> String {
    match pointer.and_then(|pointer| event.pointer(pointer)) {
        Some(Value::String(key)) if !key.is_empty() => key.clone(),
        Some(Value::Number(key)) => key.to_string(),
        _ => DEFAULT_KEY.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The rules of the Claude Code plugin of annex D.
    fn claude_code() -> Vec<Rule> {
        let rules = json!([
            { "when": { "/hook_event_name": "SessionEnd" }, "clear": true },
            { "when": { "/hook_event_name": "PreToolUse", "/tool_name": ["AskUserQuestion", "ExitPlanMode"] }, "state": "waiting", "alert": true },
            { "when": { "/hook_event_name": ["PermissionRequest", "Elicitation", "StopFailure"] }, "state": "waiting", "alert": true },
            { "when": { "/hook_event_name": "Stop", "/background_tasks": { "nonEmpty": true } }, "state": "working" },
            { "when": { "/hook_event_name": "Stop" }, "state": "done" },
            { "when": { "/hook_event_name": "Notification" }, "ignore": true },
            { "when": { "/hook_event_name": ["UserPromptSubmit", "PreToolUse", "PostToolUse", "PostToolUseFailure", "ElicitationResult"] }, "state": "working" }
        ]);
        parse(&rules, |state| {
            ["waiting", "working", "done"].contains(&state)
        })
        .expect("rules")
    }

    fn state(state: &str, alert: bool) -> Option<Action> {
        Some(Action::State {
            state: state.into(),
            alert,
        })
    }

    #[test]
    fn the_events_of_claude_code_give_their_states() {
        let rules = claude_code();
        let action = |event: Value| action_for(&rules, &event).cloned();
        // Events as Claude Code sends them, with the fields the rules read.
        assert_eq!(
            action(json!({
                "session_id": "abc123", "transcript_path": "/tmp/t.jsonl", "cwd": "/work",
                "hook_event_name": "UserPromptSubmit", "prompt": "Fix the tests"
            })),
            state("working", false)
        );
        assert_eq!(
            action(json!({
                "session_id": "abc123", "hook_event_name": "PreToolUse",
                "tool_name": "Bash", "tool_input": { "command": "cargo test" }
            })),
            state("working", false)
        );
        assert_eq!(
            action(json!({
                "session_id": "abc123", "hook_event_name": "PreToolUse",
                "tool_name": "AskUserQuestion", "tool_input": {}
            })),
            state("waiting", true)
        );
        assert_eq!(
            action(json!({
                "session_id": "abc123", "hook_event_name": "PermissionRequest",
                "tool_name": "Write", "tool_input": { "file_path": "/work/a.rs" }
            })),
            state("waiting", true)
        );
        assert_eq!(
            action(json!({
                "session_id": "abc123", "hook_event_name": "Stop",
                "stop_hook_active": false, "background_tasks": []
            })),
            state("done", false)
        );
        assert_eq!(
            action(json!({
                "session_id": "abc123", "hook_event_name": "Stop",
                "background_tasks": [{ "id": "t1" }]
            })),
            state("working", false)
        );
        assert_eq!(
            action(json!({
                "session_id": "abc123", "hook_event_name": "Notification",
                "notification_type": "idle_prompt", "message": "Claude is waiting"
            })),
            Some(Action::Ignore)
        );
        assert_eq!(
            action(
                json!({ "session_id": "abc123", "hook_event_name": "SessionEnd", "reason": "exit" })
            ),
            Some(Action::Clear)
        );
        // An event that no rule knows does nothing.
        assert_eq!(
            action(json!({ "session_id": "abc123", "hook_event_name": "PreCompact" })),
            None
        );
    }

    #[test]
    fn the_first_rule_that_matches_wins() {
        let rules = parse(
            &json!([
                { "when": { "/a": 1 }, "state": "one" },
                { "when": { "/a": 1, "/b": true }, "state": "two" }
            ]),
            |_| true,
        )
        .expect("rules");
        assert_eq!(
            action_for(&rules, &json!({ "a": 1, "b": true })).cloned(),
            state("one", false)
        );
    }

    #[test]
    fn a_missing_value_holds_only_for_its_absence() {
        let rules = parse(
            &json!([
                { "when": { "/tool": { "exists": false } }, "state": "none" },
                { "when": { "/tool": { "nonEmpty": false } }, "state": "empty" },
                { "when": { "/missing/deeper": "x" }, "state": "never" }
            ]),
            |_| true,
        )
        .expect("rules");
        assert_eq!(
            action_for(&rules, &json!({})).cloned(),
            state("none", false)
        );
        assert_eq!(
            action_for(&rules, &json!({ "tool": "" })).cloned(),
            state("empty", false)
        );
        assert_eq!(action_for(&rules, &json!({ "tool": "Bash" })), None);
    }

    #[test]
    fn broken_rules_are_refused() {
        let known = |state: &str| state == "on";
        for rules in [
            json!({ "when": {} }),
            json!([{ "state": "on" }]),
            json!([{ "when": { "a": 1 }, "state": "on" }]),
            json!([{ "when": { "/a": { "regex": ".*" } }, "state": "on" }]),
            json!([{ "when": { "/a": [] }, "state": "on" }]),
            json!([{ "when": { "/a": 1 }, "state": "off" }]),
            json!([{ "when": { "/a": 1 } }]),
        ] {
            assert_eq!(parse(&rules, known), None, "{rules}");
        }
        let many: Vec<Value> = (0..65)
            .map(|_| json!({ "when": { "/a": 1 }, "ignore": true }))
            .collect();
        assert_eq!(parse(&Value::Array(many), known), None);
    }

    #[test]
    fn the_key_tells_the_sessions_apart() {
        let event = json!({ "session_id": "abc123", "pid": 42 });
        assert_eq!(key_of(&event, Some("/session_id")), "abc123");
        assert_eq!(key_of(&event, Some("/pid")), "42");
        assert_eq!(key_of(&event, Some("/missing")), DEFAULT_KEY);
        assert_eq!(key_of(&event, None), DEFAULT_KEY);
    }
}
