// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! What the app sets on the lamp beyond the choices of the user: the source
//! the lamp follows, the user in manual mode or an integration that picks
//! its color, the call color while the microphone is in use, the alerts of
//! the plugins, and the lamp handed back to its own behavior when the app
//! quits.
//!
//! A pure core, `Control::step`, decides from what happens to the lamp, in
//! the windows, in the plugins and on the microphone; the functions around
//! it carry out what it decides, the microphone monitor included.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use disled_protocol::{
    ChangeSource, Command, FIRST_CUSTOM_SLOT, LinearRgb, ProtocolError, SLOT_COUNT, SetState,
    State, TouchMode,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::calls::Monitor;
use crate::colors::DEFAULT_CYCLE;
use crate::device::DeviceHandle;
use crate::settings::CallSettings;

const EVENT_CONTROL: &str = "control-changed";

/// Breaths of an alert, the same for every plugin.
pub const ALERT_COUNT: u8 = 3;
/// Length of a breath, until the owner picks 1 300 or 1 500 ms with the
/// lamp closed.
pub const ALERT_PERIOD_MS: u16 = 1300;
/// An alert counts as over this long after its breaths, should its
/// ALERT_DONE never come.
const ALERT_MARGIN: Duration = Duration::from_secs(2);
/// One alert at most this often for each plugin.
const ALERT_SPACING: Duration = Duration::from_secs(10);
/// How long the app waits for the lamp to answer the commands of its exit.
const QUIT_WAIT: Duration = Duration::from_millis(200);

/// An integration the lamp can follow: an approved and enabled plugin that
/// picks a color.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct Integration {
    pub id: String,
    pub name: String,
    /// The Lucide icon of the plugin, such as "terminal".
    pub icon: String,
}

/// The state an integration announces, and the color it gives the lamp.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct IntegrationState {
    /// Such as "Claude travaille".
    pub label: String,
    pub slot: u8,
    /// When the state began, seconds since 1970-01-01 UTC.
    pub since: u64,
}

/// What the call line of the popup reads.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct CallStatus {
    /// This system can tell a call from the microphone.
    pub available: bool,
    /// A call is under way, in manual mode with the microphone on.
    pub in_call: bool,
    /// The lamp shows the call color, which the user has not overridden.
    pub imposed: bool,
    /// The color the lamp gets back after the call, while imposed.
    pub back: Option<u8>,
}

/// What the windows show of the control.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct ControlStatus {
    /// The integration the lamp follows, none in manual mode.
    pub mode: Option<String>,
    /// The state of that integration, none before its first event.
    pub state: Option<IntegrationState>,
    pub calls: CallStatus,
    /// The integrations that the menu of the mode offers.
    pub integrations: Vec<Integration>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Input {
    /// A version 2 lamp got the connection sequence.
    Connected,
    Disconnected,
    /// A STATE of the lamp, a reply or a change.
    State(State),
    /// The user chose in the popup: off, or on, with a slot in manual mode.
    Choose {
        on: bool,
        slot: Option<u8>,
    },
    /// The source the lamp follows: an integration, or none for manual.
    Follow(Option<String>),
    /// The integrations on offer, from the plugins.
    Integrations(Vec<Integration>),
    /// The state of an integration, none when it has none.
    Announce {
        id: String,
        state: Option<IntegrationState>,
    },
    CallSettings(CallSettings),
    /// This system can, or cannot, tell a call from the microphone.
    CallsAvailable(bool),
    /// A run of the microphone monitor hears a call begin, or end.
    Call {
        run: u64,
        in_call: bool,
    },
    /// After a deletion, the color the lamp gets back after the call.
    CallBack(Option<u8>),
    /// A plugin asks for an alert.
    Alert {
        plugin: String,
    },
    AlertDone,
    /// The lamp refused a command that the backend sent on its own.
    Refused {
        command: u8,
        error: ProtocolError,
    },
    /// A self-test started, or ended.
    SelfTest(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    /// A command for the lamp; `user` when it carries a choice of the user,
    /// whose refusal the windows show.
    Send { command: Command, user: bool },
    /// The heartbeat of the integrations mode, on or off.
    Heartbeat(bool),
    /// The microphone monitor starts under the number of its run, or
    /// stops.
    Monitor(Option<u64>),
}

/// A command that the backend sends on its own.
fn automatic(command: Command) -> Effect {
    Effect::Send {
        command,
        user: false,
    }
}

fn set_state(state: SetState) -> Command {
    Command::SetState(state)
}

/// A call that the app follows: in manual mode, with a lamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Call {
    /// The color the lamp had when the call began, which it gets back.
    back: u8,
    /// Until the user chooses a color during the call.
    imposed: bool,
}

