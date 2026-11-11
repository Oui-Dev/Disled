// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! Linux: the capture streams of the sound server, through libpulse, which
//! speaks to PulseAudio and to pipewire-pulse alike. The library loads at
//! run time, so that the app starts without it, the calls group hidden:
//! an update of the package installs no dependency. The streams are read
//! again a tenth of a second after the server announces a change, and every
//! 10 s in any case. A server out of reach, at the start of the session or
//! later, is tried again every 5 s.
//!
//! Only the start of the structures of libpulse is described here, up to
//! the last field read, as `introspect.h` lays them out.

use std::ffi::{CStr, c_char, c_int, c_void};
use std::ptr::{self, NonNull};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use dlopen2::wrapper::{Container, WrapperApi};

use super::rules::{self, Source, Stream};
use super::{Reading, Stop};

const LIBRARY: &str = "libpulse.so.0";
const NAME: &CStr = c"Disled";

/// The changes come in bursts: one reading for all of them.
const GATHER: Duration = Duration::from_millis(100);
const REREAD: Duration = Duration::from_secs(10);
const RECONNECT: Duration = Duration::from_secs(5);
/// The longest wait for the server, so that the debounce moves on.
const TICK: Duration = Duration::from_secs(1);

const INVALID_INDEX: u32 = u32::MAX;
const CHANNELS_MAX: usize = 32;
const CONTEXT_NOAUTOSPAWN: c_int = 0x0001;
const CONTEXT_READY: c_int = 4;
const CONTEXT_FAILED: c_int = 5;
const CONTEXT_TERMINATED: c_int = 6;
const SUBSCRIPTION_MASK_SOURCE: c_int = 0x0002;
const SUBSCRIPTION_MASK_SOURCE_OUTPUT: c_int = 0x0008;
const SOURCE_RUNNING: c_int = 0;

#[repr(C)]
struct Mainloop {
    _opaque: [u8; 0],
}

#[repr(C)]
struct MainloopApi {
    _opaque: [u8; 0],
}

#[repr(C)]
struct Context {
    _opaque: [u8; 0],
}

#[repr(C)]
struct Operation {
    _opaque: [u8; 0],
}

#[repr(C)]
struct Proplist {
    _opaque: [u8; 0],
}

#[repr(C)]
struct SampleSpec {
    format: c_int,
    rate: u32,
    channels: u8,
}

#[repr(C)]
struct ChannelMap {
    channels: u8,
    map: [c_int; CHANNELS_MAX],
}

#[repr(C)]
struct ChannelVolumes {
    channels: u8,
    values: [u32; CHANNELS_MAX],
}

/// `pa_source_output_info`, up to `corked`.
#[repr(C)]
struct SourceOutputInfo {
    index: u32,
    name: *const c_char,
    owner_module: u32,
    client: u32,
    source: u32,
    sample_spec: SampleSpec,
    channel_map: ChannelMap,
    buffer_usec: u64,
    source_usec: u64,
    resample_method: *const c_char,
    driver: *const c_char,
    proplist: *const Proplist,
    corked: c_int,
}

/// `pa_source_info`, up to `state`.
#[repr(C)]
struct SourceInfo {
    name: *const c_char,
    index: u32,
    description: *const c_char,
    sample_spec: SampleSpec,
    channel_map: ChannelMap,
    owner_module: u32,
    volume: ChannelVolumes,
    mute: c_int,
    monitor_of_sink: u32,
    monitor_of_sink_name: *const c_char,
    latency: u64,
    driver: *const c_char,
    flags: c_int,
    proplist: *const Proplist,
    configured_latency: u64,
    base_volume: u32,
    state: c_int,
}

// The fields read sit where `introspect.h` puts them, on 64-bit Linux.
#[cfg(target_pointer_width = "64")]
const _: () = {
    assert!(std::mem::offset_of!(SourceOutputInfo, client) == 20);
    assert!(std::mem::offset_of!(SourceOutputInfo, source) == 24);
    assert!(std::mem::offset_of!(SourceOutputInfo, proplist) == 208);
    assert!(std::mem::offset_of!(SourceOutputInfo, corked) == 216);
    assert!(std::mem::offset_of!(SourceInfo, index) == 8);
    assert!(std::mem::offset_of!(SourceInfo, monitor_of_sink) == 308);
    assert!(std::mem::offset_of!(SourceInfo, proplist) == 344);
    assert!(std::mem::offset_of!(SourceInfo, state) == 364);
};

