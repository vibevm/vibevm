use std::path::Path;

use anyhow::{Context, Result};
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_lifecycle::RunMetadata;
use vibe_lifecycle::process::StreamMode;
use vibe_workspace::Workspace;
use vibe_workspace::install::ResolvedDep;
use vibe_workspace::vibedeps;

use crate::output;

pub(super) fn metadata(
    ctx: &output::Context,
    root: &Path,
    requested: &str,
    offline: bool,
    assume_yes: bool,
) -> Result<RunMetadata> {
    Ok(RunMetadata {
        requested: requested.into(),
        chain: vec!["install".into()],
        offline,
        assume_yes: assume_yes || ctx.is_unattended() || ctx.is_json(),
        agent_mode: vibe_wire::generated::lifecycle::e1::context::RunAgentMode::Cli,
        force: false,
        run_id: vibe_lifecycle::process::allocate_run_id(root)?,
        started: crate::commands::init::current_timestamp_utc(),
    })
}

pub(super) fn stream_mode(ctx: &output::Context) -> StreamMode {
    if ctx.is_json() {
        StreamMode::Capture
    } else if ctx.suppresses_output() {
        StreamMode::Null
    } else {
        StreamMode::Inherit
    }
}

pub(super) fn provisional_world(
    workspace: &Workspace,
    lockfile: &Lockfile,
    updated: &[ResolvedDep],
) -> Result<Vec<ResolvedDep>> {
    let mut world = Vec::with_capacity(lockfile.packages.len().max(updated.len()));
    for locked in &lockfile.packages {
        if let Some(replacement) = updated
            .iter()
            .find(|dep| dep.group == locked.group && dep.name == locked.name.as_str())
        {
            world.push(replacement.clone());
            continue;
        }
        let slot = if locked.materialization.is_in_place() {
            vibedeps::in_place_slot_abs_path(&workspace.root, &locked.group, &locked.name)
        } else {
            vibedeps::slot_abs_path(
                &workspace.root,
                &locked.group,
                &locked.name,
                &locked.version,
            )
        };
        let manifest = Manifest::read(slot.join(Manifest::FILENAME)).with_context(|| {
            format!(
                "reading unchanged provisional lifecycle provider `{}/{}@{}`",
                locked.group, locked.name, locked.version
            )
        })?;
        world.push(ResolvedDep {
            kind: locked.kind,
            group: locked.group.clone(),
            name: locked.name.to_string(),
            version: locked.version.clone(),
            content_dir: slot,
            source_hash: Some(locked.content_hash.clone()),
            manifest,
            requires: locked
                .dependencies
                .iter()
                .filter_map(|dependency| {
                    dependency
                        .group
                        .clone()
                        .map(|group| (group, dependency.name.to_string()))
                })
                .collect(),
            admitted_by: locked.admitted_by.clone(),
            via_override: locked.via_override.clone(),
            source_mutable: false,
            in_place_changed: None,
        });
    }
    for dep in updated {
        if !world
            .iter()
            .any(|row| row.group == dep.group && row.name == dep.name)
        {
            world.push(dep.clone());
        }
    }
    Ok(world)
}
