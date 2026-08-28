//! Read-only hosted-task projection over one exact lifecycle-state snapshot.
//!
//! The state record is workspace-global while task paths are relative to the
//! selected node. This cell joins those roots without a lock: it pins both
//! capabilities once, reads `state → exact task files → state`, and returns a
//! generated report (or a byte-stable provisional refusal) only when the two
//! state byte strings are identical.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE");

use std::path::{Path, PathBuf};

use specmark::spec;
use thiserror::Error;
use vibe_safefs::Project;
use vibe_wire::generated::lifecycle_state::{
    ExecutionRecordScope, ExecutionRecordStatus, StateRun,
};
use vibe_wire::generated::lifecycle_tasks::{
    LifecycleTasks, LifecycleTasksStatus, PendingTask, PendingTaskScope, TasksRun,
};
use vibe_workspace::{SelectedWorkspace, Workspace, WorkspaceError};

use super::error::LifecycleStateError;
use super::io;
use super::store::LifecycleStateStore;
use crate::TASK_CAP;

/// Three retries after the initial attempt: four complete
/// `state → tasks → state` attempts at most.
const MAX_RETRIES: usize = 3;
/// A hostile state cannot make one query open an unbounded number of files.
const MAX_DELEGATED_ROWS: usize = 64;
/// Logical returned payload budget: first-state bytes plus task documents.
/// The exact second-state verification buffer is transient, so peak memory is
/// still bounded at this value plus [`io::STATE_CAP`] (24 MiB today).
const AGGREGATE_CAP: usize = 16 * 1024 * 1024;

/// Why the read-only hosted-task projection could not return one exact
/// snapshot. This is intentionally distinct from [`LifecycleStateError`]:
/// retry exhaustion, selected-node ownership and task-document failures do
/// not belong to the mutating state store.
///
/// ```
/// use vibe_lifecycle::LifecycleTasksError;
///
/// let error = LifecycleTasksError::UnstableSnapshot { attempts: 4 };
/// assert!(matches!(
///     error,
///     LifecycleTasksError::UnstableSnapshot { attempts: 4 }
/// ));
/// ```
#[derive(Debug, Error)]
#[non_exhaustive]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE")]
pub enum LifecycleTasksError {
    #[error(transparent)]
    Workspace(Box<WorkspaceError>),
    #[error(transparent)]
    State(Box<LifecycleStateError>),
    #[error(
        "cannot pin selected lifecycle-task root `{path}`: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE; \
          fix: ensure the selected node still exists as a readable directory, then retry)"
    )]
    SelectedRoot { path: PathBuf, reason: String },
    #[error(
        "lifecycle state `{path}` parks run `{run_id}` for selected node `{stored}`, but this \
         task query selected `{selected}`; refusing before any task file is opened \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE; \
          fix: query or resume from the owning node `{stored}`)"
    )]
    ForeignPark {
        path: PathBuf,
        stored: String,
        selected: String,
        run_id: String,
    },
    #[error(
        "lifecycle state `{path}` carries {count} delegated rows, over the bounded query ceiling \
         of {cap}; refusing before any task file is opened \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE; \
          fix: complete or cancel parked work until at most {cap} rows remain)"
    )]
    TooManyRows {
        path: PathBuf,
        count: usize,
        cap: usize,
    },
    #[error(
        "lifecycle state owns task `{task}` under selected root `{root}`, but that exact file is \
         absent \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE; \
          fix: restore the state-owned task or remove the abandoned erasable state and rerun)"
    )]
    TaskMissing { root: PathBuf, task: String },
    #[error(
        "cannot safe-read lifecycle task `{task}` under `{root}` with {budget} byte(s) of the \
         aggregate budget remaining: {reason} \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE; \
          fix: repair the exact state-owned task shape/size or reduce the aggregate handoff)"
    )]
    TaskRead {
        root: PathBuf,
        task: String,
        budget: usize,
        reason: String,
    },
    #[error(
        "lifecycle task `{task}` is not valid UTF-8 \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE; \
          fix: write the hosted task document as UTF-8 and retry)"
    )]
    TaskNotUtf8 { task: String },
    #[error(
        "lifecycle state changed during all {attempts} bounded task-query attempts; no single \
         state/task snapshot was stable enough to return \
         (governed by spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE; \
          fix: let the concurrent lifecycle command finish, then retry)"
    )]
    UnstableSnapshot { attempts: usize },
}

