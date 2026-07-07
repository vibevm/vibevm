//! `tcg-rust bench` — the differential/latency harness (D10): every
//! corpus case runs through the LIVE oracle (warm ×3) and through
//! `cargo check --message-format=json` in ONE reused scratch
//! materialisation, codes translate through the committed r-a↔rustc
//! mapping table, and the documented-gap case asserts the asymmetry
//! as its expectation. Targets are recorded, never CI-gated; the
//! REPORT is the ratchet.

specmark::scope!("spec://rust-ai-native/mechanisms/TCG-ORACLE-RUST-v0.1#approximation");

use std::collections::BTreeSet;
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::{Policy, enrich_validate, spawn_oracle};

/// The r-a-code ↔ rustc-code mapping table, spike-verified against
/// 1.93.1 (ORACLE-RUST §5). Codes absent here map to themselves.
pub const CODE_MAP: &[(&str, &str)] = &[
    ("E0308", "E0308"),
    ("E0425", "E0425"),
    ("E0107", "E0061"), // arity: r-a names it E0107, rustc E0061
    ("E0559", "E0609"), // unknown field: r-a E0559, rustc E0609
    ("E0063", "E0063"),
    ("E0599", "E0599"),
];

/// Translate an oracle code into the rustc code the differential
/// compares against.
///
/// ```
/// assert_eq!(tcg_cli_rust::bench::map_code("E0107"), "E0061");
/// assert_eq!(tcg_cli_rust::bench::map_code("E0308"), "E0308");
/// assert_eq!(tcg_cli_rust::bench::map_code("weird"), "weird");
/// ```
pub fn map_code(ra: &str) -> &str {
    CODE_MAP
        .iter()
        .find(|(from, _)| *from == ra)
        .map_or(ra, |(_, to)| *to)
}

#[derive(Debug, Deserialize)]
struct Case {
    name: String,
    file: String,
    /// Corpus-dir-relative content file; absent = the disk state.
    #[serde(default)]
    content_from: Option<String>,
    expect: Expect,
}

#[derive(Debug, Deserialize)]
struct Expect {
    /// rustc codes cargo check must report (empty = clean).
    #[serde(default)]
    cargo_codes: Vec<String>,
    /// Non-baselined conform rules the enrichment must surface.
    #[serde(default)]
    conform_rules: Vec<String>,
    /// The documented-gap posture: the oracle stays SILENT while
    /// cargo speaks — asserted as the pass condition (ORACLE-RUST §5).
    #[serde(default)]
    known_gap: bool,
}

#[derive(Debug, serde::Serialize)]
struct CaseOutcome {
    name: String,
    pass: bool,
    known_gap: bool,
    oracle_codes: Vec<String>,
    mapped_codes: Vec<String>,
    cargo_codes: Vec<String>,
    conform_rules: Vec<String>,
    warm_ms: Vec<u128>,
    detail: String,
}

/// Copy the project into scratch once (skipping build output, slots,
/// VCS) so every case's cargo check shares one dep build.
fn materialise(root: &Path, dst: &Path) -> Result<()> {
    const SKIP: &[&str] = &["target", "vibedeps", ".git", ".vibe", "node_modules"];
    for entry in walk(root)? {
        let rel = entry
            .strip_prefix(root)
            .unwrap_or(&entry)
            .to_string_lossy()
            .replace('\\', "/");
        if SKIP
            .iter()
            .any(|s| rel == *s || rel.starts_with(&format!("{s}/")))
        {
            continue;
        }
        let to = dst.join(&rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&to)?;
        } else {
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&entry, &to)?;
        }
    }
    Ok(())
}

fn walk(root: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)
            .with_context(|| format!("reading {}", dir.display()))?
            .filter_map(std::result::Result::ok)
        {
            let path = entry.path();
            out.push(path.clone());
            if path.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !matches!(
                    name.as_str(),
                    "target" | "vibedeps" | ".git" | ".vibe" | "node_modules"
                ) {
                    stack.push(path);
                }
            }
        }
    }
    Ok(out)
}

