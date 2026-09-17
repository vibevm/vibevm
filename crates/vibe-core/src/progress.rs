//! Invocation-owned progress observations.
//!
//! This module deliberately knows nothing about terminals, output modes, or
//! verbosity. Libraries report typed events through an explicitly supplied
//! observer; the composition root decides whether and how to render them.
//!
//! ```
//! use std::sync::Arc;
//! use vibe_core::progress::{Progress, ProgressEvent, ProgressObserver};
//!
//! struct Ignore;
//! impl ProgressObserver for Ignore {
//!     fn observe(&self, _event: ProgressEvent) {}
//! }
//!
//! let progress = Progress::new(Arc::new(Ignore));
//! let task = progress.task("resolve packages");
//! task.set_progress(1, Some(2), "packages");
//! task.finish();
//! ```

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-060#OBSERVATION");

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::time::{Duration, Instant};

const ACTIVE: u8 = 0;
const TERMINAL: u8 = 1;

/// An identity unique within one [`Progress`] observation tree.
///
/// ```
/// use vibe_core::progress::Progress;
///
/// let inert_task = Progress::default().task("silent");
/// assert_eq!(inert_task.id().get(), 0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TaskId(u64);

impl TaskId {
    /// The invocation-local numeric identity.
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// One typed observation emitted by a progress task.
///
/// ```
/// use std::sync::Arc;
/// use vibe_core::progress::{Progress, ProgressEvent, ProgressObserver};
///
/// struct CheckIdentity;
/// impl ProgressObserver for CheckIdentity {
///     fn observe(&self, event: ProgressEvent) {
///         assert!(event.task_id.get() > 0);
///     }
/// }
/// let task = Progress::new(Arc::new(CheckIdentity)).task("inspect");
/// task.finish();
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressEvent {
    pub task_id: TaskId,
    pub parent_id: Option<TaskId>,
    pub elapsed: Duration,
    pub kind: ProgressEventKind,
}

/// The payload carried by a [`ProgressEvent`].
///
/// ```
/// use vibe_core::progress::ProgressEventKind;
///
/// let outcome = ProgressEventKind::Skipped {
///     reason: "already current".to_string(),
/// };
/// assert!(matches!(outcome, ProgressEventKind::Skipped { .. }));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressEventKind {
    Started {
        label: String,
    },
    Progress {
        completed: u64,
        total: Option<u64>,
        unit: String,
    },
    Detail {
        message: String,
    },
    Diagnostic {
        level: ProgressDiagnosticLevel,
        message: String,
    },
    Finished,
    Skipped {
        reason: String,
    },
    Failed {
        message: String,
    },
    /// The task handle disappeared without an explicit outcome.
    Stopped,
}

/// A child-operation diagnostic that remains visible in ordinary human mode.
///
/// ```
/// use vibe_core::progress::ProgressDiagnosticLevel;
///
/// let level = ProgressDiagnosticLevel::Warning;
/// assert_eq!(level, ProgressDiagnosticLevel::Warning);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressDiagnosticLevel {
    Warning,
    Error,
}

/// Receives progress observations for one invocation.
///
/// Implementations must return promptly and must not let output failures
/// escape as panics: observing progress is ancillary to the operation being
/// observed.
///
/// ```
/// use std::sync::Arc;
/// use std::sync::atomic::{AtomicUsize, Ordering};
/// use vibe_core::progress::{Progress, ProgressEvent, ProgressObserver};
///
/// struct Counter(AtomicUsize);
/// impl ProgressObserver for Counter {
///     fn observe(&self, _event: ProgressEvent) {
///         self.0.fetch_add(1, Ordering::Relaxed);
///     }
/// }
/// let observer = Arc::new(Counter(AtomicUsize::new(0)));
/// let task = Progress::new(observer.clone()).task("counted");
/// task.finish();
/// assert_eq!(observer.0.load(Ordering::Relaxed), 2);
/// ```
pub trait ProgressObserver {
    fn observe(&self, event: ProgressEvent);
}

struct ProgressInner {
    observer: Arc<dyn ProgressObserver + Send + Sync>,
    next_id: AtomicU64,
}

/// A cheap, cloneable handle used to start tasks.
///
/// `Default` is deliberately inert so library callers can accept progress
/// without requiring every embedding surface to provide an observer.
///
/// ```
/// use vibe_core::progress::Progress;
///
/// let progress = Progress::default();
/// let task = progress.task("optional observation");
/// task.finish();
/// ```
#[derive(Clone, Default)]
pub struct Progress {
    inner: Option<Arc<ProgressInner>>,
    parent_id: Option<TaskId>,
}

impl fmt::Debug for Progress {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Progress")
            .field("enabled", &self.inner.is_some())
            .field("parent_id", &self.parent_id)
            .finish()
    }
}

