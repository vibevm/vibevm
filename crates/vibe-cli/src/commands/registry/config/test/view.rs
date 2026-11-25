//! Registry probe rows and their reachable count.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#registry");

use super::TestReportRegistry;
use crate::output::language::{Cell, Glyph, List, Meta, Role, Row, cut, hint, rule_at};

/// Draw every probe as a measured row with its diagnostic beneath the URL.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
pub(super) fn render(rows: &[TestReportRegistry], width: usize, rule: usize) -> String {
    let rendered = rows
        .iter()
        .map(|row| {
            let reachable = row.status == "reachable";
            let (glyph, role) = if reachable {
                (Glyph::Done, Role::Ok)
            } else {
                (Glyph::Failed, Role::Alert)
            };
            let status = if reachable {
                Meta::note(row.status)
            } else {
                Meta::Alert(row.status.into())
            };
            let item = Row::new(vec![Cell::text(&row.name), Cell::text(&row.url)])
                .glyph(glyph, role)
                .meta(vec![status, Meta::note(format!("auth {}", row.auth))]);
            match &row.note {
                Some(note) => item.child(note.replace('`', "")),
                None => item,
            }
        })
        .collect();
    format!(
        "{}\n{}\n{}",
        rule_at("registry test", None, rule),
        List::new(rendered).children_under(1).render_at(width),
        cut(&format!("  {}", hint(&reachable_line(rows))), width)
    )
}

/// Count reachable registries without discarding any failed rows.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
pub(super) fn reachable_line(rows: &[TestReportRegistry]) -> String {
    let reachable = rows.iter().filter(|row| row.status == "reachable").count();
    format!("{reachable} of {} reachable", rows.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_test_rows_classify_and_count() {
        let rows = vec![
            TestReportRegistry {
                name: "corp2".into(),
                url: "file:///srv/gitreg".into(),
                auth: "none",
                status: "reachable",
                note: Some("index: absent".into()),
            },
            TestReportRegistry {
                name: "corp4".into(),
                url: "file:///srv/reg2".into(),
                auth: "none",
                status: "unknown",
                note: Some("registry returned an unclassified failure".into()),
            },
            TestReportRegistry {
                name: "broken".into(),
                url: "file:///nonexistent/reg".into(),
                auth: "transport-default",
                status: "missing",
                note: Some("registry root /nonexistent/reg does not exist".into()),
            },
        ];
        assert_eq!(
            render(&rows, 100, 100),
            concat!(
                "── registry test ───────────────────────────────────────────────────────────────────────────────────\n",
                "✓ corp2   file:///srv/gitreg · reachable · auth none\n",
                "          └ index: absent\n",
                "✗ corp4   file:///srv/reg2 · unknown · auth none\n",
                "          └ registry returned an unclassified failure\n",
                "✗ broken  file:///nonexistent/reg · missing · auth transport-default\n",
                "          └ registry root /nonexistent/reg does not exist\n",
                "  1 of 3 reachable"
            )
        );
    }
}
