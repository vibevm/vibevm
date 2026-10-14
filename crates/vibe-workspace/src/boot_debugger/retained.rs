//! Retained inspection deliberately has no TraceRun or provider handle.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#READ-ONLY");

use std::path::Path;
use vibe_safefs::Project;
use vibe_spec::BootGraph;
use vibe_wire::{
    behaviour::compiler_trace_index::validate,
    generated::compiler_trace_index::e1::index::CompilerTraceIndex,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetainedUnavailable {
    Missing,
    /// Reserved for an explicit retention witness; absence alone is Missing.
    Evicted,
    Unavailable,
    Unreadable(String),
}

#[derive(Debug)]
pub struct RetainedGraph {
    /// Exact index captured by this read, including project and scope identity.
    pub index: CompilerTraceIndex,
    pub sequence: u32,
    pub scope: String,
    pub invocation: u32,
    pub snapshot: String,
    /// Exact bytes remain available even when a carrier has a projection gap.
    pub bytes: Vec<u8>,
    pub graph: BootGraph,
}

fn unreadable(error: impl std::fmt::Display) -> RetainedUnavailable {
    RetainedUnavailable::Unreadable(crate::compile_trace::bounded_diagnostic(format_args!(
        "{error}"
    )))
}

/// Inspect one exact event in an existing run. Uses only pinned read APIs:
/// no lock, directory creation, writer, retention, provider or live fallback.
pub fn read(
    root: &Path,
    run_id: &str,
    sequence: u32,
) -> Result<RetainedGraph, RetainedUnavailable> {
    // A path-component check is not a second run-ID grammar: the authoritative
    // index validator below still judges the stored identity.
    if run_id.is_empty() || run_id == "." || run_id == ".." || run_id.contains(['/', '\\']) {
        return Err(unreadable("run id must be one component"));
    }
    let project = Project::open(root).map_err(unreadable)?;
    let directory = project
        .dir_if_present(&[".vibe", "trace", run_id])
        .map_err(unreadable)?
        .ok_or(RetainedUnavailable::Missing)?;
    let bytes = project
        .read_file_in(&directory, "index.json")
        .map_err(unreadable)?
        .ok_or(RetainedUnavailable::Missing)?;
    let index: CompilerTraceIndex = serde_json::from_slice(&bytes).map_err(unreadable)?;
    validate(&index).map_err(unreadable)?;
    if index.run_id != run_id {
        return Err(unreadable("run identity mismatch"));
    }
    read_event(&project, &directory, &index, sequence)
}

fn read_event(
    project: &Project,
    directory: &vibe_safefs::Pinned,
    index: &CompilerTraceIndex,
    sequence: u32,
) -> Result<RetainedGraph, RetainedUnavailable> {
    let event = index
        .events
        .iter()
        .find(|e| e.sequence == sequence)
        .ok_or(RetainedUnavailable::Unavailable)?;
    let snapshot = event
        .snapshot
        .clone()
        .ok_or(RetainedUnavailable::Unavailable)?;
    let bytes = project
        .read_file_in(&directory, &snapshot)
        .map_err(unreadable)?
        .ok_or(RetainedUnavailable::Missing)?;
    let graph = vibe_spec::inspect_boot_snapshot(&bytes).map_err(unreadable)?;
    let scope = event.scope.clone();
    let invocation = event.invocation;
    Ok(RetainedGraph {
        index: index.clone(),
        sequence,
        scope,
        invocation,
        snapshot,
        bytes,
        graph,
    })
}

/// One frozen index traversal, including scopes with no snapshots and every
/// event's explicit availability. No failed child hides its siblings.
#[derive(Debug)]
pub struct RetainedRun {
    pub index: CompilerTraceIndex,
    pub snapshots: Vec<Result<RetainedGraph, RetainedUnavailable>>,
}

pub fn read_run(root: &Path, run_id: &str) -> Result<RetainedRun, RetainedUnavailable> {
    // A path-component check is not a second run-ID grammar: the authoritative
    // index validator below still judges the stored identity.
    if run_id.is_empty() || run_id == "." || run_id == ".." || run_id.contains(['/', '\\']) {
        return Err(unreadable("run id must be one component"));
    }
    let project = Project::open(root).map_err(unreadable)?;
    let directory = project
        .dir_if_present(&[".vibe", "trace", run_id])
        .map_err(unreadable)?
        .ok_or(RetainedUnavailable::Missing)?;
    let bytes = project
        .read_file_in(&directory, "index.json")
        .map_err(unreadable)?
        .ok_or(RetainedUnavailable::Missing)?;
    let index: CompilerTraceIndex = serde_json::from_slice(&bytes).map_err(unreadable)?;
    validate(&index).map_err(unreadable)?;
    if index.run_id != run_id {
        return Err(unreadable("run identity mismatch"));
    }
    let snapshots = index
        .events
        .iter()
        .map(|event| read_event(&project, &directory, &index, event.sequence))
        .collect();
    Ok(RetainedRun { index, snapshots })
}
