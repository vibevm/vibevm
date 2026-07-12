//! The xfail-strict test gate (BROWNFIELD-PROTOCOL §4).
//!
//! Diff a nextest run against the project's tests-baseline registry (`discipline/registry/tests-baseline.json` by convention)
//! and fail on either of:
//!
//! 1. **newly failing** — a test absent from the baseline failed;
//! 2. **unexpectedly passing, unpromoted** — a `failing-known` baseline
//!    test passed; promote it via the explicit protocol (PLAYBOOK §7.2).
//!
//! `flaky` entries are reported, never gating. A baseline entry that
//! never appears in the run is reported as a warning (possible rename /
//! deletion — the promotion protocol covers shrinking the registry).

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/BROWNFIELD-PROTOCOL-v0.1#test-gate");

use std::collections::BTreeMap;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

/// One `tests-baseline.json` entry (extra fields tolerated).
#[derive(Debug, Clone, Deserialize)]
pub struct BaselineEntry {
    pub test: String,
    pub status: String,
    #[serde(default)]
    pub debt: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BaselineFile {
    entries: Vec<BaselineEntry>,
}

pub fn parse_baseline(json: &str) -> Result<Vec<BaselineEntry>> {
    let file: BaselineFile = serde_json::from_str(json).context("parsing tests-baseline.json")?;
    for e in &file.entries {
        match e.status.as_str() {
            "passing" | "failing-known" | "flaky" | "obsolete" => {}
            other => bail!(
                "baseline entry `{}` has unknown status `{other}` \
                 (expected passing | failing-known | flaky | obsolete)",
                e.test
            ),
        }
    }
    Ok(file.entries)
}

/// Final status of one test in a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Pass,
    Fail,
    Skip,
}

/// Parse nextest's human run output (stdout and stderr concatenated is
/// fine) into `test id → status`. The id format matches the baseline:
/// `<binary-id> <test-name>`, e.g.
/// `vibe-cli::cli_live_e2e install_github_smoke_alone`.
///
/// Run nextest with `--status-level all` so skipped (`#[ignore]`d)
/// tests emit a line. Counter tokens `(123/998)` and timing `[ 1.2s]`
/// are skipped positionally; unknown line shapes are ignored.
pub fn parse_nextest_output(output: &str) -> BTreeMap<String, RunStatus> {
    let mut results = BTreeMap::new();
    for raw in output.lines() {
        let line = raw.trim_start();
        let mut tokens = line.split_whitespace().peekable();
        let Some(first) = tokens.next() else { continue };
        // Retry lines look like `TRY 2 PASS [...] …`.
        let status_token = if first == "TRY" {
            tokens.next(); // the attempt number
            match tokens.next() {
                Some(s) => s,
                None => continue,
            }
        } else {
            first
        };
        let status = match status_token {
            "PASS" | "LEAK" => RunStatus::Pass,
            "FAIL" | "TIMEOUT" | "ABORT" | "SIGSEGV" | "SIGABRT" | "SIGBUS" | "SIGILL" => {
                RunStatus::Fail
            }
            "SKIP" => RunStatus::Skip,
            _ => continue,
        };
        // `[   0.123s]` timing token — consume-if-matches, so there is
        // no peek-then-next gap to bridge.
        match tokens.next_if(|t| t.starts_with('[')) {
            Some(t0) => {
                // Timing may split into two tokens: `[` + `0.123s]`.
                if !t0.ends_with(']') {
                    for t in tokens.by_ref() {
                        if t.ends_with(']') {
                            break;
                        }
                    }
                }
            }
            None => continue, // a prose line that merely starts with PASS/…
        }
        // Optional `(i/n)` progress counter.
        if let Some(t) = tokens.peek()
            && t.starts_with('(')
            && t.ends_with(')')
        {
            tokens.next();
        }
        let Some(binary_id) = tokens.next() else {
            continue;
        };
        let rest: Vec<&str> = tokens.collect();
        if rest.is_empty() {
            continue;
        }
        let id = format!("{binary_id} {}", rest.join(" "));
        results.insert(id, status);
    }
    results
}

