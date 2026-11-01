//! The running registry of a home (`PROP-059#launcher-runtime`): a detached
//! start leaves `<home>/applications/running/<pid>.json`, `vibe app running`
//! answers the records whose process is still alive and prunes the rest,
//! and `vibe app stop KEY|PID` ends one the way `PROP-065#cancellation`
//! ends a step — ask, wait a bounded time, then end it without asking.
//!
//! The platform's process calls are behind [`ProcessTable`]: this crate
//! reads and writes files and decides, the CLI owns the system calls.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime");

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use specmark::spec;
use vibe_core::manifest::LauncherKind;
use vibe_wire::generated::application_running as wire;
use vibe_wire::generated::application_start as start_wire;

use super::events::{EventKind, HomeEvent};
use crate::error::{AppError, StateError};
use crate::error_catalog::CatalogError;
use crate::settings::{SettingsRoot, write_atomic};

/// The protocol tag of the `running` and `stop` answers.
pub const RUNNING_PROTOCOL: &str = "vibe-app-running/1";

/// How long a stop waits between asking and ending without asking, and how
/// long it waits after that for the system to reap the process.
const PATIENCE: Duration = Duration::from_secs(5);
const AFTER_KILL: Duration = Duration::from_secs(2);
const POLL: Duration = Duration::from_millis(50);

/// What the platform knows of one process id right now.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime")]
pub enum ProcessState {
    /// No such process, or one that has exited and waits only to be reaped.
    Gone,
    /// A live process; `started` is the platform's own start time when it
    /// gives one, an opaque token compared for equality so a reused process
    /// id is not mistaken for the launcher that had it.
    Alive { started: Option<String> },
}

/// The system calls behind the running registry: probe a process, ask it to
/// end, end it without asking. The CLI implements it per platform (`kill`
/// on Unix, `OpenProcess` and friends on Windows); a test passes a table of
/// its own.
///
/// ```
/// use vibe_app::{ProcessState, ProcessTable, SettingsRoot};
/// use vibe_core::manifest::LauncherKind;
///
/// /// A system where nothing is alive any more.
/// struct Empty;
/// impl ProcessTable for Empty {
///     fn probe(&self, _pid: u32) -> ProcessState { ProcessState::Gone }
///     fn ask_to_end(&self, _pid: u32, _kind: LauncherKind) -> bool { false }
///     fn force_end(&self, _pid: u32) -> bool { false }
/// }
///
/// let dir = tempfile::tempdir().unwrap();
/// let settings = SettingsRoot::new(dir.path());
/// let answer = vibe_app::running(&settings, &Empty);
/// assert_eq!(answer.protocol, "vibe-app-running/1");
/// assert!(answer.running.is_empty() && answer.stopped.is_empty());
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime")]
pub trait ProcessTable {
    /// What the platform knows of `pid` now.
    fn probe(&self, pid: u32) -> ProcessState;
    /// Ask the process to end: `SIGTERM` to its process group on Unix, a
    /// console break on Windows for a launcher that kept the console.
    /// Whether the request could be delivered.
    fn ask_to_end(&self, pid: u32, kind: LauncherKind) -> bool;
    /// End the process without asking: `SIGKILL`, `TerminateProcess`.
    fn force_end(&self, pid: u32) -> bool;
}

/// One record of the registry, as the start wrote it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RunningRecord {
    pub(crate) key: String,
    pub(crate) pid: u32,
    /// RFC 3339 time of the start.
    pub(crate) started: String,
    pub(crate) kind: wire::LauncherKind,
    pub(crate) command: String,
    pub(crate) log: String,
    /// The platform's start time of `pid` read right after the spawn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) process_start: Option<String>,
}

impl RunningRecord {
    fn alive(&self, processes: &dyn ProcessTable) -> bool {
        match processes.probe(self.pid) {
            ProcessState::Gone => false,
            ProcessState::Alive { started } => match (&self.process_start, started) {
                (Some(recorded), Some(now)) => *recorded == now,
                _ => true,
            },
        }
    }

