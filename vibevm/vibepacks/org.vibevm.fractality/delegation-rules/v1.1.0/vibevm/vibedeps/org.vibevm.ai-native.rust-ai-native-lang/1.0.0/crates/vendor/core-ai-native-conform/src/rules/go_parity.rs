//! The Go parity rule family (B-033 + B-030, the seam-error and
//! conformance-assertion parity lift): a seam's closed error set
//! carries its REQ URI (`go-seam-error-cites-req`), and every cell
//! carries the loud-conformance assertion (`go-conformance-assertion`).
//! Split out of `go.rs` along the parity seam so neither file crosses
//! the 600-line budget; the scope unit matches its siblings so
//! self-trace finds no orphan.

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules");

use crate::facts::{Fact, SourceFacts};
use crate::finding::{Finding, FindingStatus, Rule};
use crate::rules::req_message;

use super::go::GO_GUIDE_ERRORS;

const GO_GUIDE_CONFORMANCE: &str = "discipline://go-ai-native-lang/guide#conformance-is-made-loud";

/// `go-seam-error-cites-req` — a seam's closed error set carries its
/// REQ URI. This is the dedicated home for the two seam-error halves
/// that previously rode the `go-unsafe-in-domain` umbrella: the
/// **structure half** (`seam_error_missing_req` — the `*Error` type
/// owns an `Error()` method but carries no `Spec` field, so it cannot
/// hold the violated `spec://` URI) and the **message half**
/// (`seam_error_message_no_req` — its `Error()` renders no `spec://`,
/// the direct Go analogue of Rust's `message.contains("spec://")` gate
/// in `error-message-cites-req`). One Go rule checks both halves (Go's
/// idiom is one struct with two obligations), but each half emits under
/// its own fingerprint suffix (`…-structure` / `…-message`) so the
/// ratchet tightens them independently and SARIF separates them.
///
/// A site covered by a reasoned `//spec:deviates … reason="…"` is
/// recorded testimony — B-025 (mark, don't suppress): it is stamped
/// `DeviationAcknowledged` (visible, gate-green, reason carried), not
/// skipped (the same posture the umbrella now uses); `_test.go` files
/// are out of scope (carried verbatim from `go-unsafe-in-domain`).
///
/// ```
/// use core_ai_native_conform::rules::GoSeamErrorCitesReq;
/// use core_ai_native_conform::{Fact, Rule, SourceFacts};
///
/// let facts = vec![SourceFacts {
///     file: "internal/cells/plan/plan.go".into(),
///     crate_name: "demo".into(),
///     facts: vec![Fact::GoUnsafe {
///         kind: "seam_error_missing_req".into(),
///         line: 17,
///         in_test: false,
///         reason: None,
///     }],
/// }];
/// let findings = GoSeamErrorCitesReq.check(&facts);
/// assert_eq!(findings.len(), 1);
/// assert!(findings[0].fingerprint.contains("structure"));
/// ```
pub struct GoSeamErrorCitesReq;

