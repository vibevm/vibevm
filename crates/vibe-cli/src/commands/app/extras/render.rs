//! Measured ecosystem listings and the glyph blocks of an extras run.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#list");

use vibe_wire::generated::application_ecosystem as wire;
use vibe_wire::generated::application_extras as extras_wire;

use super::super::report::prose;
use crate::output::language::{
    Block, Cell, Glyph, LabelValue, Line, List, Meta, Role, Row, hint, rule_at, short_path,
};

/// One rule per ecosystem over its recommendations and observed inventory.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
pub(super) fn listing_text(
    document: &extras_wire::ApplicationExtras,
    width: usize,
    rule: usize,
) -> String {
    let sections = document
        .ecosystems
        .iter()
        .map(|row| listing_section(document, row, width, rule))
        .collect::<Vec<_>>()
        .join("\n\n");
    let notices = document
        .notices
        .iter()
        .map(|notice| format!("  {}", hint(&prose(notice))))
        .collect::<Vec<_>>()
        .join("\n");
    join_parts(&sections, &notices)
}

fn listing_section(
    document: &extras_wire::ApplicationExtras,
    row: &extras_wire::Ecosystem,
    width: usize,
    rule: usize,
) -> String {
    let heading = rule_at(
        &format!(
            "extras of {} {} · {}",
            document.slot, document.version, row.id
        ),
        None,
        rule,
    );
    let mut items = Vec::new();
    for recommendation in &row.recommendations {
        let (glyph, role, word) = if recommendation.dropped {
            (Glyph::Failed, Role::Alert, "dropped")
        } else if recommendation.enabled {
            (Glyph::Done, Role::Ok, "chosen")
        } else {
            (Glyph::Skipped, Role::Dim, "not chosen")
        };
        items.push(
            Row::new(vec![
                Cell::text(&recommendation.id),
                Cell::text(&recommendation.item),
                Cell::text(&recommendation.name),
            ])
            .glyph(glyph, role)
            .dim(!recommendation.dropped && !recommendation.enabled)
            .meta(vec![Meta::State(word.into())]),
        );
    }
    for item in &row.add {
        items.push(Row::new(vec![
            Cell::new("yours", Role::Dim),
            Cell::text(item),
        ]));
    }
    let mut summary = vec![summary_row("final list", item_cell(&row.items))];
    if !row.applied.is_empty() {
        summary.push(summary_row(
            "last applied",
            Cell::text(row.applied.join(" ")),
        ));
    }
    summary.push(match &row.observed_failed {
        Some(reason) => summary_row("not observed", Cell::new(prose(reason), Role::Warn)),
        None => summary_row("installed", item_cell(&row.observed)),
    });
    let summary = List::new(summary).render_at(width);
    if items.is_empty() {
        format!("{heading}\n\n{summary}")
    } else {
        format!(
            "{heading}\n\n{}\n{summary}",
            List::new(items).meta_column().render_at(width)
        )
    }
}

fn summary_row(label: &str, value: Cell) -> Row {
    Row::new(vec![Cell::new(label, Role::Dim), value])
}

fn item_cell(items: &[String]) -> Cell {
    if items.is_empty() {
        Cell::new("none", Role::Dim)
    } else {
        Cell::text(items.join(" "))
    }
}

/// Quiet output retains one plain final item list per ecosystem.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
pub(super) fn listing_quiet(document: &extras_wire::ApplicationExtras) -> Vec<String> {
    document
        .ecosystems
        .iter()
        .map(|row| format!("{}  {}", row.id, item_cell(&row.items).text))
        .collect()
}

/// One glyph block per adapter, retaining why, retry, notices and work.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
pub(super) fn run_text(document: &wire::ApplicationEcosystem, width: usize) -> String {
    let w = document
        .adapters
        .iter()
        .map(|row| row.id.chars().count())
        .max()
        .unwrap_or(0);
    let title = match document.verb {
        wire::Verb::Install => "install the extras",
        wire::Verb::List => "list the installed items",
        wire::Verb::Remove => "remove the items",
        wire::Verb::SelfUpgrade => "upgrade the package manager",
    };
    let blocks = document
        .adapters
        .iter()
        .map(|row| run_block(row, &format!("{:<w$}  {title}", row.id)).render_at(width))
        .collect::<Vec<_>>()
        .join("\n");
    let mut hints = document
        .notices
        .iter()
        .map(|notice| prose(notice))
        .collect::<Vec<_>>();
    if !document.work.is_empty() {
        hints.push(format!(
            "work directory kept {}",
            short_path(&document.work)
        ));
    }
    let hints = hints
        .iter()
        .map(|text| format!("  {}", hint(text)))
        .collect::<Vec<_>>()
        .join("\n");
    join_parts(&blocks, &hints)
}

fn run_block(row: &wire::Adapter, headline: &str) -> Block {
    let mut meta = Vec::new();
    let mut body = Vec::new();
    let glyph = match row.outcome {
        wire::Outcome::Done => {
            if !row.observed.is_empty() {
                meta.push(Meta::note(format!("installed {}", row.observed.join(" "))));
            }
            Glyph::Done
        }
        wire::Outcome::Failed => {
            meta.push(Meta::Alert("failed".into()));
            if let Some(reason) = &row.reason {
                body.push(Line::Label(
                    "why".into(),
                    vec![LabelValue::Text(prose(reason))],
                ));
            }
            if let Some(retry) = &row.retry {
                body.push(Line::Label(
                    "retry".into(),
                    vec![LabelValue::Command(retry.clone())],
                ));
            }
            Glyph::Failed
        }
        wire::Outcome::Skipped => {
            meta.push(Meta::note(row.reason.as_ref().map_or_else(
                || "skipped".into(),
                |reason| format!("skipped: {}", prose(reason)),
            )));
            Glyph::Skipped
        }
    };
    if row.outcome != wire::Outcome::Skipped
        && let Some(log) = &row.log
    {
        meta.push(Meta::note(format!("log {}", short_path(log))));
    }
    Block::new(glyph, headline).with_meta(meta).with_body(body)
}

fn join_parts(first: &str, second: &str) -> String {
    [first, second]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// The final verdict belongs to the run, not to a successful runtime install.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#trailer")]
pub(super) fn run_trailer(document: &wire::ApplicationEcosystem) -> (bool, String) {
    let word = match document.verb {
        wire::Verb::Install => "applied",
        wire::Verb::List => "listed",
        wire::Verb::Remove => "removed",
        wire::Verb::SelfUpgrade => "upgraded",
    };
    let failed = document
        .adapters
        .iter()
        .filter(|row| row.outcome == wire::Outcome::Failed)
        .count();
    let done = document
        .adapters
        .iter()
        .filter(|row| row.outcome == wire::Outcome::Done)
        .count();
    let text = if document.ok {
        format!("extras of {} {word}", document.slot)
    } else if done > 0 {
        format!(
            "extras of {} partly {word} · {failed} failed",
            document.slot
        )
    } else {
        format!("extras of {} not {word} · {failed} failed", document.slot)
    };
    (document.ok, text)
}

#[cfg(test)]
#[path = "render/tests.rs"]
mod tests;
