specmark::scope!("spec://discipline-core/mechanisms/ENGINE-CONFORM-v0.1#rules");

use std::collections::BTreeMap;

use crate::facts::SourceFacts;

/// One finding with its A1 chain.
///
/// ```
/// use conform_core::Finding;
///
/// let f = Finding {
///     rule: "unsafe-gate",
///     file: "crates/x/src/lib.rs".into(),
///     line: 5,
///     message: conform_core::rules::req_message(
///         "discipline://rust-ai-native/guide#bans-and-escape-hatches",
///         "`unsafe` (block) outside a designated audit crate",
///         "move it or record the deviation",
///     ),
///     why: "unsafe is an audit boundary",
///     fingerprint: "unsafe-gate|crates/x/src/lib.rs|block#0".into(),
/// };
/// assert!(conform_core::rules::matches_req_grammar(&f.message));
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
}

/// A rule is a compiled query over facts (ENGINE-CONFORM §4).
///
/// The canonical implementation shape — pure query in, findings out:
///
/// ```
/// use conform_core::{Finding, Rule, SourceFacts};
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
/// use conform_core::rules::UnsafeGate;
/// use conform_core::{Fact, SourceFacts, check};
///
/// let gate = UnsafeGate { audit_crates: vec![] };
/// let facts = vec![SourceFacts {
///     file: "crates/a/src/lib.rs".into(),
///     crate_name: "a".into(),
///     facts: vec![Fact::UnsafeUse {
///         context: "block".into(), line: 5,
///         in_test: false, in_deviation: false,
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
/// use conform_core::rules::UnsafeGate;
/// use conform_core::{Fact, Rule, SourceFacts, count_by_rule};
///
/// let gate = UnsafeGate { audit_crates: vec![] };
/// let facts = vec![SourceFacts {
///     file: "crates/a/src/lib.rs".into(),
///     crate_name: "a".into(),
///     facts: vec![Fact::UnsafeUse {
///         context: "block".into(), line: 5,
///         in_test: false, in_deviation: false,
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
