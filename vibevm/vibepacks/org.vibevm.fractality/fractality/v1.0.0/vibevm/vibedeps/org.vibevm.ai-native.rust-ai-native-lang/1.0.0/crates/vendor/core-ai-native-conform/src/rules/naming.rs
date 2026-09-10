//! The naming rule family — `cell-name-is-computed` (B-038): a cell's
//! canonical type name is COMPUTED from its manifest, not chosen freely.
//! The composer is `Pascal(variant)` followed by the seam SPELLED AS
//! WRITTEN — `SatDepSolver`, not `SatDepsolver` — so a multi-word seam
//! survives intact (the owner's fork №1, taken 2026-08-04). One engine
//! rule reads both languages' manifests through the SAME attr text: the
//! Rust frontend lowers `#[cell(seam, variant)]` verbatim and the Go
//! bridge renders `//spec:cell seam= variant=` into the same
//! `cell(seam = "…", variant = "…")` string, so a second grammar form
//! never appears. TypeScript carries a recorded reason — it has no cell
//! manifest to compute from.

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules");

use crate::facts::{Fact, SourceFacts};
use crate::finding::{Finding, FindingStatus, Rule};

use super::req_message;

/// The naming card (B-038). Authored per-projection — both the Rust and
/// Go card indexes list `rule-closed-vocabulary-naming` under «pending
/// cards (named, not yet authored)» — so the shared rule cites the card
/// by name in the Rust projection, its lineage home: the `pascal`
/// codemod and the `WalFreshnessCheck` house style both live there, and
/// the design (§4 `b038-fork`) measures the convention on the Rust
/// tree. Provisional until the card is authored; the card name is
/// identical across projections, so a future per-projection or neutral
/// home is a one-line edit of this constant.
const NAMING_CARD: &str =
    "discipline://rust-ai-native-lang/cards/rule-closed-vocabulary-naming#ops";

/// `cell-name-is-computed` — a cell's type name is its manifest
/// composed: `Pascal(variant)` + the `seam` spelled verbatim. The rule
/// reads the `cell(seam = "…", variant = "…")` attr — the Rust frontend
/// lowers it from `#[cell(…)]` and the Go bridge renders it from
/// `//spec:cell seam= variant=` — and compares the declared type name's
/// final path segment against the composed one. A manifest missing
/// either key has nothing to compose and is skipped silently (the
/// vacuity guard); extra keys (`replaces`, `flag`) are tolerated and
/// ignored.
///
/// The seam is substituted verbatim, never re-cased: `SatDepSolver`,
/// not `SatDepsolver` — a multi-word seam keeps its spelling, which is
/// the whole point of computing the name from the manifest.
///
/// ```
/// use core_ai_native_conform::rules::CellNameIsComputed;
/// use core_ai_native_conform::{Fact, Rule, SourceFacts};
///
/// let rule = CellNameIsComputed;
/// // Compliant: variant `sat`, seam `DepSolver` → `SatDepSolver`.
/// let ok = SourceFacts {
///     file: "crates/x/src/sat.rs".into(), crate_name: "x".into(),
///     facts: vec![Fact::Item {
///         kind: "struct".into(), symbol: "x::sat::SatDepSolver".into(), line: 3,
///         attrs: vec!["cell(seam = \"DepSolver\", variant = \"sat\")".into()],
///         is_pub: true, has_doctest: false,
///     }],
/// };
/// assert!(rule.check(&[ok]).is_empty());
/// // A divergent name reds, and the message names both names.
/// let bad = SourceFacts {
///     file: "crates/x/src/sat.rs".into(), crate_name: "x".into(),
///     facts: vec![Fact::Item {
///         kind: "struct".into(), symbol: "x::sat::Sat".into(), line: 3,
///         attrs: vec!["cell(seam = \"DepSolver\", variant = \"sat\")".into()],
///         is_pub: true, has_doctest: false,
///     }],
/// };
/// let findings = rule.check(&[bad]);
/// assert_eq!(findings.len(), 1);
/// assert!(findings[0].message.contains("SatDepSolver"));
/// assert!(findings[0].message.contains("`Sat`"));
/// assert!(core_ai_native_conform::rules::matches_req_grammar(&findings[0].message));
/// // A manifest with neither key has nothing to compose — silent.
/// let bare = SourceFacts {
///     file: "crates/x/src/sat.rs".into(), crate_name: "x".into(),
///     facts: vec![Fact::Item {
///         kind: "struct".into(), symbol: "x::sat::S".into(), line: 3,
///         attrs: vec!["cell(replaces = \"naive\")".into()],
///         is_pub: true, has_doctest: false,
///     }],
/// };
/// assert!(rule.check(&[bare]).is_empty());
/// ```
pub struct CellNameIsComputed;