impl Rule for GoSeamErrorCitesReq {
    fn id(&self) -> &'static str {
        "go-seam-error-cites-req"
    }
    fn why(&self) -> &'static str {
        "a seam's closed error set carries its REQ URI: the type holds the \
         violated spec:// (Code + Spec + Err) and Error() renders it, so a \
         failing run is navigable back to the requirement (GUIDE-AI-NATIVE-GO \
         §5; the structure half and the message half)"
    }
    fn check(&self, facts: &[SourceFacts]) -> Vec<Finding> {
        let mut out = Vec::new();
        for source in facts {
            for fact in &source.facts {
                let Fact::GoUnsafe {
                    kind,
                    line,
                    in_test,
                    reason,
                } = fact
                else {
                    continue;
                };
                // B-025 (mark, don't suppress): a reasoned
                // `//spec:deviates` covering a SEAM-ERROR site is MARKED
                // acknowledged, not skipped. Only the two seam_error
                // kinds belong here — a reasoned deviation of another
                // kind is `GoUnsafeInDomain`'s acknowledged finding, not
                // a duplicate under this rule. The `half` still keys the
                // fingerprint so the two obligations stay separable.
                let half = match kind.as_str() {
                    "seam_error_missing_req" => "structure",
                    "seam_error_message_no_req" => "message",
                    _ => "deviation",
                };
                if reason.is_some() && half != "deviation" {
                    out.push(Finding {
                        rule: self.id(),
                        file: source.file.clone(),
                        line: *line,
                        message: req_message(
                            GO_GUIDE_ERRORS,
                            &format!(
                                "`{kind}` seam-error obligation is covered by a recorded \
                                 //spec:deviates deviation"
                            ),
                            "keep the deviation recorded, or remediate the seam error and \
                             remove the directive",
                        ),
                        why: self.why(),
                        fingerprint: format!(
                            "go-seam-error-cites-req-{half}|{}|{line}",
                            source.file
                        ),
                        status: FindingStatus::DeviationAcknowledged {
                            reason: reason.clone(),
                        },
                        evidence: fact.summary(),
                    });
                    continue;
                }
                let (half, why, fix) = match kind.as_str() {
                    "seam_error_missing_req" if !in_test => (
                        "structure",
                        "a seam error type carries no `Spec` field — cannot cite its REQ",
                        "carry the violated spec:// URI as a `Spec` field (Code + Spec + Err)",
                    ),
                    "seam_error_message_no_req" if !in_test => (
                        "message",
                        "a seam error `Error()` renders no spec:// REQ",
                        "render the violated spec:// URI in the `Error()` format string",
                    ),
                    _ => continue,
                };
                out.push(Finding {
                    rule: self.id(),
                    file: source.file.clone(),
                    line: *line,
                    message: req_message(GO_GUIDE_ERRORS, why, fix),
                    why: self.why(),
                    fingerprint: format!("go-seam-error-cites-req-{half}|{}|{line}", source.file),
                    status: FindingStatus::Live,
                    evidence: fact.summary(),
                });
            }
        }
        out
    }
}

/// `go-conformance-assertion` — the «conformance is made loud»
/// presence check (GUIDE-AI-NATIVE-GO §2, B-030): a **gated** cell (a
/// package directory directly under `cells_dir`, named in the gate list)
/// carries the compile-time assertion `var _ <seam> = (*<Impl>)(nil)`.
/// This is the Go analogue of Rust's `cargo check` at the use site — the
/// one seam-conformance signal that can drift silently, so a presence
/// check earns its keep. It is the absence-check twin of
/// `cell-has-oracle`: keyed on the cell set derived from the fact
/// file-paths and the `Fact::GoConformance` facts, it fires for a gated
/// cell that declares no assertion. Gating scopes the rule to the cells
/// a project polices, so a seam-less or exempt cell — one with nothing to
/// assert — is never falsely flagged.
///
/// Mounted conditional on `cells_dir` (the `go-cell-isolation`
/// template), so a project without cells never runs it; a project
/// mounts it by setting `[go] cells_dir` and gating its cells. Findings
/// land soft through the ratchet baseline until the tree is clean.
///
/// ```
/// use core_ai_native_conform::rules::GoConformanceAssertion;
/// use core_ai_native_conform::{Fact, Rule, SourceFacts};
///
/// // A cell with no conformance assertion is flagged.
/// let rule = GoConformanceAssertion::new(Some("internal/cells"), &["internal/cells/plan".into()]);
/// let facts = vec![SourceFacts {
///     file: "internal/cells/plan/plan.go".into(),
///     crate_name: "demo".into(),
///     facts: vec![],
/// }];
/// assert_eq!(rule.check(&facts).len(), 1);
/// ```
pub struct GoConformanceAssertion {
    cells_dir: Option<String>,
    /// The gated cell packages (repo-relative `cells_dir/<cell>`). Only a
    /// GATED cell owes the assertion: a project gates the cells it
    /// polices, so an exempt or ungated cell — including one that
    /// satisfies no seam and has nothing to assert — is out of scope.
    gated: std::collections::BTreeSet<String>,
}

impl GoConformanceAssertion {
    pub fn new(cells_dir: Option<&str>, gated: &[String]) -> GoConformanceAssertion {
        GoConformanceAssertion {
            cells_dir: cells_dir.map(|d| d.trim_matches('/').to_string()),
            gated: gated
                .iter()
                .map(|g| g.trim_matches('/').to_string())
                .collect(),
        }
    }

