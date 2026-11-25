//! Human mirror results, kept separate from the machine report.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror");

use anyhow::Result;
use vibe_publish::git_publish::redact_credentials;
use vibe_publish::layout_publish::LayoutTarget;
use vibe_wire::generated::registry_mirror_report::{MirrorAction, RegistryMirrorReport};

use super::target::Target;
use crate::output;
use crate::output::language::{Cell, Glyph, List, Role, Row};

pub(super) fn heading(ctx: &output::Context, source: &str, target: &Target, dry_run: bool) {
    ctx.rule(&heading_label(source, &target.name, dry_run));
}

fn heading_label(source: &str, target: &str, dry_run: bool) -> String {
    format!(
        "mirror {source} into {target}{}",
        if dry_run { " · dry run" } else { "" }
    )
}

pub(super) fn render(
    ctx: &output::Context,
    report: &RegistryMirrorReport,
    target: &Target,
    snapshots: usize,
) -> Result<()> {
    if ctx.is_json() {
        return ctx.emit_json(report);
    }
    let rows = version_rows(report);
    if !rows.rows.is_empty() {
        ctx.block(&rows.render());
    }
    let what = summary(report, &target.name, snapshots);
    let record = target_record(target);
    if report.ok {
        ctx.trailer_at(&what, &record);
    } else {
        // The refusal is written by the caller. Keep its trailer last.
        ctx.defer_trailer(ctx.trailer_text(false, &format!("{what} · {record}")));
    }
    Ok(())
}

fn target_record(target: &Target) -> String {
    match &target.layout {
        LayoutTarget::Localdir { root } => root.display().to_string(),
        LayoutTarget::WorkingCopy { path } => path.display().to_string(),
        LayoutTarget::Monorepo { url, .. } => redact_credentials(url),
    }
}

fn version_rows(report: &RegistryMirrorReport) -> List {
    List::new(
        report
            .versions
            .iter()
            .filter(|row| row.action != MirrorAction::Unchanged)
            .map(|row| {
                let (glyph, role) = match row.action {
                    MirrorAction::Alarm | MirrorAction::Unavailable => (Glyph::Failed, Role::Alert),
                    MirrorAction::SnapshotDiffers => (Glyph::Skipped, Role::Dim),
                    _ => (Glyph::Done, Role::Ok),
                };
                let note = row
                    .note
                    .as_deref()
                    .map(|note| format!(" — {note}"))
                    .unwrap_or_default();
                Row::new(vec![
                    Cell::new(format!("{}@{}", row.package, row.version), Role::Accent),
                    Cell::new(format!("{}{note}", action_word(&row.action, report.dry_run)), role),
                ])
                .glyph(glyph, role)
            })
            .collect(),
    )
}

fn summary(report: &RegistryMirrorReport, target: &str, snapshots: usize) -> String {
    let count = |action: MirrorAction| {
        report.versions.iter().filter(|row| row.action == action).count()
    };
    let current = report.ok
        && !report.dry_run
        && report.versions.iter().all(|row| row.action == MirrorAction::Unchanged)
        && report.facts_recorded == 0
        && !report.index_refreshed
        && report.commit.is_none()
        && report.bundles == 0
        && snapshots == 0
        && report.pruned.is_empty()
        && report.removed.unwrap_or(0) == 0
        && report.templates.iter().all(|row| {
            row.snapshotted == 0
                && row.payloads.as_ref().is_none_or(|payloads| {
                    payloads.inputs == payloads.present && payloads.fetch_bytes == 0
                })
        });
    let headline = if current {
        format!("{target} is current")
    } else if report.dry_run {
        format!("would mirror {} into {target}", report.source)
    } else if report.ok {
        format!("mirrored {} into {target}", report.source)
    } else {
        format!("mirror {} into {target} incomplete", report.source)
    };
    let mut parts = vec![headline];
    let mut positive = |count: usize, label: &str| {
        if count > 0 {
            parts.push(format!("{count} {label}"));
        }
    };
    positive(count(MirrorAction::Copied), if report.dry_run { "would be copied" } else { "copied" });
    positive(count(MirrorAction::Updated), if report.dry_run { "would be updated" } else { "updated" });
    positive(count(MirrorAction::Unchanged), "unchanged");
    positive(count(MirrorAction::SnapshotDiffers), "snapshots left as they are");
    positive(count(MirrorAction::Alarm), "alarms");
    positive(count(MirrorAction::Unavailable), "unavailable");
    positive(report.facts_recorded as usize, if report.dry_run { "facts would be recorded" } else { "facts recorded" });
    positive(report.bundles as usize, if report.dry_run { "bundles would be saved" } else { "bundles saved" });
    positive(snapshots, if report.dry_run { "snapshots would be written" } else { "snapshots written" });
    positive(report.removed.unwrap_or(0) as usize, "removed");
    if !report.pruned.is_empty() && report.removed.unwrap_or(0) == 0 {
        positive(report.pruned.len(), if report.dry_run { "would be removed" } else { "planned removals left in place" });
    }
    if report.index_refreshed {
        parts.push(if report.dry_run { "index would be refreshed" } else { "index refreshed" }.into());
    }
    if let Some(commit) = &report.commit {
        parts.push(format!("commit {commit}"));
    }
    parts.join(" · ")
}

fn action_word(action: &MirrorAction, dry_run: bool) -> &'static str {
    match (action, dry_run) {
        (MirrorAction::Copied, true) => "would be copied",
        (MirrorAction::Copied, false) => "copied",
        (MirrorAction::Updated, true) => "would be updated (snapshot moved at the source)",
        (MirrorAction::Updated, false) => "updated (snapshot moved at the source)",
        (MirrorAction::Unchanged, _) => "unchanged",
        (MirrorAction::SnapshotDiffers, _) => {
            "differs at the source (snapshot; left as it is — pass --update-snapshots)"
        }
        (MirrorAction::Alarm, _) => "ALARM",
        (MirrorAction::Unavailable, _) => "unavailable at the source",
    }
}

#[cfg(test)]
mod tests;
