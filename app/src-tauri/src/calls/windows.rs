// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! Windows: the capture sessions of WASAPI, read every second. A session
//! is the use of a capture endpoint by an app, active while its stream
//! runs. Measured on Windows 11: active within 0.2 s of the start of the
//! capture, inactive within 20 ms of its end, about 1.2 ms of processor
//! for a reading.

use std::time::Duration;

use windows::Win32::Foundation::S_OK;
use windows::Win32::Media::Audio::{
    AudioSessionStateActive, DEVICE_STATE_ACTIVE, IAudioSessionControl2, IAudioSessionEnumerator,
    IAudioSessionManager2, IMMDeviceEnumerator, MMDeviceEnumerator, eCapture,
};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
};
use windows::core::Interface;

use super::rules::{self, Session};
use super::{Reading, Stop};

const POLL: Duration = Duration::from_secs(1);

/// Every Windows has the sessions: a failure shows in the readings.
pub fn available() -> bool {
    true
}

/// Reads the sessions every second until asked to stop.
pub fn run(stop: &Stop, mut heard: impl FnMut(Reading)) {
    // SAFETY: COM for this thread only, left below if it came.
    let com = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok();
    {
        let mut enumerator = None;
        loop {
            heard(if com {
                read(&mut enumerator)
            } else {
                Reading::Unknown
            });
            if stop.wait(POLL) {
                break;
            }
        }
    }
    if com {
        // SAFETY: balances the initialization above, once its objects are
        // gone.
        unsafe { CoUninitialize() };
    }
}

/// One reading. A failed enumeration, as after a restart of the audio
/// service, gets a new enumerator the next time.
fn read(enumerator: &mut Option<IMMDeviceEnumerator>) -> Reading {
    if enumerator.is_none() {
        // SAFETY: a COM object of this thread, which COM serves.
        *enumerator = unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }.ok();
    }
    let Some(devices) = enumerator.as_ref() else {
        return Reading::Unknown;
    };
    match sessions(devices) {
        Ok(sessions) => rules::sessions(&sessions, std::process::id()),
        Err(_) => {
            *enumerator = None;
            Reading::Unknown
        }
    }
}

/// The sessions of every active capture endpoint, through a new session
/// enumerator each time, since an old one may miss new sessions.
fn sessions(devices: &IMMDeviceEnumerator) -> windows::core::Result<Vec<Session>> {
    let mut found = Vec::new();
    // SAFETY: COM calls on objects of this thread, whose results the
    // windows crate checks.
    unsafe {
        let endpoints = devices.EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE)?;
        for index in 0..endpoints.GetCount()? {
            let listed = endpoints
                .Item(index)
                .and_then(|endpoint| endpoint.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None))
                .and_then(|manager| manager.GetSessionEnumerator());
            // An endpoint that cannot be read may hide the call.
            match listed {
                Ok(listed) => endpoint_sessions(&listed, &mut found),
                Err(_) => found.push(Session::Unreadable),
            }
        }
    }
    Ok(found)
}

fn endpoint_sessions(listed: &IAudioSessionEnumerator, found: &mut Vec<Session>) {
    // SAFETY: COM calls on objects of this thread, whose results the
    // windows crate checks.
    unsafe {
        let Ok(count) = listed.GetCount() else {
            found.push(Session::Unreadable);
            return;
        };
        for index in 0..count {
            let control = listed
                .GetSession(index)
                .and_then(|session| session.cast::<IAudioSessionControl2>());
            found.push(match control {
                Ok(control) => Session::Read {
                    system: control.IsSystemSoundsSession() == S_OK,
                    pid: control.GetProcessId().ok(),
                    active: control
                        .GetState()
                        .ok()
                        .map(|state| state == AudioSessionStateActive),
                },
                Err(_) => Session::Unreadable,
            });
        }
    }
}
