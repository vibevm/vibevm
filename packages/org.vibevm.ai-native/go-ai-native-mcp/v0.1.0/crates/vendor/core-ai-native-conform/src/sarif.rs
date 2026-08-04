specmark::scope!(
    "spec://org.vibevm.ai-native/core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#determinism"
);

use crate::finding::{Finding, FindingStatus, Rule};

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