impl Progress {
    /// Creates an invocation-owned progress tree backed by `observer`.
    pub fn new(observer: Arc<dyn ProgressObserver + Send + Sync>) -> Self {
        Self {
            inner: Some(Arc::new(ProgressInner {
                observer,
                next_id: AtomicU64::new(1),
            })),
            parent_id: None,
        }
    }

    /// Starts a task beneath this handle's parent, emitting `Started` before
    /// the handle is returned.
    pub fn task(&self, label: impl Into<String>) -> ProgressTask {
        let Some(inner) = &self.inner else {
            return ProgressTask::inert();
        };
        let state = Arc::new(TaskState {
            inner: Arc::clone(inner),
            id: TaskId(inner.next_id.fetch_add(1, Ordering::Relaxed)),
            parent_id: self.parent_id,
            started: Instant::now(),
            terminal: AtomicU8::new(ACTIVE),
        });
        state.emit(ProgressEventKind::Started {
            label: label.into(),
        });
        ProgressTask { state: Some(state) }
    }
}

struct TaskState {
    inner: Arc<ProgressInner>,
    id: TaskId,
    parent_id: Option<TaskId>,
    started: Instant,
    terminal: AtomicU8,
}

impl TaskState {
    fn emit(&self, kind: ProgressEventKind) {
        self.inner.observer.observe(ProgressEvent {
            task_id: self.id,
            parent_id: self.parent_id,
            elapsed: self.started.elapsed(),
            kind,
        });
    }

    fn terminate(&self, kind: ProgressEventKind) {
        if self
            .terminal
            .compare_exchange(ACTIVE, TERMINAL, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            self.emit(kind);
        }
    }

    fn is_active(&self) -> bool {
        self.terminal.load(Ordering::Acquire) == ACTIVE
    }
}

/// An in-flight operation.
///
/// Completion is always explicit. Dropping an active task emits `Stopped`,
/// which prevents an early return or unwind from being reported as success.
///
/// ```
/// use vibe_core::progress::Progress;
///
/// let parent = Progress::default().task("install");
/// let child = parent.progress().task("package");
/// child.finish();
/// parent.finish();
/// ```
#[must_use = "dropping an unfinished progress task reports it as stopped"]
pub struct ProgressTask {
    state: Option<Arc<TaskState>>,
}

impl fmt::Debug for ProgressTask {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProgressTask")
            .field("id", &self.state.as_ref().map(|state| state.id))
            .finish()
    }
}

impl ProgressTask {
    fn inert() -> Self {
        Self { state: None }
    }

    /// Returns the task's invocation-local identity.
    ///
    /// A task from a no-op [`Progress`] has identity zero and emits no events.
    pub fn id(&self) -> TaskId {
        self.state.as_ref().map_or(TaskId(0), |state| state.id)
    }

    /// Returns a progress handle whose new tasks are children of this task.
    pub fn progress(&self) -> Progress {
        let Some(state) = &self.state else {
            return Progress::default();
        };
        Progress {
            inner: Some(Arc::clone(&state.inner)),
            parent_id: Some(state.id),
        }
    }

    /// Reports a measurement obtained from work the operation already needs.
    pub fn set_progress(&self, completed: u64, total: Option<u64>, unit: &str) {
        if let Some(state) = &self.state
            && state.is_active()
        {
            state.emit(ProgressEventKind::Progress {
                completed,
                total,
                unit: unit.to_owned(),
            });
        }
    }

    /// Reports a bounded diagnostic detail for this task.
    pub fn detail(&self, detail: impl Into<String>) {
        if let Some(state) = &self.state
            && state.is_active()
        {
            state.emit(ProgressEventKind::Detail {
                message: detail.into(),
            });
        }
    }

    /// Reports a warning or error produced by child work. Unlike routine
    /// detail, CLI renderers keep these visible without verbose mode.
    pub fn diagnostic(&self, level: ProgressDiagnosticLevel, message: impl Into<String>) {
        if let Some(state) = &self.state
            && state.is_active()
        {
            state.emit(ProgressEventKind::Diagnostic {
                level,
                message: message.into(),
            });
        }
    }

    /// Marks the task successful.
    pub fn finish(&self) {
        if let Some(state) = &self.state {
            state.terminate(ProgressEventKind::Finished);
        }
    }

    /// Marks the task intentionally skipped.
    pub fn skip(&self, reason: impl Into<String>) {
        if let Some(state) = &self.state {
            state.terminate(ProgressEventKind::Skipped {
                reason: reason.into(),
            });
        }
    }

    /// Marks the task failed without changing the operation's own error.
    pub fn fail(&self, message: impl Into<String>) {
        if let Some(state) = &self.state {
            state.terminate(ProgressEventKind::Failed {
                message: message.into(),
            });
        }
    }
}

impl Drop for ProgressTask {
    fn drop(&mut self) {
        if let Some(state) = &self.state {
            state.terminate(ProgressEventKind::Stopped);
        }
    }
}

#[cfg(test)]
#[path = "progress/tests.rs"]
mod tests;
