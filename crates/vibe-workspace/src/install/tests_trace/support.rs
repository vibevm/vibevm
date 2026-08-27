//! The shared traced-install fixture: an injected recorder, the two-shape
//! workspace every red installs, and the on-disk readers they assert through.
//!
//! Time and run ids arrive as arguments — nothing here calls `now()` or
//! allocates a run id, so a red is a statement about structure rather than
//! about how fast or how concurrent the host is.

use std::path::{Path, PathBuf};

use tempfile::TempDir;
use vibe_core::manifest::SpecFormat;
use vibe_core::user_config::SlotIntegrity;
use vibe_wire::behaviour::compiler_trace_index::validate;
use vibe_wire::generated::compiler_trace_index::e1::index::{CompilerTraceIndex, Timestamp};

use super::super::test_helpers::*;
use super::super::*;

use crate::compile_trace::{TraceLimits, TraceRun};

/// One injected run id: exactly 32 lowercase hex.
pub(super) const RUN: &str = "0123456789abcdef0123456789abcdef";

#[cfg(test)]
pub(super) fn at(seconds: i64) -> Timestamp {
    Timestamp::from_timestamp(seconds, 0).expect("a fixture instant is representable")
}

#[cfg(test)]
pub(super) fn traced_run(root: &Path) -> TraceRun {
    TraceRun::open_with_limits(root, RUN, at(1_000), TraceLimits::for_test(u64::MAX, 9))
        .expect("a fresh run under a temporary root opens")
}

/// Read the run's index exactly as an outside reader would.
#[cfg(test)]
pub(super) fn run_index(root: &Path) -> CompilerTraceIndex {
    let bytes = fs::read(
        root.join(".vibe")
            .join("trace")
            .join(RUN)
            .join("index.json"),
    )
    .expect("the run index is on disk");
    let index: CompilerTraceIndex =
        serde_json::from_slice(&bytes).expect("the index parses as the generated type");
    validate(&index).expect("the index obeys every relational law");
    index
}

pub(super) fn apply_traced(
    ws: &Workspace,
    resolution: &[ResolvedDep],
    trace: Option<&TraceRun>,
) -> Result<InstallOutcome, WorkspaceError> {
    apply_lifecycle_traced(ws, resolution, SlotLifecycleMode::None, trace)
}

/// The same, under an explicit lifecycle mode — the seam a pre-install
/// callback failure enters through.
pub(super) fn apply_lifecycle_traced(
    ws: &Workspace,
    resolution: &[ResolvedDep],
    lifecycle: SlotLifecycleMode<'_>,
    trace: Option<&TraceRun>,
) -> Result<InstallOutcome, WorkspaceError> {
    apply_resolution_with_spec_format_and_slot_lifecycle_traced(
        ws,
        resolution,
        SlotIntegrity::TrustPresence,
        SpecFormat::Mixed,
        None,
        lifecycle,
        trace,
    )
}

/// `parent` statically links `child`: the pair whose zone makes `parent` a
/// dirty package unit on a fresh install.
pub(super) fn static_pair(
    parent: &str,
    child: &str,
) -> (ResolvedDep, TempDir, ResolvedDep, TempDir) {
    let (parent_dep, parent_src) = dep_with_requires(
        parent,
        "1.0.0",
        &format!(
            "[boot_snippet]\nsource = \"boot/{parent}.md\"\n\n\
             [requires.packages]\n\"org.vibevm/{child}\" = {{ version = \"^1.0\", link = \"static\" }}"
        ),
        &format!("boot/{parent}.md"),
        &format!("# {parent} boot"),
        &[child],
    );
    let (child_dep, child_src) = dep_with_boot(
        child,
        "1.0.0",
        &format!("[boot_snippet]\nsource = \"boot/{child}.md\""),
        &format!("boot/{child}.md"),
        &format!("# {child} boot"),
    );
    (parent_dep, parent_src, child_dep, child_src)
}

/// The standard fixture: a root (`demo`) whose own boot is dynamic-only but
/// which STATICALLY links the `corelib` leaf, and which requires `parent`
/// (and transitively `child`) — so one install compiles BOTH a dirty package
/// unit (`parent`'s zone) and the root node's own static lane (`corelib`).
#[cfg(test)]
pub(super) fn unit_and_root_fixture() -> (TempDir, Workspace, Vec<ResolvedDep>, Vec<TempDir>) {
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.0.1\"\n\n\
         [requires.packages]\n\"org.vibevm/parent\" = \"^1.0\"\n\
         \"org.vibevm/corelib\" = { version = \"^1.0\", link = \"static\" }\n",
    );
    write(ws_dir.path(), boot_rel("00-core.md"), "# core");
    let (parent, parent_src, child, child_src) = static_pair("parent", "child");
    let (corelib, corelib_src) = dep_with_boot(
        "corelib",
        "1.0.0",
        "[boot_snippet]\nsource = \"boot/corelib.md\"",
        "boot/corelib.md",
        "# corelib boot",
    );
    let ws = Workspace::load(ws_dir.path()).unwrap();
    (
        ws_dir,
        ws,
        vec![parent, child, corelib],
        vec![parent_src, child_src, corelib_src],
    )
}

/// The absolute path of a materialised unit's compiled STATIC lane.
pub(super) fn unit_static(root: &Path, unit: &str) -> PathBuf {
    root.join(deps_slot_specs(
        format!("org.vibevm.{unit}/1.0.0"),
        "boot/STATIC.md",
    ))
}

/// The scope ids of one base, in index order — the projection every attempt
/// red compares.
pub(super) fn occurrences<'a>(index: &'a CompilerTraceIndex, base: &str) -> Vec<&'a str> {
    index
        .scopes
        .iter()
        .filter(|scope| scope.id.starts_with(&format!("{base}::attempt:")))
        .map(|scope| scope.id.as_str())
        .collect()
}
