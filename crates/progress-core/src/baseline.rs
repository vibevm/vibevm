//! The inter-campaign baseline and `rescan` (PROP-043 §7.3):
//! the artifact that turns a re-run from O(corpus) into O(delta).

specmark::scope!("spec://vibevm/modules/vibe-progress/PROP-043#baseline");

use crate::doc::ParsedDoc;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

pub const BASELINE_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineUnit {
    /// `path#anchor` (or `path#L<line>` for anchor-less units).
    pub addr: String,
    pub unit_hash: String,
    /// confirmed | drift-fixed | unverifiable | …
    pub verdict: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<String>,
    pub verified_at: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub crates: Vec<String>,
    /// Snapshot of the governing marker at verdict time ("stage/state").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marker: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Baseline {
    pub schema: u32,
    pub written_at: String,
    pub campaign_id: String,
    pub units: BTreeMap<String, BaselineUnit>,
}

impl Baseline {
    pub fn load(path: &Path) -> Result<Baseline> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading baseline {}", path.display()))?;
        serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }
}

/// The three-way rescan verdict per unit (sources ↔ markers ↔ baseline).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RescanClass {
    /// Unit absent from the baseline.
    New,
    /// Unit hash differs from the baseline — verdict suspect, re-verify.
    Changed,
    /// Hash matches — verdict carries forward.
    CarriedForward,
}

#[derive(Debug, Clone, Serialize)]
pub struct RescanRow {
    pub addr: String,
    pub class: RescanClass,
    /// The current governing marker diverges from the baseline snapshot
    /// while the unit text did NOT change — "marker edited outside a
    /// campaign" (PROP-043 §7.3 flag).
    pub marker_diverged: bool,
}

pub fn unit_addr(doc: &ParsedDoc, unit_idx: usize) -> String {
    let u = &doc.units[unit_idx];
    match &u.anchor {
        Some(a) => format!("{}#{}", doc.path, a),
        None => format!("{}#L{}", doc.path, u.line_start),
    }
}

/// Compare parsed docs against a baseline.
pub fn rescan<'a>(
    docs: impl IntoIterator<Item = &'a ParsedDoc>,
    baseline: &Baseline,
) -> Vec<RescanRow> {
    let mut rows = Vec::new();
    for doc in docs {
        let doc_marker = doc
            .document_marker()
            .map(|m| format!("{}/{}", m.stage, m.state));
        for (i, u) in doc.units.iter().enumerate() {
            let addr = unit_addr(doc, i);
            match baseline.units.get(&addr) {
                None => rows.push(RescanRow {
                    addr,
                    class: RescanClass::New,
                    marker_diverged: false,
                }),
                Some(b) if b.unit_hash != u.content_hash => rows.push(RescanRow {
                    addr,
                    class: RescanClass::Changed,
                    marker_diverged: false,
                }),
                Some(b) => {
                    // Unit text unchanged: did the governing marker move?
                    // Section markers are attached by line ranges; the
                    // document marker is the coarse fallback snapshot.
                    let current = doc
                        .markers
                        .iter()
                        .filter(|m| {
                            m.granularity == crate::model::Granularity::Section
                                && m.line >= u.line_start
                                && m.line <= u.line_end
                        })
                        .map(|m| format!("{}/{}", m.stage, m.state))
                        .next()
                        .or_else(|| doc_marker.clone());
                    let diverged = match (&b.marker, &current) {
                        (Some(snap), Some(cur)) => snap != cur,
                        _ => false,
                    };
                    rows.push(RescanRow {
                        addr,
                        class: RescanClass::CarriedForward,
                        marker_diverged: diverged,
                    });
                }
            }
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::now_utc;
    use crate::parse::parse_document;

    #[test]
    fn rescan_classifies_new_changed_carried() {
        let v1 = parse_document("a.md", "# One {#one}\n\nbody v1\n\n# Two {#two}\n\nbody\n");
        let mut baseline = Baseline {
            schema: BASELINE_SCHEMA,
            written_at: now_utc(),
            campaign_id: "t".into(),
            units: BTreeMap::new(),
        };
        for (i, u) in v1.units.iter().enumerate() {
            baseline.units.insert(
                unit_addr(&v1, i),
                BaselineUnit {
                    addr: unit_addr(&v1, i),
                    unit_hash: u.content_hash.clone(),
                    verdict: "confirmed".into(),
                    evidence: vec![],
                    verified_at: now_utc(),
                    crates: vec![],
                    marker: None,
                },
            );
        }
        // v2: unit one edited, unit three added.
        let v2 = parse_document(
            "a.md",
            "# One {#one}\n\nbody v2\n\n# Two {#two}\n\nbody\n\n# Three {#three}\n\nnew\n",
        );
        let rows = rescan([&v2], &baseline);
        let class_of = |addr: &str| {
            rows.iter()
                .find(|r| r.addr == addr)
                .map(|r| r.class.clone())
                .expect("row")
        };
        assert_eq!(class_of("a.md#one"), RescanClass::Changed);
        assert_eq!(class_of("a.md#two"), RescanClass::CarriedForward);
        assert_eq!(class_of("a.md#three"), RescanClass::New);
    }
}
