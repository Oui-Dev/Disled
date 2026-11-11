// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! The calls, told by the microphone in use: on Windows by the capture
//! sessions of WASAPI, on macOS by the process objects of CoreAudio, on
//! Linux by the capture streams of the sound server. Disled only reads what
//! the system shows: it never opens a stream, so it hears nothing, and no
//! app name leaves this module.
//!
//! The monitor runs only while the control wants it: the "Microphone in
//! use" switch on, in manual mode, with a version 2 lamp. It reads on a
//! thread of its own, `disled-mic`, and a debounce turns the readings into
//! calls: 2 s of capture to begin one, 5 s without to end it.

mod rules;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
use self::linux as platform;
#[cfg(target_os = "macos")]
use self::macos as platform;
#[cfg(windows)]
use self::windows as platform;

use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use tauri::AppHandle;

use crate::control::{self, Input};

/// Capture without a break that begins a call.
const ON_DELAY: Duration = Duration::from_secs(2);
/// No capture without a break that ends it.
const OFF_DELAY: Duration = Duration::from_secs(5);
/// Readings that keep failing end a call after this long, so that the lamp
/// never stays in a call for good.
const UNKNOWN_LIMIT: Duration = Duration::from_secs(30);

/// What the system shows of the microphone at one moment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reading {
    /// No app captures.
    Free,
    /// At least one app captures.
    Busy,
    /// The system could not be read: neither a capture nor its end.
    Unknown,
}

/// Turns the readings into calls. It takes the latest reading at least
/// every second, new or heard again, since Linux reads only on a change.
#[derive(Debug, Default)]
pub struct Debounce {
    in_call: bool,
    /// Since when the readings say Busy, without a break.
    busy: Option<Instant>,
    /// Since when they say Free, without a break.
    free: Option<Instant>,
    /// Since when none says Busy.
    quiet: Option<Instant>,
}

impl Debounce {
    /// Takes a reading: true when a call begins, false when it ends.
    pub fn step(&mut self, now: Instant, reading: Reading) -> Option<bool> {
        match reading {
            Reading::Busy => {
                self.busy.get_or_insert(now);
                self.free = None;
                self.quiet = None;
            }
            Reading::Free => {
                self.busy = None;
                self.free.get_or_insert(now);
                self.quiet.get_or_insert(now);
            }
            Reading::Unknown => {
                self.busy = None;
                self.free = None;
                self.quiet.get_or_insert(now);
            }
        }
        let held = |since: Option<Instant>, delay: Duration| {
            since.is_some_and(|since| now.saturating_duration_since(since) >= delay)
        };
        let change = if self.in_call {
            held(self.free, OFF_DELAY) || held(self.quiet, UNKNOWN_LIMIT)
        } else {
            held(self.busy, ON_DELAY)
        };
        change.then(|| {
            self.in_call = !self.in_call;
            self.in_call
        })
    }
}

/// Asks the thread of the microphone to stop, and wakes it at once.
#[derive(Default)]
pub struct Stop {
    asked: Mutex<bool>,
    woken: Condvar,
    /// The main loop of libpulse waits on its own: this wakes it.
    #[cfg(target_os = "linux")]
    waker: Mutex<Option<Box<dyn Fn() + Send>>>,
}

impl Stop {
    /// Waits at most `timeout`: true when asked to stop.
    pub fn wait(&self, timeout: Duration) -> bool {
        let Ok(asked) = self.asked.lock() else {
            return true;
        };
        self.woken
            .wait_timeout_while(asked, timeout, |asked| !*asked)
            .map_or(true, |(asked, _)| *asked)
    }

    #[cfg(any(test, target_os = "linux"))]
    pub fn asked(&self) -> bool {
        self.asked.lock().map_or(true, |asked| *asked)
    }

    fn ask(&self) {
        if let Ok(mut asked) = self.asked.lock() {
            *asked = true;
        }
        self.woken.notify_all();
        #[cfg(target_os = "linux")]
        if let Ok(waker) = self.waker.lock()
            && let Some(wake) = waker.as_ref()
        {
            wake();
        }
    }

    /// Sets what wakes the thread, or none before what it wakes goes.
    #[cfg(target_os = "linux")]
    pub fn wake_with(&self, waker: Option<Box<dyn Fn() + Send>>) {
        if let Ok(mut current) = self.waker.lock() {
            *current = waker;
        }
    }
}

/// The thread that follows the microphone, for one run of the control.
pub struct Monitor {
    stop: Arc<Stop>,
    thread: JoinHandle<()>,
}

impl Monitor {
    /// Follows the microphone and tells the control of each call, under
    /// the number of this run, so that a call heard by a run stopped since
    /// changes nothing.
    pub fn start(app: AppHandle, run: u64) -> Option<Self> {
        let stop = Arc::new(Stop::default());
        let asked = Arc::clone(&stop);
        let thread = thread::Builder::new()
            .name("disled-mic".into())
            .spawn(move || {
                let mut debounce = Debounce::default();
                platform::run(&asked, |reading| {
                    if let Some(in_call) = debounce.step(Instant::now(), reading) {
                        let _ = control::input(&app, Input::Call { run, in_call });
                    }
                });
            })
            .ok()?;
        Some(Self { stop, thread })
    }