type SuccessCallback = unsafe extern "C" fn(*mut Context, c_int, *mut c_void);
type SubscribeCallback = unsafe extern "C" fn(*mut Context, c_int, u32, *mut c_void);
type SourceOutputCallback =
    unsafe extern "C" fn(*mut Context, *const SourceOutputInfo, c_int, *mut c_void);
type SourceCallback = unsafe extern "C" fn(*mut Context, *const SourceInfo, c_int, *mut c_void);

/// The functions of libpulse that the monitor calls.
#[derive(WrapperApi)]
struct Pulse {
    pa_mainloop_new: unsafe extern "C" fn() -> *mut Mainloop,
    pa_mainloop_free: unsafe extern "C" fn(mainloop: *mut Mainloop),
    pa_mainloop_get_api: unsafe extern "C" fn(mainloop: *mut Mainloop) -> *mut MainloopApi,
    pa_mainloop_prepare: unsafe extern "C" fn(mainloop: *mut Mainloop, timeout: c_int) -> c_int,
    pa_mainloop_poll: unsafe extern "C" fn(mainloop: *mut Mainloop) -> c_int,
    pa_mainloop_dispatch: unsafe extern "C" fn(mainloop: *mut Mainloop) -> c_int,
    pa_mainloop_wakeup: unsafe extern "C" fn(mainloop: *mut Mainloop),
    pa_context_new:
        unsafe extern "C" fn(api: *mut MainloopApi, name: *const c_char) -> *mut Context,
    pa_context_connect: unsafe extern "C" fn(
        context: *mut Context,
        server: *const c_char,
        flags: c_int,
        spawn: *const c_void,
    ) -> c_int,
    pa_context_disconnect: unsafe extern "C" fn(context: *mut Context),
    pa_context_unref: unsafe extern "C" fn(context: *mut Context),
    pa_context_get_state: unsafe extern "C" fn(context: *const Context) -> c_int,
    pa_context_set_subscribe_callback: unsafe extern "C" fn(
        context: *mut Context,
        callback: Option<SubscribeCallback>,
        userdata: *mut c_void,
    ),
    pa_context_subscribe: unsafe extern "C" fn(
        context: *mut Context,
        mask: c_int,
        callback: Option<SuccessCallback>,
        userdata: *mut c_void,
    ) -> *mut Operation,
    pa_context_get_source_output_info_list: unsafe extern "C" fn(
        context: *mut Context,
        callback: Option<SourceOutputCallback>,
        userdata: *mut c_void,
    ) -> *mut Operation,
    pa_context_get_source_info_list: unsafe extern "C" fn(
        context: *mut Context,
        callback: Option<SourceCallback>,
        userdata: *mut c_void,
    ) -> *mut Operation,
    pa_operation_unref: unsafe extern "C" fn(operation: *mut Operation),
    pa_proplist_gets:
        unsafe extern "C" fn(proplist: *const Proplist, key: *const c_char) -> *const c_char,
}

/// libpulse, loaded once, or None without it.
fn pulse() -> Option<&'static Container<Pulse>> {
    static PULSE: OnceLock<Option<Container<Pulse>>> = OnceLock::new();
    PULSE
        .get_or_init(|| {
            // SAFETY: libpulse runs no code of its own as it loads, and the
            // signatures above are those of its headers.
            unsafe { Container::load(LIBRARY) }.ok()
        })
        .as_ref()
}

/// The sound servers of Linux speak through libpulse: without it, no call.
pub fn available() -> bool {
    pulse().is_some()
}