    fn to_wire(&self) -> wire::RunningLauncher {
        wire::RunningLauncher {
            key: self.key.clone(),
            pid: u64::from(self.pid),
            started: self.started.clone(),
            kind: self.kind.clone(),
            command: self.command.clone(),
            log: self.log.clone(),
        }
    }
}

fn wire_kind(kind: LauncherKind) -> wire::LauncherKind {
    match kind {
        LauncherKind::Gui => wire::LauncherKind::Gui,
        LauncherKind::Tui => wire::LauncherKind::Tui,
        LauncherKind::Command => wire::LauncherKind::Command,
    }
}

fn core_kind(kind: &wire::LauncherKind) -> LauncherKind {
    match kind {
        wire::LauncherKind::Gui => LauncherKind::Gui,
        wire::LauncherKind::Tui => LauncherKind::Tui,
        wire::LauncherKind::Command => LauncherKind::Command,
    }
}

/// Record a detached start in the registry. No lock: the file is named by
/// the process id and replaced atomically, so it never collides with
/// another start, and a prune that races it only ever removes the record
/// of a process that is gone.
pub(crate) fn register(
    settings: &SettingsRoot,
    answer: &start_wire::ApplicationStart,
    kind: LauncherKind,
    processes: &dyn ProcessTable,
) -> Result<(), StateError> {
    let pid = u32::try_from(answer.pid).unwrap_or(u32::MAX);
    let record = RunningRecord {
        key: answer.key.clone(),
        pid,
        started: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        kind: wire_kind(kind),
        command: answer.command.clone(),
        log: answer.log.clone(),
        process_start: match processes.probe(pid) {
            ProcessState::Alive { started } => started,
            ProcessState::Gone => None,
        },
    };
    let path = settings.running_dir().join(format!("{pid}.json"));
    let mut bytes = serde_json::to_vec_pretty(&record).map_err(|error| StateError::Io {
        action: "encoding",
        path: path.display().to_string(),
        source: std::io::Error::other(error),
    })?;
    bytes.push(b'\n');
    write_atomic(&path, &bytes)
}

/// Every record of the registry with the file it lives in; a file that does
/// not parse is returned as `None` so a prune removes it.
fn records(settings: &SettingsRoot) -> Vec<(PathBuf, Option<RunningRecord>)> {
    let Ok(entries) = fs::read_dir(settings.running_dir()) else {
        return Vec::new();
    };
    let mut found: Vec<_> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .map(|path| {
            let record = fs::read(&path)
                .ok()
                .and_then(|bytes| serde_json::from_slice(&bytes).ok());
            (path, record)
        })
        .collect();
    found.sort_by(|(_, a), (_, b)| {
        let order = |record: &Option<RunningRecord>| {
            record
                .as_ref()
                .map(|record| (record.started.clone(), record.pid))
        };
        order(a).cmp(&order(b))
    });
    found
}

/// Remove the records of processes that are gone and append one `exited`
/// line each, under the home's lock. A lock another command holds is not
/// waited for: the dead records stay until the next listing, which is all
/// a delay costs. Returns the live records.
pub(crate) fn sweep(settings: &SettingsRoot, processes: &dyn ProcessTable) -> Vec<RunningRecord> {
    let mut live = Vec::new();
    let mut dead = Vec::new();
    for (path, record) in records(settings) {
        match record {
            Some(record) if record.alive(processes) => live.push(record),
            record => dead.push((path, record)),
        }
    }
    if !dead.is_empty()
        && let Ok(_lock) = settings.lock()
    {
        for (path, record) in dead {
            if fs::remove_file(&path).is_ok()
                && let Some(record) = record
            {
                let exited =
                    HomeEvent::about(EventKind::Exited, record.key).with_pid(u64::from(record.pid));
                let _ = super::events::record_event(settings, &exited);
            }
        }
    }
    live
}