#[derive(Debug, Default)]
pub struct Control {
    mode: Option<String>,
    integrations: Vec<Integration>,
    states: HashMap<String, IntegrationState>,
    /// A version 2 lamp is there.
    connected: bool,
    /// Its last STATE.
    state: Option<State>,
    calls: CallSettings,
    calls_available: bool,
    /// The run of the microphone monitor, while it runs.
    monitor: Option<u64>,
    /// The runs started so far.
    runs: u64,
    /// The monitor that runs hears a call.
    in_call: bool,
    call: Option<Call>,
    /// When the alert under way began.
    alert: Option<Instant>,
    last_alerts: HashMap<String, Instant>,
    self_test: bool,
    /// The self-test refused a command of the backend.
    refused_in_test: bool,
}

impl Control {
    pub fn new(calls: CallSettings) -> Self {
        Self {
            calls,
            ..Self::default()
        }
    }

    pub fn status(&self) -> ControlStatus {
        let imposed = self.call.filter(|call| call.imposed);
        ControlStatus {
            mode: self.mode.clone(),
            state: self.integration_state().cloned(),
            calls: CallStatus {
                available: self.calls_available,
                in_call: self.call.is_some(),
                imposed: imposed.is_some(),
                back: imposed.map(|call| call.back),
            },
            integrations: self.integrations.clone(),
        }
    }

    fn integration_state(&self) -> Option<&IntegrationState> {
        self.states.get(self.mode.as_ref()?)
    }

    /// The color of the state of the integration the lamp follows.
    pub fn integration_slot(&self) -> Option<u8> {
        self.integration_state().map(|state| state.slot)
    }

    /// While the call color shows: the color the lamp gets back after it.
    pub fn call_back(&self) -> Option<u8> {
        self.call.filter(|call| call.imposed).map(|call| call.back)
    }

    /// The touch mode and the color that end the connection sequence. A
    /// call does not outlive the connection: the monitor hears it again.
    pub fn on_connect(&self) -> (TouchMode, Option<u8>) {
        match self.mode {
            Some(_) => (TouchMode::PowerOnly, self.integration_slot()),
            None => (TouchMode::Normal, None),
        }
    }

    /// What the app sets on the lamp, sent again after a self-test that
    /// refused it.
    fn imposed(&self) -> SetState {
        let (touch_mode, color) = self.on_connect();
        SetState {
            touch_mode: Some(touch_mode),
            color: color.or(self.call_back().map(|_| self.calls.color)),
            ..SetState::default()
        }
    }

    /// The commands that hand the lamp back to its own behavior when the
    /// app quits, the heartbeat off first: normal touch after an
    /// integration, then no sensitivity correction, the touch cycle of
    /// version 1 and no custom color.
    pub fn quit(&self) -> Vec<Effect> {
        let mut effects = Vec::new();
        if !self.connected {
            return effects;
        }
        if self.mode.is_some() {
            effects.push(Effect::Heartbeat(false));
            effects.push(automatic(set_state(SetState {
                touch_mode: Some(TouchMode::Normal),
                ..SetState::default()
            })));
        }
        effects.push(automatic(Command::SetTouchSensitivity { percent: 0 }));
        effects.push(automatic(Command::SetTouchCycle {
            slots: DEFAULT_CYCLE.to_vec(),
        }));
        effects.extend((FIRST_CUSTOM_SLOT..SLOT_COUNT).map(|slot| {
            automatic(Command::SetColor {
                slot,
                rgb: LinearRgb::default(),
            })
        }));
        effects
    }

