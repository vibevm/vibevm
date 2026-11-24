//! Human alias answers; the machine report stays with the operation.

use super::AliasReport;
use crate::output::{
    Context,
    language::{Cell, Glyph, List, Role, Row, paint},
};

pub(super) fn render(ctx: &Context, report: &AliasReport) {
    if report.action == "list" {
        if ctx.is_quiet() {
            let count = report.aliases.len();
            let unit = if count == 1 { "alias" } else { "aliases" };
            ctx.summary(&format!(
                "vibe registry alias: {count} {unit} in {}",
                report.registry
            ));
        } else if report.aliases.is_empty() {
            ctx.block(&format!(
                "{} registry {} issues no short names\n  {}",
                paint(Role::Dim, Glyph::Skipped.text()),
                paint(Role::Accent, &report.registry),
                paint(
                    Role::Accent,
                    &format!(
                        "vibe registry alias {} add <alias> <group>/<name>",
                        report.registry
                    )
                )
            ));
        } else {
            let rows = report
                .aliases
                .iter()
                .map(|row| {
                    Row::new(vec![
                        Cell::new(&row.alias, Role::Accent),
                        Cell::text("→"),
                        Cell::new(&row.target, Role::Accent),
                    ])
                })
                .collect();
            ctx.block(&List::new(rows).render());
        }
        return;
    }

    let alias = report.alias.as_deref().unwrap_or_default();
    let target = report.target.as_deref().unwrap_or_default();
    let what = match report.action {
        "unchanged" => format!("{alias} already means {target}"),
        "add" if report.dry_run => format!("would add {alias} meaning {target}"),
        "add" => format!("{alias} now means {target}"),
        "remove" if report.dry_run => format!("would withdraw {alias}"),
        "remove" => format!("withdrew {alias}"),
        _ => return,
    };
    ctx.trailer(&format!("{what} · registry {}", report.registry));
}
