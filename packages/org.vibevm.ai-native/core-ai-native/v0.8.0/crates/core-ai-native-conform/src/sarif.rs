specmark::scope!(
    "spec://org.vibevm.ai-native/core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#determinism"
);

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::facts::{Fact, SourceFacts};
use crate::finding::{Finding, FindingStatus, Rule};
use crate::store::sort_source_facts;

/// Byte-stable minimal SARIF 2.1.0: stable ordering (findings are
/// pre-sorted), no wall-clock, no absolute paths.
///
/// ```
/// use core_ai_native_conform::rules::CellIsolation;
/// use core_ai_native_conform::sarif;
///
/// let report = sarif::render(&[&CellIsolation], &[]);
/// assert!(report.contains("\"version\": \"2.1.0\""));
/// assert_eq!(report, sarif::render(&[&CellIsolation], &[]));
/// ```
pub fn render(rules: &[&dyn Rule], findings: &[Finding]) -> String {
    let rule_objs: Vec<serde_json::Value> = rules
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id(),
                "shortDescription": { "text": r.why() }
            })
        })
        .collect();
    let results: Vec<serde_json::Value> = findings
        .iter()
        .map(|f| {
            let mut result = serde_json::json!({
                "ruleId": f.rule,
                "level": "error",
                "message": { "text": f.message },
                "partialFingerprints": { "vibevmConform/v1": f.fingerprint },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": { "uri": f.file },
                        "region": { "startLine": f.line }
                    }
                }],
                "properties": {
                    "vibevmConform/evidence": f.evidence,
                    "vibevmConform/status": status_name(&f.status)
                }
            });
            // B-025 (mark, don't suppress): an acknowledged deviation
            // STAYS in the report — the SARIF `suppressions` idiom for
            // "known and accepted in source" (`kind: "inSource"`). The
            // result remains visible (the owner wants everything seen),
            // it is simply marked, never failing the gate (`diff` keeps
            // acknowledged out of `new`). `justification` carries the
            // deviation's recorded reason text when the frontend captured
            // it (TypeScript/Go facts carry `reason`); the Rust facts
            // carry only the boolean, so it falls back to a fixed marker
            // — plumbing the reason through the rust-syn frontend is a
            // recorded leftover (see WORKER-REPORT).
            if let FindingStatus::DeviationAcknowledged { reason } = &f.status {
                let justification = reason.clone().unwrap_or_else(|| {
                    "acknowledged in-source deviation (#[spec(deviates)] testimony)".to_string()
                });
                result["suppressions"] = serde_json::json!([{
                    "kind": "inSource",
                    "justification": justification
                }]);
            }
            result
        })
        .collect();
    let doc = serde_json::json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": { "driver": {
                "name": "vibevm-conform",
                "version": "0.1.0",
                "rules": rule_objs
            }},
            "results": results
        }]
    });
    // to_string_pretty over a serde_json::Value cannot fail — Value's
    // Serialize never errors and the sink is a String — so the default
    // branch is unreachable; a Result signature would thread an
    // impossible error through every gate caller of this byte-stable
    // renderer (ENGINE-CONFORM #rules: no-unwrap-in-domain).
    let mut s = serde_json::to_string_pretty(&doc).unwrap_or_default();
    s.push('\n');
    s
}

/// The stable SARIF `properties.vibevmConform/status` label for a
/// finding's status — a lowercase token a visualizer switches on. Kept
/// next to the renderer so the on-the-wire name cannot drift from the
/// enum (byte-stability: a rename here is a deliberate SARIF edit).
fn status_name(status: &FindingStatus) -> &'static str {
    match status {
        FindingStatus::Live => "live",
        FindingStatus::DeviationAcknowledged { .. } => "deviation-acknowledged",
    }
}

