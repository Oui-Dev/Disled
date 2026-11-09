// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! Connection to the lamp on a worker thread: discovery and reconnection,
//! command writes, message reads. Results reach the frontend as events.
//!
//! Until the INFO of a lamp says it speaks protocol 2, only GET_INFO goes to
//! it and only its INFO comes back: a version 1 lamp gets the lock of the
//! app, and nothing that it sends shows as an error.

use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use hidapi::{HidApi, HidDevice};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use disled_protocol::{
    self as protocol, ChangeSource, Command, DecodeError, Frame, Info, Message, PAYLOAD_SIZE,
    PROTOCOL_MAJOR, REPORT_ID_COMMAND, REPORT_ID_MESSAGE, SetState, State, TouchMode,
    USB_PRODUCT_ID_DEBUG, USB_VENDOR_ID,
};

use crate::colors::{ColorsState, Palette, PreviewState};
use crate::i18n::LocaleState;
use crate::settings::{Settings, SettingsState};
use crate::tray::{self, Look};
use crate::windows;

const SCAN_INTERVAL: Duration = Duration::from_millis(1000);
const READ_TIMEOUT_MS: i32 = 20;

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
/// the state the app sets.
pub fn commands_after_info(info: &Info, settings: &Settings, palette: &Palette) -> Vec<Command> {
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
        brightness_mode: Some(settings.brightness_mode),
        manual_level: Some(settings.manual_level),
        touch_mode: Some(TouchMode::Normal),
        ..SetState::default()
    }));
    commands
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
    commands: mpsc::Sender<Command>,
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

    /// Queues a command. A lamp that does not speak protocol 2 gets only
    /// GET_INFO: the worker drops the others, and the caller hears of no
    /// error, since the windows show the lock instead.
    pub fn send(&self, command: Command) -> Result<(), String> {
        if !self.connection().connected {
            return Err("no Disled lamp connected".into());
        }
        self.commands
            .send(command)
            .map_err(|_| "the USB worker stopped".to_string())
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
    receiver: mpsc::Receiver<Command>,
    connection: Arc<Mutex<Connection>>,
    info: Arc<Mutex<Option<Info>>>,
    state: Arc<Mutex<Option<State>>>,
    seq: u8,
    session: Session,
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
                    if let Exit::AppClosed = self.serve(&device) {
                        return;
                    }
                    self.set_info(None);
                    self.set_state(None);
                    self.app.state::<PreviewState>().forget();
                    self.set_connection(Connection::default());
                    tray::show(&self.app, Look::NoLamp);
                }
                None => {
                    // Commands sent while no lamp is connected are dropped.
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
    fn write(&mut self, device: &HidDevice, command: &Command) -> bool {
        if !self.session.allows(command) {
            return true;
        }
        let mut report = [0u8; PAYLOAD_SIZE + 1];
        report[0] = REPORT_ID_COMMAND;
        report[1..].copy_from_slice(&command.encode(self.next_seq()));
        device.write(&report).is_ok()
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
                    for command in commands_after_info(info, &settings, &palette) {
                        if !self.write(device, &command) {
                            return false;
                        }
                    }
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
        if !self.write(device, &Command::GetInfo) {
            return Exit::Disconnected;
        }
        let mut buffer = [0u8; PAYLOAD_SIZE + 1];
        loop {
            loop {
                match self.receiver.try_recv() {
                    Ok(command) => {
                        if !self.write(device, &command) {
                            return Exit::Disconnected;
                        }
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => return Exit::AppClosed,
                }
            }
            match device.read_timeout(&mut buffer, READ_TIMEOUT_MS) {
                Ok(0) => {}
                Ok(length) if buffer[0] == REPORT_ID_MESSAGE => {
                    let first_info = self.session.generation.is_none();
                    match self.session.receive(&buffer[1..length]) {
                        Incoming::Forward(frame) => {
                            if !self.follow(device, &frame.message, first_info) {
                                return Exit::Disconnected;
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
        assert!(commands_after_info(&info, &Settings::default(), &Palette::default()).is_empty());
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
        assert!(commands_after_info(&info, &Settings::default(), &Palette::default()).is_empty());
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
        let commands = commands_after_info(&info, &settings, &palette);
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
    fn a_locked_lamp_shows_at_once_in_the_tray() {
        assert_eq!(
            look_of_info(&info_of("info_of_a_version_1_lamp")),
            Some(Look::NeedsUpdate)
        );
        assert_eq!(look_of_info(&info_of("info")), None);
    }
}
