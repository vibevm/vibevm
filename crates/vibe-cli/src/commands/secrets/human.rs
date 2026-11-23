//! Human rendering of keyring metadata; value bytes never reach it.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-000#TS-USER-KEYRING");

use crate::output::language::{Cell, List, Role, Row, paint, rule_at, rule_width, viewport};

/// One item as the listing shows it: id, label, kind, current revision.
pub(super) type ItemRow = (String, String, String, String);

/// The keyring listing at this terminal's width.
pub(super) fn items(store_revision: u64, rows: &[ItemRow]) -> String {
    items_at(store_revision, rows, viewport(), rule_width())
}

/// The keyring listing: a rule naming the store revision, then one row per
/// item (id, label, kind, `[value hidden]`) with its revision as a child, so
/// neither identifier is ever cut. `width` bounds the rows, `rule` the rule.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
pub(super) fn items_at(store_revision: u64, rows: &[ItemRow], width: usize, rule: usize) -> String {
    let separator = crate::output::language::heading_separator();
    let heading = rule_at(
        &format!("local keyring {separator} store revision {store_revision}"),
        None,
        rule,
    );
    if rows.is_empty() {
        return format!(
            "{heading}\n\n  The local keyring is empty. Add an item with\n  {}",
            paint(
                Role::Accent,
                "vibe secrets create --kind <kind> --label <label> --value-stdin"
            )
        );
    }
    let rows = rows
        .iter()
        .map(|(id, label, kind, revision)| {
            Row::new(vec![
                Cell::text(id.clone()),
                Cell::text(label.clone()),
                Cell::new(format!("({kind})"), Role::Dim),
                Cell::new("[value hidden]", Role::Dim),
            ])
            .child(format!("revision {revision}"))
        })
        .collect();
    format!(
        "{heading}\n\n{}",
        List::new(rows).children_under(0).render_at(width)
    )
}

/// The consumers of one item at this terminal's width.
pub(super) fn references(id: &str, consumers: &[String]) -> String {
    references_at(id, consumers, rule_width())
}

/// The consumers of one item: a rule naming the item, then one consumer id
/// per line, or the sentence that there is none.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#heading")]
pub(super) fn references_at(id: &str, consumers: &[String], rule: usize) -> String {
    let heading = rule_at(&format!("consumers of {id}"), None, rule);
    if consumers.is_empty() {
        return format!("{heading}\n\n  No registered consumer references this item.");
    }
    let lines: Vec<String> = consumers
        .iter()
        .map(|consumer| format!("  {consumer}"))
        .collect();
    format!("{heading}\n\n{}", lines.join("\n"))
}

/// The one line of a quiet listing: `2 keyring items · store revision 5`.
pub(super) fn count_line(items: usize, store_revision: u64) -> String {
    let noun = if items == 1 {
        "keyring item"
    } else {
        "keyring items"
    };
    format!("{items} {noun} · store revision {store_revision}")
}

/// The one line of quiet references: `1 consumer references <id>`.
pub(super) fn consumers_line(id: &str, consumers: usize) -> String {
    match consumers {
        0 => format!("no consumer references {id}"),
        1 => format!("1 consumer references {id}"),
        many => format!("{many} consumers reference {id}"),
    }
}
