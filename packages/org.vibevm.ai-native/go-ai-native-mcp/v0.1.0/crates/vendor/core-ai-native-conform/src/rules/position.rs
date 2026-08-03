//! The position rule family — the guide §2 "position is a resource"
//! lens applied to invariant-bearing comments: `invariant-comment-position`
//! fires when a comment that carries an invariant marker is buried in a
//! file's middle third, where a reader skimming the edges pages past it.

specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules");

use crate::facts::{Fact, SourceFacts};
use crate::finding::{Finding, Rule};

use super::req_message;

/// Guide §2 — `invariant-comment-position`: a comment carrying an
/// invariant marker (`SAFETY:` / `INVARIANT:` / `PANICS` / `WARNING:` /
/// `MUST` / `NEVER`, …) that lands in the MIDDLE THIRD of a file long
/// enough for thirds to mean anything is buried where a reader skimming
/// the edges pages past it. Critical invariants belong at a file's
/// edges — its top or bottom — where they survive a skim; the remedy is
/// the guide's own: move the comment to the top or bottom, or split the
/// file (R3-003 "position is a resource"). Test-context comments
/// (`in_test`) are out of scope, and files below [`Self::min_lines`] are
/// skipped — a third is meaningless on a short file.
///
/// ```
/// use core_ai_native_conform::rules::InvariantCommentPosition;
/// use core_ai_native_conform::{Fact, Rule, SourceFacts};
///
/// let rule = InvariantCommentPosition {
///     markers: vec!["INVARIANT:".into()],
///     min_lines: 120,
/// };
/// // A 120-line file: the middle third is lines 41..=80, so a marker on
/// // line 60 is buried; one on line 5 (top third) is not.
/// let buried = SourceFacts {
///     file: "crates/x/src/m.rs".into(),
///     crate_name: "x".into(),
///     facts: vec![
///         Fact::FileMetrics { lines: 120 },
///         Fact::InvariantComment { marker: "INVARIANT:".into(), line: 60, in_test: false },
///         Fact::InvariantComment { marker: "INVARIANT:".into(), line: 5, in_test: false },
///     ],
/// };
/// let findings = rule.check(&[buried]);
/// assert_eq!(findings.len(), 1);
/// assert!(core_ai_native_conform::rules::matches_req_grammar(&findings[0].message));
/// // A short file is skipped entirely — thirds are meaningless below the floor.
/// let short = SourceFacts {
///     file: "crates/x/src/s.rs".into(),
///     crate_name: "x".into(),
///     facts: vec![
///         Fact::FileMetrics { lines: 40 },
///         Fact::InvariantComment { marker: "INVARIANT:".into(), line: 20, in_test: false },
///     ],
/// };
/// assert!(rule.check(&[short]).is_empty());
/// // A test-context marker never fires, even mid-file.
/// let tested = SourceFacts {
///     file: "crates/x/src/t.rs".into(),
///     crate_name: "x".into(),
///     facts: vec![
///         Fact::FileMetrics { lines: 120 },
///         Fact::InvariantComment { marker: "INVARIANT:".into(), line: 60, in_test: true },
///     ],
/// };
/// assert!(rule.check(&[tested]).is_empty());
/// ```
pub struct InvariantCommentPosition {
    /// The invariant-marker vocabulary; only comments whose (normalized)
    /// marker is in this list can be a finding. The rule is the authority
    /// — it re-checks the vocabulary rather than trusting the extractor,
    /// so a marker dropped from the config (or a stale cached fact) does
    /// not red a frozen baseline.
    pub markers: Vec<String>,
    /// The minimum file length below which «thirds» are meaningless — a
    /// short file has no buried middle. Defaults to 120 lines.
    pub min_lines: u32,
}

impl Rule for InvariantCommentPosition {
    fn id(&self) -> &'static str {
        "invariant-comment-position"
    }
    fn why(&self) -> &'static str {
        "position is a resource: an invariant marker in a file's middle \
         third is buried where a reader pages past it — lift it to the \
         file's top or bottom, or split the file (GUIDE-AI-NATIVE-RUST \
         §2, R3-003)"
    }
    fn check(&self, facts: &[SourceFacts]) -> Vec<Finding> {
        let mut out = Vec::new();
        for sf in facts {
            if !super::in_src(&sf.file) {
                continue;
            }
            // The denominator: one FileMetrics per file (every frontend
            // emits it first). No metric, or a file below the floor,
            // makes «thirds» meaningless — skip the whole file.
            let Some(lines) = sf.facts.iter().find_map(|f| match f {
                Fact::FileMetrics { lines } => Some(*lines),
                _ => None,
            }) else {
                continue;
            };
            if lines < self.min_lines {
                continue;
            }
            // The middle third, integer-divided: line > lines/3 and
            // line <= 2*lines/3. For a 120-line file that is 41..=80 —
            // a clean, symmetric third; the single-line remainder of an
            // uneven split falls to the bottom third, the cheaper edge
            // to grow into.
            let lower = lines / 3;
            let upper = 2 * lines / 3;
            // Per-file per-marker ordinal fingerprints, never line
            // numbers: a line-keyed fingerprint rots on any edit above
            // the comment (the stop.rs 33→35 lesson — the adopt-v0.3
            // Phase-0 shift), and a baseline that rots on unrelated
            // edits is a checker that lies. The ordinal advances over
            // EVERY invariant comment of this marker, before any filter,
            // so a neighbour gaining or losing test context — or sliding
            // in or out of the middle third — never re-keys a frozen
            // entry (same posture as UnsafeGate / NoUnwrapInDomain).
            let mut seen: std::collections::BTreeMap<&str, u32> = std::collections::BTreeMap::new();
            for f in &sf.facts {
                let Fact::InvariantComment {
                    marker,
                    line,
                    in_test,
                } = f
                else {
                    continue;
                };
                let counter = seen.entry(marker.as_str()).or_insert(0);
                let ordinal = *counter;
                *counter += 1;
                if *in_test {
                    continue;
                }
                if !self.markers.contains(marker) {
                    continue;
                }
                if *line <= lower || *line > upper {
                    continue;
                }
                out.push(Finding {
                    rule: self.id(),
                    file: sf.file.clone(),
                    line: *line,
                    message: req_message(
                        "discipline://rust-ai-native-lang/guide#surface-form",
                        &format!(
                            "invariant marker `{marker}` at line {line} sits in the \
                             file's middle third"
                        ),
                        "move the comment to the file's top or bottom, or split the \
                         file along its responsibility seams",
                    ),
                    why: self.why(),
                    fingerprint: format!(
                        "invariant-comment-position|{}|{marker}#{ordinal}",
                        sf.file
                    ),
                });
            }
        }
        out.sort();
        out
    }
}
