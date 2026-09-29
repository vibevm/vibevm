//! `go test -json` into the language-free test-gate model — the Go
//! twin of the Rust nextest parser and the TS TAP parser, feeding the
//! SAME `testgate::evaluate`, so the three languages cannot disagree
//! on what a test result is.
//!
//! go emits one JSON event per line; the terminal per-test verdicts
//! are `{"Action":"pass|fail|skip","Package":"…","Test":"…"}` (events
//! without a `Test` field are package-grain and ignored). Subtests
//! arrive as `TestX/sub` and are kept as-is; test identity is
//! `<package import path>::<test name>` so two packages' same-named
//! tests never collide.

use std::collections::BTreeMap;

use serde::Deserialize;
use specmap_core::testgate::RunStatus;

#[derive(Deserialize)]
struct Event {
    #[serde(rename = "Action")]
    action: String,
    #[serde(rename = "Package")]
    package: Option<String>,
    #[serde(rename = "Test")]
    test: Option<String>,
}

/// Parse a `go test -json` stream into `test identity → status`.
/// Non-JSON lines (build errors interleave as plain text) are skipped —
/// the gate's zero-results guard catches a stream that parsed nothing.
pub fn parse_gotest_json(output: &str) -> BTreeMap<String, RunStatus> {
    let mut results = BTreeMap::new();
    for line in output.lines() {
        let Ok(event) = serde_json::from_str::<Event>(line) else {
            continue;
        };
        let Some(test) = event.test else {
            continue; // package-grain event
        };
        let status = match event.action.as_str() {
            "pass" => RunStatus::Pass,
            "fail" => RunStatus::Fail,
            "skip" => RunStatus::Skip,
            _ => continue, // run/output/pause/cont — not verdicts
        };
        let package = event.package.unwrap_or_default();
        results.insert(format!("{package}::{test}"), status);
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPLAY: &str = concat!(
        r#"{"Time":"t","Action":"run","Package":"demo/internal/cells/plan","Test":"TestSolve"}"#,
        "\n",
        r#"{"Time":"t","Action":"output","Package":"demo/internal/cells/plan","Test":"TestSolve","Output":"=== RUN\n"}"#,
        "\n",
        r#"{"Time":"t","Action":"pass","Package":"demo/internal/cells/plan","Test":"TestSolve","Elapsed":0}"#,
        "\n",
        r#"{"Time":"t","Action":"fail","Package":"demo/internal/cells/plan","Test":"TestDiff/empty","Elapsed":0}"#,
        "\n",
        r#"{"Time":"t","Action":"skip","Package":"demo/internal/sim","Test":"TestSlow","Elapsed":0}"#,
        "\n",
        r#"{"Time":"t","Action":"pass","Package":"demo/internal/sim","Elapsed":0.1}"#,
        "\n",
        "not json at all\n",
    );

    #[test]
    fn verdict_events_parse_and_package_grain_lines_are_ignored() {
        let results = parse_gotest_json(REPLAY);
        assert_eq!(results.len(), 3);
        assert_eq!(
            results.get("demo/internal/cells/plan::TestSolve"),
            Some(&RunStatus::Pass)
        );
        assert_eq!(
            results.get("demo/internal/cells/plan::TestDiff/empty"),
            Some(&RunStatus::Fail)
        );
        assert_eq!(
            results.get("demo/internal/sim::TestSlow"),
            Some(&RunStatus::Skip)
        );
    }
}
