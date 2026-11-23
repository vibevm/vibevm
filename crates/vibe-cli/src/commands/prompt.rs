//! `vibe prompt init | build | check | prepare | path | flags` — the explicit
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
use specmark::spec;

use crate::cli::{PromptArgs, PromptCommand};
use crate::output;

mod adapter;
mod build;
mod capture;
mod check;
mod compose;
mod flags;
mod harness_version;
mod init;
mod local_artifact;
mod path;
mod personal;
mod personal_launch;
mod prepare;
mod qwen_identity;
mod resolve;
mod supplied_base;
mod types;

pub(crate) use build::summary_line;
pub(crate) use flags::{Undeliverable, compose_for_launch as compose_launch_flags, opted_in};
pub(crate) use qwen_identity::inherited_carrier_to_scrub;
pub(crate) use resolve::{Selected, select};
pub(crate) use types::refresh_agent_types;

/// Prepare the existing selected-node native lane, including fresh personal layers.
pub(crate) fn native_launch_lane(
    ctx: &output::Context,
    selected: &Selected,
) -> Result<std::path::PathBuf> {
    let tracked = selected.tracked();
    let answer =
        vibe_workspace::system_prompt::check_node_prompt(&selected.request(&tracked, None))?;
    if let Some((path, _)) = personal_launch::compose_native(ctx, selected, &answer)? {
        return Ok(path);
    }
    if !answer.verdict.is_equal() {
        anyhow::bail!("native launch requires a current prepared project lane");
    }
    Ok(answer.lane.path)
}

/// Whether the operation a prompt summary follows succeeded: a failed one
/// says nothing about the prompt, because its refusal is the account.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-OPERATIONS-PATH-AND-FLAGS")]
pub(crate) trait Succeeded {
    /// True when the operation did its work (a park counts: it regenerated).
    fn succeeded(&self) -> bool;
}

impl<T, E> Succeeded for Result<T, E> {
    fn succeeded(&self) -> bool {
        self.is_ok()
    }
}

impl<R> Succeeded for crate::commands::compile_trace::CommandExit<R> {
    fn succeeded(&self) -> bool {
        !matches!(self, Self::Failed { .. })
    }
}

/// Run one regeneration and print its system-prompt step row on success.
/// A failed operation prints no prompt row: its refusal is the account.
///
/// `vibe install` and `vibe reinstall` publish an opted-in node's prefix
/// along with its other boot artifacts, and whoever builds the artifacts
/// says what it did and where. `None` from the snapshot is "say nothing":
/// a project that neither had nor has a generated prefix would get noise
/// rather than information (`#SYS-OPERATIONS-PATH-AND-FLAGS`).
pub(crate) fn reporting_the_prompt<T: Succeeded>(
    ctx: &output::Context,
    project_root: &std::path::Path,
    regenerate: impl FnOnce() -> T,
) -> T {
    let before = build::Snapshot::of(project_root);
    let outcome = regenerate();
    if outcome.succeeded()
        && let Some(summary) = before.summary(project_root)
    {
        ctx.block(&summary.row_at(crate::output::language::viewport()));
    }
    outcome
}

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-OPERATIONS-SURFACE")]
pub fn run(ctx: &output::Context, args: PromptArgs) -> Result<()> {
    match args.command {
        PromptCommand::Init { node, path } => init::run(ctx, &select(&path, node.as_deref())?),
        PromptCommand::Build { node, path } => build::run(ctx, &select(&path, node.as_deref())?),
        PromptCommand::Compose {
            node,
            path,
            session_target,
        } => compose::run(
            ctx,
            &select(&path, node.as_deref())?,
            session_target.as_deref(),
        ),
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
        PromptCommand::Capture {
            node,
            path,
            session_target,
            task,
            with_text,
            usage,
        } => capture::run(
            ctx,
            &select(&path, node.as_deref())?,
            capture::Request {
                session_target: session_target.as_deref(),
                task: task.as_deref(),
                with_text,
                usage: usage.as_deref(),
            },
        ),
        PromptCommand::Path { part, node, path } => {
            path::run(ctx, &select(&path, node.as_deref())?, part)
        }
        PromptCommand::Flags {
            harness,
            node,
            path,
            workers,
        } => flags::run(ctx, &select(&path, node.as_deref())?, &harness, workers),
        PromptCommand::Types { node, path, check } => {
            types::run(ctx, &select(&path, node.as_deref())?, check)
        }
    }
}
