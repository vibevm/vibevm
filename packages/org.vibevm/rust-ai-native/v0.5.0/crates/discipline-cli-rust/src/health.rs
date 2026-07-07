//! `discipline-rust health` — the Discipline health collector (the Sweep
//! Playbook's fact-gatherer — `spec://discipline-core/04-SWEEP-PLAYBOOK#collector`).
//!
//! The binary gates (`conform check`, `specmap --check`, `self-check.sh`)
//! answer one question — *is the declared surface conformant right now?* —
//! and answer it pass/fail. This collector answers the questions the gates
//! leave on the table, the ones a recurring sweep needs to keep the tree
//! inside the Discipline as it grows:
//!
//! - **Coverage beyond the gated set.** `pub-doctest` gates only
//!   the crates in conform.toml's `gated_pub_doctest`; every other crate's *types*
//!   are doctest-unchecked. This reports each crate's public-type doctest
//!   coverage so the sweep can drain the smallest gap next and widen the gate
//!   (the PUBDOC-DRAIN ratchet, generalised).
//! - **Early warning, not violation.** Files in the `[540, 600)` danger band
//!   are not yet `file-length` findings — but one added line trips them ("files
//!   at 600 are landmines", CONVERT-PLAN v0.1). The sweep splits them first.
//! - **The drain / promotion backlog**, ranked, so the recurring refactor has
//!   an objective next action rather than a vibe.
//! - **Deviation-debt census.** Every `#[spec(deviates)]` is a recorded escape
//!   that should still be justified; their count is a debt to watch.
//!
//! It reuses the conform fact frontend (`Store::extract_workspace`), so its
//! numbers can never drift from what the gates see, and it reads the gating
//! lists straight from `conform.toml` rather than hardcoding a count (the
//! "instrument discipline — count the list, not the record" lesson,
//! SHRINK-PLAN v0.1 §0). Pure function of the source tree: re-running on an
//! unchanged tree writes a byte-identical file, so a committed snapshot's git
//! diff IS the health delta. No LLM; no network — the collector is offline
//! by contract (a project wrapper may append its own extra sections, e.g. a
//! dev repo's mirror-sync probe). Automated fact-gathering is itself a
//! Discipline value (a check that could be a checker is a WISH until it is
//! one).
//!
//! Output: a machine-readable JSON snapshot (default
//! `discipline/health/latest.json`) plus a human summary on stdout. Advisory
//! only — it never fails the build; the gates do the failing.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result};
use conform_core::{Config, ExtractionLog, Fact, Store};
use conform_frontend_rust::RustFrontend;
use serde_json::{Value, json};

/// The line budget the `file-length` rule enforces (budget.rs `max_lines`).
const BUDGET: u32 = 600;
/// At or above this many lines a file is in the danger band — split it before
/// an added line trips the budget. Keep in step with the rule's `BUDGET`.
const DANGER_FLOOR: u32 = 540;

/// Per-crate coverage tallies, accumulated from the conform facts.
#[derive(Default)]
struct CrateHealth {
    pub_types: u32,
    typed_doctested: u32,
    error_enums: BTreeSet<String>,
    error_enums_missing_req: BTreeSet<String>,
    cells: u32,
    deviations: u32,
    unwrap_domain: u32,
    env_nonroot: u32,
    unsafe_nonaudit: u32,
    /// `(file, lines)` for files in `[DANGER_FLOOR, BUDGET]`.
    danger_files: Vec<(String, u32)>,
    /// `(file, lines)` for files over `BUDGET` — current `file-length` matter.
    over_budget: Vec<(String, u32)>,
}

impl CrateHealth {
    /// Public types that carry neither a doctest nor a `#[spec(documents)]`
    /// edge — what `pub-doctest` would flag if this crate were gated.
    fn typed_gap(&self) -> u32 {
        self.pub_types - self.typed_doctested
    }
}

