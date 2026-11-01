//! The rule that opens a section (PROP-067 `#heading`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#heading");

use super::roles::{Role, paint};
use super::width::{measure, rule_width};

/// `── <label> ────…` to the viewport, dim with the label in the text
/// role, and an optional right-aligned label for context that is not the
/// section's own (the invoking tool, the home).
///
/// ```text
/// rule("plan for org.vibevm.apps/vim@9.1.0", Some("installed by vibevm-app"))
///   ->  ── plan for org.vibevm.apps/vim@9.1.0 ──────── installed by vibevm-app
/// ```
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-067#heading")]
pub(crate) fn rule(label: &str, right: Option<&str>) -> String {
    rule_at(label, right, rule_width())
}

pub(super) fn rule_at(label: &str, right: Option<&str>, width: usize) -> String {
    let left = format!("── {label} ");
    let right_text = right.map(|right| format!(" {right}")).unwrap_or_default();
    let fill = width
        .saturating_sub(measure(&left) + measure(&right_text))
        .max(4);
    format!(
        "{}{}{}{}",
        paint(Role::Dim, "── "),
        label,
        paint(Role::Dim, &format!(" {}", "─".repeat(fill))),
        paint(Role::Dim, &right_text)
    )
}
