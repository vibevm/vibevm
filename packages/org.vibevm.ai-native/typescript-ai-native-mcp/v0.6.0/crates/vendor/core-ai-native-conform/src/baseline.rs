specmark::scope!("spec://org.vibevm.ai-native/core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules");

use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::finding::{Finding, FindingStatus};

/// `conform-baseline.json`: frozen pre-existing findings, by
/// fingerprint. The file only shrinks.
///
/// ```
/// use core_ai_native_conform::baseline::Baseline;
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
/// let base = core_ai_native_conform::baseline::load(
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
/// B-025 (mark, don't suppress): a `DeviationAcknowledged` finding
/// NEVER enters `new` — it is visible in the IR/SARIF but cannot fail
/// the gate, so it is gate-inert. This is the ONE place the gate's
/// pass/fail sees the status: the three drivers all read `new` from
/// here, so excluding acknowledged once (here) keeps them all honest
/// without a per-driver edit. A leftover acknowledged entry already IN
/// the baseline is NOT reported `stale`: `stale` means "the site is
/// gone from the tree," and an acknowledged finding is still present
/// (it changed status, it did not disappear) — so the entry simply
/// becomes inert rather than crying a false "prune me."
///
/// ```
/// use core_ai_native_conform::baseline::{Baseline, diff};
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
        // B-025: an acknowledged deviation is gate-inert — never `new`.
        .filter(|f| !matches!(f.status, FindingStatus::DeviationAcknowledged { .. }))
        .filter(|f| !baseline.findings.contains(&f.fingerprint))
        .collect();
    let stale = baseline
        .findings
        .iter()
        .filter(|fp| !findings.iter().any(|f| &f.fingerprint == *fp))
        .collect();
    (new, stale)
}

/// The fingerprints a `freeze` should write: every LIVE finding's
/// identity, sorted and de-duplicated. An acknowledged deviation is
/// never gateable (it never reaches `new`), so freezing it would grow
/// the baseline with a fingerprint that protects nothing — the file
/// must not grow with acknowledged prints (B-025). Used by every
/// driver's `run_freeze` so the exclusion lives in one place, exactly
/// as the gate's exclusion lives in [`diff`].
pub fn freezeable(findings: &[Finding]) -> Vec<&str> {
    let mut fps: Vec<&str> = findings
        .iter()
        .filter(|f| !matches!(f.status, FindingStatus::DeviationAcknowledged { .. }))
        .map(|f| f.fingerprint.as_str())
        .collect();
    fps.sort_unstable();
    fps.dedup();
    fps
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

    /// B-025: an acknowledged deviation is gate-inert. It never enters
    /// `new` (so it cannot fail the gate), a leftover entry matching it
    /// is NOT `stale` (the site is still present — it changed status, it
    /// did not vanish), and `freezeable` never writes its fingerprint.
    #[test]
    fn acknowledged_findings_are_gate_inert() {
        use crate::FindingStatus;
        let gate = rules::UnsafeGate {
            audit_crates: vec![],
        };
        // Two unsafe uses in one file: one Live, one acknowledged
        // (in_deviation). The rule stamps both (B-025).
        let facts = vec![sf(
            "crates/a/src/lib.rs",
            "a",
            vec![
                Fact::UnsafeUse {
                    context: "block".into(),
                    line: 5,
                    in_test: false,
                    in_deviation: false,
                },
                Fact::UnsafeUse {
                    context: "block".into(),
                    line: 9,
                    in_test: false,
                    in_deviation: true,
                },
            ],
        )];
        let findings = check(&[&gate], &facts, None);
        assert_eq!(findings.len(), 2);
        let ack = findings
            .iter()
            .find(|f| matches!(f.status, FindingStatus::DeviationAcknowledged { .. }))
            .expect("the in_deviation use is stamped acknowledged");
        let ack_fp = ack.fingerprint.clone();

        // Empty baseline: the acknowledged finding never enters `new`.
        let empty = baseline::Baseline {
            schema: 1,
            findings: vec![],
        };
        let (new, stale) = baseline::diff(&empty, &findings);
        assert_eq!(new.len(), 1, "only the Live finding is new");
        assert!(new[0].fingerprint != ack_fp);
        assert!(stale.is_empty());

        // A leftover acknowledged entry already in the baseline is NOT
        // stale — the finding is still present (as acknowledged), so the
        // entry is inert, not a prune candidate. (The Live finding is
        // still `new` here — it is not baselined — which is correct and
        // separate from the acknowledged one's inertness.)
        let leftover = baseline::Baseline {
            schema: 1,
            findings: vec![ack_fp.clone()],
        };
        let (new, stale) = baseline::diff(&leftover, &findings);
        assert!(
            new.iter().all(|f| f.fingerprint != ack_fp),
            "an acknowledged finding never enters new"
        );
        assert!(
            !stale.contains(&&ack_fp),
            "a leftover acknowledged entry is inert, not stale"
        );

        // freezeable excludes the acknowledged fingerprint entirely.
        let frozen = baseline::freezeable(&findings);
        assert!(!frozen.contains(&ack_fp.as_str()));
        assert_eq!(frozen.len(), 1, "only the Live fingerprint is freezeable");
    }
}