pub fn run_health(root: &Path, out_rel: &str, extra_sections: &[(String, Value)]) -> Result<()> {
    let (config, _origin) = conform_cli_rust::load_config_or_default(root)?;
    let store = Store::at_repo(root, &config);
    let mut log = ExtractionLog::default();
    let frontend = RustFrontend;
    let facts = store.extract_workspace(root, &frontend, &mut log)?;

    let gated: BTreeSet<&str> = config.gated_crates.iter().map(|s| s.as_str()).collect();
    let pub_doctest_gated: BTreeSet<&str> = config
        .gated_pub_doctest
        .iter()
        .map(|s| s.as_str())
        .collect();
    let env_roots: BTreeSet<&str> = config.env_roots.iter().map(|s| s.as_str()).collect();

    let mut crates: BTreeMap<String, CrateHealth> = BTreeMap::new();
    for sf in &facts {
        // The gates only weigh `/src/` files (budget.rs, diagnostics.rs); match
        // that lens so coverage numbers line up with what would be enforced.
        if !sf.file.contains("/src/") {
            continue;
        }
        let h = crates.entry(sf.crate_name.clone()).or_default();
        for f in &sf.facts {
            match f {
                Fact::Item {
                    kind,
                    is_pub,
                    has_doctest,
                    attrs,
                    ..
                } => {
                    if *is_pub && matches!(kind.as_str(), "struct" | "enum" | "trait" | "union") {
                        h.pub_types += 1;
                        if *has_doctest || attrs.iter().any(|a| a.contains("documents")) {
                            h.typed_doctested += 1;
                        }
                    }
                    if attrs.iter().any(|a| a.contains("cell")) {
                        h.cells += 1;
                    }
                    if attrs.iter().any(|a| a.contains("deviates")) {
                        h.deviations += 1;
                    }
                }
                Fact::ErrorVariant {
                    enum_symbol,
                    enum_attrs,
                    ..
                } => {
                    h.error_enums.insert(enum_symbol.clone());
                    if !enum_attrs.iter().any(|a| a.starts_with("spec(")) {
                        h.error_enums_missing_req.insert(enum_symbol.clone());
                    }
                }
                Fact::UnwrapUse {
                    in_test,
                    in_deviation,
                    ..
                } => {
                    if !in_test && !in_deviation {
                        h.unwrap_domain += 1;
                    }
                }
                Fact::EnvRead {
                    in_test,
                    in_deviation,
                    ..
                } => {
                    if !in_test
                        && !in_deviation
                        && !config.audit_crates.contains(&sf.crate_name)
                        && !env_roots.contains(sf.file.as_str())
                    {
                        h.env_nonroot += 1;
                    }
                }
                Fact::UnsafeUse { in_deviation, .. } => {
                    if !in_deviation && !config.audit_crates.contains(&sf.crate_name) {
                        h.unsafe_nonaudit += 1;
                    }
                }
                Fact::FileMetrics { lines } => {
                    if *lines > BUDGET {
                        h.over_budget.push((sf.file.clone(), *lines));
                    } else if *lines >= DANGER_FLOOR {
                        h.danger_files.push((sf.file.clone(), *lines));
                    }
                }
                // TsUnsafe is the ts-tsc frontend's fact; this collector
                // runs over rust-syn facts, so it never appears here —
                // its census belongs to the TypeScript health twin.
                Fact::Import { .. } | Fact::Ctor { .. } | Fact::TsUnsafe { .. } => {}
            }
        }
    }

    // Workspace rollups.
    let baseline_by_rule = baseline_by_rule(root)?;
    let baseline_total: u32 = baseline_by_rule.values().sum();

    let mut danger_all: Vec<(String, u32)> = Vec::new();
    let mut over_all: Vec<(String, u32)> = Vec::new();
    let mut deviation_total = 0u32;
    for h in crates.values() {
        danger_all.extend(h.danger_files.iter().cloned());
        over_all.extend(h.over_budget.iter().cloned());
        deviation_total += h.deviations;
    }
    danger_all.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    over_all.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    // The pub-doctest ratchet drivers: a gated crate whose public types are
    // fully documented is ready to join conform.toml's gated_pub_doctest with zero drain; one
    // with a gap is backlog, ranked smallest-first (cheapest to drain next).
    let mut promotion_candidates: Vec<&str> = Vec::new();
    let mut drain_backlog: Vec<(&str, u32)> = Vec::new();
    for (name, h) in &crates {
        if !gated.contains(name.as_str()) || pub_doctest_gated.contains(name.as_str()) {
            continue;
        }
        if h.pub_types == 0 {
            continue;
        }
        if h.typed_gap() == 0 {
            promotion_candidates.push(name.as_str());
        } else {
            drain_backlog.push((name.as_str(), h.typed_gap()));
        }
    }
    drain_backlog.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(b.0)));

    let crates_json: Vec<Value> = crates
        .iter()
        .map(|(name, h)| {
            json!({
                "crate": name,
                "gated": gated.contains(name.as_str()),
                "pub_doctest_gated": pub_doctest_gated.contains(name.as_str()),
                "pub_types": h.pub_types,
                "typed_doctested": h.typed_doctested,
                "typed_gap": h.typed_gap(),
                "error_enums": h.error_enums.len(),
                "error_enums_missing_req": h.error_enums_missing_req.iter().collect::<Vec<_>>(),
                "cells": h.cells,
                "deviations": h.deviations,
                "unwrap_domain": h.unwrap_domain,
                "env_nonroot": h.env_nonroot,
                "unsafe_nonaudit": h.unsafe_nonaudit,
                "danger_files": files_json(&h.danger_files),
                "over_budget": files_json(&h.over_budget),
            })
        })
        .collect();

    let mut report = json!({
        "schema": 1,
        "note": "Discipline health snapshot — generated by `discipline-rust health` \
                 (the Sweep Playbook's fact-gatherer). Advisory early-warning / \
                 coverage facts that sit above the binary conform + specmap gates. \
                 Deterministic given the source tree; the git diff of this file is \
                 the health delta. No LLM.",
        "budget": { "file_length": BUDGET, "danger_floor": DANGER_FLOOR },
        "summary": {
            "gated_crates": config.gated_crates.len(),
            "exempt_crates": config.exempt.len(),
            "pub_doctest_gated": config.gated_pub_doctest,
            "conform_baseline_total": baseline_total,
            "conform_baseline_by_rule": baseline_by_rule,
            "files_over_budget": over_all.len(),
            "files_in_danger_band": danger_all.len(),
            "deviation_debt": deviation_total,
            "pub_doctest_promotion_candidates": promotion_candidates,
            "pub_doctest_drain_backlog": drain_backlog
                .iter()
                .map(|(c, g)| json!({ "crate": c, "typed_gap": g }))
                .collect::<Vec<_>>(),
            "danger_band_files": files_json(&danger_all),
            "over_budget_files": files_json(&over_all),
        },
        "crates": crates_json,
    });

    // Project-wrapper extension point: a caller may append its own
    // pre-computed sections (e.g. a dev repo's mirror-sync probe). The core
    // collector itself stays deterministic + offline by contract.
    if let Some(obj) = report.as_object_mut() {
        for (key, value) in extra_sections {
            obj.insert(key.clone(), value.clone());
        }
    }

    let out_path = root.join(out_rel);
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let mut text = serde_json::to_string_pretty(&report).context("serialising health report")?;
    text.push('\n');
    std::fs::write(&out_path, &text).with_context(|| format!("writing {}", out_path.display()))?;

    print_summary(
        &config,
        baseline_total,
        &baseline_by_rule,
        &over_all,
        &danger_all,
        deviation_total,
        &promotion_candidates,
        &drain_backlog,
        out_rel,
    );
    Ok(())
}