    pub fn step(&mut self, now: Instant, input: Input) -> Vec<Effect> {
        let mut effects = Vec::new();
        match input {
            Input::Connected => {
                self.connected = true;
                self.state = None;
                self.call = None;
                if self.mode.is_some() {
                    effects.push(Effect::Heartbeat(true));
                }
            }
            Input::Disconnected => {
                self.connected = false;
                self.state = None;
                self.call = None;
                self.alert = None;
                self.self_test = false;
                self.refused_in_test = false;
            }
            Input::State(state) => {
                // A touch shows as a change that the touch made: a reply
                // repeats the number and the source of the last change.
                let touched = self
                    .state
                    .as_ref()
                    .is_some_and(|previous| previous.state_seq != state.state_seq)
                    && state.source == ChangeSource::Touch;
                if touched && let Some(call) = &mut self.call {
                    call.imposed = false;
                }
                self.state = Some(state);
                // A call heard before the lamp told its state.
                self.start_call(&mut effects);
            }
            Input::Choose { on, slot } => {
                // A choice during a call keeps the hand until its end.
                if let Some(call) = &mut self.call {
                    call.imposed = false;
                }
                // The card of an integration turns the lamp on with its color.
                let color = match self.mode {
                    Some(_) => self.integration_slot(),
                    None => slot,
                };
                effects.push(Effect::Send {
                    command: set_state(SetState {
                        on: Some(on),
                        color: color.filter(|_| on),
                        ..SetState::default()
                    }),
                    user: true,
                });
            }
            Input::Follow(mode) => {
                let offered = mode
                    .as_ref()
                    .is_none_or(|id| self.integrations.iter().any(|known| &known.id == id));
                if mode != self.mode && offered {
                    self.follow(mode, &mut effects);
                }
            }
            Input::Integrations(integrations) => {
                self.states
                    .retain(|id, _| integrations.iter().any(|known| &known.id == id));
                self.integrations = integrations;
                // A plugin turned off, removed or unreadable leaves the lamp
                // to the user.
                let gone = self
                    .mode
                    .as_ref()
                    .is_some_and(|id| !self.integrations.iter().any(|known| &known.id == id));
                if gone {
                    self.follow(None, &mut effects);
                }
            }
            Input::Announce { id, state } => {
                let before = self.states.get(&id).map(|known| known.slot);
                let slot = state.as_ref().map(|state| state.slot);
                match state {
                    Some(state) => self.states.insert(id.clone(), state),
                    None => self.states.remove(&id),
                };
                // The color follows the state, the power never. Without a
                // state, the lamp keeps the color it has.
                let followed = self.connected && self.mode.as_ref() == Some(&id);
                if let Some(slot) = slot.filter(|slot| followed && before != Some(*slot)) {
                    effects.push(automatic(set_state(SetState {
                        color: Some(slot),
                        ..SetState::default()
                    })));
                }
            }
            Input::CallSettings(calls) => {
                let before = std::mem::replace(&mut self.calls, calls);
                if !calls.mic {
                    self.end_call(&mut effects);
                } else if calls.color != before.color && self.call_back().is_some() {
                    effects.push(automatic(set_state(SetState {
                        color: Some(calls.color),
                        ..SetState::default()
                    })));
                } else {
                    self.start_call(&mut effects);
                }
            }
            Input::CallsAvailable(available) => {
                self.calls_available = available;
                if !available {
                    self.end_call(&mut effects);
                }
            }
            // A run stopped since may still have spoken.
            Input::Call { run, in_call } if self.monitor == Some(run) => {
                self.in_call = in_call;
                if in_call {
                    self.start_call(&mut effects);
                } else {
                    self.end_call(&mut effects);
                }
            }
            Input::Call { .. } => {}
            Input::CallBack(back) => {
                if let (Some(call), Some(back)) = (&mut self.call, back) {
                    call.back = back;
                }
            }
            Input::Alert { plugin } => {
                let lit = self.state.as_ref().is_some_and(|state| state.on);
                let length =
                    Duration::from_millis(u64::from(ALERT_COUNT) * u64::from(ALERT_PERIOD_MS))
                        + ALERT_MARGIN;
                let running = self
                    .alert
                    .is_some_and(|since| now.saturating_duration_since(since) < length);
                let spaced = self
                    .last_alerts
                    .get(&plugin)
                    .is_none_or(|last| now.saturating_duration_since(*last) >= ALERT_SPACING);
                // No queue: a request that cannot play now is lost.
                if self.connected && lit && !self.self_test && !running && spaced {
                    self.alert = Some(now);
                    self.last_alerts.insert(plugin, now);
                    effects.push(automatic(Command::Alert {
                        count: ALERT_COUNT,
                        period_ms: ALERT_PERIOD_MS,
                    }));
                }
            }
            Input::AlertDone => self.alert = None,
            Input::Refused { command, error } => {
                let alert = Command::Alert {
                    count: ALERT_COUNT,
                    period_ms: ALERT_PERIOD_MS,
                };
                if command == alert.code() {
                    self.alert = None;
                }
                if error == ProtocolError::Busy && self.self_test {
                    self.refused_in_test = true;
                }
            }
            Input::SelfTest(running) => {
                self.self_test = running;
                if !running && std::mem::take(&mut self.refused_in_test) && self.connected {
                    effects.push(automatic(set_state(self.imposed())));
                }
                if running {
                    self.refused_in_test = false;
                }
            }
        }
        self.watch(&mut effects);
        effects
    }

    /// The microphone monitor runs only while it serves: the switch on, a
    /// system that tells calls, in manual mode, with a version 2 lamp. Each
    /// start is a new run, which first hears no call.
    fn watch(&mut self, effects: &mut Vec<Effect>) {
        let wanted =
            self.calls.mic && self.calls_available && self.mode.is_none() && self.connected;
        if wanted == self.monitor.is_some() {
            return;
        }
        self.in_call = false;
        self.monitor = wanted.then(|| {
            self.runs += 1;
            self.runs
        });
        effects.push(Effect::Monitor(self.monitor));
    }