/// The gate verdict.
#[derive(Debug, Default)]
pub struct GateReport {
    /// Failed and not excused by the baseline → gate fails.
    pub newly_failing: Vec<String>,
    /// `failing-known` in the baseline but passed → gate fails
    /// (promote via the explicit protocol instead).
    pub unexpectedly_passing: Vec<String>,
    /// `flaky` entries and their observed status — informational.
    pub flaky_observed: Vec<(String, &'static str)>,
    /// Baseline entries that never appeared in the run — warning only.
    pub missing_from_run: Vec<String>,
}

impl GateReport {
    pub fn is_green(&self) -> bool {
        self.newly_failing.is_empty() && self.unexpectedly_passing.is_empty()
    }
}

pub fn evaluate(baseline: &[BaselineEntry], results: &BTreeMap<String, RunStatus>) -> GateReport {
    let mut report = GateReport::default();
    let by_test: BTreeMap<&str, &BaselineEntry> =
        baseline.iter().map(|e| (e.test.as_str(), e)).collect();

    for (test, status) in results {
        match by_test.get(test.as_str()) {
            None => {
                if *status == RunStatus::Fail {
                    report.newly_failing.push(test.clone());
                }
            }
            Some(entry) => match (entry.status.as_str(), status) {
                ("failing-known", RunStatus::Pass) => {
                    report.unexpectedly_passing.push(test.clone());
                }
                ("failing-known", _) => {}
                ("flaky", s) => {
                    let word = match s {
                        RunStatus::Pass => "pass",
                        RunStatus::Fail => "fail",
                        RunStatus::Skip => "skip",
                    };
                    report.flaky_observed.push((test.clone(), word));
                }
                ("passing", RunStatus::Fail) => {
                    report.newly_failing.push(test.clone());
                }
                _ => {}
            },
        }
    }
    for entry in baseline {
        if !results.contains_key(&entry.test) {
            report.missing_from_run.push(entry.test.clone());
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASELINE: &str = r#"{
      "schema": 1,
      "entries": [
        { "test": "vibe-cli::cli_live_e2e install_github_smoke_alone", "status": "failing-known", "debt": "DBT-0002" },
        { "test": "x::suite known_flake", "status": "flaky", "debt": "DBT-9999" }
      ]
    }"#;

    #[test]
    fn parses_real_nextest_line_shapes() {
        let out = "\
        PASS [   0.010s] (986/998) vibe-workspace vibedeps::tests::slot_rel_path_is_kind_name_version\n\
        PASS [  11.719s] (998/998) vibe-cli::cli_e2e install_expands_cascading_conditional_dependencies\n\
        SKIP [         ] vibe-cli::cli_live_e2e install_github_smoke_alone\n\
        FAIL [   0.300s] vibe-core manifest::tests::broken_case\n\
        TRY 2 PASS [   0.100s] x::suite known_flake\n\
        SLOW [> 60.000s] vibe-cli::cli_e2e something_slow\n\
        Summary [  11.776s] 998 tests run: 998 passed, 3 skipped\n";
        let map = parse_nextest_output(out);
        assert_eq!(
            map.get("vibe-workspace vibedeps::tests::slot_rel_path_is_kind_name_version"),
            Some(&RunStatus::Pass)
        );
        assert_eq!(
            map.get("vibe-cli::cli_live_e2e install_github_smoke_alone"),
            Some(&RunStatus::Skip)
        );
        assert_eq!(
            map.get("vibe-core manifest::tests::broken_case"),
            Some(&RunStatus::Fail)
        );
        assert_eq!(map.get("x::suite known_flake"), Some(&RunStatus::Pass));
        assert_eq!(map.len(), 5, "{map:?}");
    }

    #[test]
    fn green_run_with_quarantined_skips_is_green() {
        let baseline = parse_baseline(BASELINE).unwrap();
        let mut results = BTreeMap::new();
        results.insert("a::t one".to_string(), RunStatus::Pass);
        results.insert(
            "vibe-cli::cli_live_e2e install_github_smoke_alone".to_string(),
            RunStatus::Skip,
        );
        results.insert("x::suite known_flake".to_string(), RunStatus::Fail);
        let report = evaluate(&baseline, &results);
        assert!(report.is_green(), "{report:?}");
        assert_eq!(report.flaky_observed.len(), 1);
    }

    #[test]
    fn newly_failing_trips_the_gate() {
        let baseline = parse_baseline(BASELINE).unwrap();
        let mut results = BTreeMap::new();
        results.insert("a::t regression".to_string(), RunStatus::Fail);
        let report = evaluate(&baseline, &results);
        assert!(!report.is_green());
        assert_eq!(report.newly_failing, vec!["a::t regression"]);
    }

    #[test]
    fn unexpectedly_passing_trips_the_gate() {
        let baseline = parse_baseline(BASELINE).unwrap();
        let mut results = BTreeMap::new();
        results.insert(
            "vibe-cli::cli_live_e2e install_github_smoke_alone".to_string(),
            RunStatus::Pass,
        );
        let report = evaluate(&baseline, &results);
        assert!(!report.is_green());
        assert_eq!(report.unexpectedly_passing.len(), 1);
    }

    #[test]
    fn baseline_entry_missing_from_run_is_a_warning_not_a_failure() {
        let baseline = parse_baseline(BASELINE).unwrap();
        let mut results = BTreeMap::new();
        results.insert("a::t one".to_string(), RunStatus::Pass);
        let report = evaluate(&baseline, &results);
        assert!(report.is_green());
        assert_eq!(report.missing_from_run.len(), 2);
    }

    #[test]
    fn unknown_baseline_status_is_rejected() {
        let bad = r#"{ "entries": [ { "test": "t", "status": "quarantined" } ] }"#;
        assert!(parse_baseline(bad).is_err());
    }
}
