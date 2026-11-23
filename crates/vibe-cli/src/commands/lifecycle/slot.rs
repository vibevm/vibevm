use anyhow::Result;
use specmark::spec;
use vibe_lifecycle::RunMetadata;
use vibe_wire::generated::lifecycle_plan::{LifecyclePlan, PlannedContribution};

use crate::output;
use crate::output::language::{
    Cell, Glyph, GlyphSet, List, Meta, Role, Row, join_meta, paint, short_path,
};

/// One row of the human slot plan, read off a plan entry.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-plan")]
pub(super) struct PlanRow<'a> {
    /// The contribution's own id.
    pub(super) id: &'a str,
    /// The handler kind (`script`, `binary`, `agent`, …).
    pub(super) handler: &'a str,
    /// The declared title, if any.
    pub(super) title: Option<&'a str>,
    /// Whether it runs in this pass.
    pub(super) run: bool,
    /// Why it does not, when it does not.
    pub(super) reason: Option<&'a str>,
    /// The conditions the verdict was decided from.
    pub(super) conditions: &'a [String],
    /// An unmet need or a failure does not stop the rest.
    pub(super) optional: bool,
}

/// The label of the rule over one slot target's rows.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#heading")]
pub(super) fn plan_rule_label(target: &str, point: &str) -> String {
    format!("plan for {target} · {point}")
}

/// The rows as one measured list: `N.  id  (handler)  title`, the skip
/// reason or the conditions as metadata, no backticks.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-plan")]
pub(super) fn plan_list(rows: &[PlanRow<'_>], width: usize) -> String {
    let plain = |text: &str| text.replace('`', "");
    let rows = rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let meta = if row.run {
                let mut meta: Vec<Meta> = row
                    .conditions
                    .iter()
                    .map(|c| Meta::note(plain(c)))
                    .collect();
                if row.optional {
                    meta.push(Meta::note("optional"));
                }
                meta
            } else {
                vec![
                    Meta::note(plain(row.reason.unwrap_or("no reason recorded"))),
                    Meta::State("skip".into()),
                ]
            };
            Row::new(vec![
                Cell::text(format!("{}.", index + 1)),
                Cell::text(row.id),
                Cell::new(format!("({})", row.handler), Role::Dim),
                Cell::text(row.title.unwrap_or("")),
            ])
            .meta(meta)
            .dim(!row.run)
        })
        .collect();
    List::new(rows).meta_column().render_lossless_at(width)
}

/// The contribution id inside an execution key
/// `<group>/<name>#<id>@slot(<target>)`; the key itself when it has no `#`.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-plan")]
pub(super) fn id_of_key(key: &str) -> &str {
    key.split_once('#')
        .map(|(_, rest)| rest.split_once("@slot(").map_or(rest, |(id, _)| id))
        .unwrap_or(key)
}

/// The flat record of one contribution's outcome (PROP-067 `#block`,
/// `#children`): the status glyph, the child branch, the id, the title, then
/// a skip's reason, a failure's message and the log path as metadata.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#children")]
pub(super) fn outcome_line(
    set: GlyphSet,
    status: &str,
    id: &str,
    title: Option<&str>,
    message: Option<&str>,
    log: Option<&str>,
) -> String {
    let plain = |text: &str| text.replace('`', "");
    let (glyph, role) = match status {
        "ok" | "fresh" => (Glyph::Done, Role::Ok),
        "skip" | "cancelled" | "delegated" => (Glyph::Skipped, Role::Dim),
        _ => (Glyph::Failed, Role::Alert),
    };
    let mut meta = Vec::new();
    match status {
        "ok" => {}
        "fresh" => meta.push(Meta::note("fresh")),
        "skip" => meta.push(Meta::note(format!(
            "skipped: {}",
            plain(message.unwrap_or("no reason recorded"))
        ))),
        "delegated" => meta.push(Meta::note("waiting for the hosting agent")),
        "cancelled" => meta.push(Meta::note("cancelled")),
        other => meta.push(Meta::Alert(plain(message.unwrap_or(other)))),
    }
    if let Some(log) = log {
        meta.push(Meta::note(format!("log {}", short_path(log))));
    }
    let title = title.map(|title| format!("  {title}")).unwrap_or_default();
    format!(
        "{} {} {id}{title}{}",
        paint(role, glyph.in_set(set)),
        Glyph::Child.in_set(set),
        join_meta(&meta)
    )
}

