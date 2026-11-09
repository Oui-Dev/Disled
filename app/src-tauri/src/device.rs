// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! Connection to the lamp on a worker thread: discovery and reconnection,
//! command writes, message reads, and the heartbeat of the integrations
//! mode. Results reach the frontend as events, and the control of the app.
//!
//! Until the INFO of a lamp says it speaks protocol 2, only GET_INFO goes to
//! it and only its INFO comes back: a version 1 lamp gets the lock of the
//! app, and nothing that it sends shows as an error.
//!
//! The replies to the commands that the backend sends on its own, such as
//! the connection sequence, the heartbeat and those of the control, stay in
//! the backend: a window would show their refusal, during a self-test for
//! instance, as an error of the user.

use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use hidapi::{HidApi, HidDevice};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use disled_protocol::{
    self as protocol, ChangeSource, Command, DecodeError, Frame, Info, Message, PAYLOAD_SIZE,
    PROTOCOL_MAJOR, REPORT_ID_COMMAND, REPORT_ID_MESSAGE, SetState, State, TouchMode,
    USB_PRODUCT_ID_DEBUG, USB_VENDOR_ID,
};

use crate::colors::{ColorsState, Palette, PreviewState};
use crate::control::{self, ControlState, Input};
use crate::i18n::LocaleState;
use crate::settings::{Settings, SettingsState};
use crate::tray::{self, Look};
use crate::windows;

const SCAN_INTERVAL: Duration = Duration::from_millis(1000);
const READ_TIMEOUT_MS: i32 = 20;
/// In the integrations mode, a command at least this often keeps the touch
/// in power only: the lamp goes back to normal touch after 10 s of silence.
const HEARTBEAT: Duration = Duration::from_secs(3);

const EVENT_CONNECTION: &str = "device-connection";
const EVENT_MESSAGE: &str = "device-message";
const EVENT_PROTOCOL_ERROR: &str = "device-protocol-error";

#[derive(Debug, Clone, Default, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct Connection {
    pub connected: bool,
    pub debug_build: bool,
    pub serial: Option<String>,
}

/// What the protocol of a lamp lets the app do, read in its INFO.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Generation {
    /// Protocol 1, firmware 1.0.0 and before: the app only reads its INFO.
    V1,
    /// Protocol 2, which this app drives.
    V2,
    /// A later protocol, which this app does not know.
    Newer,
}

impl Generation {
    pub fn of(info: &Info) -> Self {
        match info.protocol[0] {
            PROTOCOL_MAJOR => Generation::V2,
            major if major < PROTOCOL_MAJOR => Generation::V1,
            _ => Generation::Newer,
        }
    }
}

/// What becomes of a report from the lamp.
#[derive(Debug, PartialEq)]
enum Incoming {
    /// A message for the windows.
    Forward(Frame),
    /// A report that a version 2 lamp should not send, for the windows to show.
    Unreadable(String),
    /// Dropped without a word.
    Ignore,
}

/// The lamp of one connection, as far as its INFO tells.
#[derive(Debug, Default)]
struct Session {
    generation: Option<Generation>,
}

impl Session {
    /// Until a version 2 lamp sent its INFO, only INFO passes: anything else
    /// from that lamp, readable or not, such as the 9-byte STATE of a version
    /// 1 lamp, drops without an error.
    fn receive(&mut self, report: &[u8]) -> Incoming {
        match protocol::decode_message(report) {
            Ok(frame) => {
                if let Message::Info(info) = &frame.message {
                    self.generation = Some(Generation::of(info));
                    Incoming::Forward(frame)
                } else if self.drives() {
                    Incoming::Forward(frame)
                } else {
                    Incoming::Ignore
                }
            }
            // A newer lamp may send message types this app does not know,
            // which the protocol says to ignore.
            Err(DecodeError::UnknownType(_)) => Incoming::Ignore,
            Err(error) if self.drives() => Incoming::Unreadable(error.to_string()),
            Err(_) => Incoming::Ignore,
        }
    }

    /// Only GET_INFO goes to a lamp until it proved to speak protocol 2. The
    /// others drop without an error, as the GET_STATE that a window sends
    /// when it opens.
    fn allows(&self, command: &Command) -> bool {
        matches!(command, Command::GetInfo) || self.drives()
    }