// ===========================================================================
// B-026 — the read half. A flora step deposits foreign-linter SARIF reports
// into a known place; `ingest` turns one report's results into facts, and
// `load_reports` reads the configured place. The Discipline quotes a foreign
// linter rather than reinventing it: clippy / eslint / golangci-lint already
// find what they find, and a rule may cite one of their diagnoses
// (`Fact::LintDiagnosis`, found via `Fact::cites_lint`).
//
// No schema dependency: the engine vendors into six copies, and each new
// crate dep rides into all of them. SARIF 2.1.0 is enormous but the gate
// cites only `runs[].tool.driver.name` and `runs[].results[].{ruleId,
// message.text, locations[].physicalLocation, suppressions}` — so this
// walks that subset with serde_json::Value (already a dep), and a broken or
// unfamiliar report degrades to "fewer facts", never a panic or a failed
// gate. The unread report is the absence of facts, not a refusal.
// ===========================================================================

/// Parse one SARIF 2.1.0 report's text into [`Fact::LintDiagnosis`] facts.
/// Pure (no I/O): the loader ([`load_reports`]) owns reading + announcing.
///
/// Walks the cited subset with `serde_json::Value` — a missing or
/// unfamiliar field degrades gracefully: a run with no `tool.driver.name`
/// is attributed to `"unknown"`; a result with no `ruleId` or no located
/// site is skipped (a diagnosis that names neither a rule nor a place
/// cannot be cited); a missing `message.text` becomes the empty string.
/// `suppressed` is `true` when the result carries any `suppressions`
/// entry (the foreign-linter shape of «known and accepted in source»),
/// and `reason` is that suppression's `justification` when the report
/// gave a non-empty one.
///
/// ```
/// use core_ai_native_conform::sarif;
/// use core_ai_native_conform::Fact;
///
/// let report = r#"{
///   "version": "2.1.0",
///   "runs": [{
///     "tool": { "driver": { "name": "clippy" } },
///     "results": [
///       { "ruleId": "clippy::unwrap_used",
///         "message": { "text": "used .unwrap()" },
///         "locations": [{ "physicalLocation": {
///           "artifactLocation": { "uri": "src/a.rs" },
///           "region": { "startLine": 4 } }}] },
///       { "ruleId": "clippy::unwrap_used",
///         "message": { "text": "used .unwrap()" },
///         "locations": [{ "physicalLocation": {
///           "artifactLocation": { "uri": "src/a.rs" },
///           "region": { "startLine": 9 } }}],
///         "suppressions": [{ "kind": "inSource", "justification": "FFI" }] }
///     ]
///   }]
/// }"#;
/// let facts = sarif::ingest(report);
/// assert_eq!(facts.len(), 2);
/// assert!(facts[0].cites_lint("clippy", "clippy::unwrap_used", Some(false)));
/// assert!(facts[1].cites_lint("clippy", "clippy::unwrap_used", Some(true)));
/// ```
pub fn ingest(text: &str) -> Vec<Fact> {
    let Ok(doc) = serde_json::from_str::<serde_json::Value>(text) else {
        return Vec::new();
    };
    let Some(runs) = doc.get("runs").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for run in runs {
        let tool = run
            .get("tool")
            .and_then(|t| t.get("driver"))
            .and_then(|d| d.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("unknown")
            .to_string();
        let Some(results) = run.get("results").and_then(|v| v.as_array()) else {
            continue;
        };
        for res in results {
            let Some(rule_id) = res.get("ruleId").and_then(|v| v.as_str()) else {
                continue;
            };
            let message = res
                .get("message")
                .and_then(|m| m.get("text"))
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();
            let Some((file, line)) = first_location(res) else {
                continue;
            };
            let suppressions = res.get("suppressions").and_then(|v| v.as_array());
            let suppressed = suppressions.is_some_and(|s| !s.is_empty());
            // `reason` is the suppression's justification when the report
            // carried a non-empty one — None both for a live diagnosis and
            // for a suppressed one whose author left no reason (the case
            // `lint-suppression-needs-reason` fires on).
            let reason = suppressions.and_then(|s| {
                s.iter().find_map(|sup| {
                    sup.get("justification")
                        .and_then(|j| j.as_str())
                        .map(str::trim)
                        .filter(|t| !t.is_empty())
                        .map(|t| t.to_string())
                })
            });
            out.push(Fact::LintDiagnosis {
                tool: tool.clone(),
                rule_id: rule_id.to_string(),
                file,
                line,
                message,
                suppressed,
                reason,
            });
        }
    }
    out
}

/// The first result location's `(file, line)`, or `None` when the result
/// carries no located site. `line` is `region.startLine` when present, else
/// `0` (a diagnosis that names a file but no line is still citable by file).
fn first_location(res: &serde_json::Value) -> Option<(String, u32)> {
    let pl = res
        .get("locations")
        .and_then(|v| v.as_array())?
        .first()?
        .get("physicalLocation")?;
    let file = pl
        .get("artifactLocation")
        .and_then(|a| a.get("uri"))
        .and_then(|u| u.as_str())?
        .to_string();
    let line = pl
        .get("region")
        .and_then(|r| r.get("startLine"))
        .and_then(|s| s.as_u64())
        .map(|n| n as u32)
        .unwrap_or(0);
    Some((file, line))
}

/// Read every SARIF report under the configured `paths` (each a file read
/// directly or a directory walked for `*.sarif` / `*.json`), ingest each,
/// and bucket the diagnoses into [`SourceFacts`] by file (a foreign
/// diagnosis has no crate affiliation, so `crate_name` is empty). Returns
/// `(facts, reports_ingested, diagnoses)` for the caller's announce line.
///
/// The unread report is the absence of facts, not a refusal (B-026): an
/// unreadable file, an unparseable document, or a JSON blob with no `runs`
/// is announced visibly on stderr (so a dropped report is never silent)
/// and skipped — the gate never fails because a foreign report was broken.
/// An absent path or an empty list is the norm today (no project deposits
/// reports yet) and stays silent. Deterministic: files are sorted, facts
/// within a file are sorted by `(line, rule_id, tool)`, and the records
/// are sorted by file.
pub fn load_reports(root: &Path, paths: &[String]) -> (Vec<SourceFacts>, usize, usize) {
    let mut bucket: BTreeMap<String, Vec<Fact>> = BTreeMap::new();
    let mut reports = 0usize;
    let mut diagnoses = 0usize;
    for entry in paths {
        for file in collect_report_files(&root.join(entry)) {
            let text = match std::fs::read_to_string(&file) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!(
                        "conform: SARIF report {} unreadable — skipped ({}). \
                         A broken report never fails the gate.",
                        file.display(),
                        e
                    );
                    continue;
                }
            };
            match serde_json::from_str::<serde_json::Value>(&text) {
                Ok(doc) => {
                    if doc.get("runs").and_then(|v| v.as_array()).is_none() {
                        eprintln!(
                            "conform: {} parsed as JSON but carries no `runs` — not a SARIF \
                             report? Skipped (no facts read; never fatal).",
                            file.display()
                        );
                        continue;
                    }
                    reports += 1;
                    let parsed = ingest(&text);
                    diagnoses += parsed.len();
                    for f in parsed {
                        if let Fact::LintDiagnosis { file: rel, .. } = &f {
                            bucket.entry(rel.clone()).or_default().push(f);
                        }
                    }
                }
                Err(e) => {
                    eprintln!(
                        "conform: SARIF report {} did not parse as JSON — skipped ({}). \
                         No facts read; a broken report never fails the gate.",
                        file.display(),
                        e
                    );
                }
            }
        }
    }
    let facts: Vec<SourceFacts> = bucket
        .into_iter()
        .map(|(file, mut fs)| {
            fs.sort_by(|a, b| {
                line_of(a)
                    .cmp(&line_of(b))
                    .then_with(|| rule_id_of(a).cmp(rule_id_of(b)))
                    .then_with(|| tool_of(a).cmp(tool_of(b)))
            });
            SourceFacts {
                file,
                crate_name: String::new(),
                facts: fs,
            }
        })
        .collect();
    (sort_source_facts(facts), reports, diagnoses)
}