    /// Changes the source the lamp follows. An integration takes the touch
    /// to power only and the color of its state, and the call is forgotten.
    /// Back to manual, the heartbeat stops before the touch goes back to
    /// normal, so that none follows, and a call under way starts afresh.
    fn follow(&mut self, mode: Option<String>, effects: &mut Vec<Effect>) {
        let was_manual = self.mode.is_none();
        self.mode = mode;
        if self.mode.is_some() {
            self.call = None;
            if self.connected {
                effects.push(automatic(set_state(SetState {
                    touch_mode: Some(TouchMode::PowerOnly),
                    color: self.integration_slot(),
                    ..SetState::default()
                })));
                if was_manual {
                    effects.push(Effect::Heartbeat(true));
                }
            }
        } else if self.connected {
            effects.push(Effect::Heartbeat(false));
            effects.push(automatic(set_state(SetState {
                touch_mode: Some(TouchMode::Normal),
                ..SetState::default()
            })));
            self.start_call(effects);
        }
    }

    /// A call begins, in manual mode with the microphone on: the lamp keeps
    /// its color for after, and takes the call color, lit or not.
    fn start_call(&mut self, effects: &mut Vec<Effect>) {
        let wanted = self.in_call
            && self.calls.mic
            && self.calls_available
            && self.mode.is_none()
            && self.call.is_none()
            && self.connected;
        let Some(state) = self.state.as_ref().filter(|_| wanted) else {
            return;
        };
        self.call = Some(Call {
            back: state.color,
            imposed: true,
        });
        effects.push(automatic(set_state(SetState {
            color: Some(self.calls.color),
            ..SetState::default()
        })));
    }

    /// The call ends: the lamp gets its color back, unless the user chose
    /// another during the call.
    fn end_call(&mut self, effects: &mut Vec<Effect>) {
        if let Some(back) = self
            .call
            .take()
            .filter(|call| call.imposed)
            .map(|call| call.back)
            && self.connected
        {
            effects.push(automatic(set_state(SetState {
                color: Some(back),
                ..SetState::default()
            })));
        }
    }
}

/// The control of the app, its status as the windows last heard it, and
/// the microphone monitor that it runs.
pub struct ControlState {
    core: Mutex<Control>,
    shown: Mutex<ControlStatus>,
    monitor: Mutex<Option<Monitor>>,
}

impl ControlState {
    pub fn new(calls: CallSettings) -> Self {
        let core = Control::new(calls);
        let shown = core.status();
        Self {
            core: Mutex::new(core),
            shown: Mutex::new(shown),
            monitor: Mutex::new(None),
        }
    }

    pub fn status(&self) -> ControlStatus {
        self.core
            .lock()
            .map(|core| core.status())
            .unwrap_or_default()
    }

    fn read<T: Default>(&self, read: impl FnOnce(&Control) -> T) -> T {
        self.core.lock().map(|core| read(&core)).unwrap_or_default()
    }

    pub fn on_connect(&self) -> (TouchMode, Option<u8>) {
        self.core
            .lock()
            .map(|core| core.on_connect())
            .unwrap_or((TouchMode::Normal, None))
    }

    pub fn call_back(&self) -> Option<u8> {
        self.read(Control::call_back)
    }

    pub fn integration_slot(&self) -> Option<u8> {
        self.read(Control::integration_slot)
    }
}

/// Feeds the core, carries out its decisions and tells the windows of a new
/// status. Fails when the lamp refuses to take a choice of the user.
pub fn input(app: &AppHandle, input: Input) -> Result<(), String> {
    let control = app.state::<ControlState>();
    let Ok((effects, status, stopped)) = control.core.lock().map(|mut core| {
        let effects = core.step(Instant::now(), input);
        // In the order of the core, under its lock: a monitor asked to stop
        // here never outlives the start of the next one.
        let stopped = effects
            .iter()
            .find_map(|effect| match effect {
                Effect::Monitor(run) => Some(*run),
                _ => None,
            })
            .and_then(|run| switch_monitor(app, &control, run));
        (effects, core.status(), stopped)
    }) else {
        return Ok(());
    };
    // Out of the lock, which its last call may be waiting for.
    if let Some(monitor) = stopped {
        monitor.join();
    }
    let result = carry_out(app, effects);
    let changed = control.shown.lock().is_ok_and(|mut shown| {
        let changed = *shown != status;
        if changed {
            shown.clone_from(&status);
        }
        changed
    });
    if changed {
        let _ = app.emit(EVENT_CONTROL, status);
    }
    result
}

/// Asks the monitor that runs to stop, and starts the run `run` if any.
/// Returns the monitor stopped, to wait for.
fn switch_monitor(app: &AppHandle, control: &ControlState, run: Option<u64>) -> Option<Monitor> {
    let mut current = control.monitor.lock().ok()?;
    let stopped = current.take();
    if let Some(monitor) = &stopped {
        monitor.ask();
    }
    *current = run.and_then(|run| Monitor::start(app.clone(), run));
    stopped
}

