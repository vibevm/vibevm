specmark::scope!("spec://vibevm/discipline/ENGINE-CONFORM-v0.1#rules");

use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::finding::Finding;

/// `conform-baseline.json`: frozen pre-existing findings, by
/// fingerprint. The file only shrinks.
///
/// ```
/// use conform_core::baseline::Baseline;
///
/// let frozen = Baseline {
///     schema: 1,
///     findings: vec!["unsafe-gate|crates/x/src/lib.rs|block#0".into()],
/// };
/// assert_eq!(frozen.findings.len(), 1);
/// ```
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Baseline {
    pub schema: u32,
    #[serde(default)]
    pub findings: Vec<String>,
}

/// Load the baseline; an absent file is an empty baseline (the
/// gate is then "no findings allowed at all").
///
/// ```no_run
/// let base = conform_core::baseline::load(
///     std::path::Path::new("conform-baseline.json"),
/// ).unwrap();
/// println!("{} frozen", base.findings.len());
/// ```
pub fn load(path: &Path) -> Result<Baseline> {
    if !path.exists() {
        return Ok(Baseline {
            schema: 1,
            findings: Vec::new(),
        });
    }
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

/// Diff findings against the baseline: `(new, stale)` — new ones
/// fail the gate; stale entries are prune candidates (the file may
/// only shrink, so pruning is the legal direction).
///
/// ```
/// use conform_core::baseline::{Baseline, diff};
///
/// let frozen = Baseline { schema: 1, findings: vec!["gone|x|0".into()] };
/// let (new, stale) = diff(&frozen, &[]);
/// assert!(new.is_empty());
/// assert_eq!(stale, vec![&"gone|x|0".to_string()]);
/// ```
pub fn diff<'a>(
    baseline: &'a Baseline,
    findings: &'a [Finding],
) -> (Vec<&'a Finding>, Vec<&'a String>) {
    let new = findings
        .iter()
        .filter(|f| !baseline.findings.contains(&f.fingerprint))
        .collect();
    let stale = baseline
        .findings
        .iter()
        .filter(|fp| !findings.iter().any(|f| &f.fingerprint == *fp))
        .collect();
    (new, stale)
}

#[cfg(test)]
mod tests {
    use crate::baseline;
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
    fn baseline_diff_news_and_stales() {
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
        let empty = baseline::Baseline {
            schema: 1,
            findings: vec![],
        };
        let (new, stale) = baseline::diff(&empty, &findings);
        assert_eq!(new.len(), 1);
        assert!(stale.is_empty());

        let frozen = baseline::Baseline {
            schema: 1,
            findings: vec![findings[0].fingerprint.clone(), "gone|x|1".into()],
        };
        let (new, stale) = baseline::diff(&frozen, &findings);
        assert!(new.is_empty());
        assert_eq!(stale.len(), 1);
    }
}