    fn drives(&self) -> bool {
        self.generation == Some(Generation::V2)
    }
}

/// The commands that follow the first INFO of a connection. A lamp that
/// does not speak protocol 2 gets none. A version 2 lamp keeps nothing of
/// the app across a power cycle, so it gets everything again, in the order
/// of the protocol: the state first, then the corrections, the palette, and
/// the state the app sets, with the touch mode and the color that the
/// control imposes.
pub fn commands_after_info(
    info: &Info,
    settings: &Settings,
    palette: &Palette,
    (touch_mode, color): (TouchMode, Option<u8>),
) -> Vec<Command> {
    if Generation::of(info) != Generation::V2 {
        return Vec::new();
    }
    let mut commands = vec![
        Command::GetState,
        Command::SetAutoBias {
            bias: settings.auto_bias,
        },
        Command::SetTouchSensitivity {
            percent: settings.touch_sensitivity,
        },
    ];
    commands.extend(palette.commands());
    commands.push(Command::SetState(SetState {
        color,
        brightness_mode: Some(settings.brightness_mode),
        manual_level: Some(settings.manual_level),
        touch_mode: Some(touch_mode),
        ..SetState::default()
    }));
    commands
}

/// Whether the heartbeat is due: on, and no command for `HEARTBEAT`, since
/// every command counts as one.
pub fn heartbeat_due(on: bool, last_write: Instant, now: Instant) -> bool {
    on && now.saturating_duration_since(last_write) >= HEARTBEAT
}

/// The heartbeat: a SET_STATE that only sets the touch to power only, which
/// also brings it back after a fallback.
fn heartbeat() -> Command {
    Command::SetState(SetState {
        touch_mode: Some(TouchMode::PowerOnly),
        ..SetState::default()
    })
}

/// Whether a message of the lamp goes to the windows. The acknowledgments,
/// refusals and alert ends that answer a command of the backend stay with
/// it. A STATE always goes, but the reply to a heartbeat only when it
/// changed something, after the lamp fell back to normal touch.
fn shown(message: &Message, automatic: bool, heartbeat: bool, previous: Option<u16>) -> bool {
    match message {
        Message::State(state) => !heartbeat || previous != Some(state.state_seq),
        Message::Ack { .. } | Message::Error { .. } | Message::AlertDone { .. } => !automatic,
        _ => true,
    }
}

/// What the worker gets from the rest of the app, in order.
enum Request {
    /// A command, `automatic` when the backend sends it on its own.
    Command { command: Command, automatic: bool },
    /// The heartbeat on or off. Through the same queue as the commands, so
    /// that none follows the command that turns the touch back to normal.
    Heartbeat(bool),
    /// Told once the lamp answered the commands queued before.
    Flush(mpsc::Sender<()>),
}

/// The tray icon for an INFO: a lamp the app cannot drive shows at once,
/// and a version 2 lamp waits for its STATE.
fn look_of_info(info: &Info) -> Option<Look> {
    match Generation::of(info) {
        Generation::V1 => Some(Look::NeedsUpdate),
        Generation::Newer => Some(Look::Newer),
        Generation::V2 => None,
    }
}

pub struct DeviceHandle {
    commands: mpsc::Sender<Request>,
    connection: Arc<Mutex<Connection>>,
    info: Arc<Mutex<Option<Info>>>,
    state: Arc<Mutex<Option<State>>>,
}

impl DeviceHandle {
    pub fn spawn(app: AppHandle) -> Self {
        let (commands, receiver) = mpsc::channel();
        let connection = Arc::new(Mutex::new(Connection::default()));
        let info = Arc::new(Mutex::new(None));
        let state = Arc::new(Mutex::new(None));
        let (shared, shared_info, shared_state) = (
            Arc::clone(&connection),
            Arc::clone(&info),
            Arc::clone(&state),
        );
        thread::Builder::new()
            .name("disled-usb".into())
            .spawn(move || {
                Worker {
                    app,
                    receiver,
                    connection: shared,
                    info: shared_info,
                    state: shared_state,
                    seq: 0,
                    session: Session::default(),
                    automatic: [false; 256],
                    last_seq: None,
                    last_write: Instant::now(),
                    heartbeat: false,
                    heartbeat_seq: None,
                    flush: None,
                }
                .run()
            })
            .expect("spawn the USB worker thread");
        Self {
            commands,
            connection,
            info,
            state,
        }
    }

