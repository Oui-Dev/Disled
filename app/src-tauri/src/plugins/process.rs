// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! The programs of the plugins. Disled runs the command of the manifest in
//! the folder of the plugin, reads one JSON-RPC 2.0 notification per line on
//! its output, as the stdio transport of MCP does, keeps its error output as
//! its log, and stops it by closing its input. A program that crashes starts
//! again after 1, 5, 30 then 60 s, until its fifth crash in ten minutes. A
//! job object on Windows, a process group elsewhere, and on Linux the death
//! signal of its parent, take its own children down with it.

use std::collections::VecDeque;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::Value;

/// A line of the output, at most.
const MAX_LINE: usize = 64 * 1024;
/// The log of a plugin, before it moves to its `.1` file.
const MAX_LOG: u64 = 256 * 1024;
/// How often the supervisor looks at its program.
const POLL: Duration = Duration::from_millis(100);

/// What a program asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notification {
    /// A key takes a state, the key `default` without one.
    State {
        state: String,
        key: Option<String>,
    },
    /// A key leaves, or all of them without one.
    Clear {
        key: Option<String>,
    },
    Alert,
}

/// A line of the output: a JSON-RPC 2.0 notification, or none.
pub fn parse(line: &str) -> Option<Notification> {
    let value: Value = serde_json::from_str(line).ok()?;
    if value.get("jsonrpc")? != "2.0" {
        return None;
    }
    let params = value.get("params");
    let key = params
        .and_then(|params| params.get("key"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    match value.get("method")?.as_str()? {
        "state" => Some(Notification::State {
            state: params?.get("state")?.as_str()?.to_owned(),
            key,
        }),
        "clear" => Some(Notification::Clear { key }),
        "alert" => Some(Notification::Alert),
        _ => None,
    }
}

/// Batch files are refused: Rust runs them only with an escape that a
/// plugin could get wrong.
fn allowed(path: &Path) -> bool {
    !path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("bat") || extension.eq_ignore_ascii_case("cmd")
        })
}

/// Where the program is: a path inside the folder of the plugin, never out
/// of it, or a name on the PATH. An app started from the Finder does not get
/// the PATH of the shell, so macOS also looks in the folders of Homebrew, and
/// macOS and Linux in `~/.local/bin`.
pub fn resolve(
    program: &str,
    folder: &Path,
    path: Option<OsString>,
    home: Option<PathBuf>,
) -> Option<PathBuf> {
    if program.contains('/') || program.contains('\\') {
        let folder = folder.canonicalize().ok()?;
        let candidate = folder.join(program).canonicalize().ok()?;
        return (candidate.starts_with(&folder) && candidate.is_file() && allowed(&candidate))
            .then_some(candidate);
    }
    let mut folders: Vec<PathBuf> = path
        .map(|path| std::env::split_paths(&path).collect())
        .unwrap_or_default();
    if cfg!(target_os = "macos") {
        folders.push("/opt/homebrew/bin".into());
        folders.push("/usr/local/bin".into());
    }
    if cfg!(unix)
        && let Some(home) = home
    {
        folders.push(home.join(".local/bin"));
    }
    let names: Vec<String> = if cfg!(windows) && Path::new(program).extension().is_none() {
        vec![format!("{program}.exe"), format!("{program}.com")]
    } else {
        vec![program.to_owned()]
    };
    folders
        .iter()
        .flat_map(|folder| names.iter().map(move |name| folder.join(name)))
        .find(|candidate| candidate.is_file() && allowed(candidate))
}

/// How a program fares, for the details of its plugin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProgramStatus {
    Running,
    /// It stopped, with the last line of its error output.
    Stopped {
        error: Option<String>,
    },
    /// The program of the command is nowhere.
    NotFound {
        program: String,
    },
    /// Five crashes in ten minutes: it waits to be turned off and on again.
    Failed,
}

/// What a program needs to run.
#[derive(Debug, Clone)]
pub struct Spec {
    pub id: String,
    pub program: PathBuf,
    pub arguments: Vec<String>,
    pub folder: PathBuf,
    /// Its own folder for data, outside its folder, whose fingerprint would
    /// change.
    pub data: PathBuf,
    pub language: String,
    pub log: PathBuf,
}

