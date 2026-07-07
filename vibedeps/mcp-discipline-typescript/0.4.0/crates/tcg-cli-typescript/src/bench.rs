//! `tcg-typescript bench` — the corpus harness behind the battery's
//! measured (not CI-gated) half: differential agreement of the oracle's
//! diagnostics against each case's expected error codes, plus warm/cold
//! latency distributions (TCG-ORACLE §7; research/tcg-bench feeds the
//! corpus and commits the reports).
//!
//! Corpus layout: `<corpus>/cases/*.json`, each
//! `{ "name": str, "file": str, "content_file": str?, "expect_codes": [u64] }`
//! — `file` is the project-root-relative target; `content_file` (corpus-
//! relative) carries the hypothetical content to overlay (absent = the
//! file's disk state); `expect_codes` is the set of compiler error codes
//! the case must produce (empty = must be clean).

use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use tcg_oracle_bridge::{OracleTransport, SystemOracle};

use crate::{ORACLE_TIMEOUT, Policy};

#[derive(Deserialize)]
struct CaseSpec {
    name: String,
    file: String,
    #[serde(default)]
    content_file: Option<String>,
    #[serde(default)]
    expect_codes: Vec<u64>,
}

#[derive(Serialize)]
struct CaseResult {
    name: String,
    agree: bool,
    expected: Vec<u64>,
    got: Vec<u64>,
    ms: f64,
}

#[derive(Serialize)]
struct BenchReport {
    ts_version: String,
    cases: Vec<CaseResult>,
    agreement_pct: f64,
    cold_init_ms: f64,
    validate_p50_ms: f64,
    validate_p95_ms: f64,
}

fn percentile(sorted_ms: &[f64], p: f64) -> f64 {
    if sorted_ms.is_empty() {
        return 0.0;
    }
    let idx = ((sorted_ms.len() as f64 - 1.0) * p).round() as usize;
    sorted_ms[idx.min(sorted_ms.len() - 1)]
}

/// Run the corpus; write the report JSON; print the human table.
/// Exit code 0 — the bench MEASURES, the package tests GATE (the §7
/// split); a broken corpus (unreadable case) still errors hard.
pub fn run_bench(root: &Path, corpus: &Path, report_path: &Path) -> Result<i32> {
    let cases_dir = corpus.join("cases");
    let mut case_files: Vec<_> = std::fs::read_dir(&cases_dir)
        .with_context(|| format!("reading corpus {}", cases_dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    case_files.sort();
    if case_files.is_empty() {
        bail!("corpus has no cases/*.json at {}", cases_dir.display());
    }

    let policy = Policy::load(root)?;
    let t0 = Instant::now();
    let mut oracle = SystemOracle::spawn(root, ORACLE_TIMEOUT)?;
    let init = oracle.init(
        root,
        policy.config.typescript.cells_dir.as_deref(),
        &policy.config.typescript.seam,
    )?;
    // The first validate builds the program: measure it as cold init.
    let warmup_file = &load_case(&case_files[0])?.file;
    let _ = oracle.validate(warmup_file, None)?;
    let cold_init = t0.elapsed();

    let mut results: Vec<CaseResult> = Vec::new();
    let mut timings: Vec<f64> = Vec::new();
    for path in &case_files {
        let case = load_case(path)?;
        let content = match &case.content_file {
            Some(rel) => Some(
                std::fs::read_to_string(corpus.join(rel))
                    .with_context(|| format!("reading corpus content {rel}"))?,
            ),
            None => None,
        };
        let started = Instant::now();
        let v = oracle.validate(&case.file, content.as_deref())?;
        let ms = as_ms(started.elapsed());
        timings.push(ms);

        let mut got: Vec<u64> = v
            .diagnostics
            .iter()
            .filter(|d| d.category == "error")
            .map(|d| d.code)
            .collect();
        got.sort_unstable();
        got.dedup();
        let mut expected = case.expect_codes.clone();
        expected.sort_unstable();
        expected.dedup();
        let agree = got == expected;
        results.push(CaseResult {
            name: case.name,
            agree,
            expected,
            got,
            ms,
        });
    }
    let _ = oracle.shutdown();

    let agreeing = results.iter().filter(|r| r.agree).count();
    let mut sorted = timings.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).expect("finite timings"));
    let report = BenchReport {
        ts_version: init.ts_version,
        agreement_pct: 100.0 * agreeing as f64 / results.len() as f64,
        cold_init_ms: as_ms(cold_init),
        validate_p50_ms: percentile(&sorted, 0.50),
        validate_p95_ms: percentile(&sorted, 0.95),
        cases: results,
    };

    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let mut body = serde_json::to_string_pretty(&report)?;
    body.push('\n');
    std::fs::write(report_path, body)
        .with_context(|| format!("writing {}", report_path.display()))?;

    println!(
        "tcg-typescript bench: {} case(s), agreement {:.1}%, cold init {:.0} ms, \
         validate p50 {:.1} ms / p95 {:.1} ms — report at {}",
        report.cases.len(),
        report.agreement_pct,
        report.cold_init_ms,
        report.validate_p50_ms,
        report.validate_p95_ms,
        report_path.display(),
    );
    for c in &report.cases {
        println!(
            "  {} {} (expected {:?}, got {:?}, {:.1} ms)",
            if c.agree { "AGREE " } else { "differ" },
            c.name,
            c.expected,
            c.got,
            c.ms,
        );
    }
    Ok(0)
}

fn load_case(path: &Path) -> Result<CaseSpec> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading case {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parsing case {}", path.display()))
}

fn as_ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_math_holds_on_edges() {
        assert_eq!(percentile(&[], 0.5), 0.0);
        assert_eq!(percentile(&[7.0], 0.5), 7.0);
        let v = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(percentile(&v, 0.5), 3.0);
        assert_eq!(percentile(&v, 0.95), 5.0);
    }
}