/// Free `fn`s (not closures) so the borrow lifetime of the returned `&str`
/// is explicit — a closure returning `&str` from its argument fails lifetime
/// elision. The bucket only ever holds `LintDiagnosis`, but the `Vec<Fact>`
/// type forces a total match.
fn line_of(f: &Fact) -> u32 {
    match f {
        Fact::LintDiagnosis { line, .. } => *line,
        _ => 0,
    }
}
fn rule_id_of(f: &Fact) -> &str {
    match f {
        Fact::LintDiagnosis { rule_id, .. } => rule_id.as_str(),
        _ => "",
    }
}
fn tool_of(f: &Fact) -> &str {
    match f {
        Fact::LintDiagnosis { tool, .. } => tool.as_str(),
        _ => "",
    }
}

/// Every `*.sarif` / `*.json` file under `path`: the path itself if it is a
/// report file, or every matching file under it (sorted) if it is a
/// directory. An absent path yields nothing (the norm — no report deposited).
fn collect_report_files(path: &Path) -> Vec<PathBuf> {
    if !path.exists() {
        return Vec::new();
    }
    if path.is_file() {
        return vec![path.to_path_buf()];
    }
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(path)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|e| {
            if !e.file_type().is_dir() {
                return true;
            }
            !matches!(
                e.file_name().to_str(),
                Some("target") | Some(".git") | Some("node_modules")
            )
        })
        .filter_map(Result::ok)
    {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        if p.extension().and_then(|e| e.to_str()).is_some_and(|ext| {
            ext.eq_ignore_ascii_case("sarif") || ext.eq_ignore_ascii_case("json")
        }) {
            out.push(p.to_path_buf());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use crate::rules;
    use crate::sarif;
    use crate::{Fact, SourceFacts, check};

    fn sf(file: &str, crate_name: &str, facts: Vec<Fact>) -> SourceFacts {
        SourceFacts {
            file: file.to_string(),
            crate_name: crate_name.to_string(),
            facts,
        }
    }

    #[test]
    fn sarif_is_byte_stable() {
        let gate = rules::UnsafeGate {
            audit_crates: vec![],
        };
        let facts = vec![sf(
            "crates/a/src/lib.rs",
            "a",
            vec![Fact::UnsafeUse {
                context: "block".into(),
                line: 5,
                in_test: false,
                in_deviation: false,
            }],
        )];
        let findings = check(&[&gate], &facts, None);
        let a = sarif::render(&[&gate], &findings);
        let b = sarif::render(&[&gate], &findings);
        assert_eq!(a, b);
        assert!(a.contains("\"ruleId\": \"unsafe-gate\""));
        // A Live finding carries no suppressions and is tagged live.
        assert!(a.contains("\"vibevmConform/status\": \"live\""));
        assert!(!a.contains("suppressions"));
    }

    /// B-025: an acknowledged deviation STAYS in the SARIF, marked with
    /// an `inSource` suppression whose justification is the recorded
    /// reason. (The Rust facts carry no reason text — the rust-syn
    /// plumbing is a leftover — so the Rust driver falls back to a
    /// fixed marker; this test uses a direct construction to prove the
    /// reason IS rendered when present, which is the TS/Go path.)
    #[test]
    fn acknowledged_finding_renders_with_in_source_suppression() {
        use crate::Finding;
        use crate::finding::FindingStatus;
        use crate::rules::req_message;
        let finding = Finding {
            rule: "unsafe-gate",
            file: "crates/a/src/lib.rs".into(),
            line: 9,
            message: req_message(
                "discipline://rust-ai-native-lang/guide#bans-and-escape-hatches",
                "`unsafe` (block) outside a designated audit crate",
                "recorded deviation",
            ),
            why: "unsafe is an audit boundary",
            fingerprint: "unsafe-gate|crates/a/src/lib.rs|block#0".into(),
            status: FindingStatus::DeviationAcknowledged {
                reason: Some("FFI boundary, audited".into()),
            },
            evidence: "UnsafeUse(block,test=false,dev=true)".into(),
        };
        let report = sarif::render(&[], std::slice::from_ref(&finding));
        assert!(report.contains("\"vibevmConform/status\": \"deviation-acknowledged\""));
        assert!(report.contains("\"kind\": \"inSource\""));
        assert!(report.contains("\"justification\": \"FFI boundary, audited\""));
        // The result is still present (visible), just suppressed.
        assert!(report.contains("\"ruleId\": \"unsafe-gate\""));
    }
}
