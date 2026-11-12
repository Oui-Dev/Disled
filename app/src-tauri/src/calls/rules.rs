// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! What each system shows of the microphone, turned into a reading. Apart
//! from the calls into the systems, so that the tests of the three rules
//! run on every system. No rule names an app: any capture counts, but the
//! sounds of the system, the level meters of the Linux desktops and Disled.

use super::Reading;

/// A capture session of WASAPI, on Windows.
#[cfg(any(test, windows))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Session {
    /// A session that could not be read, or an endpoint whose sessions
    /// could not: it may be the call.
    Unreadable,
    Read {
        /// The sounds of the system, never a call.
        system: bool,
        /// None when Windows does not tell: the session still counts.
        pid: Option<u32>,
        /// Whether its stream runs, None when its state could not be read.
        active: Option<bool>,
    },
}

/// Busy as soon as a session of another app is active. A session that
/// could not be read makes the reading Unknown, unless another is active.
#[cfg(any(test, windows))]
pub fn sessions(sessions: &[Session], own: u32) -> Reading {
    let mut unknown = false;
    for session in sessions {
        match *session {
            Session::Unreadable => unknown = true,
            Session::Read { system: true, .. } => {}
            Session::Read { pid, .. } if pid == Some(own) => {}
            Session::Read {
                active: Some(true), ..
            } => return Reading::Busy,
            Session::Read { active: None, .. } => unknown = true,
            Session::Read { .. } => {}
        }
    }
    if unknown {
        Reading::Unknown
    } else {
        Reading::Free
    }
}

/// An OSStatus of CoreAudio.
#[cfg(any(test, target_os = "macos"))]
pub type Status = i32;

/// '!obj' and '!dev': the object left after its list was read, as a
/// process that quit.
#[cfg(any(test, target_os = "macos"))]
pub const BAD_OBJECT: Status = 0x216f_626a;
#[cfg(any(test, target_os = "macos"))]
pub const BAD_DEVICE: Status = 0x2164_6576;

/// A process object of CoreAudio, on macOS 14.2 and later.
#[cfg(any(test, target_os = "macos"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Process {
    pub pid: Result<i32, Status>,
    /// `kAudioProcessPropertyIsRunningInput`.
    pub input: Result<bool, Status>,
}

/// Busy as soon as another process captures. A process that left since
/// the list was read does not count, and any other failed read makes the
/// reading Unknown, unless another process captures.
#[cfg(any(test, target_os = "macos"))]
pub fn processes(processes: &[Process], own: i32) -> Reading {
    let gone = |status: &Status| matches!(*status, BAD_OBJECT | BAD_DEVICE);
    let mut unknown = false;
    for process in processes {
        if process.pid == Ok(own) || process.pid.as_ref().is_err_and(gone) {
            continue;
        }
        match process.input {
            Ok(true) => return Reading::Busy,
            Ok(false) => {}
            Err(status) if gone(&status) => {}
            Err(_) => unknown = true,
        }
    }
    if unknown {
        Reading::Unknown
    } else {
        Reading::Free
    }
}

/// The process objects came with macOS 14.2: "14.2.1" has them, "14.1"
/// not. A missing or unreadable part counts as 0.
#[cfg(any(test, target_os = "macos"))]
pub fn supported(version: &str) -> bool {
    let mut parts = version
        .trim()
        .split('.')
        .map(|part| part.parse::<u32>().unwrap_or(0));
    let major = parts.next().unwrap_or(0);
    let minor = parts.next().unwrap_or(0);
    (major, minor) >= (14, 2)
}

/// The level meters of the desktops: they capture only to draw a meter.
/// GNOME skips the first two, the sound applet of KDE all four. Compared
/// in lower case, since pavucontrol names itself both ways.
#[cfg(any(test, target_os = "linux"))]
const LEVEL_METERS: [&str; 4] = [
    "org.gnome.volumecontrol",
    "org.pulseaudio.pavucontrol",
    "org.kde.kmixd",
    "org.kde.plasma-pa",
];

/// A capture stream of the sound server (a source output), on Linux.
#[cfg(any(test, target_os = "linux"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stream {
    /// `application.id`.
    pub app_id: Option<String>,
    /// It has a client, so it is not a virtual stream of the server.
    pub client: bool,
    /// Paused by its app.
    pub corked: bool,
    /// The index of its source.
    pub source: u32,
    /// `application.process.id`.
    pub pid: Option<u32>,
}

/// A source of the sound server: a microphone, or the monitor of an
/// output.
#[cfg(any(test, target_os = "linux"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Source {
    pub index: u32,
    pub running: bool,
    pub monitor: bool,
}

/// Whether a source listens to an output: it monitors a sink, by its
/// index, its name or its class.
#[cfg(any(test, target_os = "linux"))]
pub fn is_monitor(monitor_of_sink: Option<u32>, name: &str, class: Option<&str>) -> bool {
    monitor_of_sink.is_some() || name.ends_with(".monitor") || class == Some("monitor")
}

