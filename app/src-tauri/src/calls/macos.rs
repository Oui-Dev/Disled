// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! macOS: the process objects of CoreAudio, from macOS 14.2, read every
//! second. Each tells whether its process runs an input. Only properties
//! are read: no stream, no tap, no IOProc, and no AVFoundation, so macOS
//! has nothing to ask the user, and the bundle declares no use of the
//! microphone (`NSMicrophoneUsageDescription`), which the CI checks.

use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr::{self, NonNull};
use std::time::Duration;

use objc2_core_audio::{
    AudioObjectGetPropertyData, AudioObjectGetPropertyDataSize, AudioObjectHasProperty,
    AudioObjectID, AudioObjectPropertyAddress, AudioObjectPropertySelector,
    kAudioHardwareBadDeviceError, kAudioHardwareBadObjectError, kAudioHardwareBadPropertySizeError,
    kAudioHardwarePropertyProcessObjectList, kAudioObjectPropertyElementMain,
    kAudioObjectPropertyScopeGlobal, kAudioObjectSystemObject, kAudioProcessPropertyIsRunningInput,
    kAudioProcessPropertyPID,
};

use super::rules::{self, Process, Status};
use super::{Reading, Stop};

const POLL: Duration = Duration::from_secs(1);
const SYSTEM: AudioObjectID = kAudioObjectSystemObject as AudioObjectID;

// The rules know the codes of an object that left.
const _: () = assert!(
    kAudioHardwareBadObjectError == rules::BAD_OBJECT
        && kAudioHardwareBadDeviceError == rules::BAD_DEVICE
);

/// macOS 14.2 or later, whose system object lists its processes.
pub fn available() -> bool {
    objc2::available!(macos = 14.2) && has_property(SYSTEM, kAudioHardwarePropertyProcessObjectList)
}

/// Reads the process objects every second until asked to stop.
pub fn run(stop: &Stop, mut heard: impl FnMut(Reading)) {
    // The PID of an object never changes: read once.
    let mut pids = HashMap::new();
    loop {
        heard(read(&mut pids));
        if stop.wait(POLL) {
            break;
        }
    }
}

fn read(pids: &mut HashMap<AudioObjectID, i32>) -> Reading {
    let Ok(objects) = ids(SYSTEM, kAudioHardwarePropertyProcessObjectList) else {
        return Reading::Unknown;
    };
    pids.retain(|object, _| objects.contains(object));
    let processes: Vec<Process> = objects
        .iter()
        .map(|&object| Process {
            pid: match pids.get(&object) {
                Some(&pid) => Ok(pid),
                None => get::<i32>(object, kAudioProcessPropertyPID).inspect(|&pid| {
                    pids.insert(object, pid);
                }),
            },
            input: get::<u32>(object, kAudioProcessPropertyIsRunningInput).map(|input| input != 0),
        })
        .collect();
    let own = i32::try_from(std::process::id()).unwrap_or(i32::MAX);
    rules::processes(&processes, own)
}

fn address(selector: AudioObjectPropertySelector) -> AudioObjectPropertyAddress {
    AudioObjectPropertyAddress {
        mSelector: selector,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain,
    }
}

fn has_property(object: AudioObjectID, selector: AudioObjectPropertySelector) -> bool {
    let address = address(selector);
    // SAFETY: a valid address, read by the call only.
    unsafe { AudioObjectHasProperty(object, NonNull::from(&address)) }
}

/// A property of a fixed size: a u32 or a pid_t.
fn get<T: Copy + Default>(
    object: AudioObjectID,
    selector: AudioObjectPropertySelector,
) -> Result<T, Status> {
    let address = address(selector);
    let mut value = T::default();
    let mut size = u32::try_from(size_of::<T>()).unwrap_or(0);
    // SAFETY: `value` holds `size` bytes, and CoreAudio writes no more.
    let status = unsafe {
        AudioObjectGetPropertyData(
            object,
            NonNull::from(&address),
            0,
            ptr::null(),
            NonNull::from(&mut size),
            NonNull::from(&mut value).cast(),
        )
    };
    if status == 0 { Ok(value) } else { Err(status) }
}

/// A list of objects. It may change between the question of its size and
/// that of its data: room for 16 more, and the size given back decides the
/// length.
fn ids(
    object: AudioObjectID,
    selector: AudioObjectPropertySelector,
) -> Result<Vec<AudioObjectID>, Status> {
    let address = address(selector);
    let item = size_of::<AudioObjectID>();
    for _ in 0..3 {
        let mut size = 0u32;
        // SAFETY: a valid address, and a size to write.
        let status = unsafe {
            AudioObjectGetPropertyDataSize(
                object,
                NonNull::from(&address),
                0,
                ptr::null(),
                NonNull::from(&mut size),
            )
        };
        if status != 0 {
            return Err(status);
        }
        let count = usize::try_from(size).unwrap_or(0) / item;
        if count == 0 {
            return Ok(Vec::new());
        }
        let mut list: Vec<AudioObjectID> = vec![0; count + 16];
        let mut used = u32::try_from(list.len() * item).unwrap_or(u32::MAX);
        // SAFETY: `list` holds `used` bytes, and CoreAudio writes no more.
        let status = unsafe {
            AudioObjectGetPropertyData(
                object,
                NonNull::from(&address),
                0,
                ptr::null(),
                NonNull::from(&mut used),
                NonNull::new(list.as_mut_ptr().cast::<c_void>()).unwrap_or(NonNull::dangling()),
            )
        };
        if status == kAudioHardwareBadPropertySizeError {
            // It grew past the room: ask its size again.
            continue;
        }
        if status != 0 {
            return Err(status);
        }
        list.truncate(usize::try_from(used).unwrap_or(0) / item);
        return Ok(list);
    }
    Err(kAudioHardwareBadPropertySizeError)
}