    /// The INFO message of the connected lamp, once it answered.
    pub fn info(&self) -> Option<Info> {
        self.info.lock().ok().and_then(|info| info.clone())
    }

    /// The last STATE of a version 2 lamp, which the app drives.
    pub fn state(&self) -> Option<State> {
        self.state.lock().ok().and_then(|state| state.clone())
    }

    /// Queues a command of a window, whose replies the windows get. A lamp
    /// that does not speak protocol 2 gets only GET_INFO: the worker drops
    /// the others, and the caller hears of no error, since the windows show
    /// the lock instead.
    pub fn send(&self, command: Command) -> Result<(), String> {
        self.queue(command, false)
    }

    /// Queues a command that the backend sends on its own: its refusal goes
    /// to the control, never to the windows.
    pub fn send_automatic(&self, command: Command) -> Result<(), String> {
        self.queue(command, true)
    }

    fn queue(&self, command: Command, automatic: bool) -> Result<(), String> {
        if !self.connection().connected {
            return Err("no Disled lamp connected".into());
        }
        self.commands
            .send(Request::Command { command, automatic })
            .map_err(|_| "the USB worker stopped".to_string())
    }

    /// Turns the heartbeat of the integrations mode on or off.
    pub fn heartbeat(&self, on: bool) {
        let _ = self.commands.send(Request::Heartbeat(on));
    }

    /// Waits until the lamp answered the commands queued so far, or
    /// `timeout`. Returns at once without a lamp.
    pub fn flush(&self, timeout: Duration) {
        let (done, answered) = mpsc::channel();
        if self.commands.send(Request::Flush(done)).is_ok() {
            let _ = answered.recv_timeout(timeout);
        }
    }

    pub fn connection(&self) -> Connection {
        self.connection
            .lock()
            .map(|c| c.clone())
            .unwrap_or_default()
    }
}

struct Worker {
    app: AppHandle,
    receiver: mpsc::Receiver<Request>,
    connection: Arc<Mutex<Connection>>,
    info: Arc<Mutex<Option<Info>>>,
    state: Arc<Mutex<Option<State>>>,
    seq: u8,
    session: Session,
    /// For each sequence number, whether the backend sent that command on
    /// its own.
    automatic: [bool; 256],
    /// The sequence number of the last command written.
    last_seq: Option<u8>,
    last_write: Instant,
    heartbeat: bool,
    /// The sequence number of the last heartbeat.
    heartbeat_seq: Option<u8>,
    /// The sequence number whose reply ends a flush, and whom to tell.
    flush: Option<(u8, mpsc::Sender<()>)>,
}

/// Why the connected loop ended.
enum Exit {
    Disconnected,
    AppClosed,
}

impl Worker {
    fn run(mut self) {
        let mut api = loop {
            match HidApi::new() {
                Ok(api) => break api,
                Err(_) => thread::sleep(SCAN_INTERVAL),
            }
        };
        loop {
            match self.open(&mut api) {
                Some(device) => {
                    self.session = Session::default();
                    self.automatic = [false; 256];
                    self.last_seq = None;
                    self.heartbeat = false;
                    self.heartbeat_seq = None;
                    if let Exit::AppClosed = self.serve(&device) {
                        return;
                    }
                    // A flush under way waits no more.
                    self.flush = None;
                    self.set_info(None);
                    self.set_state(None);
                    self.app.state::<PreviewState>().forget();
                    let _ = control::input(&self.app, Input::Disconnected);
                    self.set_connection(Connection::default());
                    tray::show(&self.app, Look::NoLamp);
                }
                None => {
                    // Requests made while no lamp is connected are dropped,
                    // and a flush with them.
                    loop {
                        match self.receiver.try_recv() {
                            Ok(_) => continue,
                            Err(mpsc::TryRecvError::Empty) => break,
                            Err(mpsc::TryRecvError::Disconnected) => return,
                        }
                    }
                    thread::sleep(SCAN_INTERVAL);
                }
            }
        }
    }