/// cargo check in the scratch copy; returns the E-code set. The slot
/// path-dep (specmark) is repointed at the REAL slot so the scratch
/// tree needs no vibedeps of its own.
fn cargo_codes(scratch: &Path, real_root: &Path) -> Result<BTreeSet<String>> {
    let out = std::process::Command::new("cargo")
        .args(["check", "--message-format=json", "--quiet"])
        .current_dir(scratch)
        .env("CARGO_TARGET_DIR", scratch.join("target"))
        .env(
            "CARGO_NET_OFFLINE",
            std::env::var("CARGO_NET_OFFLINE").unwrap_or_else(|_| "false".into()),
        )
        .output()
        .with_context(|| {
            format!(
                "cargo check in {} (root {})",
                scratch.display(),
                real_root.display()
            )
        })?;
    let mut codes = BTreeSet::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if v.get("reason").and_then(|r| r.as_str()) == Some("compiler-message")
            && let Some(code) = v.pointer("/message/code/code").and_then(|c| c.as_str())
            && code.starts_with('E')
        {
            codes.insert(code.to_string());
        }
    }
    Ok(codes)
}

/// Rewire the scratch copy's slot path-deps to ABSOLUTE paths into the
/// real project's vibedeps (the scratch tree skipped them).
fn repoint_slot_deps(scratch: &Path, real_root: &Path) -> Result<()> {
    for name in ["Cargo.toml"] {
        let manifest = scratch.join(name);
        if !manifest.is_file() {
            continue;
        }
        let text = std::fs::read_to_string(&manifest)?;
        let real = real_root.to_string_lossy().replace('\\', "/");
        let rewired = text.replace("path = \"vibedeps/", &format!("path = \"{real}/vibedeps/"));
        std::fs::write(&manifest, rewired)?;
    }
    Ok(())
}

