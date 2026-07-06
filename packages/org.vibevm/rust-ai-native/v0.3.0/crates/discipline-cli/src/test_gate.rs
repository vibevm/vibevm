//! `cargo test-gate` — run the workspace tests through nextest
//! and diff the outcome against the xfail-strict baseline
//! (BROWNFIELD §4). Replaces bare `cargo test` in terraform
//! acceptance lines.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

pub fn run_test_gate(root: &Path, baseline_rel: &str) -> Result<()> {
    use specmap_core::testgate;

    let baseline_path = root.join(baseline_rel);
    let baseline_json = std::fs::read_to_string(&baseline_path)
        .with_context(|| format!("reading {}", baseline_path.display()))?;
    let baseline = testgate::parse_baseline(&baseline_json)?;

    eprintln!("test-gate: running `cargo nextest run --workspace --no-fail-fast` …");
    let out = Command::new("cargo")
        .args([
            "nextest",
            "run",
            "--workspace",
            "--no-fail-fast",
            "--status-level",
            "all",
            "--color",
            "never",
        ])
        .current_dir(root)
        .output()
        .context("spawning cargo nextest (install: `cargo install cargo-nextest --locked`)")?;

    let mut combined = String::from_utf8_lossy(&out.stdout).into_owned();
    combined.push('\n');
    combined.push_str(&String::from_utf8_lossy(&out.stderr));
    let results = testgate::parse_nextest_output(&combined);

    // A gate that parsed nothing must never report green: that is how
    // gates get gamed by accident (PLAYBOOK §8).
    if results.is_empty() {
        bail!(
            "test-gate parsed zero test results out of the nextest run \
             (nextest exit: {:?}); refusing to conclude anything",
            out.status.code()
        );
    }

    let report = testgate::evaluate(&baseline, &results);
    let total = results.len();
    let skipped = results
        .values()
        .filter(|s| **s == testgate::RunStatus::Skip)
        .count();
    let failed = results
        .values()
        .filter(|s| **s == testgate::RunStatus::Fail)
        .count();
    eprintln!(
        "test-gate: {total} results parsed ({failed} failed, {skipped} skipped), \
         baseline entries: {}",
        baseline.len()
    );
    for (test, status) in &report.flaky_observed {
        eprintln!("  flaky (never gating): {test} — {status}");
    }
    for test in &report.missing_from_run {
        eprintln!(
            "  warning: baseline entry never appeared in the run \
             (renamed or deleted? shrink the baseline via the promotion \
             protocol): {test}"
        );
    }
    if report.is_green() {
        eprintln!("test-gate: green (xfail-strict).");
        return Ok(());
    }
    for test in &report.newly_failing {
        eprintln!("  NEWLY FAILING: {test}");
    }
    for test in &report.unexpectedly_passing {
        eprintln!("  UNEXPECTEDLY PASSING (unpromoted — see PLAYBOOK §7.2): {test}");
    }
    bail!(
        "test-gate failed: {} newly failing, {} unexpectedly passing",
        report.newly_failing.len(),
        report.unexpectedly_passing.len()
    );
}