    fn open(&mut self, api: &mut HidApi) -> Option<HidDevice> {
        // Only the lamp's vendor. hidapi then skips the other devices before
        // reading their strings, so a slow one cannot keep the lamp out of
        // sight.
        api.reset_devices().ok()?;
        api.add_devices(USB_VENDOR_ID, 0).ok()?;
        let info = api
            .device_list()
            .find(|d| protocol::is_lamp_interface(d.vendor_id(), d.product_id(), d.usage_page()))?;
        let device = info.open_device(api).ok()?;
        self.set_connection(Connection {
            connected: true,
            debug_build: info.product_id() == USB_PRODUCT_ID_DEBUG,
            serial: info.serial_number().map(str::to_owned),
        });
        Some(device)
    }

    fn set_connection(&self, connection: Connection) {
        if let Ok(mut shared) = self.connection.lock() {
            *shared = connection.clone();
        }
        let _ = self.app.emit(EVENT_CONNECTION, connection);
    }

    fn set_info(&self, info: Option<Info>) {
        if let Ok(mut shared) = self.info.lock() {
            *shared = info;
        }
    }

    fn set_state(&self, state: Option<State>) {
        if let Ok(mut shared) = self.state.lock() {
            *shared = state;
        }
    }

    fn next_seq(&mut self) -> u8 {
        // 0 marks unsolicited messages, so a request never uses it.
        self.seq = if self.seq == u8::MAX { 1 } else { self.seq + 1 };
        self.seq
    }

    /// Writes a command the session allows, and drops the others. False if
    /// the lamp is gone.
    fn write(&mut self, device: &HidDevice, command: &Command, automatic: bool) -> bool {
        if !self.session.allows(command) {
            return true;
        }
        let seq = self.next_seq();
        self.automatic[usize::from(seq)] = automatic;
        self.last_seq = Some(seq);
        self.last_write = Instant::now();
        let mut report = [0u8; PAYLOAD_SIZE + 1];
        report[0] = REPORT_ID_COMMAND;
        report[1..].copy_from_slice(&command.encode(seq));
        let written = device.write(&report).is_ok();
        // The lamp refuses the commands of the control during a self-test,
        // which the control hears of.
        if written && *command == Command::SelfTest {
            let _ = control::input(&self.app, Input::SelfTest(true));
        }
        written
    }

    /// The reply to the last command written ends a flush. Without a lamp
    /// that the app drives, no reply comes.
    fn begin_flush(&mut self, done: mpsc::Sender<()>) {
        match self.last_seq {
            Some(seq) if self.session.drives() => self.flush = Some((seq, done)),
            _ => {
                let _ = done.send(());
            }
        }
    }

    /// Hands the control what concerns it, and tells whether the windows
    /// get the message.
    fn route(&mut self, frame: &Frame, previous: Option<u16>) -> bool {
        let automatic = frame.seq != 0 && self.automatic[usize::from(frame.seq)];
        let heartbeat = frame.seq != 0 && self.heartbeat_seq == Some(frame.seq);
        if let Some((seq, done)) = self.flush.take() {
            if seq == frame.seq {
                let _ = done.send(());
            } else {
                self.flush = Some((seq, done));
            }
        }
        let input = match &frame.message {
            Message::State(state) => Some(Input::State(state.clone())),
            Message::AlertDone { .. } => Some(Input::AlertDone),
            Message::SelfTestResult(_) => Some(Input::SelfTest(false)),
            Message::Error { command, .. } if *command == Command::SelfTest.code() => {
                Some(Input::SelfTest(false))
            }
            Message::Error { command, error } if automatic => Some(Input::Refused {
                command: *command,
                error: *error,
            }),
            _ => None,
        };
        if let Some(input) = input {
            let _ = control::input(&self.app, input);
        }
        shown(&frame.message, automatic, heartbeat, previous)
    }