/// The rule of the microphone indicator of GNOME, stream by stream: not a
/// level meter, a client, not corked, from a running source that is not a
/// monitor, and not Disled.
#[cfg(any(test, target_os = "linux"))]
pub fn counts(stream: &Stream, sources: &[Source], own: u32) -> bool {
    let meter = stream
        .app_id
        .as_deref()
        .is_some_and(|id| LEVEL_METERS.contains(&id.to_lowercase().as_str()));
    let source = sources.iter().find(|source| source.index == stream.source);
    !meter
        && stream.client
        && !stream.corked
        && source.is_some_and(|source| source.running && !source.monitor)
        && stream.pid != Some(own)
}

/// Busy as soon as one stream counts.
#[cfg(any(test, target_os = "linux"))]
pub fn streams(streams: &[Stream], sources: &[Source], own: u32) -> Reading {
    if streams.iter().any(|stream| counts(stream, sources, own)) {
        Reading::Busy
    } else {
        Reading::Free
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OWN: u32 = 4242;

    fn read(system: bool, pid: Option<u32>, active: Option<bool>) -> Session {
        Session::Read {
            system,
            pid,
            active,
        }
    }

    #[test]
    fn a_session_of_another_app_that_runs_is_a_capture() {
        // At rest, as measured on Windows 11: the sounds of the system
        // only, one session on each endpoint.
        let rest = [read(true, Some(0), Some(true)), read(true, Some(0), None)];
        assert_eq!(sessions(&rest, OWN), Reading::Free);
        assert_eq!(sessions(&[], OWN), Reading::Free);

        let call = [
            read(true, Some(0), Some(false)),
            read(false, Some(9001), Some(false)),
            read(false, Some(9002), Some(true)),
        ];
        assert_eq!(sessions(&call, OWN), Reading::Busy);
        // Disled never counts.
        assert_eq!(
            sessions(&[read(false, Some(OWN), Some(true))], OWN),
            Reading::Free
        );
        // A session whose process Windows does not tell still counts.
        assert_eq!(
            sessions(&[read(false, None, Some(true))], OWN),
            Reading::Busy
        );
    }

    #[test]
    fn a_session_that_cannot_be_read_keeps_the_state() {
        let unreadable = [read(false, Some(9001), Some(false)), Session::Unreadable];
        assert_eq!(sessions(&unreadable, OWN), Reading::Unknown);
        assert_eq!(
            sessions(&[read(false, Some(9001), None)], OWN),
            Reading::Unknown
        );
        // Another session that runs says enough.
        let mut busy = unreadable.to_vec();
        busy.push(read(false, Some(9002), Some(true)));
        assert_eq!(sessions(&busy, OWN), Reading::Busy);
    }

    fn process(pid: Result<i32, Status>, input: Result<bool, Status>) -> Process {
        Process { pid, input }
    }

    #[test]
    fn a_process_that_runs_its_input_is_a_capture() {
        let own = 500;
        // Music on a Bluetooth headset: the processes play, none captures.
        let music = [process(Ok(301), Ok(false)), process(Ok(302), Ok(false))];
        assert_eq!(processes(&music, own), Reading::Free);
        assert_eq!(
            processes(
                &[process(Ok(301), Ok(false)), process(Ok(303), Ok(true))],
                own
            ),
            Reading::Busy
        );
        // Disled never counts, nor a process that quit meanwhile.
        assert_eq!(processes(&[process(Ok(own), Ok(true))], own), Reading::Free);
        assert_eq!(
            processes(&[process(Ok(304), Err(BAD_OBJECT))], own),
            Reading::Free
        );
        assert_eq!(
            processes(&[process(Err(BAD_OBJECT), Ok(true))], own),
            Reading::Free
        );
        // Any other failure keeps the state, unless another captures.
        let unreadable = process(Ok(305), Err(0x7768_6174));
        assert_eq!(processes(&[unreadable], own), Reading::Unknown);
        assert_eq!(
            processes(&[unreadable, process(Ok(303), Ok(true))], own),
            Reading::Busy
        );
        // A process whose PID could not be read still counts.
        assert_eq!(
            processes(&[process(Err(0x7768_6174), Ok(true))], own),
            Reading::Busy
        );
    }

    #[test]
    fn the_calls_need_macos_14_2() {
        for version in ["14.2", "14.2.1", "15.0", "26.1", "27"] {
            assert!(supported(version), "{version}");
        }
        for version in ["14.1", "14.1.2", "13.6.9", "11.0", "", "garbage"] {
            assert!(!supported(version), "{version}");
        }
    }

    const PIPEWIRE_USB: u32 = 56;
    const PIPEWIRE_MONITOR: u32 = 57;
    const PIPEWIRE_BLUETOOTH: u32 = 70;

    fn source(index: u32, running: bool, monitor: bool) -> Source {
        Source {
            index,
            running,
            monitor,
        }
    }

    fn stream(app_id: Option<&str>, client: bool, corked: bool, source: u32, pid: u32) -> Stream {
        Stream {
            app_id: app_id.map(str::to_owned),
            client,
            corked,
            source,
            pid: Some(pid),
        }
    }

    /// The sources of the `cmd-pipewire` fixtures of the Linux kit: a USB
    /// microphone that runs, the monitor of the built-in output, and a
    /// Bluetooth microphone left suspended.
    fn pipewire_sources() -> Vec<Source> {
        vec![
            source(PIPEWIRE_USB, true, false),
            source(
                PIPEWIRE_MONITOR,
                false,
                is_monitor(
                    None,
                    "alsa_output.pci-0000_00_1f.3.analog-stereo.monitor",
                    Some("monitor"),
                ),
            ),
            source(
                PIPEWIRE_BLUETOOTH,
                false,
                is_monitor(None, "bluez_input.AC_80_0A_12_34_56.1", Some("sound")),
            ),
        ]
    }

    #[test]
    fn the_rule_of_gnome_on_pipewire() {
        let sources = pipewire_sources();
        // The streams of the fixtures, with whether each counts.
        let streams = [
            // Firefox records the USB microphone.
            (stream(None, true, false, PIPEWIRE_USB, 4321), true),
            // The level meters of pavucontrol, under both its ids, and of
            // the GNOME settings.
            (
                stream(
                    Some("org.PulseAudio.pavucontrol"),
                    true,
                    false,
                    PIPEWIRE_USB,
                    5010,
                ),
                false,
            ),
            (
                stream(
                    Some("org.pulseaudio.pavucontrol"),
                    true,
                    false,
                    PIPEWIRE_USB,
                    5020,
                ),
                false,
            ),
            (
                stream(
                    Some("org.gnome.VolumeControl"),
                    true,
                    false,
                    PIPEWIRE_USB,
                    5030,
                ),
                false,
            ),
            // OBS records the sound of the desktop, from a monitor.
            (stream(None, true, false, PIPEWIRE_MONITOR, 5040), false),
            // The echo canceller of the server: no client.
            (
                Stream {
                    app_id: None,
                    client: false,
                    corked: false,
                    source: PIPEWIRE_USB,
                    pid: None,
                },
                false,
            ),
            // Chrome paused its stream.
            (stream(None, true, true, PIPEWIRE_USB, 6001), false),
            // Discord on the Bluetooth microphone, still suspended.
            (stream(None, true, false, PIPEWIRE_BLUETOOTH, 6100), false),
            // Slack, linked to no source.
            (stream(None, true, false, u32::MAX, 6200), false),
        ];
        for (stream, counted) in &streams {
            assert_eq!(counts(stream, &sources, OWN), *counted, "{stream:?}");
        }
        let all: Vec<Stream> = streams.iter().map(|(stream, _)| stream.clone()).collect();
        assert_eq!(super::streams(&all, &sources, OWN), Reading::Busy);
        assert_eq!(super::streams(&all[1..], &sources, OWN), Reading::Free);
        assert_eq!(super::streams(&[], &sources, OWN), Reading::Free);
    }

    #[test]
    fn the_rule_of_gnome_on_pulseaudio() {
        let sources = [
            source(
                0,
                false,
                is_monitor(
                    None,
                    "alsa_output.pci-0000_00_1f.3.analog-stereo.monitor",
                    Some("monitor"),
                ),
            ),
            source(1, true, false),
        ];
        let streams = [
            // Zoom records the USB microphone.
            (stream(None, true, false, 1, 7100), true),
            // A loopback module: no client.
            (
                Stream {
                    app_id: None,
                    client: false,
                    corked: false,
                    source: 1,
                    pid: None,
                },
                false,
            ),
            (
                stream(Some("org.PulseAudio.pavucontrol"), true, false, 1, 7200),
                false,
            ),
            // OBS on the monitor of the output.
            (stream(None, true, false, 0, 7300), false),
        ];
        for (stream, counted) in &streams {
            assert_eq!(counts(stream, &sources, OWN), *counted, "{stream:?}");
        }
    }

    #[test]
    fn disled_and_the_monitors_never_count() {
        let mut sources = pipewire_sources();
        assert!(!counts(
            &stream(None, true, false, PIPEWIRE_USB, OWN),
            &sources,
            OWN
        ));
        // A monitor runs while it is recorded, as by OBS.
        sources.push(source(80, true, true));
        assert!(!counts(&stream(None, true, false, 80, 5040), &sources, OWN));
        // A monitor shows by its index, its name or its class.
        assert!(is_monitor(Some(3), "alsa_input.usb", None));
        assert!(is_monitor(None, "alsa_output.usb.monitor", None));
        assert!(is_monitor(None, "virtual", Some("monitor")));
        assert!(!is_monitor(None, "alsa_input.usb", Some("sound")));
    }
}
