//! `vibe prompt build | check | prepare | path | flags` — the explicit
//! operations surface of the cache-stable prefix.
//!
//! The driver is thin on purpose. Freshness, the request materials, the
//! build and the per-kind flags are all `vibe-workspace`'s
//! (`system_prompt::{check_node_prompt, prepare_request, launch_flags}`,
//! `boot_artifacts::build_node_prompt`); this file resolves the node,
//! lowers each answer into its registered report and prints.
//!
//! **Two of the five verbs promise read-only and must keep it.** `check` and
//! `prepare` write no product, cache, profile or config file, open no
//! connection and reach no compile provider — the workspace cell they call
//! holds that property, and the one effect either of them has beyond
//! reading files is `git ls-files`, which answers the ownership question of
//! `#SYS-ARTIFACTS-NOT-TRACKED` and is spawned only where a `.git` entry
//! exists (`#SYS-READ-ONLY-CHECK`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-OPERATIONS-SURFACE");

use anyhow::Result;

use crate::cli::{PromptArgs, PromptCommand};
use crate::output;

mod build;
mod check;
mod flags;
mod path;
mod prepare;
mod resolve;

pub(crate) use build::summary_line;
pub(crate) use flags::compose as compose_flags;
pub(crate) use resolve::{Selected, select};

/// Run one regeneration and end it with the system-prompt summary line.
///
/// `vibe install` and `vibe reinstall` publish an opted-in node's prefix
/// along with its other boot artifacts, and whoever builds the artifacts
/// says what it did and where. `None` from the snapshot is "say nothing":
/// a project that neither had nor has a generated prefix would get noise
/// rather than information (`#SYS-OPERATIONS-PATH-AND-FLAGS`).
pub(crate) fn reporting_the_prompt<T>(
    ctx: &output::Context,
    project_root: &std::path::Path,
    regenerate: impl FnOnce() -> T,
) -> T {
    let before = build::Snapshot::of(project_root);
    let outcome = regenerate();
    if let Some(line) = before.summary(project_root) {
        ctx.summary(&line);
    }
    outcome
}

pub fn run(ctx: &output::Context, args: PromptArgs) -> Result<()> {
    match args.command {
        PromptCommand::Build { node, path } => build::run(ctx, &select(&path, node.as_deref())?),
        PromptCommand::Check {
            node,
            path,
            model_window,
        } => check::run(ctx, &select(&path, node.as_deref())?, model_window),
        PromptCommand::Prepare {
            node,
            path,
            session_target,
        } => prepare::run(
            ctx,
            &select(&path, node.as_deref())?,
            session_target.as_deref(),
        ),
        PromptCommand::Path { part, node, path } => {
            path::run(ctx, &select(&path, node.as_deref())?, part)
        }
        PromptCommand::Flags {
            harness,
            node,
            path,
        } => flags::run(ctx, &select(&path, node.as_deref())?, &harness),
    }
}