/// `vibe app running`: the launchers started through vibe that are still
/// alive, oldest first; the records of the others are pruned.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime")]
pub fn running(settings: &SettingsRoot, processes: &dyn ProcessTable) -> wire::ApplicationRunning {
    answer(settings, &sweep(settings, processes), Vec::new())
}

fn answer(
    settings: &SettingsRoot,
    live: &[RunningRecord],
    stopped: Vec<wire::StoppedLauncher>,
) -> wire::ApplicationRunning {
    wire::ApplicationRunning {
        protocol: RUNNING_PROTOCOL.into(),
        home: settings.path().display().to_string(),
        running: live.iter().map(RunningRecord::to_wire).collect(),
        stopped,
    }
}

/// Whether a record answers to what `vibe app stop` was given: its process
/// id, its launcher key, or the package key of its launcher.
fn matches(record: &RunningRecord, target: &str) -> bool {
    target.parse::<u32>().ok() == Some(record.pid)
        || record.key == target
        || super::split_launcher_key(&record.key).is_some_and(|(package, _)| package == target)
}

fn wait_until_gone(record: &RunningRecord, processes: &dyn ProcessTable, limit: Duration) -> bool {
    let deadline = Instant::now() + limit;
    loop {
        if !record.alive(processes) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(POLL);
    }
}

/// `vibe app stop KEY|PID`: ask every matching live launcher to end, wait a
/// bounded time, end what is left without asking, then prune the registry
/// and answer what was stopped beside what still runs. Nothing matching is
/// the refusal `not_running`, before any effect.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-runtime")]
pub fn stop(
    settings: &SettingsRoot,
    target: &str,
    processes: &dyn ProcessTable,
) -> Result<wire::ApplicationRunning, AppError> {
    stop_within(settings, target, processes, PATIENCE)
}