    /// Sends the state of the app after the first INFO of a connection, and
    /// shows the lamp in the tray. False if the lamp is gone.
    fn follow(&mut self, device: &HidDevice, message: &Message, first_info: bool) -> bool {
        match message {
            Message::Info(info) => {
                self.set_info(Some(info.clone()));
                if let Some(look) = look_of_info(info) {
                    tray::show(&self.app, look);
                }
                // Only at the first INFO of a connection: every window that
                // opens asks for one, and the settings sent again then would
                // be refused during a self-test.
                if first_info {
                    let settings = self.app.state::<SettingsState>().get();
                    let palette = self.app.state::<ColorsState>().get();
                    let imposed = self.app.state::<ControlState>().on_connect();
                    let commands = commands_after_info(info, &settings, &palette, imposed);
                    if commands.is_empty() {
                        return true;
                    }
                    for command in commands {
                        if !self.write(device, &command, true) {
                            return false;
                        }
                    }
                    let _ = control::input(&self.app, Input::Connected);
                }
            }
            Message::State(state) => {
                self.set_state(Some(state.clone()));
                // A touch took the lamp off the preview slot: the user chose.
                if state.source == ChangeSource::Touch {
                    self.app.state::<PreviewState>().forget();
                }
                let palette = self.app.state::<ColorsState>().get();
                let locale = self.app.state::<LocaleState>().get();
                if let Some(look) = tray::look_of(state, &palette, locale) {
                    tray::show(&self.app, look);
                }
            }
            _ => {}
        }
        true
    }

