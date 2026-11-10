//! The mirror report's own human form: one line per version that changed,
//! and the summary.
//!
//! The other two halves of a run's output are drawn where their facts are
//! known — the `[[template]]` rows by `super::payloads` once the payload
//! projection exists, and the prune plan by `super::prune` before anything
//! is removed. What is left here is the report itself, which is why it
//! lives beside the run rather than in it.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror");

use anyhow::Result;
use vibe_wire::generated::registry_mirror_report::{MirrorAction, RegistryMirrorReport};

use crate::output;

pub(super) fn render(ctx: &output::Context, report: &RegistryMirrorReport) -> Result<()> {
    if ctx.is_json() {
        return ctx.emit_json(report);
    }
    let count = |action: MirrorAction| {
        report
            .versions
            .iter()
            .filter(|v| v.action == action)
            .count()
    };
    for row in &report.versions {
        if row.action == MirrorAction::Unchanged {
            continue;
        }
        let verb = action_word(&row.action, report.dry_run);
        let note = row
            .note
            .as_deref()
            .map(|n| format!(" — {n}"))
            .unwrap_or_default();
        ctx.step(&format!("{}@{} {verb}{note}", row.package, row.version));
    }
    let commit = report
        .commit
        .as_deref()
        .map(|c| format!(", commit {c}"))
        .unwrap_or_default();
    // What `--prune` did, said once in the summary: the rows themselves
    // were printed as the plan, before anything went.
    let pruned = match (report.pruned.is_empty(), report.removed) {
        (true, _) => String::new(),
        (false, Some(0)) => format!(", {} to remove", report.pruned.len()),
        (false, Some(removed)) => format!(", {removed} removed"),
        (false, None) => String::new(),
    };
    ctx.summary(&format!(
        "\nvibe registry mirror: {} {}, {} {}, {} unchanged, {} snapshot(s) left as they are, \
         {} alarm(s), {} unavailable; {} fact(s) recorded{pruned}{commit}.",
        count(MirrorAction::Copied),
        if report.dry_run { "to copy" } else { "copied" },
        count(MirrorAction::Updated),
        if report.dry_run {
            "to update"
        } else {
            "updated"
        },
        count(MirrorAction::Unchanged),
        count(MirrorAction::SnapshotDiffers),
        count(MirrorAction::Alarm),
        count(MirrorAction::Unavailable),
        report.facts_recorded,
    ));
    Ok(())
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