impl From<WorkspaceError> for LifecycleTasksError {
    fn from(error: WorkspaceError) -> Self {
        Self::Workspace(Box::new(error))
    }
}

impl From<LifecycleStateError> for LifecycleTasksError {
    fn from(error: LifecycleStateError) -> Self {
        Self::State(Box::new(error))
    }
}

/// Return what validated lifecycle state owes a hosting agent at
/// `selected_root`, without creating `.vibe`, a lock or any other file.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE")]
pub fn pending_hosted_tasks(selected_root: &Path) -> Result<LifecycleTasks, LifecycleTasksError> {
    let SelectedWorkspace {
        workspace,
        selected_root,
        selected,
    } = Workspace::discover_selected(selected_root)?;
    let state_path = workspace.root.join(LifecycleStateStore::FILE);
    let state_project = io::open_project(&workspace.root, &state_path)?;
    // The workspace root and selected root are the same capability in the
    // single-node/root case; do not ambient-open it twice. A member gets its
    // own pinned task capability, also once before the retry loop.
    let selected_project = if selected_root == workspace.root {
        None
    } else {
        Some(
            Project::open(&selected_root).map_err(|error| LifecycleTasksError::SelectedRoot {
                path: selected_root.clone(),
                reason: format!("{error:#}"),
            })?,
        )
    };
    let task_project = selected_project.as_ref().unwrap_or(&state_project);

    for attempt in 0..=MAX_RETRIES {
        let Some(first_bytes) = io::read_state_bytes(&state_project, &state_path)? else {
            // Absence linearizes before a concurrent state creation. It is the
            // one result that deliberately owes no second read.
            return Ok(absent_report());
        };
        let provisional = build_provisional(
            &first_bytes,
            &state_path,
            &selected_root,
            selected.as_str(),
            task_project,
        );

        before_second_state_read(attempt);
        let second = io::read_state_bytes(&state_project, &state_path)?;
        if second.as_deref() == Some(first_bytes.as_slice()) {
            return provisional;
        }
        if attempt == MAX_RETRIES {
            return Err(LifecycleTasksError::UnstableSnapshot {
                attempts: MAX_RETRIES + 1,
            });
        }
        // Both exact state byte strings and the whole provisional value/error
        // drop here before the next attempt starts from scratch.
    }
    Err(LifecycleTasksError::UnstableSnapshot {
        attempts: MAX_RETRIES + 1,
    })
}

