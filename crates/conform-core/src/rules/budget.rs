//! Budget-and-bans rules — the guide's quantitative gates: unsafe
//! stays inside designated audit crates (unsafe-gate), files stay
//! within the line budget (file-length), and unwrap/expect stays out
//! of domain logic (no-unwrap-in-domain).

specmark::scope!("spec://vibevm/discipline/ENGINE-CONFORM-v0.1#rules");

use crate::facts::{Fact, SourceFacts};
use crate::finding::{Finding, Rule};

use super::req_message;

/// unsafe-gate: `unsafe` appears only inside designated audit
/// crates, or under a recorded fn-grain deviation. The posture
/// (AUD-0016, redesigned 2026-06-12): an audit crate owns the
/// unsafety behind a safe API and is exempt wholesale; everywhere
/// else a justified boundary testifies via
/// `#[spec(deviates = …, reason = …)]` on the carrying fn
/// (`in_deviation`, frontend v5) and the rule honors it
/// (ENGINE-CONFORM §4). Test-context unsafe (`in_test`) is
/// deliberately NOT exempt — unsoundness in tests is still
/// unsoundness; tests use the audit crate's safe API instead.
///
/// ```
/// use conform_core::rules::UnsafeGate;
/// use conform_core::{Fact, Rule, SourceFacts};
///
/// let rule = UnsafeGate { audit_crates: &["audited"] };
/// let outside = SourceFacts {
///     file: "crates/a/src/lib.rs".into(),
///     crate_name: "a".into(),
///     facts: vec![
///         Fact::UnsafeUse {
///             context: "block".into(), line: 5,
///             in_test: false, in_deviation: false,
///         },
///         // Testified boundary — honored, not flagged.
///         Fact::UnsafeUse {
///             context: "block".into(), line: 9,
///             in_test: false, in_deviation: true,
///         },
///         // Test context — still gated.
///         Fact::UnsafeUse {
///             context: "block".into(), line: 40,
///             in_test: true, in_deviation: false,
///         },
///     ],
/// };
/// let findings = rule.check(&[outside]);
/// assert_eq!(findings.len(), 2);
/// assert!(conform_core::rules::matches_req_grammar(&findings[0].message));
/// ```
pub struct UnsafeGate {
    pub audit_crates: &'static [&'static str],
}

impl Rule for UnsafeGate {
    fn id(&self) -> &'static str {
        "unsafe-gate"
    }
    fn why(&self) -> &'static str {
        "unsafe is an audit boundary: it lives in designated audit crates \
         or not at all (GUIDE-RUST §8, house rule)"
    }
    fn check(&self, facts: &[SourceFacts]) -> Vec<Finding> {
        let mut out = Vec::new();
        for sf in facts {
            if self.audit_crates.contains(&sf.crate_name.as_str()) {
                continue;
            }
            // Fingerprint by context + per-file ordinal, NOT by
            // line: a line-keyed fingerprint goes stale on any
            // edit above the block (the adopt-v0.3 Phase-0 lesson
            // — the stop.rs 33→35 shift), and a baseline that
            // rots on unrelated edits is a checker that lies.
            let mut seen: std::collections::BTreeMap<String, u32> =
                std::collections::BTreeMap::new();
            for f in &sf.facts {
                if let Fact::UnsafeUse {
                    context,
                    line,
                    in_test: _,
                    in_deviation,
                } = f
                {
                    // The ordinal advances over every unsafe use —
                    // testified or not — so an existing fingerprint
                    // never silently re-keys when a NEIGHBOUR gains
                    // or loses its testimony.
                    let counter = seen.entry(context.clone()).or_insert(0);
                    let ordinal = *counter;
                    *counter += 1;
                    if *in_deviation {
                        continue;
                    }
                    out.push(Finding {
                        rule: self.id(),
                        file: sf.file.clone(),
                        line: *line,
                        message: req_message(
                            "discipline://rust-ai-native/guide#bans-and-escape-hatches",
                            &format!("`unsafe` ({context}) outside a designated audit crate"),
                            "move the unsafe behind an audit crate's safe API, or \
                             record #[spec(deviates = <uri>, reason = …)] on the \
                             carrying fn",
                        ),
                        why: self.why(),
                        fingerprint: format!("unsafe-gate|{}|{context}#{ordinal}", sf.file),
                    });
                }
            }
        }
        out.sort();
        out
    }
}

/// Guide §2 — `file-length`: a source file over the line budget
/// pages badly and buries invariants in its middle third; prefer
/// more, smaller, single-purpose files at equal token mass
/// (R3-003 "position is a resource"). The audit's god-file
/// inventory (2026-06-12-07) is this rule's frozen baseline; the
/// decomposition backlog shrinks it.
///
/// ```
/// use conform_core::rules::FileLength;
/// use conform_core::{Fact, Rule, SourceFacts};
///
/// let rule = FileLength { max_lines: 600 };
/// let big = SourceFacts {
///     file: "crates/x/src/big.rs".into(),
///     crate_name: "x".into(),
///     facts: vec![Fact::FileMetrics { lines: 1200 }],
/// };
/// assert_eq!(rule.check(&[big]).len(), 1);
/// ```
pub struct FileLength {
    pub max_lines: u32,
}