/// The delays of the supervisor.
#[derive(Debug, Clone, Copy)]
pub struct Timing {
    pub restarts: [Duration; 4],
    pub window: Duration,
    pub crashes: usize,
    /// How long a program has to stop by itself once its input closes.
    pub grace: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            restarts: [1, 5, 30, 60].map(Duration::from_secs),
            window: Duration::from_secs(600),
            crashes: 5,
            grace: Duration::from_secs(2),
        }
    }
}

/// A program under supervision, until `stop`.
pub struct Program {
    stop: mpsc::Sender<()>,
    thread: Option<JoinHandle<()>>,
}

impl Program {
    pub fn start(
        spec: Spec,
        timing: Timing,
        on_note: impl Fn(Notification) + Send + Sync + 'static,
        on_status: impl Fn(ProgramStatus) + Send + 'static,
    ) -> Self {
        let (stop, stopped) = mpsc::channel();
        let thread = thread::Builder::new()
            .name(format!("disled-plugin-{}", spec.id))
            .spawn(move || supervise(&spec, timing, &stopped, Arc::new(on_note), &on_status))
            .ok();
        Self { stop, thread }
    }

    /// Closes the input of the program, gives it its grace to stop, then
    /// ends its whole tree.
    pub fn stop(mut self) {
        let _ = self.stop.send(());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn supervise(
    spec: &Spec,
    timing: Timing,
    stopped: &mpsc::Receiver<()>,
    on_note: Arc<dyn Fn(Notification) + Send + Sync>,
    on_status: &dyn Fn(ProgramStatus),
) {
    let mut crashes: VecDeque<Instant> = VecDeque::new();
    loop {
        let mut running = match launch(spec, Arc::clone(&on_note)) {
            Ok(running) => running,
            Err(error) => {
                on_status(ProgramStatus::Stopped {
                    error: Some(error.to_string()),
                });
                let _ = stopped.recv();
                return;
            }
        };
        on_status(ProgramStatus::Running);
        loop {
            match stopped.recv_timeout(POLL) {
                Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                    running.end(timing.grace);
                    return;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            if matches!(running.child.try_wait(), Ok(Some(_)) | Err(_)) {
                break;
            }
        }
        let error = running.end(Duration::ZERO);
        let now = Instant::now();
        crashes.push_back(now);
        while crashes
            .front()
            .is_some_and(|first| now.duration_since(*first) > timing.window)
        {
            crashes.pop_front();
        }
        if crashes.len() >= timing.crashes {
            on_status(ProgramStatus::Failed);
            let _ = stopped.recv();
            return;
        }
        on_status(ProgramStatus::Stopped { error });
        let delay = timing.restarts[(crashes.len() - 1).min(timing.restarts.len() - 1)];
        if !matches!(
            stopped.recv_timeout(delay),
            Err(mpsc::RecvTimeoutError::Timeout)
        ) {
            return;
        }
    }
}

/// A program that runs, with what takes it down.
struct Running {
    child: Child,
    input: Option<ChildStdin>,
    #[cfg(windows)]
    job: Option<job::Job>,
    readers: Vec<JoinHandle<()>>,
    last_error: Arc<Mutex<Option<String>>>,
}

impl Running {
    /// Closes the input, waits at most `grace`, then ends the tree. Returns
    /// the last line of the error output.
    fn end(&mut self, grace: Duration) -> Option<String> {
        drop(self.input.take());
        let deadline = Instant::now() + grace;
        while Instant::now() < deadline && matches!(self.child.try_wait(), Ok(None)) {
            thread::sleep(Duration::from_millis(20));
        }
        #[cfg(windows)]
        if let Some(job) = self.job.take() {
            job.terminate();
        }
        #[cfg(unix)]
        if let Ok(group) = i32::try_from(self.child.id()) {
            // SAFETY: killpg only sends a signal to the group of the
            // program, which `process_group(0)` made.
            unsafe {
                libc::killpg(group, libc::SIGKILL);
            }
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        for reader in self.readers.drain(..) {
            let _ = reader.join();
        }
        self.last_error.lock().ok().and_then(|last| last.clone())
    }
}

fn launch(spec: &Spec, on_note: Arc<dyn Fn(Notification) + Send + Sync>) -> io::Result<Running> {
    let mut command = Command::new(&spec.program);
    command
        .args(&spec.arguments)
        .current_dir(&spec.folder)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("DISLED_API", "1")
        .env("DISLED_PLUGIN_ID", &spec.id)
        .env("DISLED_PLUGIN_DATA", &spec.data)
        .env("DISLED_LANG", &spec.language);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Without it, a console program started by a GUI app opens a
        // console window.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;
        // SAFETY: prctl only asks the kernel to signal the program when the
        // app dies, and allocates nothing between the fork and the exec.
        unsafe {
            command.pre_exec(|| {
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
                Ok(())
            });
        }
    }
    let _ = fs::create_dir_all(&spec.data);
    let mut child = command.spawn()?;
    #[cfg(windows)]
    let job = job::Job::holding(&child).ok();
    let log = Arc::new(Mutex::new(Log::open(spec.log.clone())));
    let last_error = Arc::new(Mutex::new(None));
    let mut readers = Vec::new();
    if let Some(output) = child.stdout.take() {
        let log = Arc::clone(&log);
        readers.push(thread::spawn(move || {
            lines(output, |line| match line.and_then(parse) {
                Some(note) => on_note(note),
                None => log
                    .lock()
                    .map(|mut log| log.write("refused line"))
                    .unwrap_or(()),
            });
        }));
    }
    if let Some(errors) = child.stderr.take() {
        let last = Arc::clone(&last_error);
        readers.push(thread::spawn(move || {
            lines(errors, |line| {
                let line = line.unwrap_or("(a line too long)");
                if let Ok(mut log) = log.lock() {
                    log.write(line);
                }
                if !line.trim().is_empty()
                    && let Ok(mut last) = last.lock()
                {
                    *last = Some(line.trim().to_owned());
                }
            });
        }));
    }
    Ok(Running {
        input: child.stdin.take(),
        child,
        #[cfg(windows)]
        job,
        readers,
        last_error,
    })
}

/// Calls `each` with every line of `reader`, none for a line of more than
/// `MAX_LINE` bytes or not in UTF-8, until the end.
fn lines(reader: impl Read, mut each: impl FnMut(Option<&str>)) {
    let mut reader = BufReader::new(reader);
    let mut line = Vec::new();
    loop {
        line.clear();
        let read = (&mut reader)
            .take(MAX_LINE as u64 + 1)
            .read_until(b'\n', &mut line);
        match read {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        if line.len() > MAX_LINE && line.last() != Some(&b'\n') {
            // The rest of the line goes too.
            let mut rest = Vec::new();
            if reader.read_until(b'\n', &mut rest).is_err() {
                return;
            }
            each(None);
            continue;
        }
        let text = std::str::from_utf8(&line)
            .ok()
            .map(|text| text.trim_end_matches(['\n', '\r']));
        each(text);
    }
}

/// The log of a plugin: its error output and the lines that Disled refused,
/// 256 KiB at most, then once more in its `.1` file.
struct Log {
    path: PathBuf,
    file: Option<File>,
}

impl Log {
    fn open(path: PathBuf) -> Self {
        if let Some(folder) = path.parent() {
            let _ = fs::create_dir_all(folder);
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .ok();
        Self { path, file }
    }

    fn write(&mut self, line: &str) {
        let full = self
            .file
            .as_ref()
            .and_then(|file| file.metadata().ok())
            .is_some_and(|metadata| metadata.len() > MAX_LOG);
        if full {
            self.file = None;
            let _ = fs::rename(&self.path, self.path.with_extension("log.1"));
            self.file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
                .ok();
        }
        if let Some(file) = &mut self.file {
            let _ = writeln!(file, "{line}");
        }
    }
}

/// On Windows, a job object that ends the tree of a program when Disled
/// closes it, or dies.
#[cfg(windows)]
mod job {
    use std::io;
    use std::os::windows::io::AsRawHandle;
    use std::process::Child;

    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
        SetInformationJobObject, TerminateJobObject,
    };
    use windows::core::PCWSTR;

    pub struct Job(HANDLE);

    // SAFETY: a job handle may be used and closed from any thread.
    unsafe impl Send for Job {}

    impl Job {
        /// A job that holds the program and every process it starts.
        pub fn holding(child: &Child) -> io::Result<Self> {
            // SAFETY: the calls get a handle that this job owns and closes,
            // a limit structure of the size given, and the handle of a
            // program that is still running.
            unsafe {
                let job = Job(CreateJobObjectW(None, PCWSTR::null()).map_err(io::Error::other)?);
                let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
                limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                SetInformationJobObject(
                    job.0,
                    JobObjectExtendedLimitInformation,
                    std::ptr::from_ref(&limits).cast(),
                    u32::try_from(size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>())
                        .unwrap_or(u32::MAX),
                )
                .map_err(io::Error::other)?;
                AssignProcessToJobObject(job.0, HANDLE(child.as_raw_handle()))
                    .map_err(io::Error::other)?;
                Ok(job)
            }
        }

        pub fn terminate(self) {
            // SAFETY: the handle is that of this job, which still owns it.
            unsafe {
                let _ = TerminateJobObject(self.0, 1);
            }
        }
    }

    impl Drop for Job {
        fn drop(&mut self) {
            // SAFETY: the handle is that of this job, closed once.
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Not a test: the test program itself runs this, as the program of a
    /// plugin, when the tests below start it with a `mode=` argument, which
    /// matches no test name.
    #[test]
    fn program() {
        let Some(mode) =
            std::env::args().find_map(|arg| arg.strip_prefix("mode=").map(str::to_owned))
        else {
            return;
        };
        // The test harness may have written the name of the test on this
        // line already.
        println!();
        println!(r#"{{"jsonrpc":"2.0","method":"state","params":{{"state":"work"}}}}"#);
        println!("not json");
        println!(r#"{{"jsonrpc":"2.0","method":"alert"}}"#);
        io::stdout().flush().expect("flush");
        match mode.as_str() {
            "crash" => {
                eprintln!("boom");
                std::process::exit(3);
            }
            "obey" => {
                // Ends when its input closes.
                let _ = io::stdin().read_to_end(&mut Vec::new());
            }
            _ => thread::sleep(Duration::from_secs(60)),
        }
    }

    fn spec(folder: &Path, mode: &str) -> Spec {
        Spec {
            id: format!("test-{mode}"),
            program: std::env::current_exe().expect("test program"),
            arguments: vec![
                "plugins::process::tests::program".into(),
                "--exact".into(),
                "--nocapture".into(),
                "--test-threads=1".into(),
                format!("mode={mode}"),
            ],
            folder: folder.to_owned(),
            data: folder.join("data"),
            language: "fr".into(),
            log: folder.join(format!("{mode}.log")),
        }
    }

    fn fast() -> Timing {
        Timing {
            restarts: [Duration::from_millis(10); 4],
            grace: Duration::from_millis(300),
            ..Timing::default()
        }
    }

    /// Starts a program and follows what it says and how it fares.
    fn start(
        spec: Spec,
        timing: Timing,
    ) -> (
        Program,
        mpsc::Receiver<Notification>,
        mpsc::Receiver<ProgramStatus>,
    ) {
        let (notes, noted) = mpsc::channel();
        let (statuses, status) = mpsc::channel();
        let notes = Mutex::new(notes);
        let program = Program::start(
            spec,
            timing,
            move |note| {
                let _ = notes.lock().map(|notes| notes.send(note));
            },
            move |next| {
                let _ = statuses.send(next);
            },
        );
        (program, noted, status)
    }

    #[test]
    fn a_line_is_a_notification_or_nothing() {
        assert_eq!(
            parse(r#"{"jsonrpc":"2.0","method":"state","params":{"state":"work","key":"t2"}}"#),
            Some(Notification::State {
                state: "work".into(),
                key: Some("t2".into())
            })
        );
        assert_eq!(
            parse(r#"{"jsonrpc":"2.0","method":"clear"}"#),
            Some(Notification::Clear { key: None })
        );
        assert_eq!(
            parse(r#"{"jsonrpc":"2.0","method":"alert"}"#),
            Some(Notification::Alert)
        );
        assert_eq!(parse(r#"{"jsonrpc":"1.0","method":"alert"}"#), None);
        assert_eq!(parse(r#"{"jsonrpc":"2.0","method":"state"}"#), None);
        assert_eq!(parse("alert"), None);
    }

    #[test]
    fn a_long_line_is_dropped_whole() {
        let text = format!("{}\nshort\n", "a".repeat(MAX_LINE + 10));
        let mut seen = Vec::new();
        lines(text.as_bytes(), |line| seen.push(line.map(str::to_owned)));
        assert_eq!(seen, [None, Some("short".into())]);
    }

    #[test]
    fn a_program_stays_in_its_folder() {
        let folder = tempfile::tempdir().expect("folder");
        let plugin = folder.path().join("demo");
        fs::create_dir_all(plugin.join("bin")).expect("plugin");
        fs::write(plugin.join("bin/run"), "").expect("program");
        fs::write(plugin.join("bin/run.cmd"), "").expect("batch");
        fs::write(folder.path().join("outside"), "").expect("outside");
        assert!(resolve("bin/run", &plugin, None, None).is_some());
        assert_eq!(resolve("../outside", &plugin, None, None), None);
        assert_eq!(resolve("bin/run.cmd", &plugin, None, None), None);
        assert_eq!(resolve("bin/missing", &plugin, None, None), None);
        // A name comes from the PATH.
        let path = std::env::join_paths([plugin.join("bin")]).expect("PATH");
        let name = if cfg!(windows) { "run.exe" } else { "run" };
        if cfg!(windows) {
            fs::write(plugin.join("bin/run.exe"), "").expect("program");
        }
        assert_eq!(
            resolve("run", &plugin, Some(path), None),
            Some(plugin.join("bin").join(name))
        );
        assert_eq!(resolve("run", &plugin, None, None), None);
    }

    #[test]
    fn a_program_that_crashes_starts_again_then_gives_up() {
        let folder = tempfile::tempdir().expect("folder");
        let (program, noted, status) = start(spec(folder.path(), "crash"), fast());
        let mut statuses = Vec::new();
        while let Ok(next) = status.recv_timeout(Duration::from_secs(30)) {
            let failed = next == ProgramStatus::Failed;
            statuses.push(next);
            if failed {
                break;
            }
        }
        program.stop();
        assert_eq!(statuses.last(), Some(&ProgramStatus::Failed));
        let runs = statuses
            .iter()
            .filter(|status| **status == ProgramStatus::Running)
            .count();
        assert_eq!(runs, 5);
        assert!(statuses.contains(&ProgramStatus::Stopped {
            error: Some("boom".into())
        }));
        let notes: Vec<Notification> = noted.try_iter().collect();
        assert!(notes.contains(&Notification::State {
            state: "work".into(),
            key: None
        }));
        assert!(notes.contains(&Notification::Alert));
        let log = fs::read_to_string(folder.path().join("crash.log")).expect("log");
        assert!(log.contains("boom") && log.contains("refused line"));
    }

    #[test]
    fn a_program_stops_when_its_input_closes() {
        let folder = tempfile::tempdir().expect("folder");
        let timing = Timing {
            grace: Duration::from_secs(20),
            ..fast()
        };
        let (program, noted, status) = start(spec(folder.path(), "obey"), timing);
        assert_eq!(
            status.recv_timeout(Duration::from_secs(30)),
            Ok(ProgramStatus::Running)
        );
        let _ = noted.recv_timeout(Duration::from_secs(30));
        let asked = Instant::now();
        program.stop();
        // Well before its grace ran out.
        assert!(asked.elapsed() < Duration::from_secs(10));
    }

    #[test]
    fn a_program_that_does_not_stop_is_ended() {
        let folder = tempfile::tempdir().expect("folder");
        let (program, noted, status) = start(spec(folder.path(), "deaf"), fast());
        assert_eq!(
            status.recv_timeout(Duration::from_secs(30)),
            Ok(ProgramStatus::Running)
        );
        let _ = noted.recv_timeout(Duration::from_secs(30));
        let asked = Instant::now();
        program.stop();
        assert!(asked.elapsed() < Duration::from_secs(10), "it slept 60 s");
    }
}
