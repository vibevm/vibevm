//! The two answers a payload projection owes: `AV-FMT-009`'s
//! `templates[]` for a client that reads the document, and the row's human
//! form for a person reading the run.
//!
//! They live together because they are the same facts — a `[[template]]`
//! row's coordinate and revision, what it selected, the channels it
//! lowered, the options it kept, the rows `template_optional` dropped and
//! the projection's counts and bytes — and a row must be drawn once,
//! which is only possible once the projection exists.
//!
//! The human form is the display language `#MIRROR-DRY-RUN-OUTPUT` asks
//! for: one node per row, its versions as children.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-DRY-RUN-OUTPUT");

use vibe_wire::generated::registry_mirror_report as wire;

use super::{Projection, RowProjection, Selection, SnapshotOutcome};
use crate::output;

/// `AV-FMT-009`'s `templates[]`: the same facts the human form draws, for
/// a client that reads the document instead.
pub(crate) fn rows(
    selection: &Selection,
    outcomes: &[SnapshotOutcome],
    projection: &Projection,
) -> Vec<wire::MirroredTemplate> {
    outcomes
        .iter()
        .map(|outcome| {
            let row = projection.row(&outcome.package);
            wire::MirroredTemplate {
                package: outcome.package.clone(),
                revision: outcome.revision.clone(),
                selected: u32::try_from(outcome.versions.len()).unwrap_or(u32::MAX),
                snapshotted: u32::try_from(outcome.added).unwrap_or(u32::MAX),
                unchanged: u32::try_from(outcome.unchanged).unwrap_or(u32::MAX),
                first: outcome.versions.first().map(ToString::to_string),
                last: outcome.versions.last().map(ToString::to_string),
                channels: outcome
                    .channels
                    .iter()
                    .map(|(channel, resolved)| wire::ResolvedChannel {
                        channel: channel.clone(),
                        version: resolved.as_ref().map(ToString::to_string),
                    })
                    .collect(),
                installers: outcome.installers.clone(),
                dropped: outcome
                    .dropped
                    .iter()
                    .map(|(version, option, missing)| wire::DroppedOption {
                        version: version.clone(),
                        option: option.clone(),
                        missing: missing.clone(),
                    })
                    .collect(),
                versions: outcome
                    .versions
                    .iter()
                    .map(|version| version_row(version, row))
                    .collect(),
                payloads: row.zip(selection.payloads()).map(|(row, plan)| {
                    let facts = row.facts();
                    wire::TemplatePayloads {
                        inputs: u32::try_from(facts.inputs).unwrap_or(u32::MAX),
                        present: u32::try_from(facts.present).unwrap_or(u32::MAX),
                        present_bytes: facts.present_bytes,
                        fetch_bytes: facts.fetch_bytes,
                        total_bytes: facts.total_bytes,
                        unknown: u32::try_from(facts.unknown).unwrap_or(u32::MAX),
                        into: vibe_core::remap::drop::shown_path(&plan.into),
                    }
                }),
            }
        })
        .collect()
}

/// One selected version as the array carries it: its name always, and its
/// options and input counts when the row projects payloads for it.
fn version_row(version: &semver::Version, row: Option<&RowProjection>) -> wire::TemplateVersion {
    let inputs = row.and_then(|row| {
        row.versions
            .iter()
            .find(|projected| &projected.version == version)
    });
    wire::TemplateVersion {
        version: version.to_string(),
        installers: inputs
            .map(|projected| {
                let mut options: Vec<String> = Vec::new();
                for input in &projected.inputs {
                    if !options.contains(&input.option) {
                        options.push(input.option.clone());
                    }
                }
                options
            })
            .unwrap_or_default(),
        inputs: inputs.map(|projected| u32::try_from(projected.inputs.len()).unwrap_or(u32::MAX)),
        present: inputs.map(|projected| {
            u32::try_from(
                projected
                    .inputs
                    .iter()
                    .filter(|input| input.present)
                    .count(),
            )
            .unwrap_or(u32::MAX)
        }),
    }
}

/// The human form of the template rows: one node per row, its versions as
/// children, in the display language `#MIRROR-DRY-RUN-OUTPUT` asks for.
///
/// It is drawn here rather than where the snapshot is written because a
/// row's payload facts — the input count, the bytes already in the drop,
/// the bytes to fetch, the total and the inputs of unknown size — are
/// only known once the projection exists, and a row must be drawn once,
/// not twice.
pub(crate) fn show(ctx: &output::Context, rows: &[SnapshotOutcome], projection: &Projection) {
    if ctx.is_json() {
        return;
    }
    for outcome in rows {
        let span = match (outcome.versions.first(), outcome.versions.last()) {
            (Some(first), Some(last)) if first != last => format!(" ({first} … {last})"),
            (Some(only), _) => format!(" ({only})"),
            _ => String::new(),
        };
        ctx.step(&format!(
            "template {}@{}: {} version(s) selected{span} — {} to snapshot, {} already there",
            outcome.package,
            outcome.revision,
            outcome.versions.len(),
            outcome.added,
            outcome.unchanged
        ));
        for (name, resolved) in &outcome.channels {
            match resolved {
                Some(version) => ctx.step(&format!("  channel {name} → {version}")),
                None => ctx.step(&format!(
                    "  channel {name} → nothing (the template declares no such channel)"
                )),
            }
        }
        if !outcome.installers.is_empty() {
            ctx.step(&format!(
                "  installer option(s) kept: {}",
                outcome.installers.join(", ")
            ));
        }
        for (version, option, missing) in &outcome.dropped {
            ctx.step(&format!(
                "  {version}: installer option `{option}` dropped — upstream publishes no \
                 {missing}"
            ));
        }
        let Some(row) = projection.row(&outcome.package) else {
            continue;
        };
        for version in &row.versions {
            if version.inputs.is_empty() {
                continue;
            }
            let options: Vec<&str> = version
                .inputs
                .iter()
                .map(|input| input.option.as_str())
                .collect();
            ctx.step(&format!(
                "  {}: {} payload input(s) — {}",
                version.version,
                version.inputs.len(),
                options.join(", ")
            ));
        }
        let facts = row.facts();
        let lower = match facts.unknown {
            0 => String::new(),
            unknown => format!(" (a lower bound: {unknown} input(s) declare no size)"),
        };
        ctx.step(&format!(
            "  payloads: {} input(s), {} already in the drop ({} byte(s)), {} byte(s) to fetch, \
             {} byte(s) in all{lower}",
            facts.inputs, facts.present, facts.present_bytes, facts.fetch_bytes, facts.total_bytes
        ));
    }
}