/// The bench entry: corpus dir of `cases/*.json` + `content/*`, a
/// report path, the project root.
pub fn run_bench(corpus: &Path, report: &Path, root: &Path) -> Result<i32> {
    let root = tcg_oracle_bridge_rust::verbatim_free(
        &root.canonicalize().unwrap_or_else(|_| root.to_path_buf()),
    );
    let policy = Policy::load(&root)?;

    // Cold: spawn-to-quiescent, measured.
    let t0 = Instant::now();
    let mut oracle = spawn_oracle(&root)?;
    let cold_ms = t0.elapsed().as_millis();

    // One scratch materialisation for every case's cargo truth.
    let scratch = tempfile::tempdir().context("scratch dir")?;
    materialise(&root, scratch.path())?;
    repoint_slot_deps(scratch.path(), &root)?;

    let mut cases: Vec<Case> = Vec::new();
    let cases_dir = corpus.join("cases");
    let mut entries: Vec<_> = std::fs::read_dir(&cases_dir)
        .with_context(|| format!("reading {}", cases_dir.display()))?
        .filter_map(std::result::Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("json"))
        .collect();
    entries.sort();
    for path in entries {
        let text = std::fs::read_to_string(&path)?;
        cases.push(
            serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?,
        );
    }
    if cases.is_empty() {
        bail!("no cases under {}", cases_dir.display());
    }

    let mut outcomes: Vec<CaseOutcome> = Vec::new();
    let mut all_warm: Vec<u128> = Vec::new();
    for case in &cases {
        let content = match &case.content_from {
            Some(rel) => Some(
                std::fs::read_to_string(corpus.join(rel))
                    .with_context(|| format!("case {}: content {rel}", case.name))?,
            ),
            None => None,
        };
        let effective = match &content {
            Some(c) => c.clone(),
            None => std::fs::read_to_string(root.join(&case.file))
                .with_context(|| format!("case {}: disk {}", case.name, case.file))?,
        };

        // Warm oracle passes ×3.
        let mut warm_ms = Vec::new();
        let mut last = None;
        for _ in 0..3 {
            let t = Instant::now();
            let out = oracle
                .validate(&case.file, Some(effective.clone()))
                .map_err(|e| anyhow::anyhow!("case {}: {e}", case.name))?;
            warm_ms.push(t.elapsed().as_millis());
            last = Some(out);
        }
        let outcome = last.unwrap_or_else(|| unreachable!("three passes ran"));
        all_warm.extend(&warm_ms);
        let enriched = enrich_validate(&policy, &case.file, &effective, outcome);
        let oracle_codes: BTreeSet<String> = enriched
            .diagnostics
            .iter()
            .filter(|d| d.category == "error")
            .map(|d| d.code.clone())
            .collect();
        let mapped: BTreeSet<String> = oracle_codes
            .iter()
            .map(|c| map_code(c).to_string())
            .collect();
        let conform_rules: BTreeSet<String> = enriched
            .conform_findings
            .iter()
            .filter(|f| !f.baselined)
            .map(|f| f.rule.clone())
            .collect();

        // Cargo truth: write the case content into the scratch copy,
        // check, restore — a NEW file (the overlay-only case) is
        // created and deleted instead.
        let scratch_file = scratch.path().join(&case.file);
        let original = std::fs::read_to_string(&scratch_file).ok();
        if let Some(parent) = scratch_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&scratch_file, &effective)?;
        let cargo = cargo_codes(scratch.path(), &root)?;
        match &original {
            Some(text) => std::fs::write(&scratch_file, text)?,
            None => std::fs::remove_file(&scratch_file)?,
        }

        let expect_cargo: BTreeSet<String> = case.expect.cargo_codes.iter().cloned().collect();
        let expect_rules: BTreeSet<String> = case.expect.conform_rules.iter().cloned().collect();
        let (pass, detail) = if case.expect.known_gap {
            // The documented gap: cargo speaks, the oracle is silent.
            let ok = mapped.is_empty() && cargo == expect_cargo && !expect_cargo.is_empty();
            (
                ok,
                if ok {
                    "documented gap holds: oracle silent, cargo speaks".to_string()
                } else {
                    format!(
                        "GAP MOVED: oracle {mapped:?} (expected silence), cargo {cargo:?} \
                         (expected {expect_cargo:?}) — a future r-a may have caught up; \
                         re-curate the case"
                    )
                },
            )
        } else {
            let codes_ok = mapped == expect_cargo && cargo == expect_cargo;
            let rules_ok = expect_rules.is_subset(&conform_rules);
            (
                codes_ok && rules_ok,
                if codes_ok && rules_ok {
                    "existence-grain agreement".to_string()
                } else {
                    format!(
                        "oracle→mapped {mapped:?} vs cargo {cargo:?} vs expected \
                         {expect_cargo:?}; conform {conform_rules:?} vs expected {expect_rules:?}"
                    )
                },
            )
        };
        eprintln!(
            "bench: {} — {} ({})",
            case.name,
            if pass { "PASS" } else { "FAIL" },
            detail
        );
        outcomes.push(CaseOutcome {
            name: case.name.clone(),
            pass,
            known_gap: case.expect.known_gap,
            oracle_codes: oracle_codes.into_iter().collect(),
            mapped_codes: mapped.into_iter().collect(),
            cargo_codes: cargo.into_iter().collect(),
            conform_rules: conform_rules.into_iter().collect(),
            warm_ms,
            detail,
        });
    }
    let _ = oracle.shutdown();

    all_warm.sort_unstable();
    let pct = |p: f64| -> u128 {
        if all_warm.is_empty() {
            return 0;
        }
        let idx = ((all_warm.len() as f64 - 1.0) * p).round() as usize;
        all_warm[idx.min(all_warm.len() - 1)]
    };
    let passed = outcomes.iter().filter(|o| o.pass).count();
    let body = serde_json::json!({
        "cold_init_ms": cold_ms,
        "validate_p50_ms": pct(0.50),
        "validate_p95_ms": pct(0.95),
        "agreement": format!("{passed}/{}", outcomes.len()),
        "code_map": CODE_MAP,
        "cases": outcomes,
    });
    if let Some(parent) = report.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(report, serde_json::to_string_pretty(&body)?)
        .with_context(|| format!("writing {}", report.display()))?;
    eprintln!(
        "bench: agreement {passed}/{} — cold {cold_ms} ms, warm p50 {} ms / p95 {} ms; report at {}",
        outcomes.len(),
        pct(0.50),
        pct(0.95),
        report.display()
    );
    Ok(i32::from(passed != outcomes.len()))
}