fn build_provisional(
    first_bytes: &[u8],
    state_path: &Path,
    selected_root: &Path,
    selected: &str,
    task_project: &Project,
) -> Result<LifecycleTasks, LifecycleTasksError> {
    let state = io::decode(first_bytes, state_path)?;
    let mut rows: Vec<_> = state
        .execution
        .iter()
        .filter(|(_, record)| record.status == ExecutionRecordStatus::Delegated)
        .collect();

    if rows.is_empty() {
        // An idle workspace owes no node-relative file. Every sibling may
        // therefore observe idle while the report preserves the exact stored
        // header (including another sibling's selected spelling).
        return Ok(idle_report(&state.run));
    }

    let stored = state.run.selected.as_deref().ok_or_else(|| {
        invariant(
            state_path,
            "a delegated state reached task projection without its validated selected identity",
        )
    })?;
    let run_id = state.run.run_id.as_deref().ok_or_else(|| {
        invariant(
            state_path,
            "a delegated state reached task projection without its validated run id",
        )
    })?;
    if stored != selected {
        return Err(LifecycleTasksError::ForeignPark {
            path: state_path.to_path_buf(),
            stored: stored.to_string(),
            selected: selected.to_string(),
            run_id: run_id.to_string(),
        });
    }
    if rows.len() > MAX_DELEGATED_ROWS {
        return Err(LifecycleTasksError::TooManyRows {
            path: state_path.to_path_buf(),
            count: rows.len(),
            cap: MAX_DELEGATED_ROWS,
        });
    }

    rows.sort_by(|(left_key, left), (right_key, right)| {
        phase_index(&state.run.chain, &left.phase)
            .cmp(&phase_index(&state.run.chain, &right.phase))
            .then_with(|| left_key.cmp(right_key))
    });

    let mut used = first_bytes.len();
    let mut tasks = Vec::with_capacity(rows.len());
    for (key, record) in rows {
        let [task_path] = record.tasks.as_slice() else {
            return Err(invariant(
                state_path,
                format!("delegated execution `{key}` lost its validated single task path"),
            ));
        };
        let scope = match record.scope {
            Some(ExecutionRecordScope::Phase) => PendingTaskScope::Phase,
            Some(ExecutionRecordScope::Slot) => PendingTaskScope::Slot,
            None => {
                return Err(invariant(
                    state_path,
                    format!("delegated execution `{key}` lost its validated typed scope"),
                ));
            }
        };
        let remaining = AGGREGATE_CAP.saturating_sub(used);
        let cap = TASK_CAP.min(remaining);
        let Some(bytes) = task_project
            .read_file_bounded(task_path, cap)
            .map_err(|error| LifecycleTasksError::TaskRead {
                root: selected_root.to_path_buf(),
                task: task_path.clone(),
                budget: remaining,
                reason: format!("{error:#}"),
            })?
        else {
            return Err(LifecycleTasksError::TaskMissing {
                root: selected_root.to_path_buf(),
                task: task_path.clone(),
            });
        };
        // `read_file_bounded` mechanically guarantees `bytes.len() <= cap`,
        // hence this addition cannot cross the aggregate ceiling.
        used += bytes.len();
        let document = String::from_utf8(bytes).map_err(|_| LifecycleTasksError::TaskNotUtf8 {
            task: task_path.clone(),
        })?;
        tasks.push(PendingTask {
            document,
            execution: key.clone(),
            path: task_path.clone(),
            phase: record.phase.clone(),
            scope,
        });
    }
    Ok(parked_report(&state.run, tasks))
}

fn invariant(path: &Path, reason: impl Into<String>) -> LifecycleTasksError {
    LifecycleStateError::Invariant {
        path: path.to_path_buf(),
        reason: reason.into(),
    }
    .into()
}

fn phase_index(chain: &[String], phase: &str) -> usize {
    chain
        .iter()
        .position(|candidate| candidate == phase)
        .unwrap_or(usize::MAX)
}

fn run_report(run: &StateRun) -> TasksRun {
    TasksRun {
        chain: run.chain.clone(),
        requested: run.requested.clone(),
        started: run.started.clone(),
        run_id: run.run_id.clone(),
        selected: run.selected.clone(),
    }
}

fn absent_report() -> LifecycleTasks {
    LifecycleTasks {
        schema: 1,
        status: LifecycleTasksStatus::Absent,
        tasks: Vec::new(),
        run: None,
    }
}

fn idle_report(run: &StateRun) -> LifecycleTasks {
    LifecycleTasks {
        schema: 1,
        status: LifecycleTasksStatus::Idle,
        tasks: Vec::new(),
        run: Some(run_report(run)),
    }
}

fn parked_report(run: &StateRun, tasks: Vec<PendingTask>) -> LifecycleTasks {
    debug_assert!(!tasks.is_empty());
    LifecycleTasks {
        schema: 1,
        status: LifecycleTasksStatus::Parked,
        tasks,
        run: Some(run_report(run)),
    }
}

#[cfg(test)]
type BeforeSecondStateReadHook = Box<dyn FnMut(usize)>;

#[cfg(test)]
thread_local! {
    static BEFORE_SECOND_STATE_READ: std::cell::RefCell<Option<BeforeSecondStateReadHook>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub(crate) fn arm_before_second_state_read(hook: Option<BeforeSecondStateReadHook>) {
    BEFORE_SECOND_STATE_READ.with(|slot| *slot.borrow_mut() = hook);
}

#[cfg(test)]
fn before_second_state_read(attempt: usize) {
    BEFORE_SECOND_STATE_READ.with(|slot| {
        if let Some(hook) = slot.borrow_mut().as_mut() {
            hook(attempt);
        }
    });
}

#[cfg(not(test))]
const fn before_second_state_read(_attempt: usize) {}

#[cfg(test)]
#[path = "tasks/tests.rs"]
mod tests;