/// Follows the sound server until asked to stop.
pub fn run(stop: &Stop, mut heard: impl FnMut(Reading)) {
    let Some(mut client) = pulse().and_then(Client::new) else {
        return;
    };
    stop.wake_with(Some(client.waker()));
    loop {
        let wait = client.prepare(Instant::now());
        if !client.iterate(wait, stop) {
            break;
        }
        client.collect();
        heard(client.reading);
    }
    // Before the main loop goes, with the client.
    stop.wake_with(None);
}

/// What the callbacks of libpulse fill, on the thread of the main loop.
#[derive(Default)]
struct Heard {
    /// The server announced a change since the last reading began.
    changed: bool,
    streams: Vec<Stream>,
    sources: Vec<Source>,
    /// The lists still to come for the reading under way.
    pending: u8,
    /// A list came back with an error.
    failed: bool,
}

struct Client {
    pulse: &'static Container<Pulse>,
    mainloop: *mut Mainloop,
    /// Null between two connections.
    context: *mut Context,
    /// Shared with the callbacks, which run only within `dispatch`, so
    /// reached through this pointer alone, and freed after the context.
    heard: NonNull<Heard>,
    subscribed: bool,
    /// A reading is under way.
    reading_lists: bool,
    /// When to read next, once the server is ready.
    due: Option<Instant>,
    /// When to try the server again.
    reconnect: Instant,
    reading: Reading,
}

/// Wakes the main loop from the thread that stops the monitor.
struct Waker {
    pulse: &'static Container<Pulse>,
    mainloop: *mut Mainloop,
}

// SAFETY: `pa_mainloop_wakeup` may be called from any thread, and the
// main loop outlives the waker, which `run` takes back before the client
// frees it.
unsafe impl Send for Waker {}

impl Waker {
    fn wake(&self) {
        // SAFETY: see `Waker`.
        unsafe { self.pulse.pa_mainloop_wakeup(self.mainloop) }
    }
}

impl Client {
    fn new(pulse: &'static Container<Pulse>) -> Option<Self> {
        // SAFETY: a main loop of this client, freed in `drop`.
        let mainloop = unsafe { pulse.pa_mainloop_new() };
        (!mainloop.is_null()).then(|| Self {
            pulse,
            mainloop,
            context: ptr::null_mut(),
            heard: NonNull::from(Box::leak(Box::<Heard>::default())),
            subscribed: false,
            reading_lists: false,
            due: None,
            reconnect: Instant::now(),
            reading: Reading::Unknown,
        })
    }

    fn waker(&self) -> Box<dyn Fn() + Send> {
        let waker = Waker {
            pulse: self.pulse,
            mainloop: self.mainloop,
        };
        // The whole waker moves, and only it is Send.
        Box::new(move || waker.wake())
    }

    /// Connects, subscribes and asks for the lists when it is time, and
    /// returns how long the main loop may wait.
    fn prepare(&mut self, now: Instant) -> Duration {
        if self.context.is_null() {
            if now >= self.reconnect {
                self.connect(now);
            }
        } else {
            // SAFETY: the context of this client.
            match unsafe { self.pulse.pa_context_get_state(self.context) } {
                CONTEXT_READY if !self.subscribed => self.subscribe(now),
                CONTEXT_FAILED | CONTEXT_TERMINATED => {
                    self.disconnect();
                    self.reading = Reading::Unknown;
                    self.reconnect = now + RECONNECT;
                }
                _ => {}
            }
        }
        if std::mem::take(&mut self.heard().changed) {
            let soon = now + GATHER;
            self.due = Some(self.due.map_or(soon, |due| due.min(soon)));
        }
        if self.subscribed && !self.reading_lists && self.due.is_some_and(|due| due <= now) {
            self.ask_lists(now);
        }
        let next = if self.context.is_null() {
            Some(self.reconnect)
        } else if self.reading_lists {
            None
        } else {
            self.due
        };
        next.map_or(TICK, |next| next.saturating_duration_since(now).min(TICK))
    }

