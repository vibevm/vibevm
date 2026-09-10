specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules");

use std::collections::BTreeMap;

use crate::facts::SourceFacts;

/// One finding with its A1 chain.
///
/// ```
/// use core_ai_native_conform::{Finding, FindingStatus};
///
/// let f = Finding {
///     rule: "unsafe-gate",
///     file: "crates/x/src/lib.rs".into(),
///     line: 5,
///     message: core_ai_native_conform::rules::req_message(
///         "discipline://rust-ai-native-lang/guide#bans-and-escape-hatches",
///         "`unsafe` (block) outside a designated audit crate",
///         "move it or record the deviation",
///     ),
///     why: "unsafe is an audit boundary",
///     fingerprint: "unsafe-gate|crates/x/src/lib.rs|block#0".into(),
///     status: FindingStatus::Live,
///     evidence: "UnsafeUse(block,test=false,dev=false)".into(),
/// };
/// assert!(core_ai_native_conform::rules::matches_req_grammar(&f.message));
/// assert!(matches!(f.status, FindingStatus::Live));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding {
    pub rule: &'static str,
    pub file: String,
    pub line: u32,
    pub message: String,
    /// Why the rule exists — the axiom trace rendered into SARIF.
    pub why: &'static str,
    /// Stable identity for the baseline: `rule|file|carrier`.
    pub fingerprint: String,
    // --- B-025 (mark, don't suppress): a deviation no longer vanishes ---
    // The two fields below are declared LAST on purpose. `Finding`
    // derives `Ord`, and the declaration order fixes the lexicographic
    // compare: `(rule, file, line, message, why, fingerprint, status,
    // evidence)`. Fingerprints are unique within a run (`rule|file|
    // carrier#ordinal`), so the first six fields already pin a total
    // order and `status`/`evidence` are tie-breakers that are never
    // reached — the sort stays byte-identical to the pre-B-025 order, so
    // no counter, golden, or baseline shifts from ordering.
    /// Whether this is a live violation or a deviation the codebase has
    /// recorded and accepted. See [`FindingStatus`].
    pub status: FindingStatus,
    /// A compact rendering of the fact(s) that birthed this finding —
    /// the [`Fact::summary`](crate::Fact::summary) of the originating
    /// fact, or a short description for absence-based findings. Carried
    /// so a future visualizer can show WHAT fired, not just WHERE: the
    /// IR keeps every signal visible (B-025 — «нужно всё видеть»).
    pub evidence: String,
}

/// Whether a finding is a live violation or a deviation the codebase has
/// RECORDED and ACCEPTED — B-025, «помечать вместо гасить».
///
/// A recorded deviation (`#[spec(deviates = …, reason = …)]` on a Rust
/// fn, `@ts-expect-error -- reason` in TypeScript, `//spec:deviates …
/// reason="…"` in Go) used to make a finding DISAPPEAR: the rule skipped
/// it, so it was absent from the IR, the SARIF, every downstream view.
/// The owner ruled (2026-08-01) that this is wrong — every signal must
/// stay visible, because recording a deviation exists to SEE it, and a
/// future visualizer over the IR needs the full picture, not the gated
/// one. So the rule now STAMPS the finding `DeviationAcknowledged`
/// instead of skipping: the finding stays in the IR and the SARIF
/// (marked, via SARIF `suppressions`), it just never fails the gate —
/// [`baseline::diff`](crate::baseline::diff) keeps it out of `new`.
///
/// `reason` carries the deviation's recorded justification text WHEN THE
/// FRONTEND CAPTURED IT: TypeScript and Go carry it on the fact (the
/// `reason` field of [`Fact::TsUnsafe`](crate::Fact::TsUnsafe) /
/// [`Fact::GoUnsafe`](crate::Fact::GoUnsafe)), so an acknowledged TS/Go
/// finding reproduces the human's reason in SARIF. The three Rust facts
/// ([`Fact::UnsafeUse`](crate::Fact::UnsafeUse) /
/// [`Fact::UnwrapUse`](crate::Fact::UnwrapUse) /
/// [`Fact::EnvRead`](crate::Fact::EnvRead)) carry `reason` too and the
/// rules thread it onto this status (B-053, the engine half done); but
/// the rust-syn frontend does NOT yet populate it from
/// `#[spec(deviates = …, reason = "…")]`, so a Rust acknowledged finding
/// is still `reason: None` IN PRACTICE and its SARIF `justification`
/// falls back to the fixed marker until that frontend second pass lands.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum FindingStatus {
    /// A live violation — fails the gate unless its fingerprint is frozen
    /// in the baseline.
    Live,
    /// A violation next to a RECORDED deviation. It stays in the IR and
    /// SARIF (the deviation is acknowledged, not hidden) but never fails
    /// the gate. `reason` is the deviation's justification when the
    /// frontend captured it (`None` for the Rust facts until the rust-syn
    /// frontend populates it — see the enum doc).
    DeviationAcknowledged { reason: Option<String> },
}

