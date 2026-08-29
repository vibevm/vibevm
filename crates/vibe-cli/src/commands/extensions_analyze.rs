//! `vibe extensions analyze` — the R4.3 lane analyzer (packages-2026-09
//! architecture §9): compose and compile the selected node's static lane
//! IN PROCESS through the workspace's write-free entry — the same
//! composition regeneration runs, minus every write — collect the
//! attribution evidence through the observer seam, and lower it into
//! the registered `extensions-analyze` report.
//!
//! The frozen §9.1 ruling shapes everything here: the evidence comes from
//! an in-process observer over a REAL compile — nothing is persisted, and
//! no attribution is reconstructed by parsing the artifact's generated
//! comment markers (the compile's witnesses are the only source). The
//! totals reconcile because the report is validated through the generated
//! reader and the hand-written relational cell before a byte is printed:
//! contributions + frame == the artifact's own length, and the
//! emitted-stage deltas chain to it (§11 row 13).
//!
//! The driver is deliberately thin: the lane composition, the owner-plan
//! rules and the observed compile live in
//! [`vibe_workspace::install::analyze_node_lane`]; this file lowers,
//! validates and prints.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-REGISTRY");

use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, bail};
use vibe_spec::{CompileObserver, DeltaStage, EmissionEvent, StageDeltaEvent};
use vibe_wire::behaviour::extensions_analyze::{COMMAND, REPORT_EPOCH, spell_bytes, validate};
use vibe_wire::generated::extensions_analyze::{
    ArtifactRow, BytePair, ContributionKind, ContributionRow, DeltaRow, ExtensionsAnalyze,
    LaneIdentity, LaneIdentityNode, ProviderIdentity, ProviderIdentityDependency,
    ProviderIdentityHostCoordinate, ProviderIdentityHostUngrouped, Stage,
};
use vibe_workspace::Workspace;
use vibe_workspace::install::AnalyzedLane;

use crate::cli::AnalyzeArgs;
use crate::output;

/// Where the compile's evidence accumulates — the observer the CLI hands
/// in. One per run; the compile holds it by `Arc` for its lifetime and
/// nothing else ever sees it.
#[derive(Default)]
struct Collector {
    emissions: Mutex<Vec<EmissionEvent>>,
    deltas: Mutex<Vec<StageDeltaEvent>>,
}

