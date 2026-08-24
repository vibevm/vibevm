//! C4 and C5 — the two checks that read the GATE LOG rather than the batch.
//!
//! Everything else in this tool reads the diff or the files; these two read
//! what `progress check --exhaustive` said and compare it against the brief's
//! stated predictions. That is a different source of truth and a different
//! failure mode: a divergence here means the brief was wrong or the batch
//! missed units, never that a sentence was reworded.
//!
//! Three of the tool's checks read the brief's predictions and all three are
//! here or beside them, which is why a brief that states none cannot be
//! checked mechanically.

use std::collections::{BTreeMap, BTreeSet};

use super::report::Report;

pub(super) fn c4_gate(
    gate: &str,
    files: &[String],
    expect_unmarked: Option<usize>,
    expect_files: Option<&Vec<String>>,
    expect_total: Option<usize>,
    r: &mut Report,
) {
    // `packages/` / `spec/` are layout-root literals (the gate log
    // prints repo-relative paths); xtask carries no vibe-core edge, so
    // they stay duplicated here with their single home in
    // `crates/vibe-core/src/layout.rs` (PROP-052 L2) — the R4 relayout
    // sweep re-points both prefixes with the physical move.
    let rows: Vec<&str> = gate
        .lines()
        .filter(|l| l.starts_with("packages/") || l.starts_with("spec/"))
        .collect();
    let fset: BTreeSet<&String> = files.iter().collect();
    let mine: Vec<&&str> = rows
        .iter()
        .filter(|l| {
            l.split(':')
                .next()
                .is_some_and(|p| fset.contains(&p.to_string()))
        })
        .collect();

    match expect_total {
        Some(n) if n == rows.len() => {
            r.ok("C4 corpus total", format!("{n} unmarked, as predicted"))
        }
        Some(n) => r.fail(
            "C4 corpus total",
            format!(
                "{} unmarked, predicted {n} (delta {:+})",
                rows.len(),
                rows.len() as i64 - n as i64
            ),
        ),
        None => r.ok(
            "C4 corpus total",
            format!("{} unmarked (no prediction given)", rows.len()),
        ),
    }
    if let Some(n) = expect_unmarked {
        if mine.len() == n {
            r.ok(
                "C4b batch residual",
                format!("{n} unmarked in the batch, as predicted"),
            );
        } else {
            r.fail(
                "C4b batch residual",
                format!("{} unmarked in the batch, predicted {n}", mine.len()),
            );
        }
    }
    if let Some(want) = expect_files {
        let got: BTreeSet<String> = mine
            .iter()
            .filter_map(|l| l.split(':').next().map(str::to_string))
            .collect();
        let want: BTreeSet<String> = want.iter().cloned().collect();
        if got == want {
            r.ok(
                "C4c residual files",
                "residual sits exactly in the predicted file(s)",
            );
        } else {
            r.fail(
                "C4c residual files",
                format!("residual in {got:?}, predicted {want:?}"),
            );
        }
    }
}

pub(super) fn c5_error_classes(gate: &str, files: &[String], r: &mut Report) {
    let fset: BTreeSet<&String> = files.iter().collect();
    let mut classes: BTreeMap<String, usize> = BTreeMap::new();
    for line in gate.lines() {
        // Same layout-root prefixes as `c4_gate` above (PROP-052 L2;
        // single home `crates/vibe-core/src/layout.rs`).
        if !(line.starts_with("packages/") || line.starts_with("spec/")) {
            continue;
        }
        let Some(path) = line.split(':').next() else {
            continue;
        };
        if !fset.contains(&path.to_string()) {
            continue;
        }
        if let Some(open) = line.find('[')
            && let Some(close) = line[open..].find(']')
        {
            let class = line[open + 1..open + close].to_string();
            *classes.entry(class).or_default() += 1;
        }
    }
    let bad: BTreeMap<_, _> = classes
        .iter()
        .filter(|(k, _)| k.as_str() != "unmarked")
        .collect();
    if bad.is_empty() {
        r.ok(
            "C5 error classes",
            format!(
                "batch files carry only [unmarked] ({})",
                classes.get("unmarked").copied().unwrap_or(0)
            ),
        );
    } else {
        r.fail(
            "C5 error classes",
            format!("unexpected classes in batch files: {bad:?}"),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_gate_predictions_are_compared_exactly() {
        // `packages/…` mirrors the gate-log prefixes the filter above
        // reads — test data, not a product path; the single home of the
        // layout names is `crates/vibe-core/src/layout.rs` (PROP-052 L2).
        let gate = "packages/x/a.md:1: Error [unmarked] Para unit carries no marker\n\
                    packages/x/b.md:2: Error [unmarked] Para unit carries no marker\n";
        let files = vec!["packages/x/a.md".to_string()];
        let mut r = Report::default();
        c4_gate(gate, &files, Some(1), None, Some(2), &mut r);
        assert!(!r.failed());

        let mut r = Report::default();
        c4_gate(gate, &files, Some(2), None, Some(2), &mut r);
        assert!(r.failed(), "an off-by-one residual must fail");
    }
}
