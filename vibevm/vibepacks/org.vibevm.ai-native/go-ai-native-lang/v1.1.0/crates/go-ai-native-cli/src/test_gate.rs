//! `go-ai-native test-gate` — run the module's tests under
//! `go test -json` and diff xfail-strict against the tests baseline
//! (BROWNFIELD §4). The evaluation half is the SAME
//! `specmap_core::testgate::evaluate` the sibling gates use; only the
//! runner and its parser are Go-shaped. Test identity is
//! `<package import path>::<test name>` (GUIDE §10 — with no in-source
//! xfail twin, the registry carries full weight, so identities must be
//! collision-free).

specmark::scope!("spec://go-ai-native-lang/go/GUIDE-AI-NATIVE-GO#replacement");

use std::path::Path;

use anyhow::{Context, Result, bail};

pub fn run_test_gate(root: &Path, baseline_rel: &str) -> Result<()> {
    use specmap_core::testgate;

    let baseline_path = root.join(baseline_rel);
    let baseline_json = std::fs::read_to_string(&baseline_path)
        .with_context(|| format!("reading {}", baseline_path.display()))?;
    let baseline = testgate::parse_baseline(&baseline_json)?;

    eprintln!("test-gate: running `go test ./... -json` …");
    let mut cmd = crate::tools::go_command(root);
    cmd.args(["test", "./...", "-json"]);
    let out = cmd
        .output()
        .context("spawning go (install go >= 1.24 and put it on PATH)")?;
    let combined = String::from_utf8_lossy(&out.stdout).into_owned();
    let results = crate::gotest::parse_gotest_json(&combined);

    // A gate that parsed nothing must never report green when it had
    // expectations to check. The one legal zero: a fresh tree — empty
    // baseline AND go found no tests — where there is nothing to diff.
    if results.is_empty() {
        if baseline.is_empty() && out.status.success() {
            eprintln!(
                "test-gate: no go tests found and the baseline is empty — \
                 trivially green (a fresh tree)."
            );
            return Ok(());
        }
        bail!(
            "test-gate parsed zero test results out of the -json stream \
             (go exit: {:?}); refusing to conclude anything",
            out.status.code()
        );
    }

    let report = testgate::evaluate(&baseline, &results);
    let total = results.len();
    let failed = results
        .values()
        .filter(|s| **s == testgate::RunStatus::Fail)
        .count();
    let skipped = results
        .values()
        .filter(|s| **s == testgate::RunStatus::Skip)
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
            "  warning: baseline entry never appeared in the run (renamed or \
             deleted? shrink the baseline via the promotion protocol): {test}"
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