/// Lock one collector slot, recovering from poison. The observer seam
/// contains observer panics at ITS boundary, so a poisoned slot can only
/// mean this collector's own defect mid-push — the evidence already
/// collected is still the evidence, and recovering it keeps the refusal
/// path (partial evidence) rather than a second panic.
fn lock<T>(slot: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl CompileObserver for Collector {
    fn emission(&self, event: &EmissionEvent) {
        lock(&self.emissions).push(event.clone());
    }

    fn stage_delta(&self, event: &StageDeltaEvent) {
        lock(&self.deltas).push(event.clone());
    }
}

pub fn run(ctx: &output::Context, args: AnalyzeArgs) -> Result<()> {
    let selected = Workspace::discover_selected(&args.path).context("discovering the workspace")?;
    let node_rel = selected.selected.as_str().to_string();
    let workspace = selected.workspace;

    // THE COMPILE — the workspace's write-free entry: the regeneration
    // composition for this one node (owner plan, unit substitution,
    // hoisting, B-006 dedup), observed, writing nothing.
    let collector = Arc::new(Collector::default());
    let lane =
        vibe_workspace::install::analyze_node_lane(&workspace, &node_rel, Some(collector.clone()))
            .context("analyzing the selected node's lane")?;

    let report = match lane {
        Some(lane) => lower(&node_rel, &lane, &collector, lane.artifact.bytes().len())?,
        // A node with no static-lane contributions analyzes to the empty
        // report — the honest answer, not an error.
        None => ExtensionsAnalyze {
            schema: REPORT_EPOCH,
            command: COMMAND.to_string(),
            artifacts: Vec::new(),
        },
    };
    let report = verify_through_the_reader(report)?;
    emit(ctx, args, &report)
}

/// Lower the collected evidence into the report document.
///
/// The completeness law lives here: the compile emitted exactly one
/// artifact, so exactly one emission event must exist, and every
/// contribution it carries must align — by VALUE, origin and path — with
/// the input the workspace authored at that seat. An artifact the
/// observer never saw, or a contribution it cannot attribute, refuses
/// rather than reporting partial evidence.
fn lower(
    node_rel: &str,
    lane: &AnalyzedLane,
    collector: &Collector,
    emitted_len: usize,
) -> Result<ExtensionsAnalyze> {
    let emissions = lock(&collector.emissions);
    let deltas = lock(&collector.deltas);
    let [event] = emissions.as_slice() else {
        bail!(
            "the observer saw {} emission events for the one artifact the compile emitted — \
             partial evidence is refused",
            emissions.len()
        );
    };
    if event.total_bytes() != emitted_len {
        bail!(
            "the observer's total ({}) disagrees with the artifact's own length ({emitted_len})",
            event.total_bytes()
        );
    }
    if event.contributions().len() != lane.identities.len() {
        bail!(
            "the observer attributed {} contributions; the lane declared {} — partial evidence is refused",
            event.contributions().len(),
            lane.identities.len()
        );
    }
    let mut contributions = Vec::with_capacity(event.contributions().len());
    let mut occurrences = 0u64;
    for (seat, row) in event.contributions().iter().enumerate() {
        let (origin, path) = &lane.identities[seat];
        if row.origin() != origin || row.path() != path {
            bail!(
                "contribution {seat} came back as `{}` / `{}`; the lane declared `{origin}` / `{path}`",
                row.origin(),
                row.path()
            );
        }
        let Some(provider) = lane.providers[seat].clone() else {
            bail!(
                "contribution {seat} (`{origin}`) has no typed provider to attribute — \
                 partial attribution is refused"
            );
        };
        occurrences += u64::from(row.occurrences());
        contributions.push(ContributionRow {
            provider: provider_identity(provider),
            kind: kind_of(row.kind()),
            origin: row.origin().to_string(),
            path: row.path().to_string(),
            bytes: spell_bytes(row.bytes() as u128),
            occurrences: row.occurrences(),
        });
    }
    let artifact = ArtifactRow {
        lane: LaneIdentity::Node(Box::new(LaneIdentityNode {
            node_rel: node_rel.to_string(),
        })),
        artifact_id: event.artifact_id().to_string(),
        target: target_of(&event.target_id()),
        total_emitted_bytes: spell_bytes(event.total_bytes() as u128),
        occurrence_count: u32::try_from(occurrences).unwrap_or(u32::MAX),
        frame_overhead_bytes: spell_bytes(event.frame_bytes() as u128),
        contributions,
        deltas: deltas
            .iter()
            .map(|delta| DeltaRow {
                pass: delta.pass().to_string(),
                stage: match delta.stage() {
                    DeltaStage::Lane => Stage::Lane,
                    DeltaStage::Emitted => Stage::Emitted,
                },
                lane_byte_delta: (delta.stage() == DeltaStage::Lane).then(|| BytePair {
                    before: spell_bytes(delta.before() as u128),
                    after: spell_bytes(delta.after() as u128),
                }),
                artifact_byte_delta: (delta.stage() == DeltaStage::Emitted).then(|| BytePair {
                    before: spell_bytes(delta.before() as u128),
                    after: spell_bytes(delta.after() as u128),
                }),
            })
            .collect(),
        token_estimate: None,
        estimator_id: None,
    };
    Ok(ExtensionsAnalyze {
        schema: REPORT_EPOCH,
        command: COMMAND.to_string(),
        artifacts: vec![artifact],
    })
}

/// The report the CLI built is not the product until it survives its own
/// wire: serialize, re-read through the GENERATED reader, and validate
/// through the hand-written cell — the same path a foreign document
/// takes, so a report this command cannot re-parse never ships.
fn verify_through_the_reader(report: ExtensionsAnalyze) -> Result<ExtensionsAnalyze> {
    let value = serde_json::to_value(&report).context("serializing the analyzer report")?;
    let reread: ExtensionsAnalyze = serde_json::from_value(value)
        .context("the analyzer report re-reads through the generated reader")?;
    validate(&reread).context("the analyzer report satisfies every wire law")?;
    Ok(reread)
}

fn emit(ctx: &output::Context, args: AnalyzeArgs, report: &ExtensionsAnalyze) -> Result<()> {
    let json = serde_json::to_string_pretty(report).context("rendering the analyzer report")?;
    if let Some(out) = args.out {
        std::fs::write(&out, format!("{json}\n"))
            .with_context(|| format!("writing the analyzer report to {}", out.display()))?;
    } else if ctx.is_json() {
        println!("{json}");
    } else {
        human_summary(ctx, report);
    }
    Ok(())
}

/// The secondary human surface — short by design.
fn human_summary(ctx: &output::Context, report: &ExtensionsAnalyze) {
    ctx.heading("Extensions analyze");
    for artifact in &report.artifacts {
        ctx.step(&format!(
            "{}: {} bytes emitted, {} in frame overhead, {} occurrence(s)",
            artifact.artifact_id,
            artifact.total_emitted_bytes,
            artifact.frame_overhead_bytes,
            artifact.occurrence_count
        ));
        for row in &artifact.contributions {
            ctx.step(&format!(
                "  {} — {} bytes, {} occurrence(s) via {}",
                row.origin,
                row.bytes,
                row.occurrences,
                provider_text(&row.provider)
            ));
        }
        if artifact.deltas.is_empty() {
            ctx.step("  no transform passes ran (the empty plan)");
        }
        for delta in &artifact.deltas {
            ctx.step(&format!(
                "  {} ({} stage)",
                delta.pass,
                stage_text(&delta.stage)
            ));
        }
    }
    ctx.summary(&format!(
        "{} artifact(s) analyzed; run with --json for the full report",
        report.artifacts.len()
    ));
}

fn provider_text(provider: &ProviderIdentity) -> String {
    match provider {
        ProviderIdentity::Dependency(p) => format!("dependency {}/{}", p.group, p.name),
        ProviderIdentity::HostCoordinate(p) => format!("host {}/{}", p.group, p.name),
        ProviderIdentity::HostUngrouped(p) => format!("host {}", p.name),
        ProviderIdentity::HostVirtualWorkspace(_) => "host virtual-workspace".to_string(),
    }
}

fn stage_text(stage: &Stage) -> &'static str {
    match stage {
        Stage::Lane => "lane",
        Stage::Emitted => "emitted",
    }
}