/// `(file, lines)` pairs as a JSON array of `{file, lines}` objects.
fn files_json(files: &[(String, u32)]) -> Vec<Value> {
    files
        .iter()
        .map(|(f, n)| json!({ "file": f, "lines": n }))
        .collect()
}

/// Tally the frozen `conform-baseline.json` findings by rule (the rule id is
/// the fingerprint prefix before the first `|`). Empty array → empty map;
/// an absent baseline (a fresh, not-yet-frozen project) counts as empty too —
/// the collector is advisory and must run on any tree.
fn baseline_by_rule(root: &Path) -> Result<BTreeMap<String, u32>> {
    let path = root.join("conform-baseline.json");
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let value: Value =
        serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    let mut out: BTreeMap<String, u32> = BTreeMap::new();
    if let Some(arr) = value.get("findings").and_then(Value::as_array) {
        for f in arr {
            if let Some(s) = f.as_str() {
                let rule = s.split('|').next().unwrap_or(s);
                *out.entry(rule.to_string()).or_insert(0) += 1;
            }
        }
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
fn print_summary(
    config: &Config,
    baseline_total: u32,
    baseline_by_rule: &BTreeMap<String, u32>,
    over_all: &[(String, u32)],
    danger_all: &[(String, u32)],
    deviation_total: u32,
    promotion_candidates: &[&str],
    drain_backlog: &[(&str, u32)],
    out_rel: &str,
) {
    println!("=== Discipline health (discipline-rust health) ===");
    println!(
        "gated: {} | exempt: {} | pub-doctest-gated: {}",
        config.gated_crates.len(),
        config.exempt.len(),
        config.gated_pub_doctest.len(),
    );
    print!("conform baseline: {baseline_total} frozen");
    if !baseline_by_rule.is_empty() {
        let parts: Vec<String> = baseline_by_rule
            .iter()
            .map(|(r, n)| format!("{r}={n}"))
            .collect();
        print!("  ({})", parts.join(", "));
    }
    println!();

    println!(
        "file-length: {} over budget (>{BUDGET}) | {} in danger band [{DANGER_FLOOR},{BUDGET}]",
        over_all.len(),
        danger_all.len(),
    );
    for (f, n) in danger_all.iter().take(12) {
        println!("    {n:>4}  {f}");
    }
    if danger_all.len() > 12 {
        println!("    … and {} more (see {out_rel})", danger_all.len() - 12);
    }

    println!("deviation debt: {deviation_total} fn-grain #[spec(deviates)] site(s)");

    if promotion_candidates.is_empty() {
        println!(
            "pub-doctest promotion candidates: none (no gated crate is at full type coverage)"
        );
    } else {
        println!(
            "pub-doctest promotion candidates (gated, 0 gap — widen the gate, zero drain): {}",
            promotion_candidates.join(", "),
        );
    }
    if drain_backlog.is_empty() {
        println!("pub-doctest drain backlog: empty — every gated crate's types are documented");
    } else {
        println!("pub-doctest drain backlog (smallest gap first — drain next):");
        for (c, g) in drain_backlog.iter().take(12) {
            println!("    {g:>4}  {c}");
        }
    }
    println!("wrote {out_rel}");
}
