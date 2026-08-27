//! Plain `vibe reinstall`: recompute every node's boot artifacts from the
//! materialised `vibedeps/` tree already on disk. No fetch, no network.
//!
//! ## Why the continuation no longer ends the command
//!
//! A forced reinstall that parked left a live `requested=reinstall` slot
//! continuation, and the plain verb its handoff names is what has to finish it.
//! That much is unchanged. What changed is what happens AFTER it is satisfied:
//! this used to emit the resumed document and return, so the invocation the
//! operator actually typed — "regenerate my boot artifacts" — never ran.
//!
//! Now a satisfied continuation is a VALUE, and the command carries on into the
//! traced boot regeneration it was asked for. Its rows travel with it, so the
//! one report describes both halves of what this invocation did. A resume that
//! parks AGAIN still stops there: the chain is waiting on the hosting agent, and
//! regenerating boot underneath a live handoff would be work nobody asked for.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#cli-surface");

use anyhow::{Result, bail};
use vibe_core::manifest::{Lockfile, SpecFormat};
use vibe_install::InstallProgress;
use vibe_lifecycle::RunMetadata;
use vibe_workspace::Workspace;
use vibe_workspace::compile_trace::TraceRun;
use vibe_workspace::install::regenerate_boot_traced;
use vibe_workspace::vibedeps;

use crate::cli::ReinstallArgs;
use crate::commands::compile_trace::{RegisteredReportDraft, carry_measured};
use crate::exit_code::InstallError;
use crate::output;

use super::continuation;
use super::draft::{ReinstallDraft, ReinstallIdentity, regenerated};
use super::inputs::confirm;

pub(super) struct Plain<'a> {
    pub(super) args: &'a ReinstallArgs,
    pub(super) identity: &'a ReinstallIdentity,
    pub(super) workspace: &'a Workspace,
    pub(super) lockfile: &'a Lockfile,
    pub(super) metadata: &'a RunMetadata,
    pub(super) spec_format: SpecFormat,
    pub(super) trace: Option<&'a TraceRun>,
}

pub(super) fn run(ctx: &output::Context, inputs: Plain<'_>) -> Result<ReinstallDraft> {
    let Plain {
        args,
        identity,
        workspace,
        lockfile,
        metadata,
        spec_format,
        trace,
    } = inputs;
    // Without `--force` the materialised `vibedeps/` tree is the only content
    // source. Every locked package must have its slot on disk — a missing slot
    // is content this mode cannot conjure; only a fetch (`--force`) can.
    let missing = missing_slots(workspace, lockfile);
    if !missing.is_empty() {
        bail!(
            "the materialised `vibedeps/` tree is incomplete — {} slot{} missing:\n  {}\n\
             Run `vibe reinstall --force` to re-fetch the content from source.",
            missing.len(),
            if missing.len() == 1 { "" } else { "s" },
            missing.join("\n  "),
        );
    }

    let node_count = workspace.iter_nodes().count();
    ctx.heading(&format!(
        "\nReinstall — regenerate boot artifacts for {node_count} node{} from vibedeps/.",
        if node_count == 1 { "" } else { "s" },
    ));

    if !confirm(
        ctx,
        args,
        "Regenerate the boot artifacts from the materialised vibedeps/ tree?",
    )? {
        return Err(InstallError::UserDeclined.into());
    }

    // Service what a forced run parked, BEFORE ordinary boot regeneration.
    let serviced = continuation::service(
        ctx,
        continuation::Request {
            identity,
            workspace,
            manifest: &workspace.root_manifest,
            metadata,
            spec_format,
            progress: InstallProgress::fresh(Vec::new()),
        },
    )?;
    let rows = match serviced {
        Some(serviced) if serviced.parked.is_some() => {
            let continuation::Serviced {
                progress,
                rows,
                parked,
            } = serviced;
            return Ok(ReinstallDraft::completed(
                identity,
                progress,
                rows,
                parked.as_ref(),
            ));
        }
        // Satisfied: its rows belong to this invocation's one report, and the
        // command carries on into what it was actually asked to do.
        Some(serviced) => serviced.rows,
        None => Vec::new(),
    };

    let nodes = match regenerate_boot_traced(workspace, spec_format, trace) {
        Ok(nodes) => nodes,
        // A serviced continuation is already durable. Freezing its rows into
        // the failure keeps the report from claiming this run did nothing when
        // it had already finished somebody's parked slot work.
        Err(error) => {
            return Err(carry_measured(
                anyhow::Error::new(error).context("regenerating boot artifacts"),
                || {
                    RegisteredReportDraft::Reinstall(Box::new(ReinstallDraft::failed(
                        identity,
                        InstallProgress::default(),
                        rows.clone(),
                    )))
                },
            ));
        }
    };
    Ok(ReinstallDraft::completed(
        identity,
        regenerated(nodes, Vec::new()),
        rows,
        None,
    ))
}

/// Every locked package whose slot is absent, named the way its own
/// materialisation mode spells it.
///
/// An in-place package's slot is the unversioned git working tree (PROP-022
/// §2.4); every other mode is the versioned slot. Check, and name, the right
/// one per mode.
fn missing_slots(workspace: &Workspace, lockfile: &Lockfile) -> Vec<String> {
    lockfile
        .packages
        .iter()
        .filter(|p| {
            if p.materialization.is_in_place() {
                !vibedeps::is_in_place_slot(&workspace.root, &p.group, &p.name)
            } else {
                !vibedeps::is_materialised(&workspace.root, &p.group, &p.name, &p.version)
            }
        })
        .map(|p| {
            if p.materialization.is_in_place() {
                vibedeps::in_place_slot_rel_path(&p.group, &p.name)
            } else {
                vibedeps::slot_rel_path(&p.group, &p.name, &p.version)
            }
        })
        .collect()
}