fn kind_of(kind: vibe_spec::EmissionKind) -> ContributionKind {
    match kind {
        vibe_spec::EmissionKind::Normal => ContributionKind::Normal,
        vibe_spec::EmissionKind::Simple => ContributionKind::Simple,
        vibe_spec::EmissionKind::Elided => ContributionKind::Elided,
        vibe_spec::EmissionKind::Hoisted => ContributionKind::Hoisted,
    }
}

fn target_of(target_id: &str) -> vibe_wire::generated::extensions_analyze::ArtifactTarget {
    match target_id {
        "static-md" => vibe_wire::generated::extensions_analyze::ArtifactTarget::StaticMd,
        // The analyzer compiles the two builtin static lanes only; the
        // plan constructor refuses every other target before a compile
        // can observe one.
        _ => vibe_wire::generated::extensions_analyze::ArtifactTarget::StaticXml,
    }
}

fn provider_identity(provider: vibe_spec::DocumentProvider) -> ProviderIdentity {
    match provider {
        vibe_spec::DocumentProvider::Dependency { group, name } => {
            ProviderIdentity::Dependency(Box::new(ProviderIdentityDependency {
                group: group.as_str().to_string(),
                name: name.as_str().to_string(),
            }))
        }
        vibe_spec::DocumentProvider::HostCoordinate { group, name } => {
            ProviderIdentity::HostCoordinate(Box::new(ProviderIdentityHostCoordinate {
                group: group.as_str().to_string(),
                name: name.as_str().to_string(),
            }))
        }
        vibe_spec::DocumentProvider::HostUngrouped { name } => {
            ProviderIdentity::HostUngrouped(Box::new(ProviderIdentityHostUngrouped { name }))
        }
        vibe_spec::DocumentProvider::HostVirtualWorkspace => {
            ProviderIdentity::HostVirtualWorkspace(Box::new(
                vibe_wire::generated::extensions_analyze::ProviderIdentityHostVirtualWorkspace {},
            ))
        }
        // The two absences never reach an attribution row: the entry
        // attributes only inputs it named a typed provider for.
        absences @ (vibe_spec::DocumentProvider::Unclaimed
        | vibe_spec::DocumentProvider::Undetermined) => {
            unreachable!("an attribution row names a real provider: {absences}")
        }
    }
}

#[cfg(test)]
#[path = "extensions_analyze/tests.rs"]
mod tests;
