//! The citation family (B-026 SARIF ingest) — rules over foreign-linter
//! diagnoses ([`Fact::LintDiagnosis`]), the facts a SARIF report a flora
//! step deposited becomes. The Discipline quotes a foreign linter rather
//! than reinventing it: a rule says «this diagnosis confirms my claim» via
//! the citation primitive [`Fact::cites_lint`] (`check: { tool, rule_id,
//! status }`).

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules");

use std::collections::BTreeMap;

use crate::facts::{Fact, SourceFacts};
use crate::finding::{Finding, FindingStatus, Rule};

use super::req_message;

/// `lint-suppression-needs-reason` — a foreign-linter diagnosis the
/// codebase has SUPPRESSED must carry a recorded reason, the universal
/// Discipline posture that every escape is justified (the same rule that
/// makes `[[<lang>.exempt]]`, `[[<lang>.floor_disable]]`, and a
/// `#[spec(deviates)]` each require a `reason`). A SARIF `suppressions`
/// entry with no `justification` is a silent escape — invisible by
/// default — and this rule lifts it into the IR.
///
/// A suppressed diagnosis WITH a reason surfaces
/// [`FindingStatus::DeviationAcknowledged`] (B-025: visible in the IR and
/// the SARIF, never failing the gate — the foreign-linter shape of «known
/// and accepted in source» lays onto the acknowledged status exactly). A
/// suppressed diagnosis WITHOUT a reason surfaces `Live` — a real
/// violation that fails the gate until the suppression is given a
/// justification. A live (non-suppressed) diagnosis is citation DATA a
/// project cites via [`Fact::cites_lint`] when a specific foreign verdict
/// should gate; this rule does not surface it.
///
/// ```
/// use core_ai_native_conform::rules::LintSuppressionNeedsReason;
/// use core_ai_native_conform::{Fact, Rule, SourceFacts};
///
/// let sf = |file: &str, f: Vec<Fact>| SourceFacts {
///     file: file.into(), crate_name: String::new(), facts: f,
/// };
/// let reasoned = Fact::LintDiagnosis {
///     tool: "clippy".into(), rule_id: "clippy::unwrap_used".into(),
///     file: "src/a.rs".into(), line: 9, message: "used unwrap".into(),
///     suppressed: true, reason: Some("FFI boundary".into()),
/// };
/// let reasonless = Fact::LintDiagnosis {
///     tool: "clippy".into(), rule_id: "clippy::unwrap_used".into(),
///     file: "src/b.rs".into(), line: 2, message: "used unwrap".into(),
///     suppressed: true, reason: None,
/// };
/// let live = Fact::LintDiagnosis {
///     tool: "clippy".into(), rule_id: "clippy::unwrap_used".into(),
///     file: "src/c.rs".into(), line: 7, message: "used unwrap".into(),
///     suppressed: false, reason: None,
/// };
/// let findings = LintSuppressionNeedsReason.check(&[
///     sf("src/a.rs", vec![reasoned]),
///     sf("src/b.rs", vec![reasonless]),
///     sf("src/c.rs", vec![live]),
/// ]);
/// assert_eq!(findings.len(), 2, "the live diagnosis is not surfaced");
/// use core_ai_native_conform::FindingStatus;
/// assert!(findings.iter().any(|f| f.file == "src/a.rs"
///     && matches!(f.status, FindingStatus::DeviationAcknowledged { .. })));
/// assert!(findings.iter().any(|f| f.file == "src/b.rs"
///     && matches!(f.status, FindingStatus::Live)));
/// ```
pub struct LintSuppressionNeedsReason;