impl Rule for CellNameIsComputed {
    fn id(&self) -> &'static str {
        "cell-name-is-computed"
    }
    fn why(&self) -> &'static str {
        "a cell's name is computed from its manifest — Pascal(variant) \
         followed by the seam as written — so the type announces which \
         variant of which seam it is, and a rename rides a compiler-checked \
         edit, not memory (the cell-name convention; B-038 fork №1)"
    }
    fn check(&self, facts: &[SourceFacts]) -> Vec<Finding> {
        let mut out = Vec::new();
        for sf in facts {
            for f in &sf.facts {
                let Fact::Item {
                    symbol,
                    attrs,
                    line,
                    ..
                } = f
                else {
                    continue;
                };
                let Some(cell_attr) = attrs.iter().find(|a| a.starts_with("cell(")) else {
                    continue;
                };
                // Both keys are needed to compose a name; missing either
                // is "nothing to compute" and skipped silently (the
                // vacuity guard — a manifest with neither key, or a
                // malformed directive, never reds).
                let Some(seam) = quoted_value(cell_attr, "seam") else {
                    continue;
                };
                let Some(variant) = quoted_value(cell_attr, "variant") else {
                    continue;
                };
                let computed = format!("{}{}", pascal(&variant), seam);
                let declared = symbol.rsplit("::").next().unwrap_or(symbol);
                if declared == computed {
                    continue;
                }
                out.push(Finding {
                    rule: self.id(),
                    file: sf.file.clone(),
                    line: *line,
                    message: req_message(
                        NAMING_CARD,
                        &format!(
                            "cell type `{declared}` is not its computed name \
                             `{computed}` (Pascal(variant `{variant}`) + seam `{seam}`)"
                        ),
                        &format!("rename the type `{declared}` → `{computed}`"),
                    ),
                    why: self.why(),
                    // Fingerprint by file + declared name, never by line
                    // (the stop.rs 33→35 lesson): a line-keyed baseline
                    // rots on any edit above the cell, and a baseline that
                    // rots on unrelated edits is a checker that lies.
                    fingerprint: format!("cell-name-is-computed|{}|{declared}", sf.file),
                    status: FindingStatus::Live,
                    evidence: f.summary(),
                });
            }
        }
        out.sort();
        out
    }
}

/// Compose the PascalCase head from a `variant`: split on `_` and `-`
/// (both are word separators — the house style's `wal-freshness` variant
/// composes `WalFreshness`), capitalize each word's first letter, keep
/// the rest as written. Mirrors the `pascal` the
/// `rust-ai-native codemod add-cell` scaffolds from, so the rule and
/// the codemod agree on every name.
fn pascal(variant: &str) -> String {
    variant
        .split(['_', '-'])
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut chars = s.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

/// Extract a `key = "value"` pair's value from a `cell(…)` attr string.
/// Tolerates spacing, key order, and extra keys (`replaces`, `flag`),
/// reading the value as the text between the first `"` after the key
/// and its closing `"`. The key is matched at a token boundary (a
/// preceding non-ident byte, followed by optional spaces and `=`), so a
/// key substring inside a value or another word never matches. The one
/// parser both the Rust frontend's verbatim attr and the Go bridge's
/// rendered attr feed.
fn quoted_value(attr: &str, key: &str) -> Option<String> {
    let mut rest = attr;
    loop {
        let pos = rest.find(key)?;
        let before_ok = pos == 0 || {
            let b = rest.as_bytes()[pos - 1];
            !(b == b'_' || b.is_ascii_alphanumeric())
        };
        let after = &rest[pos + key.len()..];
        let after_eq = after.trim_start();
        if before_ok && after_eq.starts_with('=') {
            let tail = after_eq[1..].trim_start();
            let open = tail.find('"')?;
            let value = &tail[open + 1..];
            let close = value.find('"')?;
            return Some(value[..close].to_string());
        }
        rest = &rest[pos + key.len()..];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The codemod's own cases, plus the hyphen separator the house
    /// style uses (`wal-freshness` → `WalFreshness`).
    #[test]
    fn pascal_matches_the_codemod_and_splits_on_hyphen() {
        assert_eq!(pascal("sat"), "Sat");
        assert_eq!(pascal("sat_solver"), "SatSolver");
        assert_eq!(pascal("wal-freshness"), "WalFreshness");
        assert_eq!(pascal("batch"), "Batch");
    }

    #[test]
    fn quoted_value_tolerates_order_spacing_and_extra_keys() {
        // Rust frontend shape, both orderings.
        let a = r#"cell(seam = "DepSolver", variant = "sat")"#;
        let b = r#"cell(variant = "sat", seam = "DepSolver")"#;
        assert_eq!(quoted_value(a, "seam").as_deref(), Some("DepSolver"));
        assert_eq!(quoted_value(a, "variant").as_deref(), Some("sat"));
        assert_eq!(quoted_value(b, "seam").as_deref(), Some("DepSolver"));
        assert_eq!(quoted_value(b, "variant").as_deref(), Some("sat"));
        // Extra keys are ignored; tight spacing is tolerated.
        let c = r#"cell(seam="Planner",variant="batch",replaces="naive",flag="planner")"#;
        assert_eq!(quoted_value(c, "seam").as_deref(), Some("Planner"));
        assert_eq!(quoted_value(c, "variant").as_deref(), Some("batch"));
        assert_eq!(quoted_value(c, "replaces").as_deref(), Some("naive"));
    }

    #[test]
    fn quoted_value_keys_inside_values_do_not_match() {
        // `variant` appears inside the seam value first — the boundary
        // check skips it and lands on the real key.
        let attr = r#"cell(seam = "variant", variant = "x")"#;
        assert_eq!(quoted_value(attr, "variant").as_deref(), Some("x"));
        assert_eq!(quoted_value(attr, "seam").as_deref(), Some("variant"));
    }

    #[test]
    fn missing_key_is_none() {
        let attr = r#"cell(seam = "S")"#;
        assert_eq!(quoted_value(attr, "variant"), None);
        assert_eq!(quoted_value(attr, "seam").as_deref(), Some("S"));
    }
}