/// Sends the commands, the first refusal of a choice of the user kept.
/// Before the lamp worker runs, at the start of the app, there is nothing
/// to send to.
fn carry_out(app: &AppHandle, effects: Vec<Effect>) -> Result<(), String> {
    let Some(device) = app.try_state::<DeviceHandle>() else {
        return Ok(());
    };
    let mut result = Ok(());
    for effect in effects {
        match effect {
            Effect::Send {
                command,
                user: true,
            } => {
                if let Err(error) = device.send(command) {
                    result = result.and(Err(error));
                }
            }
            Effect::Send {
                command,
                user: false,
            } => {
                let _ = device.send_automatic(command);
            }
            Effect::Heartbeat(on) => device.heartbeat(on),
            // Done by `input`, under the lock of the core.
            Effect::Monitor(_) => {}
        }
    }
    result
}

/// Hands the lamp back to its own behavior before the app quits, and waits
/// a moment for its replies: "Sans l’app, la lampe reprend ses quatre
/// couleurs et le cycle d’origine." Both "Quit" go through here.
pub fn release(app: &AppHandle) {
    let effects = app.state::<ControlState>().read(|core| core.quit());
    if effects.is_empty() {
        return;
    }
    let _ = carry_out(app, effects);
    if let Some(device) = app.try_state::<DeviceHandle>() {
        device.flush(QUIT_WAIT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use disled_protocol::{BrightnessMode, SLOT_GREEN, SLOT_ORANGE, SLOT_RED, UsbLink};

    fn lamp(on: bool, color: u8, state_seq: u16, source: ChangeSource) -> State {
        State {
            state_seq,
            on,
            color,
            brightness_mode: BrightnessMode::Auto,
            manual_level: 50,
            applied_level: 40,
            link: UsbLink::Configured,
            source,
            touch_mode: TouchMode::Normal,
            rgb: LinearRgb::default(),
        }
    }

    /// A version 2 lamp, lit in green, the microphone followed, on a system
    /// that hears calls.
    fn control() -> (Control, Instant) {
        let mut control = Control::new(CallSettings {
            mic: true,
            color: SLOT_RED,
        });
        let now = Instant::now();
        control.step(now, Input::CallsAvailable(true));
        control.step(now, Input::Connected);
        control.step(
            now,
            Input::State(lamp(true, SLOT_GREEN, 1, ChangeSource::Boot)),
        );
        (control, now)
    }

    /// What the monitor that runs hears.
    fn heard(control: &Control, in_call: bool) -> Input {
        Input::Call {
            run: control.monitor.expect("the monitor runs"),
            in_call,
        }
    }

    fn color(slot: u8) -> Effect {
        automatic(set_state(SetState {
            color: Some(slot),
            ..SetState::default()
        }))
    }

    fn touch_mode(mode: TouchMode, slot: Option<u8>) -> Effect {
        automatic(set_state(SetState {
            touch_mode: Some(mode),
            color: slot,
            ..SetState::default()
        }))
    }

    /// The control above, following an integration in state "working",
    /// orange.
    fn following() -> (Control, Instant) {
        let (mut control, now) = control();
        control.step(
            now,
            Input::Integrations(vec![Integration {
                id: "claude-code".into(),
                name: "Claude Code".into(),
                icon: "terminal".into(),
            }]),
        );
        control.step(
            now,
            Input::Announce {
                id: "claude-code".into(),
                state: Some(IntegrationState {
                    label: "Claude travaille".into(),
                    slot: SLOT_ORANGE,
                    since: 1_793_000_000,
                }),
            },
        );
        control.step(now, Input::Follow(Some("claude-code".into())));
        (control, now)
    }

    #[test]
    fn a_call_shows_its_color_then_gives_the_lamp_back() {
        let (mut control, now) = control();
        assert_eq!(control.step(now, heard(&control, true)), [color(SLOT_RED)]);
        let calls = control.status().calls;
        assert!(calls.in_call && calls.imposed);
        assert_eq!(calls.back, Some(SLOT_GREEN));
        assert_eq!(
            control.step(now, heard(&control, false)),
            [color(SLOT_GREEN)]
        );
        assert_eq!(control.status().calls.back, None);
    }

    #[test]
    fn a_choice_during_a_call_keeps_the_hand() {
        let (mut control, now) = control();
        control.step(now, heard(&control, true));
        let effects = control.step(
            now,
            Input::Choose {
                on: true,
                slot: Some(SLOT_ORANGE),
            },
        );
        assert_eq!(
            effects,
            [Effect::Send {
                command: set_state(SetState {
                    on: Some(true),
                    color: Some(SLOT_ORANGE),
                    ..SetState::default()
                }),
                user: true,
            }]
        );
        assert!(!control.status().calls.imposed);
        assert_eq!(control.step(now, heard(&control, false)), []);

        // A touch, too: a change whose number moved, made by the touch.
        control.step(now, heard(&control, true));
        control.step(
            now,
            Input::State(lamp(true, SLOT_RED, 2, ChangeSource::Host)),
        );
        assert!(control.status().calls.imposed);
        control.step(
            now,
            Input::State(lamp(true, SLOT_GREEN, 3, ChangeSource::Touch)),
        );
        assert!(!control.status().calls.imposed);
        assert_eq!(control.step(now, heard(&control, false)), []);
    }

    #[test]
    fn a_reply_that_repeats_a_touch_is_no_touch() {
        let (mut control, now) = control();
        control.step(
            now,
            Input::State(lamp(true, SLOT_GREEN, 2, ChangeSource::Touch)),
        );
        control.step(now, heard(&control, true));
        control.step(
            now,
            Input::State(lamp(true, SLOT_RED, 2, ChangeSource::Touch)),
        );
        assert!(control.status().calls.imposed);
    }

    #[test]
    fn a_lamp_off_stays_off_during_a_call() {
        let (mut control, now) = control();
        control.step(
            now,
            Input::State(lamp(false, SLOT_GREEN, 2, ChangeSource::Touch)),
        );
        // The color alone: lit again during the call, the lamp shows it.
        assert_eq!(control.step(now, heard(&control, true)), [color(SLOT_RED)]);
    }

    #[test]
    fn the_microphone_turned_off_ends_the_call() {
        let (mut control, now) = control();
        control.step(now, heard(&control, true));
        let effects = control.step(
            now,
            Input::CallSettings(CallSettings {
                mic: false,
                color: SLOT_RED,
            }),
        );
        assert_eq!(effects, [color(SLOT_GREEN), Effect::Monitor(None)]);
        assert!(!control.status().calls.in_call);
        // And with it off, a call changes nothing.
        let late = Input::Call {
            run: 1,
            in_call: true,
        };
        assert_eq!(control.step(now, late), []);
    }

    #[test]
    fn a_new_call_color_shows_at_once() {
        let (mut control, now) = control();
        control.step(now, heard(&control, true));
        let effects = control.step(
            now,
            Input::CallSettings(CallSettings {
                mic: true,
                color: SLOT_ORANGE,
            }),
        );
        assert_eq!(effects, [color(SLOT_ORANGE)]);
    }

    #[test]
    fn a_deleted_color_comes_back_as_its_replacement() {
        let (mut control, now) = control();
        control.step(now, heard(&control, true));
        assert_eq!(control.call_back(), Some(SLOT_GREEN));
        control.step(now, Input::CallBack(Some(SLOT_ORANGE)));
        assert_eq!(
            control.step(now, heard(&control, false)),
            [color(SLOT_ORANGE)]
        );
    }

    #[test]
    fn an_integration_takes_the_color_and_the_call_is_forgotten() {
        let (mut control, now) = control();
        control.step(now, heard(&control, true));
        control.step(
            now,
            Input::Integrations(vec![Integration {
                id: "claude-code".into(),
                name: "Claude Code".into(),
                icon: "terminal".into(),
            }]),
        );
        control.step(
            now,
            Input::Announce {
                id: "claude-code".into(),
                state: Some(IntegrationState {
                    label: "Claude travaille".into(),
                    slot: SLOT_ORANGE,
                    since: 0,
                }),
            },
        );
        let effects = control.step(now, Input::Follow(Some("claude-code".into())));
        assert_eq!(
            effects,
            [
                touch_mode(TouchMode::PowerOnly, Some(SLOT_ORANGE)),
                Effect::Heartbeat(true),
                Effect::Monitor(None)
            ]
        );
        let status = control.status();
        assert_eq!(status.mode.as_deref(), Some("claude-code"));
        assert!(!status.calls.in_call);
        // The end of the forgotten call gives nothing back.
        let end = Input::Call {
            run: 1,
            in_call: false,
        };
        assert_eq!(control.step(now, end), []);
    }

    #[test]
    fn back_to_manual_stops_the_heartbeat_first() {
        let (mut control, now) = following();
        assert_eq!(
            control.step(now, Input::Follow(None)),
            [
                Effect::Heartbeat(false),
                touch_mode(TouchMode::Normal, None),
                Effect::Monitor(Some(2))
            ]
        );
        assert_eq!(control.status().mode, None);
    }

    #[test]
    fn back_to_manual_a_capture_starts_a_new_call() {
        let (mut control, now) = following();
        control.step(
            now,
            Input::State(lamp(true, SLOT_ORANGE, 2, ChangeSource::Host)),
        );
        control.step(now, Input::Follow(None));
        // The monitor, stopped with the integration, hears the capture again.
        assert_eq!(control.step(now, heard(&control, true)), [color(SLOT_RED)]);
        assert_eq!(control.status().calls.back, Some(SLOT_ORANGE));
    }

    #[test]
    fn the_monitor_runs_only_while_it_serves() {
        let now = Instant::now();
        let mut control = Control::new(CallSettings {
            mic: true,
            color: SLOT_RED,
        });
        assert_eq!(control.step(now, Input::CallsAvailable(true)), []);
        assert_eq!(
            control.step(now, Input::Connected),
            [Effect::Monitor(Some(1))]
        );
        let switch = |mic| {
            Input::CallSettings(CallSettings {
                mic,
                color: SLOT_RED,
            })
        };
        assert_eq!(control.step(now, switch(false)), [Effect::Monitor(None)]);
        assert_eq!(control.step(now, switch(true)), [Effect::Monitor(Some(2))]);
        // The same settings heard again change nothing.
        assert_eq!(control.step(now, switch(true)), []);
        assert_eq!(
            control.step(now, Input::Disconnected),
            [Effect::Monitor(None)]
        );
        assert_eq!(
            control.step(now, Input::Connected),
            [Effect::Monitor(Some(3))]
        );
        assert_eq!(
            control.step(now, Input::CallsAvailable(false)),
            [Effect::Monitor(None)]
        );
        // The switch off from the start: no monitor at all.
        let mut control = Control::new(CallSettings::default());
        control.step(now, Input::CallsAvailable(true));
        assert_eq!(control.step(now, Input::Connected), []);
    }

    #[test]
    fn a_call_heard_by_a_stopped_run_changes_nothing() {
        let (mut control, now) = control();
        let switch = |mic| {
            Input::CallSettings(CallSettings {
                mic,
                color: SLOT_RED,
            })
        };
        control.step(now, switch(false));
        control.step(now, switch(true));
        let late = Input::Call {
            run: 1,
            in_call: true,
        };
        assert_eq!(control.step(now, late), []);
        assert!(!control.status().calls.in_call);
        assert_eq!(control.step(now, heard(&control, true)), [color(SLOT_RED)]);
    }

    #[test]
    fn an_unplugged_lamp_forgets_the_call() {
        let (mut control, now) = control();
        control.step(now, heard(&control, true));
        assert_eq!(
            control.step(now, Input::Disconnected),
            [Effect::Monitor(None)]
        );
        assert!(!control.status().calls.in_call);
        // Back with its color of start-up, which the call gives back.
        control.step(now, Input::Connected);
        control.step(
            now,
            Input::State(lamp(true, SLOT_ORANGE, 1, ChangeSource::Boot)),
        );
        assert_eq!(control.step(now, heard(&control, true)), [color(SLOT_RED)]);
        assert_eq!(control.status().calls.back, Some(SLOT_ORANGE));
    }

    #[test]
    fn a_new_state_changes_the_color_never_the_power() {
        let (mut control, now) = following();
        let mut state = IntegrationState {
            label: "Claude attend votre réponse".into(),
            slot: SLOT_RED,
            since: 1_793_000_100,
        };
        let announce = |state: &IntegrationState| Input::Announce {
            id: "claude-code".into(),
            state: Some(state.clone()),
        };
        assert_eq!(control.step(now, announce(&state)), [color(SLOT_RED)]);
        // The same color under another label sends nothing.
        state.label = "Claude a fini".into();
        assert_eq!(control.step(now, announce(&state)), []);
        assert_eq!(
            control.status().state.map(|state| state.label).as_deref(),
            Some("Claude a fini")
        );
        // Without a state, the lamp keeps its color.
        let none = Input::Announce {
            id: "claude-code".into(),
            state: None,
        };
        assert_eq!(control.step(now, none), []);
    }

    #[test]
    fn the_card_turns_the_lamp_on_with_the_color_of_the_state() {
        let (mut control, now) = following();
        let effects = control.step(
            now,
            Input::Choose {
                on: true,
                slot: None,
            },
        );
        assert_eq!(
            effects,
            [Effect::Send {
                command: set_state(SetState {
                    on: Some(true),
                    color: Some(SLOT_ORANGE),
                    ..SetState::default()
                }),
                user: true,
            }]
        );
    }

    #[test]
    fn the_lamp_goes_back_to_manual_when_its_integration_leaves() {
        let (mut control, now) = following();
        assert_eq!(
            control.step(now, Input::Integrations(Vec::new())),
            [
                Effect::Heartbeat(false),
                touch_mode(TouchMode::Normal, None),
                Effect::Monitor(Some(2))
            ]
        );
        assert_eq!(control.status().mode, None);
        // An integration that is not on offer cannot be followed.
        assert_eq!(
            control.step(now, Input::Follow(Some("claude-code".into()))),
            []
        );
    }

    #[test]
    fn a_reconnected_lamp_follows_the_integration_again() {
        let (mut control, now) = following();
        control.step(now, Input::Disconnected);
        assert_eq!(
            control.on_connect(),
            (TouchMode::PowerOnly, Some(SLOT_ORANGE))
        );
        assert_eq!(
            control.step(now, Input::Connected),
            [Effect::Heartbeat(true)]
        );
        let (control, _) = self::control();
        assert_eq!(control.on_connect(), (TouchMode::Normal, None));
    }

    #[test]
    fn an_alert_needs_a_lit_lamp_and_no_self_test() {
        let alert = || Input::Alert {
            plugin: "claude-code".into(),
        };
        let sent = automatic(Command::Alert {
            count: 3,
            period_ms: ALERT_PERIOD_MS,
        });
        let (mut control, now) = control();
        control.step(
            now,
            Input::State(lamp(false, SLOT_GREEN, 2, ChangeSource::Touch)),
        );
        assert_eq!(control.step(now, alert()), []);

        let (mut control, now) = self::control();
        control.step(now, Input::SelfTest(true));
        assert_eq!(control.step(now, alert()), []);
        control.step(now, Input::SelfTest(false));
        assert_eq!(control.step(now, alert()), [sent]);

        let mut control = Control::default();
        assert_eq!(control.step(now, alert()), [], "no lamp");
    }

    #[test]
    fn one_alert_at_a_time_and_one_every_10_s_for_each_plugin() {
        let alert = |plugin: &str| Input::Alert {
            plugin: plugin.into(),
        };
        let (mut control, now) = control();
        assert_eq!(control.step(now, alert("a")).len(), 1);
        // During the alert, no other one, nor a restart of its breath.
        assert_eq!(control.step(now, alert("b")), []);
        control.step(now, Input::AlertDone);
        assert_eq!(control.step(now, alert("b")).len(), 1);
        control.step(now, Input::AlertDone);
        let later = now + Duration::from_secs(9);
        assert_eq!(control.step(later, alert("a")), []);
        let later = now + Duration::from_secs(10);
        assert_eq!(control.step(later, alert("a")).len(), 1);
        // An alert whose end never came is over after its breaths.
        let after = later + Duration::from_millis(5_899);
        assert_eq!(control.step(after, alert("c")), []);
        let after = later + Duration::from_millis(5_900);
        assert_eq!(control.step(after, alert("c")).len(), 1);
    }

    #[test]
    fn a_refused_alert_is_over() {
        let (mut control, now) = control();
        control.step(now, Input::Alert { plugin: "a".into() });
        control.step(
            now,
            Input::Refused {
                command: Command::Alert {
                    count: 3,
                    period_ms: ALERT_PERIOD_MS,
                }
                .code(),
                error: ProtocolError::LampOff,
            },
        );
        let effects = control.step(now, Input::Alert { plugin: "b".into() });
        assert_eq!(effects.len(), 1);
    }

    #[test]
    fn a_self_test_that_refused_the_state_gets_it_again() {
        let (mut control, now) = following();
        control.step(now, Input::SelfTest(true));
        control.step(
            now,
            Input::Refused {
                command: set_state(SetState::default()).code(),
                error: ProtocolError::Busy,
            },
        );
        assert_eq!(
            control.step(now, Input::SelfTest(false)),
            [touch_mode(TouchMode::PowerOnly, Some(SLOT_ORANGE))]
        );
        // Nothing was refused this time.
        control.step(now, Input::SelfTest(true));
        assert_eq!(control.step(now, Input::SelfTest(false)), []);
    }

    #[test]
    fn quitting_hands_the_lamp_back_to_its_own_behavior() {
        let black = |slot| {
            automatic(Command::SetColor {
                slot,
                rgb: LinearRgb::default(),
            })
        };
        let (control, _) = following();
        let effects = control.quit();
        assert_eq!(effects.len(), 16, "the heartbeat, then 15 reports");
        assert_eq!(effects[0], Effect::Heartbeat(false));
        assert_eq!(effects[1], touch_mode(TouchMode::Normal, None));
        assert_eq!(
            effects[2],
            automatic(Command::SetTouchSensitivity { percent: 0 })
        );
        assert_eq!(
            effects[3],
            automatic(Command::SetTouchCycle {
                slots: vec![0, 1, 2]
            })
        );
        assert_eq!(effects[4..], (4..16).map(black).collect::<Vec<_>>());

        // In manual mode, the touch is normal already.
        let (control, _) = self::control();
        let effects = control.quit();
        assert_eq!(effects.len(), 14);
        assert_eq!(
            effects[0],
            automatic(Command::SetTouchSensitivity { percent: 0 })
        );
        // Without a version 2 lamp, nothing to hand back.
        assert_eq!(Control::default().quit(), []);
    }
}