impl Default for FindingStatus {
    /// A finding is a live violation until the rule that birthed it says
    /// otherwise — the overwhelming majority are `Live`. `Default` exists
    /// for that ergonomics (a rule that never deals with deviations, and
    /// any future code/test), NOT as a claim that "no status" is a third
    /// state: every `Finding` is constructed with an explicit `status`.
    /// Chosen `Live` over `DeviationAcknowledged` because a deviation is
    /// the marked EXCEPTION — defaulting to the un-acknowledged form is
    /// the safe failure mode (a finding that should have been marked but
    /// wasn't still fails the gate, the loud error; the reverse would
    /// silently pass a real violation).
    fn default() -> Self {
        FindingStatus::Live
    }
}

/// A rule is a compiled query over facts (ENGINE-CONFORM §4).
///
/// The canonical implementation shape — pure query in, findings out:
///
/// ```
/// use core_ai_native_conform::{Finding, Rule, SourceFacts};
///
/// struct NoFindings;
/// impl Rule for NoFindings {
///     fn id(&self) -> &'static str { "no-findings" }
///     fn why(&self) -> &'static str { "demonstrates the query shape" }
///     fn check(&self, _facts: &[SourceFacts]) -> Vec<Finding> { Vec::new() }
/// }
/// assert!(NoFindings.check(&[]).is_empty());
/// ```
pub trait Rule {
    fn id(&self) -> &'static str;
    fn why(&self) -> &'static str;
    fn check(&self, facts: &[SourceFacts]) -> Vec<Finding>;
}

/// Run every rule over the facts; report findings only inside `scope`
/// (a repo-relative path prefix; `None` = whole workspace). Facts are
/// already workspace-wide — the frontier rule (B5).
///
/// ```
/// use core_ai_native_conform::rules::UnsafeGate;
/// use core_ai_native_conform::{Fact, SourceFacts, check};
///
/// let gate = UnsafeGate { audit_crates: vec![] };
/// let facts = vec![SourceFacts {
///     file: "crates/a/src/lib.rs".into(),
///     crate_name: "a".into(),
///     facts: vec![Fact::UnsafeUse {
///         context: "block".into(), line: 5,
///         in_test: false, in_deviation: false, reason: None,
///     }],
/// }];
/// assert_eq!(check(&[&gate], &facts, None).len(), 1);
/// assert!(check(&[&gate], &facts, Some("crates/b/")).is_empty());
/// ```
pub fn check(rules: &[&dyn Rule], facts: &[SourceFacts], scope: Option<&str>) -> Vec<Finding> {
    let mut findings: Vec<Finding> = rules.iter().flat_map(|r| r.check(facts)).collect();
    if let Some(prefix) = scope {
        findings.retain(|f| f.file.starts_with(prefix));
    }
    findings.sort();
    findings
}

/// Group findings per rule for the human one-liner.
///
/// ```
/// use core_ai_native_conform::rules::UnsafeGate;
/// use core_ai_native_conform::{Fact, Rule, SourceFacts, count_by_rule};
///
/// let gate = UnsafeGate { audit_crates: vec![] };
/// let facts = vec![SourceFacts {
///     file: "crates/a/src/lib.rs".into(),
///     crate_name: "a".into(),
///     facts: vec![Fact::UnsafeUse {
///         context: "block".into(), line: 5,
///         in_test: false, in_deviation: false, reason: None,
///     }],
/// }];
/// let counts = count_by_rule(&gate.check(&facts));
/// assert_eq!(counts["unsafe-gate"], 1);
/// ```
pub fn count_by_rule(findings: &[Finding]) -> BTreeMap<&'static str, usize> {
    let mut map = BTreeMap::new();
    for f in findings {
        *map.entry(f.rule).or_insert(0) += 1;
    }
    map
}

#[cfg(test)]
mod tests {
    use crate::rules;
    use crate::{Fact, SourceFacts, check};

    fn sf(file: &str, crate_name: &str, facts: Vec<Fact>) -> SourceFacts {
        SourceFacts {
            file: file.to_string(),
            crate_name: crate_name.to_string(),
            facts,
        }
    }

    #[test]
    fn scope_filters_findings_not_facts() {
        let facts = vec![
            sf(
                "crates/a/src/lib.rs",
                "a",
                vec![Fact::UnsafeUse {
                    context: "block".into(),
                    line: 5,
                    in_test: false,
                    in_deviation: false,
                    reason: None,
                }],
            ),
            sf(
                "crates/b/src/lib.rs",
                "b",
                vec![Fact::UnsafeUse {
                    context: "block".into(),
                    line: 5,
                    in_test: false,
                    in_deviation: false,
                    reason: None,
                }],
            ),
        ];
        let gate = rules::UnsafeGate {
            audit_crates: vec![],
        };
        let all = check(&[&gate], &facts, None);
        assert_eq!(all.len(), 2);
        let scoped = check(&[&gate], &facts, Some("crates/a/"));
        assert_eq!(scoped.len(), 1);
    }
}
