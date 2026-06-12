//! `vibe workspace …` — multi-package workspace operations.
//!
//! Spec: [PROP-007 §2.7–§2.9](../../../spec/modules/vibe-workspace/PROP-007-workspace.md#selective-publish).
//!
//! Today the one subcommand is `publish`. `vibe workspace publish` discovers
//! the workspace enclosing the current directory, selects every node that
//! carries `[package]` and is not `publish = false`, orders the selection
//! dependency-first via inter-member `path` dependencies, and publishes each
//! node as its own repository in the workspace's primary `[[registry]]` org.
//!
//! The selection / ordering / staging logic lives in `vibe-workspace`
//! (`vibe_workspace::publish`) — it is pure and hermetically tested there.
//! This file owns the side-effecting half: discovering the workspace,
//! resolving the publish token, building the `[origin]` provenance from the
//! root git repo, and threading each staged node through
//! `vibe_publish::Publisher`. The per-package publish machinery — repo
//! creation, push, tag, token discipline — is reused unchanged from
//! `vibe-publish`; nothing about tokens or push URLs is reimplemented here.
//!
//! Publishing is **not atomic** (PROP-007 §2.7). Distributed publishing
//! across N independent host repos has no transaction, so on the first
//! node's failure the command stops and reports which nodes were already
//! published and which remain — a clear partial-progress report beats a
//! rollback that would be a lie.
//!
//! Deferred (PROP-007 §2.8, noted here so the boundary is explicit):
//! `--archive` (the GitHub `archived = true` lockdown + its
//! unarchive→push→archive re-publish cycle), `has_issues = false` at repo
//! creation, the `published_repos = "read-only" | "open"` workspace
//! setting, and multi-registry fan-out (one node → several registries).
//! `vibe workspace publish` targets the workspace's primary registry only;
//! the `[origin]` marker + README banner + PR-template already make a
//! published copy unmistakably a generated read-only copy.

specmark::scope!("spec://vibevm/modules/vibe-workspace/PROP-007#surface");

mod origin;
mod publish;

use anyhow::Result;

use crate::cli::{WorkspaceArgs, WorkspaceSubcommand};
use crate::output;

pub fn run(ctx: &output::Context, args: WorkspaceArgs) -> Result<()> {
    match args.command {
        WorkspaceSubcommand::Publish(sub) => publish::run_publish(ctx, sub),
    }
}