    fn connect(&mut self, now: Instant) {
        self.reconnect = now + RECONNECT;
        // SAFETY: the main loop of this client; the context, new, is freed
        // here on failure, else by `disconnect`.
        unsafe {
            let api = self.pulse.pa_mainloop_get_api(self.mainloop);
            let context = self.pulse.pa_context_new(api, NAME.as_ptr());
            if context.is_null() {
                self.reading = Reading::Unknown;
                return;
            }
            // No server started for Disled: it waits for the session's.
            if self
                .pulse
                .pa_context_connect(context, ptr::null(), CONTEXT_NOAUTOSPAWN, ptr::null())
                < 0
            {
                self.pulse.pa_context_unref(context);
                self.reading = Reading::Unknown;
                return;
            }
            self.pulse.pa_context_set_subscribe_callback(
                context,
                Some(on_change),
                self.heard.as_ptr().cast(),
            );
            self.context = context;
        }
    }

    fn subscribe(&mut self, now: Instant) {
        // SAFETY: a ready context of this client. The operation, unneeded,
        // goes at once: the server keeps the subscription.
        unsafe {
            let operation = self.pulse.pa_context_subscribe(
                self.context,
                SUBSCRIPTION_MASK_SOURCE | SUBSCRIPTION_MASK_SOURCE_OUTPUT,
                None,
                ptr::null_mut(),
            );
            if !operation.is_null() {
                self.pulse.pa_operation_unref(operation);
            }
        }
        self.subscribed = true;
        self.due = Some(now);
    }

    /// Asks for the capture streams and the sources, which the callbacks
    /// gather.
    fn ask_lists(&mut self, now: Instant) {
        let shared = self.heard.as_ptr().cast();
        let heard = self.heard();
        heard.streams.clear();
        heard.sources.clear();
        heard.failed = false;
        heard.pending = 0;
        // SAFETY: a ready context of this client, and its `heard`, which
        // outlives the context. The operations keep running once released.
        let operations = unsafe {
            [
                self.pulse.pa_context_get_source_output_info_list(
                    self.context,
                    Some(on_stream),
                    shared,
                ),
                self.pulse
                    .pa_context_get_source_info_list(self.context, Some(on_source), shared),
            ]
        };
        for operation in operations {
            if operation.is_null() {
                self.heard().failed = true;
            } else {
                self.heard().pending += 1;
                // SAFETY: an operation of this client, released once.
                unsafe { self.pulse.pa_operation_unref(operation) };
            }
        }
        self.reading_lists = true;
        self.due = Some(now + REREAD);
    }

    /// Runs the main loop once, waiting `wait` at most: false when asked to
    /// stop.
    fn iterate(&mut self, wait: Duration, stop: &Stop) -> bool {
        let micros = c_int::try_from(wait.as_micros()).unwrap_or(c_int::MAX);
        // SAFETY: the main loop of this client, on its own thread.
        unsafe {
            if self.pulse.pa_mainloop_prepare(self.mainloop, micros) < 0 {
                return !stop.wait(wait);
            }
            // `prepare` forgets a wakeup that came before it: the question
            // comes after.
            if stop.asked() {
                return false;
            }
            if self.pulse.pa_mainloop_poll(self.mainloop) < 0 {
                return !stop.wait(wait);
            }
            self.pulse.pa_mainloop_dispatch(self.mainloop);
        }
        true
    }

    /// A reading once both lists came.
    fn collect(&mut self) {
        if !self.reading_lists || self.heard().pending > 0 {
            return;
        }
        self.reading_lists = false;
        let heard = self.heard();
        let reading = if heard.failed {
            Reading::Unknown
        } else {
            rules::streams(&heard.streams, &heard.sources, std::process::id())
        };
        self.reading = reading;
    }

    fn heard(&mut self) -> &mut Heard {
        // SAFETY: the callbacks reach it only within `dispatch`, while no
        // such borrow lives.
        unsafe { self.heard.as_mut() }
    }

    fn disconnect(&mut self) {
        if !self.context.is_null() {
            // SAFETY: the context of this client, released once.
            unsafe {
                self.pulse.pa_context_disconnect(self.context);
                self.pulse.pa_context_unref(self.context);
            }
            self.context = ptr::null_mut();
        }
        self.subscribed = false;
        self.reading_lists = false;
        self.heard().pending = 0;
        self.due = None;
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.disconnect();
        // SAFETY: the main loop of this client, freed once, after its
        // context, then what its callbacks filled, which none reaches now.
        unsafe {
            self.pulse.pa_mainloop_free(self.mainloop);
            drop(Box::from_raw(self.heard.as_ptr()));
        }
    }
}