impl Rule for FileLength {
    fn id(&self) -> &'static str {
        "file-length"
    }
    fn why(&self) -> &'static str {
        "position is a resource: past the budget a file pages badly and \
         its middle third buries invariants — prefer more, smaller, \
         single-purpose files (GUIDE-AI-NATIVE-RUST §2, R3-003)"
    }
    fn check(&self, facts: &[SourceFacts]) -> Vec<Finding> {
        let mut out = Vec::new();
        for sf in facts {
            if !sf.file.contains("/src/") {
                continue;
            }
            for f in &sf.facts {
                let Fact::FileMetrics { lines } = f else {
                    continue;
                };
                if *lines <= self.max_lines {
                    continue;
                }
                out.push(Finding {
                    rule: self.id(),
                    file: sf.file.clone(),
                    line: 1,
                    message: req_message(
                        "discipline://rust-ai-native/guide#surface-form",
                        &format!(
                            "{lines} lines exceeds the {}-line file budget",
                            self.max_lines
                        ),
                        "split along the file's responsibility seams into \
                         module-grain cells",
                    ),
                    why: self.why(),
                    fingerprint: format!("file-length|{}", sf.file),
                });
            }
        }
        out.sort();
        out
    }
}

/// Guide §6 — `no-unwrap-in-domain`: `.unwrap()` / `.expect()` in
/// domain logic converts a contract violation into a panic. Call
/// sites inside `#[cfg(test)]` modules and `#[test]` functions are
/// exempt (the facts carry `in_test`); a justified boundary records
/// `#[spec(deviates = …, reason = …)]` on the carrying fn, and the
/// facts see the testimony (`in_deviation`, frontend v4) — the rule
/// honors it instead of freezing the site in the baseline.
///
/// ```
/// use conform_core::rules::NoUnwrapInDomain;
/// use conform_core::{Fact, Rule, SourceFacts};
///
/// let rule = NoUnwrapInDomain { gated_crates: &["x"] };
/// let domain = SourceFacts {
///     file: "crates/x/src/m.rs".into(),
///     crate_name: "x".into(),
///     facts: vec![
///         Fact::UnwrapUse {
///             method: "unwrap".into(), line: 9, in_test: false, in_deviation: false,
///         },
///         Fact::UnwrapUse {
///             method: "unwrap".into(), line: 90, in_test: true, in_deviation: false,
///         },
///         Fact::UnwrapUse {
///             method: "expect".into(), line: 120, in_test: false, in_deviation: true,
///         },
///     ],
/// };
/// assert_eq!(rule.check(&[domain]).len(), 1);
/// ```
pub struct NoUnwrapInDomain {
    pub gated_crates: &'static [&'static str],
}

impl Rule for NoUnwrapInDomain {
    fn id(&self) -> &'static str {
        "no-unwrap-in-domain"
    }
    fn why(&self) -> &'static str {
        "unwrap/expect in domain logic is a panic wearing a contract's \
         clothes: return through the layer's error enum, or mark the \
         justified boundary with #[spec(deviates, reason)] \
         (GUIDE-AI-NATIVE-RUST §6)"
    }
    fn check(&self, facts: &[SourceFacts]) -> Vec<Finding> {
        let mut out = Vec::new();
        for sf in facts {
            if !self.gated_crates.contains(&sf.crate_name.as_str()) {
                continue;
            }
            if !sf.file.contains("/src/") {
                continue;
            }
            // Per-file per-method ordinal fingerprints, never line
            // numbers (the stop.rs 33→35 lesson).
            let mut seen: std::collections::BTreeMap<&str, u32> = std::collections::BTreeMap::new();
            for f in &sf.facts {
                let Fact::UnwrapUse {
                    method,
                    line,
                    in_test,
                    in_deviation,
                } = f
                else {
                    continue;
                };
                if *in_test || *in_deviation {
                    continue;
                }
                let counter = seen.entry(method.as_str()).or_insert(0);
                let ordinal = *counter;
                *counter += 1;
                out.push(Finding {
                    rule: self.id(),
                    file: sf.file.clone(),
                    line: *line,
                    message: req_message(
                        "discipline://rust-ai-native/guide#bans-and-escape-hatches",
                        &format!("`.{method}()` in domain logic"),
                        "return through the layer's error enum, or record \
                         #[spec(deviates = <uri>, reason = …)] on the \
                         carrying fn",
                    ),
                    why: self.why(),
                    fingerprint: format!("no-unwrap-in-domain|{}|{method}#{ordinal}", sf.file),
                });
            }
        }
        out.sort();
        out
    }
}