pub(crate) fn emit_transition_outcome(
    ctx: &output::Context,
    metadata: &RunMetadata,
    report: &vibe_install::SlotLifecycleReport,
    label: Option<(&str, Option<&str>)>,
    log: Option<&std::path::Path>,
) -> Result<()> {
    if ctx.is_quiet() || ctx.suppresses_output() {
        return Ok(());
    }
    // JSON narrates NOTHING per row: the outermost command emits exactly one
    // document, and every slot row this install ran reaches it as a typed
    // contribution. A per-row echo here was a second (and third) document on
    // the same stdout.
    let _ = metadata;
    if ctx.is_json() {
        return Ok(());
    }
    let (id, title) = label.unwrap_or((id_of_key(&report.key), None));
    let log = log.map(|path| path.display().to_string());
    let line = outcome_line(
        GlyphSet::detect(),
        &report.status,
        id,
        title,
        report.message.as_deref(),
        log.as_deref(),
    );
    let gap = crate::output::language::before(
        crate::output::language::Stream::Out,
        crate::output::language::Kind::Row("slot outcomes"),
    );
    ctx.suspend_progress(|| println!("{gap}{line}"));
    Ok(())
}

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#SURFACE-THE-RITUAL")]
pub(crate) fn surface_plan(
    ctx: &output::Context,
    plan: &vibe_install::SlotLifecyclePlan,
    metadata: &RunMetadata,
) -> Result<()> {
    if plan.entries.is_empty() || ctx.suppresses_output() || ctx.is_quiet() {
        return Ok(());
    }
    if ctx.is_json() {
        return ctx.defer_json_plan(&LifecyclePlan {
            chain: metadata.chain.clone(),
            command: "lifecycle:plan".into(),
            contributions: plan
                .entries
                .iter()
                .map(|entry| PlannedContribution {
                    handler: entry.handler.clone(),
                    key: entry.key.clone(),
                    phase: "install".into(),
                    point: entry.point.clone(),
                    provider: entry.provider.clone(),
                    reference: Some(entry.reference.clone()),
                    slot_target: Some(vibe_wire::generated::lifecycle_plan::SlotTarget {
                        group: entry.slot_target.group.clone(),
                        kind: entry.slot_target.kind.clone(),
                        name: entry.slot_target.name.clone(),
                        root: entry.slot_target.root.clone(),
                        version: entry.slot_target.version.clone(),
                    }),
                    tier: entry.tier.clone(),
                    version: entry.version.clone(),
                })
                .collect(),
            notices: Vec::new(),
            requested: metadata.requested.clone(),
        });
    }
    let mut entries = plan.entries.as_slice();
    while let Some(first) = entries.first() {
        let target = format!(
            "{}/{}@{}",
            first.slot_target.group, first.slot_target.name, first.slot_target.version
        );
        let count = entries
            .iter()
            .take_while(|entry| {
                entry.slot_target.group == first.slot_target.group
                    && entry.slot_target.name == first.slot_target.name
                    && entry.slot_target.version == first.slot_target.version
                    && entry.point == first.point
            })
            .count();
        let rows: Vec<_> = entries[..count]
            .iter()
            .map(|entry| PlanRow {
                id: &entry.id,
                handler: &entry.handler,
                title: entry.title.as_deref(),
                run: entry.run,
                reason: entry.reason.as_deref(),
                conditions: &entry.conditions,
                optional: entry.optional,
            })
            .collect();
        ctx.rule(&plan_rule_label(&target, &first.point));
        ctx.block(&plan_list(&rows, crate::output::language::viewport()));
        entries = &entries[count..];
    }
    Ok(())
}

#[cfg(test)]
#[path = "slot/tests.rs"]
mod tests;
