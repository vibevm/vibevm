//! One installer child: started with inherited stdin, its stdout and stderr
//! teed into the step's run log while they still stream live
//! (`PROP-065#runs`), each output line observed as progress detail
//! (`PROP-065#events`), and an interrupt forwarded to it and waited for
//! (`PROP-065#cancellation`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#child-output");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#cancellation");

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use specmark::spec;
use vibe_core::progress::ProgressTask;

use crate::error::InstallerError;

/// The caller's interrupt state; `vibe-app` installs no signal handler.
///
/// ```
/// use std::sync::atomic::{AtomicBool, Ordering};
///
/// /// A flag the caller's signal handler sets.
/// struct Flag(AtomicBool);
/// impl vibe_app::Interrupt for Flag {
///     fn poll(&self, _child: Option<u32>) -> bool {
///         self.0.load(Ordering::SeqCst)
///     }
/// }
/// let flag = Flag(AtomicBool::new(false));
/// assert!(!vibe_app::Interrupt::poll(&flag, None));
/// flag.0.store(true, Ordering::SeqCst);
/// assert!(vibe_app::Interrupt::poll(&flag, Some(42)));
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#cancellation")]
pub trait Interrupt: Sync {
    /// Forward any signal received since the last call to the running
    /// child (`None` between children) and report whether an interrupt
    /// has been requested at all during this command.
    fn poll(&self, child: Option<u32>) -> bool;
}

/// Where a child's output goes besides its run log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Relay {
    /// stdout to stdout, stderr to stderr.
    Live,
    /// Both to stderr, so a JSON envelope keeps stdout to itself.
    Stderr,
    /// Only the run log and the progress detail lines.
    Quiet,
}

/// How one child is observed.
#[derive(Clone, Copy)]
pub(crate) struct ChildIo<'a> {
    pub relay: Relay,
    pub log: Option<&'a Path>,
    /// A second log of standard output alone (an agent's `transcript.log`,
    /// `PROP-064#envelope`).
    pub stdout_log: Option<&'a Path>,
    pub task: Option<&'a ProgressTask>,
    pub interrupt: Option<&'a dyn Interrupt>,
}

#[cfg(test)]
impl ChildIo<'static> {
    /// Live (or, for `json`, stderr-relayed) output with nothing else.
    pub(crate) fn plain(json: bool) -> Self {
        Self {
            relay: if json { Relay::Stderr } else { Relay::Live },
            log: None,
            stdout_log: None,
            task: None,
            interrupt: None,
        }
    }
}

/// The longest child output line carried by one detail observation.
const MAX_DETAIL: usize = 400;

/// How long readers may outlive their child: a grandchild that keeps the
/// pipes open (a daemon started in the background) never holds a step.
const READER_GRACE: Duration = Duration::from_secs(2);

/// Run one child to completion; non-zero, or an interrupt, is an error.
pub(crate) fn run_child(
    command: &mut Command,
    what: &str,
    observe: &ChildIo<'_>,
) -> Result<(), InstallerError> {
    let spawn_error = |error: io::Error| InstallerError::Spawn {
        what: what.into(),
        reason: error.to_string(),
    };
    let log = match observe.log {
        Some(path) => Some(Arc::new(Mutex::new(open_log(path)?))),
        None => None,
    };
    let stdout_log = match observe.stdout_log {
        Some(path) => Some(Arc::new(Mutex::new(open_log(path)?))),
        None => None,
    };
    command
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(spawn_error)?;
    let pid = child.id();
    let (lines, received) = mpsc::channel::<String>();
    let streams: [(Option<Box<dyn Read + Send>>, bool); 2] = [
        (child.stdout.take().map(|s| Box::new(s) as _), false),
        (child.stderr.take().map(|s| Box::new(s) as _), true),
    ];
    for (stream, stderr) in streams {
        if let Some(stream) = stream {
            let pump = Pump {
                relay: observe.relay,
                stderr,
                log: log.clone(),
                stdout_log: if stderr { None } else { stdout_log.clone() },
                lines: lines.clone(),
                observed: observe.task.is_some(),
            };
            std::thread::spawn(move || pump.run(stream));
        }
    }
    drop(lines);
    let (mut status, mut interrupted, mut readers_done) = (None, false, false);
    let mut deadline = Instant::now();
    loop {
        match received.recv_timeout(Duration::from_millis(20)) {
            Ok(line) => {
                if let Some(task) = observe.task {
                    detail(task, &line);
                }
                continue;
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => readers_done = true,
        }
        match status {
            None => {
                if let Some(interrupt) = observe.interrupt {
                    interrupted |= interrupt.poll(Some(pid));
                }
                match child.try_wait().map_err(spawn_error)? {
                    Some(exit) => {
                        status = Some(exit);
                        deadline = Instant::now() + READER_GRACE;
                    }
                    None if readers_done => std::thread::sleep(Duration::from_millis(20)),
                    None => {}
                }
            }
            Some(_) if readers_done || Instant::now() >= deadline => break,
            Some(_) => {}
        }
    }
    let status = status.ok_or_else(|| spawn_error(io::Error::other("no exit status")))?;
    verdict(status, interrupted, what)
}

fn verdict(
    status: std::process::ExitStatus,
    interrupted: bool,
    what: &str,
) -> Result<(), InstallerError> {
    if interrupted {
        return Err(InstallerError::Interrupted { what: what.into() });
    }
    if status.success() {
        return Ok(());
    }
    Err(InstallerError::Exited {
        what: what.into(),
        code: status.code().map_or_else(
            || "without a code".into(),
            |code| format!("with code {code}"),
        ),
    })
}

/// Run one child attached to the terminal: every standard stream is
/// inherited, nothing is captured (an interactive agent session,
/// `PROP-064#modes`); an interrupt is still forwarded and waited for.
pub(crate) fn run_attached(
    command: &mut Command,
    what: &str,
    interrupt: Option<&dyn Interrupt>,
) -> Result<(), InstallerError> {
    let spawn_error = |error: io::Error| InstallerError::Spawn {
        what: what.into(),
        reason: error.to_string(),
    };
    command
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let mut child = command.spawn().map_err(spawn_error)?;
    let pid = child.id();
    let mut interrupted = false;
    loop {
        if let Some(interrupt) = interrupt {
            interrupted |= interrupt.poll(Some(pid));
        }
        if let Some(status) = child.try_wait().map_err(spawn_error)? {
            return verdict(status, interrupted, what);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Never truncate a log being written: open it for appending.
fn open_log(path: &Path) -> Result<File, InstallerError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| super::io("creating", parent, source))?;
    }
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|source| super::io("opening", path, source))
}