impl Rule for LintSuppressionNeedsReason {
    fn id(&self) -> &'static str {
        "lint-suppression-needs-reason"
    }
    fn why(&self) -> &'static str {
        "a suppressed foreign-linter diagnosis is an escape hatch, and \
         every escape carries a recorded reason — a SARIF suppression \
         with no justification is a silent one (B-026; the same posture \
         as [[<lang>.exempt]] and [[<lang>.floor_disable]] needing a reason)"
    }
    fn check(&self, facts: &[SourceFacts]) -> Vec<Finding> {
        let mut out = Vec::new();
        for sf in facts {
            // Per-(file, tool, rule_id) ordinal for stable fingerprints —
            // a line-keyed fingerprint rots on any edit above the site
            // (the stop.rs 33→35 lesson), and a baseline that rots on
            // unrelated edits is a checker that lies. The ordinal advances
            // over EVERY suppressed diagnosis of that key, before any
            // status branch, so a neighbour gaining/losing a reason never
            // re-keys a frozen entry.
            let mut seen: BTreeMap<(String, String), u32> = BTreeMap::new();
            for f in &sf.facts {
                let Fact::LintDiagnosis {
                    tool,
                    rule_id,
                    line,
                    message,
                    suppressed,
                    reason,
                    ..
                } = f
                else {
                    continue;
                };
                if !suppressed {
                    continue;
                }
                let counter = seen.entry((tool.clone(), rule_id.clone())).or_insert(0);
                let ordinal = *counter;
                *counter += 1;
                let (status, why_short) = match reason {
                    // B-025 / B-026 point 2: a reasoned suppression lays
                    // onto DeviationAcknowledged exactly — visible in the
                    // IR/SARIF, never failing the gate.
                    Some(r) => (
                        FindingStatus::DeviationAcknowledged {
                            reason: Some(r.clone()),
                        },
                        "suppressed in source with a recorded reason",
                    ),
                    // No reason: a live violation — the gate fails until the
                    // suppression carries a justification.
                    None => (FindingStatus::Live, "suppressed in source with NO reason"),
                };
                out.push(Finding {
                    rule: self.id(),
                    file: sf.file.clone(),
                    line: *line,
                    message: req_message(
                        "discipline://org.vibevm.ai-native/core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
                        &format!(
                            "{tool} `{rule_id}` at line {line} is {why_short} \
                             (\"{message}\")"
                        ),
                        "give the suppression a justification, or drop it and fix the site",
                    ),
                    why: self.why(),
                    fingerprint: format!(
                        "lint-suppression-needs-reason|{}|{tool}:{rule_id}#{ordinal}",
                        sf.file
                    ),
                    status,
                    evidence: f.summary(),
                });
            }
        }
        out.sort();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::baseline;
    use crate::{Fact, FindingStatus, Rule, SourceFacts};

    fn diag(file: &str, line: u32, suppressed: bool, reason: Option<&str>) -> Fact {
        Fact::LintDiagnosis {
            tool: "clippy".into(),
            rule_id: "clippy::unwrap_used".into(),
            file: file.into(),
            line,
            message: "used .unwrap()".into(),
            suppressed,
            reason: reason.map(|r| r.to_string()),
        }
    }

    fn sf(file: &str, facts: Vec<Fact>) -> SourceFacts {
        SourceFacts {
            file: file.into(),
            crate_name: String::new(),
            facts,
        }
    }

    /// The point-2 mapping, made exact: a reasoned suppressed diagnosis
    /// surfaces `DeviationAcknowledged`, which `baseline::diff` keeps out
    /// of `new` (gate-inert) and `baseline::freezeable` never writes — so
    /// a foreign acknowledgement rides the existing B-025 machinery whole,
    /// with no new status and no per-driver edit.
    #[test]
    fn reasoned_suppression_is_gate_inert_via_deviation_acknowledged() {
        let findings = LintSuppressionNeedsReason.check(&[sf(
            "src/a.rs",
            vec![diag("src/a.rs", 9, true, Some("FFI boundary"))],
        )]);
        assert_eq!(findings.len(), 1);
        assert!(matches!(
            findings[0].status,
            FindingStatus::DeviationAcknowledged { .. }
        ));
        // Gate-inert: never `new`, never frozen.
        let empty = baseline::Baseline {
            schema: 1,
            findings: vec![],
        };
        let (new, _) = baseline::diff(&empty, &findings);
        assert!(
            new.is_empty(),
            "an acknowledged suppression never fails the gate"
        );
        assert!(
            baseline::freezeable(&findings).is_empty(),
            "an acknowledged suppression is never frozen"
        );
    }

    /// A reasonless suppression is a live violation — it enters `new`
    /// against an empty baseline (the gate fails until a reason is given).
    #[test]
    fn reasonless_suppression_is_a_live_violation() {
        let findings = LintSuppressionNeedsReason
            .check(&[sf("src/b.rs", vec![diag("src/b.rs", 2, true, None)])]);
        assert_eq!(findings.len(), 1);
        assert!(matches!(findings[0].status, FindingStatus::Live));
        let empty = baseline::Baseline {
            schema: 1,
            findings: vec![],
        };
        let (new, _) = baseline::diff(&empty, &findings);
        assert_eq!(new.len(), 1, "the reasonless suppression fails the gate");
    }

    /// The citation primitive is live: a rule finds a diagnosis by
    /// `{tool, rule_id, status}`. (The rule itself destructures because it
    /// wants every tool/id; this test exercises the typed citation form a
    /// rule uses when it wants a SPECIFIC foreign verdict.)
    #[test]
    fn citation_primitive_finds_a_specific_foreign_verdict() {
        let facts = [sf(
            "src/a.rs",
            vec![
                diag("src/a.rs", 4, false, None),
                diag("src/a.rs", 9, true, Some("FFI")),
            ],
        )];
        let all: Vec<&Fact> = facts.iter().flat_map(|s| s.facts.iter()).collect();
        let live = all
            .iter()
            .find(|f| f.cites_lint("clippy", "clippy::unwrap_used", Some(false)));
        let ack = all
            .iter()
            .find(|f| f.cites_lint("clippy", "clippy::unwrap_used", Some(true)));
        assert!(live.is_some(), "the live diagnosis is cited by status");
        assert!(
            ack.is_some(),
            "the acknowledged diagnosis is cited by status"
        );
        assert!(
            all.iter()
                .all(|f| !f.cites_lint("eslint", "clippy::unwrap_used", None)),
            "the wrong tool does not cite"
        );
    }
}