    fn serve(&mut self, device: &HidDevice) -> Exit {
        // Learn who is there; the rest waits for its INFO.
        if !self.write(device, &Command::GetInfo, true) {
            return Exit::Disconnected;
        }
        let mut buffer = [0u8; PAYLOAD_SIZE + 1];
        loop {
            loop {
                match self.receiver.try_recv() {
                    Ok(Request::Command { command, automatic }) => {
                        if !self.write(device, &command, automatic) {
                            return Exit::Disconnected;
                        }
                    }
                    Ok(Request::Heartbeat(on)) => self.heartbeat = on,
                    Ok(Request::Flush(done)) => self.begin_flush(done),
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => return Exit::AppClosed,
                }
            }
            if self.session.drives()
                && heartbeat_due(self.heartbeat, self.last_write, Instant::now())
            {
                if !self.write(device, &heartbeat(), true) {
                    return Exit::Disconnected;
                }
                self.heartbeat_seq = self.last_seq;
            }
            match device.read_timeout(&mut buffer, READ_TIMEOUT_MS) {
                Ok(0) => {}
                Ok(length) if buffer[0] == REPORT_ID_MESSAGE => {
                    let first_info = self.session.generation.is_none();
                    match self.session.receive(&buffer[1..length]) {
                        Incoming::Forward(frame) => {
                            let previous = self
                                .state
                                .lock()
                                .ok()
                                .and_then(|state| state.as_ref().map(|state| state.state_seq));
                            if !self.follow(device, &frame.message, first_info) {
                                return Exit::Disconnected;
                            }
                            if !self.route(&frame, previous) {
                                continue;
                            }
                            // Up to 50 samples a second, which only the
                            // Diagnostics window follows.
                            let _ = if let Message::Telemetry(_) = frame.message {
                                self.app.emit_to(windows::DIAGNOSTICS, EVENT_MESSAGE, frame)
                            } else {
                                self.app.emit(EVENT_MESSAGE, frame)
                            };
                        }
                        Incoming::Unreadable(detail) => {
                            let _ = self.app.emit(EVENT_PROTOCOL_ERROR, detail);
                        }
                        Incoming::Ignore => {}
                    }
                }
                Ok(_) => {} // another report ID, not ours
                Err(_) => return Exit::Disconnected,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::BUILT_IN;
    use disled_protocol::{BrightnessMode, LinearRgb, SLOT_COUNT};
    use serde_json::{Value, json};

    /// The end of the connection sequence in manual mode.
    const MANUAL: (TouchMode, Option<u8>) = (TouchMode::Normal, None);

    fn vectors() -> Value {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../protocol/vectors/v2.json"
        );
        serde_json::from_str(&std::fs::read_to_string(path).expect("v2.json")).expect("JSON")
    }

    /// The payload of a message vector of v2.json.
    fn report(name: &str) -> Vec<u8> {
        let doc = vectors();
        let vector = doc["messages"]
            .as_array()
            .expect("messages")
            .iter()
            .find(|v| v["name"] == name)
            .unwrap_or_else(|| panic!("vector {name}"));
        let mut payload = vec![0u8; PAYLOAD_SIZE];
        for (i, pair) in vector["bytes"]
            .as_str()
            .expect("bytes")
            .split_whitespace()
            .enumerate()
        {
            payload[i] = u8::from_str_radix(pair, 16).expect("hex");
        }
        payload
    }

    fn info_of(name: &str) -> Info {
        match protocol::decode_message(&report(name))
            .expect("INFO")
            .message
        {
            Message::Info(info) => info,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_version_1_lamp_gets_nothing_after_its_info() {
        let info = info_of("info_of_a_version_1_lamp");
        assert_eq!(Generation::of(&info), Generation::V1);
        assert!(
            commands_after_info(&info, &Settings::default(), &Palette::default(), MANUAL)
                .is_empty()
        );
    }

    #[test]
    fn a_version_1_lamp_shows_nothing_but_its_info() {
        for state_first in [false, true] {
            let mut session = Session::default();
            let mut received = Vec::new();
            let mut reports = vec![report("info_of_a_version_1_lamp")];
            // The STATE it sends on its own at every change, 9 bytes long.
            let state = report("state_of_a_version_1_lamp");
            if state_first {
                reports.insert(0, state);
            } else {
                reports.push(state);
            }
            for report in &reports {
                received.push(session.receive(report));
            }
            let forwarded: Vec<&Incoming> = received
                .iter()
                .filter(|incoming| !matches!(incoming, Incoming::Ignore))
                .collect();
            assert_eq!(forwarded.len(), 1, "state first: {state_first}");
            assert!(matches!(
                forwarded[0],
                Incoming::Forward(Frame {
                    message: Message::Info(_),
                    ..
                })
            ));
            // A window that opens asks for the state: it never leaves.
            assert!(!session.allows(&Command::GetState));
            assert!(session.allows(&Command::GetInfo));
        }
    }

    #[test]
    fn only_get_info_leaves_before_the_info() {
        let session = Session::default();
        assert!(session.allows(&Command::GetInfo));
        assert!(!session.allows(&Command::GetState));
        assert!(!session.allows(&Command::Reset));
        // Nor does an unreadable report show before it.
        let mut session = Session::default();
        assert_eq!(
            session.receive(&report("state_of_a_version_1_lamp")),
            Incoming::Ignore
        );
    }

    #[test]
    fn a_version_2_lamp_is_driven() {
        let mut session = Session::default();
        assert!(matches!(
            session.receive(&report("info")),
            Incoming::Forward(_)
        ));
        assert!(session.allows(&Command::GetState));
        assert!(matches!(
            session.receive(&report("state_after_touch")),
            Incoming::Forward(_)
        ));
        // A short body from a version 2 lamp is a fault worth showing.
        assert!(matches!(
            session.receive(&report("state_of_a_version_1_lamp")),
            Incoming::Unreadable(_)
        ));
    }

    #[test]
    fn a_newer_lamp_is_locked_too() {
        let mut info = info_of("info");
        info.protocol = [3, 0];
        assert_eq!(Generation::of(&info), Generation::Newer);
        assert!(
            commands_after_info(&info, &Settings::default(), &Palette::default(), MANUAL)
                .is_empty()
        );
        assert_eq!(look_of_info(&info), Some(Look::Newer));
    }

    #[test]
    fn a_version_2_lamp_gets_the_whole_state_of_the_app() {
        let info = info_of("info");
        let settings = Settings {
            brightness_mode: BrightnessMode::Manual,
            manual_level: 30,
            auto_bias: -20,
            touch_sensitivity: 15,
            ..Settings::default()
        };
        let palette = Palette::from_value(&json!({
            "custom": [
                { "slot": 4, "name": "Musique", "hex": "#9B59F6" },
                { "slot": 6, "name": "Focus", "hex": "#3B82F6" }
            ],
            "cycle": [0, 4, 1, 6]
        }));
        let commands = commands_after_info(&info, &settings, &palette, MANUAL);
        // With GET_INFO, 18 reports: the most the protocol foresees.
        assert_eq!(commands.len(), 17);
        assert_eq!(commands[0], Command::GetState);
        assert_eq!(commands[1], Command::SetAutoBias { bias: -20 });
        assert_eq!(commands[2], Command::SetTouchSensitivity { percent: 15 });
        let colors: Vec<u8> = commands[3..5]
            .iter()
            .map(|command| match command {
                Command::SetColor { slot, rgb } if *rgb != LinearRgb::default() => *slot,
                other => panic!("unexpected {other:?}"),
            })
            .collect();
        assert_eq!(colors, [4, 6]);
        assert_eq!(
            commands[5],
            Command::SetTouchCycle {
                slots: vec![0, 4, 1, 6]
            }
        );
        let black = (BUILT_IN..SLOT_COUNT).filter(|slot| ![4, 6].contains(slot));
        for (command, slot) in commands[6..16].iter().zip(black) {
            assert_eq!(
                *command,
                Command::SetColor {
                    slot,
                    rgb: LinearRgb::default()
                }
            );
        }
        assert_eq!(
            commands[16],
            Command::SetState(SetState {
                brightness_mode: Some(BrightnessMode::Manual),
                manual_level: Some(30),
                touch_mode: Some(TouchMode::Normal),
                ..SetState::default()
            })
        );
    }

    #[test]
    fn a_lamp_that_follows_an_integration_gets_its_touch_mode_and_color() {
        let commands = commands_after_info(
            &info_of("info"),
            &Settings::default(),
            &Palette::default(),
            (TouchMode::PowerOnly, Some(1)),
        );
        assert_eq!(
            commands.last(),
            Some(&Command::SetState(SetState {
                color: Some(1),
                brightness_mode: Some(BrightnessMode::Auto),
                manual_level: Some(Settings::default().manual_level),
                touch_mode: Some(TouchMode::PowerOnly),
                ..SetState::default()
            }))
        );
    }

    #[test]
    fn the_heartbeat_comes_after_3_s_without_a_command() {
        let written = Instant::now();
        assert!(!heartbeat_due(
            true,
            written,
            written + Duration::from_millis(2_999)
        ));
        assert!(heartbeat_due(true, written, written + HEARTBEAT));
        assert!(!heartbeat_due(
            false,
            written,
            written + Duration::from_secs(60)
        ));
        // Its own bytes, which annex A fixes.
        assert_eq!(
            heartbeat().encode(7)[..9],
            [0x03, 7, 0x06, 0x10, 0, 0, 0, 0, 1]
        );
    }

    #[test]
    fn the_windows_hear_a_heartbeat_only_when_it_changed_something() {
        let state = |state_seq| match protocol::decode_message(&report("state_after_touch"))
            .expect("STATE")
            .message
        {
            Message::State(state) => Message::State(State { state_seq, ..state }),
            other => panic!("{other:?}"),
        };
        // A reply that repeats the state, as most do.
        assert!(!shown(&state(8), true, true, Some(8)));
        // After a fallback to normal touch, the heartbeat set it again.
        assert!(shown(&state(9), true, true, Some(8)));
        // Any other STATE goes, the reply to a command of the backend too.
        assert!(shown(&state(8), true, false, Some(8)));
        assert!(shown(&state(8), false, false, Some(8)));
    }

    #[test]
    fn the_refusal_of_a_command_of_the_backend_stays_out_of_the_windows() {
        let refused = Message::Error {
            command: Command::SetState(SetState::default()).code(),
            error: disled_protocol::ProtocolError::Busy,
        };
        assert!(!shown(&refused, true, true, None), "the heartbeat");
        assert!(!shown(&refused, true, false, None), "the control");
        assert!(shown(&refused, false, false, None), "a window");
        let ack = Message::Ack {
            command: Command::SetAutoBias { bias: 0 }.code(),
        };
        assert!(!shown(&ack, true, false, None));
        assert!(shown(&ack, false, false, None));
    }

    #[test]
    fn a_locked_lamp_shows_at_once_in_the_tray() {
        assert_eq!(
            look_of_info(&info_of("info_of_a_version_1_lamp")),
            Some(Look::NeedsUpdate)
        );
        assert_eq!(look_of_info(&info_of("info")), None);
    }
}
