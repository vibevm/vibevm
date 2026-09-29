//! `typescript-ai-native test-gate` — run the project's tests under
//! node's TAP reporter and diff xfail-strict against the tests
//! baseline (BROWNFIELD §4). The evaluation half is the SAME
//! `specmap_core::testgate::evaluate` the Rust gate uses; only the
//! runner and its parser are TypeScript-shaped.

use std::path::Path;

use anyhow::{Context, Result, bail};

pub fn run_test_gate(root: &Path, baseline_rel: &str) -> Result<()> {
    use specmap_core::testgate;

    let baseline_path = root.join(baseline_rel);
    let baseline_json = std::fs::read_to_string(&baseline_path)
        .with_context(|| format!("reading {}", baseline_path.display()))?;
    let baseline = testgate::parse_baseline(&baseline_json)?;

    eprintln!("test-gate: running `node --test --test-reporter=tap` over the policy's TS roots …");
    let (config, _origin) = conform_core::Config::load_or_default(root)?;
    let mut cmd = crate::tools::node_command(root);
    cmd.args(["--test", "--test-reporter=tap"]);
    for ts_root in &config.typescript.roots {
        cmd.args(crate::tools::test_globs(ts_root));
    }
    let out = cmd
        .output()
        .context("spawning node (install node >= 22.6 — strip-types runs .ts natively)")?;
    let mut combined = String::from_utf8_lossy(&out.stdout).into_owned();
    combined.push('\n');
    combined.push_str(&String::from_utf8_lossy(&out.stderr));
    let results = crate::tap::parse_tap_output(&combined);

    // A gate that parsed nothing must never report green when it had
    // expectations to check (PLAYBOOK §8). The one legal zero: a fresh
    // tree — empty baseline AND node found no test files — where there
    // is nothing to diff yet.
    if results.is_empty() {
        let no_tests = combined.contains("# tests 0") || combined.contains("no test files");
        if baseline.is_empty() && no_tests {
            eprintln!(
                "test-gate: no node tests found and the baseline is empty — \
                 trivially green (a fresh tree)."
            );
            return Ok(());
        }
        bail!(
            "test-gate parsed zero test results out of the TAP stream \
             (node exit: {:?}); refusing to conclude anything",
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