    /// The cell a repo-relative FILE path belongs to, if it is under
    /// `cells_dir`: `internal/cells/plan/plan.go` → `Some("plan")`.
    /// Delegates to the one shared cell-of-file parser
    /// (`super::go::cell_of_file`) so this rule, `GoCellIsolation`, and
    /// `GoFlagSites` agree on what a cell is.
    fn cell_of_file<'a>(&self, rel: &'a str) -> Option<&'a str> {
        let dir = self.cells_dir.as_deref()?;
        super::go::cell_of_file(dir, rel)
    }
}

impl Rule for GoConformanceAssertion {
    fn id(&self) -> &'static str {
        "go-conformance-assertion"
    }
    fn why(&self) -> &'static str {
        "silent interface conformance can drift: a cell that drops its \
         `var _ Seam = (*Impl)(nil)` assertion can stop satisfying its seam \
         with no compile error naming the seam (GUIDE-AI-NATIVE-GO §2; the \
         loud-conformance idiom)"
    }
    fn check(&self, facts: &[SourceFacts]) -> Vec<Finding> {
        let mut out = Vec::new();
        if self.cells_dir.is_none() {
            return out;
        }
        // The cell set is whatever the fact stream declares under
        // cells_dir; a GATED cell is owing if it carries no GoConformance
        // fact. Gating scopes the rule to the cells a project polices, so
        // a seam-less or exempt cell is never falsely flagged (measured:
        // research/go-demo's gated cells all carry the assertion, the
        // clean fixture's gated greet cell carries it, the dirty fixture's
        // gated plan cell does not).
        let mut first_file: std::collections::BTreeMap<String, String> = Default::default();
        let mut asserting: std::collections::BTreeSet<String> = Default::default();
        for sf in facts {
            let Some(cell) = self.cell_of_file(&sf.file) else {
                continue;
            };
            first_file
                .entry(cell.to_string())
                .and_modify(|f| {
                    if &sf.file < f {
                        *f = sf.file.clone();
                    }
                })
                .or_insert_with(|| sf.file.clone());
            if sf
                .facts
                .iter()
                .any(|f| matches!(f, Fact::GoConformance { .. }))
            {
                asserting.insert(cell.to_string());
            }
        }
        for (cell, file) in &first_file {
            if asserting.contains(cell) {
                continue;
            }
            // Only a gated cell owes the assertion.
            let pkg = match &self.cells_dir {
                Some(dir) => format!("{dir}/{cell}"),
                None => cell.clone(),
            };
            if !self.gated.contains(&pkg) {
                continue;
            }
            out.push(Finding {
                rule: self.id(),
                file: file.clone(),
                line: 1,
                message: req_message(
                    GO_GUIDE_CONFORMANCE,
                    &format!(
                        "cell `{cell}` carries no conformance assertion \
                         `var _ Seam = (*Impl)(nil)`"
                    ),
                    "add `var _ <seam> = (*<Impl>)(nil)` beside the cell's type declaration",
                ),
                why: self.why(),
                fingerprint: format!("{}|{cell}", self.id()),
                status: FindingStatus::Live,
                evidence: format!("no GoConformance assertion for cell `{cell}`"),
            });
        }
        out.sort();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn go_source(file: &str, facts: Vec<Fact>) -> SourceFacts {
        SourceFacts {
            file: file.into(),
            crate_name: "demo".into(),
            facts,
        }
    }

    #[test]
    fn seam_error_structure_half_emits_structure_fingerprint() {
        let facts = vec![go_source(
            "internal/cells/plan/plan.go",
            vec![Fact::GoUnsafe {
                kind: "seam_error_missing_req".into(),
                line: 17,
                in_test: false,
                reason: None,
            }],
        )];
        let findings = GoSeamErrorCitesReq.check(&facts);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].fingerprint.contains("structure"));
        assert!(findings[0].message.contains("Spec"));
        assert!(crate::rules::matches_req_grammar(&findings[0].message));
    }

    #[test]
    fn seam_error_message_half_emits_message_fingerprint() {
        let facts = vec![go_source(
            "internal/cells/plan/plan.go",
            vec![Fact::GoUnsafe {
                kind: "seam_error_message_no_req".into(),
                line: 22,
                in_test: false,
                reason: None,
            }],
        )];
        let findings = GoSeamErrorCitesReq.check(&facts);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].fingerprint.contains("message"));
        assert!(findings[0].message.contains("Error()"));
        assert!(crate::rules::matches_req_grammar(&findings[0].message));
    }

    #[test]
    fn seam_error_type_missing_both_halves_emits_two_distinct_findings() {
        let facts = vec![go_source(
            "internal/cells/plan/plan.go",
            vec![
                Fact::GoUnsafe {
                    kind: "seam_error_missing_req".into(),
                    line: 17,
                    in_test: false,
                    reason: None,
                },
                Fact::GoUnsafe {
                    kind: "seam_error_message_no_req".into(),
                    line: 22,
                    in_test: false,
                    reason: None,
                },
            ],
        )];
        let findings = GoSeamErrorCitesReq.check(&facts);
        assert_eq!(findings.len(), 2, "{findings:?}");
        let fps: std::collections::HashSet<&str> =
            findings.iter().map(|f| f.fingerprint.as_str()).collect();
        assert_eq!(fps.len(), 2, "two distinct fingerprints: {fps:?}");
        assert!(fps.iter().any(|f| f.contains("structure")));
        assert!(fps.iter().any(|f| f.contains("message")));
    }

    #[test]
    fn seam_error_deviation_is_marked_and_test_context_is_out_of_scope() {
        let facts = vec![go_source(
            "internal/cells/plan/plan.go",
            vec![
                // A reasoned deviation is MARKED acknowledged (B-025),
                // carrying its reason — not skipped.
                Fact::GoUnsafe {
                    kind: "seam_error_missing_req".into(),
                    line: 3,
                    in_test: false,
                    reason: Some("documented elsewhere".into()),
                },
                // A test file stays out of scope.
                Fact::GoUnsafe {
                    kind: "seam_error_message_no_req".into(),
                    line: 9,
                    in_test: true,
                    reason: None,
                },
            ],
        )];
        let findings = GoSeamErrorCitesReq.check(&facts);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].fingerprint.contains("structure"));
        assert!(matches!(
            findings[0].status,
            FindingStatus::DeviationAcknowledged { ref reason }
                if reason.as_deref() == Some("documented elsewhere")
        ));
    }

    #[test]
    fn conformance_assertion_present_cell_is_silent() {
        let rule =
            GoConformanceAssertion::new(Some("internal/cells"), &["internal/cells/plan".into()]);
        let facts = vec![go_source(
            "internal/cells/plan/planner.go",
            vec![Fact::GoConformance {
                seam: "seams.Planner".into(),
                impl_type: "Planner".into(),
                line: 14,
                in_test: false,
            }],
        )];
        assert!(rule.check(&facts).is_empty(), "an asserting cell is quiet");
    }

    #[test]
    fn conformance_assertion_absent_cell_is_flagged() {
        let rule =
            GoConformanceAssertion::new(Some("internal/cells"), &["internal/cells/plan".into()]);
        let facts = vec![go_source("internal/cells/plan/planner.go", vec![])];
        let findings = rule.check(&facts);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].fingerprint.contains("plan"));
        assert!(findings[0].message.contains("var _ Seam"));
        assert!(crate::rules::matches_req_grammar(&findings[0].message));
    }

    #[test]
    fn conformance_assertion_ungated_cell_is_silent() {
        // An ungated cell (not in the gate list — e.g. a seam-less or
        // exempt cell) owes nothing, even with no assertion.
        let rule = GoConformanceAssertion::new(Some("internal/cells"), &[]);
        let facts = vec![go_source("internal/cells/plan/planner.go", vec![])];
        assert!(
            rule.check(&facts).is_empty(),
            "an ungated cell is not policed"
        );
    }

    #[test]
    fn conformance_assertion_without_cells_dir_is_a_noop() {
        // The None constructor must not panic and emits nothing.
        let rule = GoConformanceAssertion::new(None, &[]);
        let facts = vec![go_source("internal/cells/plan/planner.go", vec![])];
        assert!(rule.check(&facts).is_empty());
    }
}
