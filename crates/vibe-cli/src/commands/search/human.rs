//! The human search answer: measured hits, issued short names and registries.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use anyhow::Result;

use super::report::{HitRow, SearchReport};
use crate::output;
use crate::output::language::{
    Block, Cell, Glyph, List, Meta, Role, Row, cut, hint, rule_at, rule_width, viewport,
};

/// Print the human answer without changing the indexless install guidance.
pub(super) fn render(ctx: &output::Context, report: &SearchReport) -> Result<()> {
    if report.hits.is_empty() && report.registries_searched.is_empty() {
        let registries = registries(report, viewport(), rule_width());
        if !registries.is_empty() {
            ctx.block(&registries);
        }
        ctx.summary(
            "(no registry has VIBEVM_INDEX_URL_<R> configured; search returns empty.\n\
             To install a known package, run `vibe install <group>/<name>` directly —\n\
             install resolves through `[[registry]]` over git and does not need an\n\
             index. The index is a discovery optimisation, not a runtime dependency.\n\
             See docs/commands/search.md for setting up an index server.)",
        );
        return Ok(());
    }
    ctx.block(&view(report, viewport(), rule_width()));
    Ok(())
}

/// The search answer at a fixed viewport.
///
/// Results retain kind, qualified name, version and registry provenance.
/// Separate measured sections explain issued short names and registry failures;
/// descriptions and remedies remain child rows under the corresponding result.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
fn view(report: &SearchReport, width: usize, rule: usize) -> String {
    let mut parts = Vec::new();
    if report.hits.is_empty() {
        if !report.registries_searched.is_empty() {
            parts.push(
                Block::new(
                    Glyph::Skipped,
                    format!("no package matches {}", report.query),
                )
                .with_meta(vec![Meta::note(format!(
                    "searched {}",
                    report.registries_searched.join(" · ")
                ))])
                .render_at(width),
            );
        }
    } else {
        let rows = report.hits.iter().map(|hit| {
            let mut meta = vec![
                Meta::note(&hit.registry),
                Meta::note(format!("score {}", hit.score)),
            ];
            if hit.source != "index" {
                meta.push(Meta::State("full-scan".into()));
            }
            let mut row = Row::new(vec![
                Cell::text(&hit.kind),
                Cell::new(qualified(hit), Role::Accent),
                Cell::text(hit.latest_stable.as_deref().unwrap_or("-")),
            ])
            .meta(meta);
            if let Some(description) = hit.description.as_deref().filter(|text| !text.is_empty()) {
                row = row.child(cut(description, width.saturating_sub(10)));
            }
            row
        });
        let right = format!(
            "{} · {} searched",
            count(report.hits.len(), "hit", "hits"),
            count(report.registries_searched.len(), "registry", "registries")
        );
        let mut hits = format!(
            "{}\n{}\n{}",
            heading(
                &format!("search {}", report.query),
                Some(&right),
                width,
                rule
            ),
            List::new(rows.collect())
                .meta_column()
                .children_under(1)
                .render_at(width),
            hint_line(
                &format!("searched {}", report.registries_searched.join(" · ")),
                width
            )
        );
        if !report.registries_full_scanned.is_empty() {
            hits.push('\n');
            hits.push_str(&hint_line(
                &format!(
                    "full-scanned {}",
                    report.registries_full_scanned.join(" · ")
                ),
                width,
            ));
        }
        parts.push(hits);
    }
    let rows: Vec<_> = report
        .hits
        .iter()
        .filter(|hit| !hit.short_names.is_empty())
        .map(|hit| {
            Row::new(vec![
                Cell::new(hit.short_names.join(", "), Role::Accent),
                Cell::text(qualified(hit)),
            ])
            .meta(vec![Meta::note(&hit.registry)])
        })
        .collect();
    if !rows.is_empty() {
        parts.push(format!(
            "{}\n{}\n{}",
            heading("short names", None, width, rule),
            List::new(rows).render_at(width),
            hint_line(
                "type these bare; every other hit needs its group/name",
                width
            )
        ));
    }
    let registries = registries(report, width, rule);
    if !registries.is_empty() {
        parts.push(registries);
    }
    bounded(&parts.join("\n\n"), width)
}

fn registries(report: &SearchReport, width: usize, rule: usize) -> String {
    let mut rows = Vec::new();
    for registry in &report.registries_unreachable {
        rows.push(
            Row::new(vec![Cell::text(&registry.name), Cell::text("unreachable")])
                .glyph(Glyph::Failed, Role::Alert)
                .child(cut(&registry.reason, width.saturating_sub(6))),
        );
    }
    for registry in &report.registries_policy_skipped {
        rows.push(
            Row::new(vec![
                Cell::text(&registry.name),
                Cell::text("skipped by source policy"),
            ])
            .glyph(Glyph::Skipped, Role::Dim)
            .child(cut(&registry.reason, width.saturating_sub(6))),
        );
    }
    for registry in &report.registries_full_scan_unsupported {
        rows.push(
            Row::new(vec![
                Cell::text(&registry.name),
                Cell::text("full scan unsupported"),
            ])
            .glyph(Glyph::Skipped, Role::Dim)
            .child(cut(&registry.reason, width.saturating_sub(6))),
        );
    }
    for name in &report.registries_unconfigured {
        rows.push(
            Row::new(vec![Cell::text(name), Cell::text("no index URL")])
                .glyph(Glyph::Skipped, Role::Dim)
                .child(cut(
                    "set VIBEVM_INDEX_URL_<R> to search it",
                    width.saturating_sub(6),
                )),
        );
    }
    if rows.is_empty() {
        return String::new();
    }
    bounded(
        &format!(
            "{}\n{}",
            heading("registries", None, width, rule),
            List::new(rows).render_at(width)
        ),
        width,
    )
}

// The shared rule reserves at least four fill cells. Budget labels before
// calling it so even wide Unicode queries leave room for those cells.
fn heading(label: &str, right: Option<&str>, width: usize, rule: usize) -> String {
    let width = width.min(rule);
    if width < 8 {
        return cut(&rule_at(label, right, width), width);
    }
    let right = right.filter(|right| {
        console::measure_text_width(label) + console::measure_text_width(right) < width - 8
    });
    let budget = width - 8 - right.map_or(0, |text| console::measure_text_width(text) + 1);
    rule_at(&cut(label, budget), right, width)
}

fn bounded(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    text.lines()
        .map(|line| cut(line, width))
        .collect::<Vec<_>>()
        .join("\n")
}

fn hint_line(text: &str, width: usize) -> String {
    cut(&format!("  {}", hint(text)), width)
}

fn qualified(hit: &HitRow) -> String {
    match &hit.group {
        Some(group) => format!("{group}/{}", hit.name),
        None => hit.name.clone(),
    }
}

fn count(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

#[cfg(test)]
#[path = "human/tests.rs"]
mod tests;