/// One reader thread: copies its stream into the log and the relay, and
/// sends its lines for detail observations; dropping its sender tells the
/// waiter the stream ended. Output failures never stop the child.
struct Pump {
    relay: Relay,
    stderr: bool,
    log: Option<Arc<Mutex<File>>>,
    stdout_log: Option<Arc<Mutex<File>>>,
    lines: mpsc::Sender<String>,
    observed: bool,
}

impl Pump {
    fn run(self, mut stream: impl Read) {
        let mut buffer = [0u8; 8192];
        let mut line = Vec::new();
        loop {
            let read = match stream.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(read) => read,
            };
            let chunk = &buffer[..read];
            for log in [&self.log, &self.stdout_log].into_iter().flatten() {
                if let Ok(mut file) = log.lock() {
                    let _ = file.write_all(chunk);
                }
            }
            match (self.relay, self.stderr) {
                (Relay::Live, false) => {
                    let mut out = io::stdout().lock();
                    let _ = out.write_all(chunk).and_then(|()| out.flush());
                }
                (Relay::Live | Relay::Stderr, _) => {
                    let mut err = io::stderr().lock();
                    let _ = err.write_all(chunk).and_then(|()| err.flush());
                }
                (Relay::Quiet, _) => {}
            }
            if !self.observed {
                continue;
            }
            for byte in chunk {
                if *byte == b'\n' {
                    let _ = self.lines.send(String::from_utf8_lossy(&line).into_owned());
                    line.clear();
                } else if line.len() < MAX_DETAIL * 4 {
                    line.push(*byte);
                }
            }
        }
        if self.observed && !line.is_empty() {
            let _ = self.lines.send(String::from_utf8_lossy(&line).into_owned());
        }
    }
}

fn detail(task: &ProgressTask, line: &str) {
    let text = line.trim_end_matches('\r');
    let bounded: String = text.chars().take(MAX_DETAIL).collect();
    task.detail(bounded);
}

#[cfg(all(test, unix))]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};

    use vibe_core::progress::{Progress, ProgressEvent, ProgressEventKind, ProgressObserver};

    use super::*;

    struct Details(Mutex<Vec<String>>);
    impl ProgressObserver for Details {
        fn observe(&self, event: ProgressEvent) {
            if let ProgressEventKind::Detail { message } = event.kind {
                self.0.lock().unwrap().push(message);
            }
        }
    }

    struct Flag(AtomicBool);
    impl Interrupt for Flag {
        fn poll(&self, child: Option<u32>) -> bool {
            if let Some(pid) = child
                && self.0.swap(false, Ordering::SeqCst)
            {
                let _ = Command::new("kill").arg(pid.to_string()).status();
                return true;
            }
            false
        }
    }

    #[test]
    fn output_is_teed_into_the_log_and_observed_line_by_line() {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("steps/build/output.log");
        fs::create_dir_all(log.parent().unwrap()).unwrap();
        fs::write(&log, "earlier\n").unwrap();
        let details = Arc::new(Details(Mutex::new(Vec::new())));
        let task = Progress::new(details.clone()).task("build");
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "echo one; echo two >&2; printf tail"]);
        let transcript = dir.path().join("transcript.log");
        let observe = ChildIo {
            relay: Relay::Quiet,
            log: Some(&log),
            stdout_log: Some(&transcript),
            task: Some(&task),
            interrupt: None,
        };
        run_child(&mut command, "recipe", &observe).unwrap();
        let text = fs::read_to_string(&log).unwrap();
        assert!(text.starts_with("earlier\n"), "never truncated: {text}");
        // stdout and stderr are pumped by two threads: their order in the log
        // is not fixed, their content is.
        for piece in ["one\n", "two\n", "tail"] {
            assert!(text.contains(piece), "{piece:?} missing from {text:?}");
        }
        assert_eq!(
            fs::read_to_string(&transcript).unwrap(),
            "one\ntail",
            "the transcript keeps standard output alone"
        );
        let mut lines = details.0.lock().unwrap().clone();
        lines.sort();
        assert_eq!(lines, ["one", "tail", "two"]);
    }

    #[test]
    fn an_interrupt_reaches_the_child_and_is_reported() {
        let flag = Flag(AtomicBool::new(true));
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "sleep 5"]);
        let observe = ChildIo {
            interrupt: Some(&flag),
            ..ChildIo::plain(true)
        };
        let started = std::time::Instant::now();
        let error = run_child(&mut command, "recipe", &observe).unwrap_err();
        assert!(
            matches!(error, InstallerError::Interrupted { .. }),
            "{error}"
        );
        assert!(started.elapsed() < Duration::from_secs(4));
    }
}