pub(crate) fn stop_within(
    settings: &SettingsRoot,
    target: &str,
    processes: &dyn ProcessTable,
    patience: Duration,
) -> Result<wire::ApplicationRunning, AppError> {
    let chosen: Vec<RunningRecord> = records(settings)
        .into_iter()
        .filter_map(|(_, record)| record)
        .filter(|record| matches(record, target))
        .collect();
    if chosen.is_empty() {
        return Err(CatalogError::NotRunning {
            target: target.to_owned(),
            home: settings.home_name(),
        }
        .into());
    }
    let mut stopped = Vec::new();
    for record in &chosen {
        let ended = if !record.alive(processes) {
            wire::Ending::Gone
        } else {
            // A request that could not be delivered (a window of its own on
            // Windows has no console to break) is not waited for.
            let asked = processes.ask_to_end(record.pid, core_kind(&record.kind));
            if asked && wait_until_gone(record, processes, patience) {
                wire::Ending::Terminated
            } else {
                processes.force_end(record.pid);
                if !wait_until_gone(record, processes, AFTER_KILL) {
                    continue;
                }
                wire::Ending::Killed
            }
        };
        stopped.push(wire::StoppedLauncher {
            key: record.key.clone(),
            pid: u64::from(record.pid),
            ended,
        });
    }
    // The prune wants the lock; a command holding it for a moment is waited
    // for briefly, since this answer should not list what it just ended.
    let deadline = Instant::now() + Duration::from_secs(1);
    let live = loop {
        let live = sweep(settings, processes);
        let pending = records(settings).len() > live.len();
        if !pending || Instant::now() >= deadline {
            break live;
        }
        std::thread::sleep(POLL);
    };
    Ok(answer(settings, &live, stopped))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    use super::*;
    use crate::catalog::events::EventTail;

    /// A process table in memory: `alive` maps a pid to its start token;
    /// asking to end removes a pid unless it is `stubborn`.
    #[derive(Default)]
    struct Table {
        alive: RefCell<BTreeMap<u32, Option<String>>>,
        stubborn: Vec<u32>,
    }

    impl ProcessTable for Table {
        fn probe(&self, pid: u32) -> ProcessState {
            match self.alive.borrow().get(&pid) {
                Some(started) => ProcessState::Alive {
                    started: started.clone(),
                },
                None => ProcessState::Gone,
            }
        }
        fn ask_to_end(&self, pid: u32, _kind: LauncherKind) -> bool {
            if !self.stubborn.contains(&pid) {
                self.alive.borrow_mut().remove(&pid);
            }
            true
        }
        fn force_end(&self, pid: u32) -> bool {
            self.alive.borrow_mut().remove(&pid);
            true
        }
    }

    fn started(key: &str, pid: u64) -> start_wire::ApplicationStart {
        start_wire::ApplicationStart {
            protocol: "vibe-app-start/1".into(),
            key: key.into(),
            pid,
            command: "/bin/true".into(),
            args: Vec::new(),
            cwd: None,
            log: "/tmp/log".into(),
        }
    }

    #[test]
    fn a_reused_pid_is_not_the_launcher_and_dead_records_are_pruned_with_an_event() {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsRoot::new(dir.path());
        let table = Table::default();
        table.alive.borrow_mut().insert(10, Some("t10".into()));
        table.alive.borrow_mut().insert(11, None);
        register(
            &settings,
            &started("a/b@1.0.0#x", 10),
            LauncherKind::Gui,
            &table,
        )
        .unwrap();
        register(
            &settings,
            &started("a/b@1.0.0#y", 11),
            LauncherKind::Tui,
            &table,
        )
        .unwrap();
        register(
            &settings,
            &started("a/b@1.0.0#z", 12),
            LauncherKind::Gui,
            &table,
        )
        .unwrap();
        let mut tail = EventTail::at_end(&settings);

        // pid 10 now belongs to another process started later.
        table.alive.borrow_mut().insert(10, Some("other".into()));
        let answer = running(&settings, &table);
        let keys: Vec<&str> = answer.running.iter().map(|row| row.key.as_str()).collect();
        assert_eq!(keys, vec!["a/b@1.0.0#y"]);
        assert_eq!(answer.running[0].kind, wire::LauncherKind::Tui);
        let exited: Vec<(String, Option<u64>)> = tail
            .read(&settings)
            .into_iter()
            .map(|event| {
                assert_eq!(event.event, EventKind::Exited);
                (event.key.unwrap(), event.pid)
            })
            .collect();
        assert_eq!(exited.len(), 2, "{exited:?}");
        assert!(!settings.running_dir().join("10.json").exists());
        assert!(!settings.running_dir().join("12.json").exists());
    }

    #[test]
    fn stop_asks_then_kills_and_refuses_what_is_not_running() {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsRoot::new(dir.path());
        let table = Table {
            stubborn: vec![21],
            ..Table::default()
        };
        table.alive.borrow_mut().insert(20, None);
        table.alive.borrow_mut().insert(21, None);
        register(
            &settings,
            &started("a/b@1.0.0#x", 20),
            LauncherKind::Gui,
            &table,
        )
        .unwrap();
        register(
            &settings,
            &started("a/b@1.0.0#y", 21),
            LauncherKind::Gui,
            &table,
        )
        .unwrap();

        let polite = stop_within(&settings, "a/b@1.0.0#x", &table, POLL).unwrap();
        assert_eq!(polite.stopped.len(), 1);
        assert_eq!(polite.stopped[0].ended, wire::Ending::Terminated);
        assert_eq!(polite.running.len(), 1, "the other one still runs");

        let forced = stop_within(&settings, "21", &table, POLL).unwrap();
        assert_eq!(forced.stopped[0].ended, wire::Ending::Killed);
        assert!(forced.running.is_empty());

        let error = stop_within(&settings, "a/b@1.0.0", &table, POLL).unwrap_err();
        assert_eq!(error.code(), "not_running");
    }
}
