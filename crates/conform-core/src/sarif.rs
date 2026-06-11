use crate::finding::{Finding, Rule};

specmark::scope!("spec://vibevm/discipline/ENGINE-CONFORM-v0.1#determinism");

/// Byte-stable minimal SARIF 2.1.0: stable ordering (findings are
/// pre-sorted), no wall-clock, no absolute paths.
///
/// ```
/// use conform_core::rules::CellIsolation;
/// use conform_core::sarif;
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
            serde_json::json!({
                "ruleId": f.rule,
                "level": "error",
                "message": { "text": f.message },
                "partialFingerprints": { "vibevmConform/v1": f.fingerprint },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": { "uri": f.file },
                        "region": { "startLine": f.line }
                    }
                }]
            })
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
    let mut s = serde_json::to_string_pretty(&doc).expect("sarif serialises");
    s.push('\n');
    s
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
        let gate = rules::UnsafeGate { audit_crates: &[] };
        let facts = vec![sf(
            "crates/a/src/lib.rs",
            "a",
            vec![Fact::UnsafeUse {
                context: "block".into(),
                line: 5,
            }],
        )];
        let findings = check(&[&gate], &facts, None);
        let a = sarif::render(&[&gate], &findings);
        let b = sarif::render(&[&gate], &findings);
        assert_eq!(a, b);
        assert!(a.contains("\"ruleId\": \"unsafe-gate\""));
    }
}