/// A text of a property list, if any.
///
/// # Safety
///
/// `proplist` is null or a property list valid during the call.
unsafe fn property(pulse: &Pulse, proplist: *const Proplist, key: &CStr) -> Option<String> {
    if proplist.is_null() {
        return None;
    }
    // SAFETY: a valid list, as the caller says, and a key ended by NUL.
    let value = unsafe { pulse.pa_proplist_gets(proplist, key.as_ptr()) };
    if value.is_null() {
        return None;
    }
    // SAFETY: a string of the list, ended by NUL, copied at once.
    let value = unsafe { CStr::from_ptr(value) };
    Some(value.to_string_lossy().into_owned())
}

/// The `Heard` behind the userdata of a callback.
///
/// # Safety
///
/// `userdata` is the `heard` of a client, on the thread of its main loop.
unsafe fn heard<'a>(userdata: *mut c_void) -> &'a mut Heard {
    // SAFETY: as the caller says.
    unsafe { &mut *userdata.cast::<Heard>() }
}

/// A source or a capture stream came, went or changed.
unsafe extern "C" fn on_change(_: *mut Context, _: c_int, _: u32, userdata: *mut c_void) {
    // SAFETY: the userdata given with this callback, by `connect`.
    unsafe { heard(userdata) }.changed = true;
}

/// One capture stream, or the end of their list.
unsafe extern "C" fn on_stream(
    _: *mut Context,
    info: *const SourceOutputInfo,
    eol: c_int,
    userdata: *mut c_void,
) {
    // SAFETY: the userdata given with this callback, by `ask_lists`.
    let heard = unsafe { heard(userdata) };
    if eol != 0 {
        heard.failed |= eol < 0;
        heard.pending = heard.pending.saturating_sub(1);
        return;
    }
    let Some(pulse) = pulse() else {
        return;
    };
    // SAFETY: libpulse gives a valid stream for the time of the call.
    let Some(info) = (unsafe { info.as_ref() }) else {
        return;
    };
    // SAFETY: the property list of this stream, valid for the call.
    let (app_id, pid) = unsafe {
        (
            property(pulse, info.proplist, c"application.id"),
            property(pulse, info.proplist, c"application.process.id"),
        )
    };
    heard.streams.push(Stream {
        app_id,
        client: info.client != INVALID_INDEX,
        corked: info.corked != 0,
        source: info.source,
        pid: pid.and_then(|pid| pid.trim().parse().ok()),
    });
}

/// One source, or the end of their list.
unsafe extern "C" fn on_source(
    _: *mut Context,
    info: *const SourceInfo,
    eol: c_int,
    userdata: *mut c_void,
) {
    // SAFETY: the userdata given with this callback, by `ask_lists`.
    let heard = unsafe { heard(userdata) };
    if eol != 0 {
        heard.failed |= eol < 0;
        heard.pending = heard.pending.saturating_sub(1);
        return;
    }
    let Some(pulse) = pulse() else {
        return;
    };
    // SAFETY: libpulse gives a valid source for the time of the call.
    let Some(info) = (unsafe { info.as_ref() }) else {
        return;
    };
    // SAFETY: the name and the property list of this source, valid for
    // the call.
    let (name, class) = unsafe {
        (
            (!info.name.is_null())
                .then(|| CStr::from_ptr(info.name).to_string_lossy().into_owned()),
            property(pulse, info.proplist, c"device.class"),
        )
    };
    heard.sources.push(Source {
        index: info.index,
        running: info.state == SOURCE_RUNNING,
        monitor: rules::is_monitor(
            (info.monitor_of_sink != INVALID_INDEX).then_some(info.monitor_of_sink),
            name.as_deref().unwrap_or_default(),
            class.as_deref(),
        ),
    });
}