    /// Asks the thread to stop, without waiting.
    pub fn ask(&self) {
        self.stop.ask();
    }

    /// Asks the thread to stop and waits for it, which takes no longer
    /// than the reading under way.
    pub fn join(self) {
        self.stop.ask();
        let _ = self.thread.join();
    }
}

/// Tells the control whether this system can tell a call, from a thread of
/// its own: the first question to CoreAudio may take a moment.
pub fn announce(app: &AppHandle) {
    let app = app.clone();
    let _ = thread::Builder::new()
        .name("disled-mic-check".into())
        .spawn(move || {
            let _ = control::input(&app, Input::CallsAvailable(platform::available()));
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use Reading::{Busy, Free, Unknown};

    /// Readings, one each second from `start`, and what the debounce said
    /// at each: true for a call that begins, false for one that ends.
    fn play(debounce: &mut Debounce, start: Instant, readings: &[Reading]) -> Vec<(u64, bool)> {
        readings
            .iter()
            .zip(0u64..)
            .filter_map(|(reading, second)| {
                debounce
                    .step(start + Duration::from_secs(second), *reading)
                    .map(|change| (second, change))
            })
            .collect()
    }

    #[test]
    fn a_call_begins_after_2_s_of_capture_and_ends_after_5_s_without() {
        let mut debounce = Debounce::default();
        let mut readings = vec![Free, Busy, Busy, Busy, Busy];
        readings.extend([Free; 6]);
        assert_eq!(
            play(&mut debounce, Instant::now(), &readings),
            [(3, true), (10, false)]
        );
    }

    #[test]
    fn a_single_busy_reading_heard_again_begins_a_call() {
        // Linux reads only on a change: the monitor gives the debounce the
        // last reading every second.
        let mut debounce = Debounce::default();
        let start = Instant::now();
        assert_eq!(debounce.step(start, Busy), None);
        assert_eq!(debounce.step(start + Duration::from_secs(1), Busy), None);
        assert_eq!(
            debounce.step(start + Duration::from_secs(2), Busy),
            Some(true)
        );
    }

    #[test]
    fn short_captures_and_short_breaks_change_nothing() {
        let mut debounce = Debounce::default();
        let flicker = [Busy, Free, Busy, Free, Busy, Unknown, Busy, Free];
        assert_eq!(play(&mut debounce, Instant::now(), &flicker), []);

        let mut debounce = Debounce::default();
        let mut readings = vec![Busy; 3];
        // A call with breaks of 4 s, then a capture again.
        for _ in 0..3 {
            readings.extend([Free; 4]);
            readings.push(Busy);
        }
        assert_eq!(play(&mut debounce, Instant::now(), &readings), [(2, true)]);
    }

    #[test]
    fn an_unknown_reading_keeps_the_call_30_s_at_most() {
        let mut debounce = Debounce::default();
        let mut readings = vec![Busy; 3];
        readings.extend([Unknown; 40]);
        // The call began at 2 s, and its last capture was seen at 2 s.
        assert_eq!(
            play(&mut debounce, Instant::now(), &readings),
            [(2, true), (33, false)]
        );

        // Unknown readings mixed with Free ones end it at 30 s too.
        let mut debounce = Debounce::default();
        let mut readings = vec![Busy; 3];
        for _ in 0..10 {
            readings.extend([Free, Free, Free, Unknown]);
        }
        assert_eq!(
            play(&mut debounce, Instant::now(), &readings),
            [(2, true), (33, false)]
        );
    }

    #[test]
    fn an_unknown_reading_never_begins_a_call() {
        let mut debounce = Debounce::default();
        assert_eq!(play(&mut debounce, Instant::now(), &[Unknown; 40]), []);
        // Nor does it carry a capture across: the 2 s start again after it.
        let mut debounce = Debounce::default();
        let readings = [Busy, Unknown, Busy, Busy, Busy];
        assert_eq!(play(&mut debounce, Instant::now(), &readings), [(4, true)]);
    }

    #[test]
    fn a_stop_wakes_the_waiting_thread() {
        let stop = Arc::new(Stop::default());
        assert!(!stop.wait(Duration::from_millis(1)));
        let asked = Arc::clone(&stop);
        let waiting = thread::spawn(move || {
            let started = Instant::now();
            assert!(asked.wait(Duration::from_secs(30)));
            started.elapsed()
        });
        thread::sleep(Duration::from_millis(50));
        stop.ask();
        let waited = waiting.join().expect("the thread ends");
        assert!(waited < Duration::from_secs(5), "{waited:?}");
        assert!(stop.asked());
        assert!(stop.wait(Duration::from_secs(30)));
    }
}
