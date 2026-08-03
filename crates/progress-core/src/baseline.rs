//! The inter-campaign baseline and `rescan` (PROP-043 §7.3):
//! the artifact that turns a re-run from O(corpus) into O(delta).
//!
//! Four rules decide a carried verdict's fate: the unit's own text moved
//! (its hash), a **named crate** moved under it (the adapter's
//! crate → last-commit map), the governing marker diverged outside a
//! campaign — and, because the first two are deliberately coarse, a
//! deterministic **control sample** of the survivors is re-verified anyway.
//!
//! The core runs no command and knows no VCS (PROP-043 §2, the separability
//! law): whether a crate moved arrives as data in [`RescanOptions`], probed
//! by whatever adapter knows how this project stores its history.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#baseline");

use crate::doc::{ParsedDoc, Unit};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// The fact → unit projection that fills a baseline, and the writer that
/// puts it on disk. Split into its own file so this one keeps the reader
/// (`rescan`) whole and inside the file-length budget; the two halves
/// share [`governing_marker`] rather than each resolving markers their
/// own way (DRIFT-023 §4.1).
pub mod project;

pub const BASELINE_SCHEMA: u32 = 1;

/// The share of carried-forward units re-verified anyway — PROP-043 §7.3's
/// "small random control sample, because code-side invalidation is
/// deliberately coarse". The contract fixes no rate; 5 % is this
/// implementation's default and the adapter's `--control-rate` overrides it.
pub const DEFAULT_CONTROL_RATE: f64 = 0.05;

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

impl BaselineUnit {
    /// Record one verdict, deriving [`crates`](Self::crates) from the
    /// evidence refs.
    ///
    /// Constructing a unit *through here* is what keeps the named-crate
    /// invalidation rule fed: a hand-built literal that forgets `crates`
    /// leaves the rule unable to fire at all, which is exactly the state
    /// DRIFT-009 §3 found the baseline in.
    ///
    /// ```
    /// use progress_core::baseline::BaselineUnit;
    ///
    /// let u = BaselineUnit::new(
    ///     "spec/x.md#a",
    ///     "hash",
    ///     "confirmed",
    ///     vec!["crates/vibe-core/src/x.rs:1".to_string()],
    ///     "2026-07-25T00:00:00Z",
    ///     None,
    /// );
    /// assert_eq!(u.crates, vec!["vibe-core"]);
    /// ```
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#baseline"
    )]
    pub fn new(
        addr: impl Into<String>,
        unit_hash: impl Into<String>,
        verdict: impl Into<String>,
        evidence: Vec<String>,
        verified_at: impl Into<String>,
        marker: Option<String>,
    ) -> BaselineUnit {
        let crates = crates_from_refs(&evidence);
        BaselineUnit {
            addr: addr.into(),
            unit_hash: unit_hash.into(),
            verdict: verdict.into(),
            evidence,
            verified_at: verified_at.into(),
            crates,
            marker,
        }
    }
}

/// The crate names a unit's evidence refs point into.
///
/// An evidence ref is a free-form provenance string (`crates/<name>/src/x.rs:12`
/// — PROP-043 §6); the crate is the segment right after a `crates/`
/// component, on either separator. A ref that points nowhere near a crate
/// contributes nothing, and a unit with no refs keeps an empty list — the
/// named-crate rule simply does not apply to it. De-duplicated and sorted,
/// so the baseline's JSON is stable across runs.
///
/// ```
/// use progress_core::baseline::crates_from_refs;
///
/// let refs = vec![
///     "crates/vibe-core/src/x.rs:1".to_string(),
///     "crates/vibe-cli/src/y.rs:2".to_string(),
///     "spec/modules/vibe-progress/PROP-043.md#tool".to_string(),
/// ];
/// assert_eq!(crates_from_refs(&refs), vec!["vibe-cli", "vibe-core"]);
/// ```
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#baseline"
)]
pub fn crates_from_refs(refs: &[String]) -> Vec<String> {
    let mut out: BTreeSet<&str> = BTreeSet::new();
    for r in refs {
        let segs: Vec<&str> = r.split(['/', '\\']).collect();
        if let Some(i) = segs.iter().position(|s| *s == "crates")
            && let Some(name) = segs.get(i + 1)
            && !name.is_empty()
        {
            out.insert(name);
        }
    }
    out.into_iter().map(str::to_string).collect()
}

/// What the adapter found out about one crate a baseline unit names.
///
/// The core never asks — it is handed these (PROP-043 §2). In vibevm the
/// adapter runs `git log -1 --format=%cI -- crates/<name>` once per crate;
/// a project that vendors its baseline without any history hands in an
/// empty map and the named-crate rule is skipped wholesale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrateState {
    /// Timestamp of the last commit touching the crate (RFC-3339).
    LastCommit(String),
    /// The crate the baseline names is gone from the tree — its code moved
    /// in the strongest possible sense.
    Gone,
}

/// What `rescan` needs from the adapter beyond the docs and the baseline.
///
/// [`Default`] is the shipped posture: no crate knowledge (rule skipped) and
/// the [`DEFAULT_CONTROL_RATE`] sample.
#[derive(Debug, Clone)]
pub struct RescanOptions {
    /// crate name → what the adapter found. Probed once per crate, never
    /// once per unit; a crate absent from the map is one nothing is known
    /// about, so the rule stays silent for it.
    pub crate_states: BTreeMap<String, CrateState>,
    /// Share of the still-carried-forward units promoted to
    /// [`RescanClass::ControlSample`]. `0.0` disables the sample.
    pub control_rate: f64,
}

impl Default for RescanOptions {
    fn default() -> Self {
        RescanOptions {
            crate_states: BTreeMap::new(),
            control_rate: DEFAULT_CONTROL_RATE,
        }
    }
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

/// The rescan verdict per unit (sources ↔ markers ↔ baseline ↔ code).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RescanClass {
    /// Unit absent from the baseline.
    New,
    /// Suspect, re-verify: the unit's hash differs from the baseline, or a
    /// crate it names moved under it (see [`RescanRow::crate_moved`]).
    Changed,
    /// Nothing moved — verdict carries forward untouched.
    CarriedForward,
    /// Nothing moved either, but the unit was drawn into the control
    /// sample: re-verified like a suspect one, kept a separate class so the
    /// report can say *why* it is being re-checked.
    ControlSample,
}

#[derive(Debug, Clone, Serialize)]
pub struct RescanRow {
    pub addr: String,
    pub class: RescanClass,
    /// The current governing marker diverges from the baseline snapshot
    /// while the unit text did NOT change — "marker edited outside a
    /// campaign" (PROP-043 §7.3 flag).
    pub marker_diverged: bool,
    /// The named crate whose code moved after the verdict — the reason a
    /// unit whose own text is unchanged is nonetheless suspect. `None` on
    /// hash-driven suspicion and on everything that carried forward.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crate_moved: Option<String>,
}

pub fn unit_addr(doc: &ParsedDoc, unit_idx: usize) -> String {
    let u = &doc.units[unit_idx];
    match &u.anchor {
        Some(a) => format!("{}#{}", doc.path, a),
        None => format!("{}#L{}", doc.path, u.line_start),
    }
}

/// The marker governing `unit`, formatted `"{stage}/{state}"`: the first
/// `Section` marker standing inside the unit's body span, and the
/// document marker where there is none.
///
/// One function, called from both sides of the baseline — the projection
/// that snapshots the marker ([`project`]) and the rescan that compares
/// against the snapshot. Two implementations of this rule that agree
/// today drift apart the first time either is touched, and the symptom is
/// silent: every unit reports `marker_diverged` forever while the text it
/// stands for never moved (DRIFT-023 §4.1.5).
///
/// ```
/// use progress_core::baseline::governing_marker;
/// use progress_core::parse::parse_document;
///
/// // (few source lines: a doctest line may not begin with `#`)
/// let doc = parse_document("a.md", "<status stage=\"impl\" state=\"work\"/>\n\n# One {#one}\n\n\
///      <status stage=\"spec\" state=\"done\"/>\n\nBody.\n\n# Two {#two}\n\nBody.\n");
/// // The section marker standing under the first heading governs it …
/// assert_eq!(governing_marker(&doc, &doc.units[0]).as_deref(), Some("spec/done"));
/// // … and a section with none falls back to the document marker.
/// assert_eq!(governing_marker(&doc, &doc.units[1]).as_deref(), Some("impl/work"));
/// ```
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#baseline"
)]
pub fn governing_marker(doc: &ParsedDoc, unit: &Unit) -> Option<String> {
    doc.markers
        .iter()
        .find(|m| {
            m.granularity == crate::model::Granularity::Section
                && m.line >= unit.line_start
                && m.line <= unit.line_end
        })
        .or_else(|| doc.document_marker())
        .map(|m| format!("{}/{}", m.stage, m.state))
}

/// Compare parsed docs against a baseline, applying all four §7.3 rules.
///
/// ```
/// use progress_core::baseline::{rescan, Baseline, RescanOptions};
///
/// let doc = progress_core::parse::parse_document("a.md", "# One {#one}\n\nbody\n");
/// let empty = Baseline::default();
/// let rows = rescan([&doc], &empty, &RescanOptions::default());
/// assert_eq!(rows.len(), 1, "a unit the baseline never saw is `new`");
/// ```
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#baseline"
)]
pub fn rescan<'a>(
    docs: impl IntoIterator<Item = &'a ParsedDoc>,
    baseline: &Baseline,
    opts: &RescanOptions,
) -> Vec<RescanRow> {
    let mut rows = Vec::new();
    for doc in docs {
        for (i, u) in doc.units.iter().enumerate() {
            let addr = unit_addr(doc, i);
            match baseline.units.get(&addr) {
                None => rows.push(RescanRow {
                    addr,
                    class: RescanClass::New,
                    marker_diverged: false,
                    crate_moved: None,
                }),
                Some(b) if b.unit_hash != u.content_hash => rows.push(RescanRow {
                    addr,
                    class: RescanClass::Changed,
                    marker_diverged: false,
                    crate_moved: None,
                }),
                Some(b) => {
                    // Unit text unchanged: did the governing marker move?
                    // Resolved by the same function that snapshotted it,
                    // so the two sides cannot drift apart (§4.1.5).
                    let current = governing_marker(doc, u);
                    let diverged = match (&b.marker, &current) {
                        (Some(snap), Some(cur)) => snap != cur,
                        _ => false,
                    };
                    // The unit's own text held still — but did the code it
                    // names move out from under the verdict?
                    let moved = moved_crate(b, &opts.crate_states);
                    rows.push(RescanRow {
                        addr,
                        class: match moved {
                            Some(_) => RescanClass::Changed,
                            None => RescanClass::CarriedForward,
                        },
                        marker_diverged: diverged,
                        crate_moved: moved,
                    });
                }
            }
        }
    }
    draw_control_sample(&mut rows, &baseline.campaign_id, opts.control_rate);
    rows
}

/// The first crate this unit names that carries work newer than the verdict
/// — PROP-043 §7.3's "named crate has commits after the verdict date".
///
/// Everything unanswerable answers `None`: a crate the adapter said nothing
/// about, a timestamp on either side that is not RFC-3339, a unit naming no
/// crates at all. The rule is skipped, never failed — a consuming project
/// may vendor a baseline into a tree with no history to ask.
fn moved_crate(unit: &BaselineUnit, states: &BTreeMap<String, CrateState>) -> Option<String> {
    let verified = chrono::DateTime::parse_from_rfc3339(&unit.verified_at).ok();
    for name in &unit.crates {
        match states.get(name) {
            Some(CrateState::Gone) => return Some(name.clone()),
            Some(CrateState::LastCommit(iso)) => {
                let (Some(v), Ok(c)) = (verified, chrono::DateTime::parse_from_rfc3339(iso)) else {
                    continue;
                };
                if c > v {
                    return Some(name.clone());
                }
            }
            None => continue,
        }
    }
    None
}

/// Promote a `rate` share of the still-carried-forward rows to
/// [`RescanClass::ControlSample`] — the §7.3 sample that keeps a coarse
/// invalidation honest.
///
/// The draw is seeded from the baseline's own content — `sha256(campaign_id
/// \0 addr)` — never from a clock or an RNG, so two rescans of the same
/// baseline pick the same units and a reviewer can re-derive the pick by
/// hand. Rounding up is deliberate: any non-empty pool yields at least one
/// unit, so the sample cannot silently switch itself off on a small corpus.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#baseline"
)]
fn draw_control_sample(rows: &mut [RescanRow], campaign_id: &str, rate: f64) {
    if rate <= 0.0 {
        return;
    }
    let mut pool: Vec<(u64, usize)> = rows
        .iter()
        .enumerate()
        .filter(|(_, r)| r.class == RescanClass::CarriedForward)
        .map(|(i, r)| (ticket(campaign_id, &r.addr), i))
        .collect();
    if pool.is_empty() {
        return;
    }
    let take = ((pool.len() as f64) * rate).ceil() as usize;
    pool.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| rows[a.1].addr.cmp(&rows[b.1].addr))
    });
    // `take` saturates at the pool's size, so a rate of 1.0 draws all of it
    // and nothing above the pool can ever be reached.
    for (_, i) in pool.into_iter().take(take) {
        rows[i].class = RescanClass::ControlSample;
    }
}

/// One unit's draw ticket: the leading 8 bytes of `sha256(campaign_id \0
/// addr)`, big-endian. Pure content — same baseline, same tickets, forever.
fn ticket(campaign_id: &str, addr: &str) -> u64 {
    use sha2::{Digest as _, Sha256};
    let mut h = Sha256::new();
    h.update(campaign_id.as_bytes());
    h.update([0u8]);
    h.update(addr.as_bytes());
    let digest = h.finalize();
    let mut head = [0u8; 8];
    head.copy_from_slice(&digest[..8]);
    u64::from_be_bytes(head)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::now_utc;
    use crate::doc::ParsedDoc;
    use crate::parse::parse_document;

    /// A baseline carrying every unit of `doc` at `verified_at`, each with
    /// the same evidence refs (so `crates` is derived, not hand-written).
    fn baseline_of(doc: &ParsedDoc, verified_at: &str, evidence: &[&str]) -> Baseline {
        let mut b = Baseline {
            schema: BASELINE_SCHEMA,
            written_at: now_utc(),
            campaign_id: "t".into(),
            units: BTreeMap::new(),
        };
        for (i, u) in doc.units.iter().enumerate() {
            let addr = unit_addr(doc, i);
            b.units.insert(
                addr.clone(),
                BaselineUnit::new(
                    addr,
                    u.content_hash.clone(),
                    "confirmed",
                    evidence.iter().map(|e| (*e).to_string()).collect(),
                    verified_at,
                    None,
                ),
            );
        }
        b
    }

    /// Neither code knowledge nor a sample — the lens for asserting on the
    /// hash rule alone.
    fn plain() -> RescanOptions {
        RescanOptions {
            crate_states: BTreeMap::new(),
            control_rate: 0.0,
        }
    }

    fn class_of(rows: &[RescanRow], addr: &str) -> RescanClass {
        rows.iter()
            .find(|r| r.addr == addr)
            .map(|r| r.class.clone())
            .expect("row")
    }

    /// A doc of `n` trivially-different sections — a corpus big enough that
    /// a 5 % sample is a real draw rather than "the only unit there is".
    fn wide_doc(n: usize) -> ParsedDoc {
        let mut src = String::new();
        for i in 0..n {
            src.push_str(&format!("# S{i} {{#s{i}}}\n\nbody {i}\n\n"));
        }
        parse_document("a.md", &src)
    }

    #[test]
    fn rescan_classifies_new_changed_carried() {
        let v1 = parse_document("a.md", "# One {#one}\n\nbody v1\n\n# Two {#two}\n\nbody\n");
        let baseline = baseline_of(&v1, &now_utc(), &[]);
        // v2: unit one edited, unit three added.
        let v2 = parse_document(
            "a.md",
            "# One {#one}\n\nbody v2\n\n# Two {#two}\n\nbody\n\n# Three {#three}\n\nnew\n",
        );
        let rows = rescan([&v2], &baseline, &plain());
        assert_eq!(class_of(&rows, "a.md#one"), RescanClass::Changed);
        assert_eq!(class_of(&rows, "a.md#two"), RescanClass::CarriedForward);
        assert_eq!(class_of(&rows, "a.md#three"), RescanClass::New);
    }

    #[test]
    fn crates_derived_from_refs() {
        let doc = parse_document("a.md", "# One {#one}\n\nbody\n");
        let b = baseline_of(
            &doc,
            "2026-07-25T00:00:00Z",
            &["crates/vibe-core/src/x.rs:1", "crates/vibe-cli/src/y.rs:2"],
        );
        let unit = b.units.get("a.md#one").expect("unit");
        assert_eq!(unit.crates, vec!["vibe-cli", "vibe-core"]);
        // Refs that point outside `crates/` add nothing, and a unit with no
        // refs keeps an empty list — the rule just does not apply to it.
        assert!(crates_from_refs(&["spec/modules/x.md#a".to_string()]).is_empty());
        assert!(crates_from_refs(&[]).is_empty());
    }

    #[test]
    fn named_crate_commit_makes_suspect() {
        let doc = parse_document("a.md", "# One {#one}\n\nbody\n");
        let b = baseline_of(
            &doc,
            "2026-07-25T00:00:00Z",
            &["crates/vibe-core/src/x.rs:1"],
        );
        let with = |state: CrateState| RescanOptions {
            crate_states: [("vibe-core".to_string(), state)].into_iter().collect(),
            control_rate: 0.0,
        };

        // T+1: the crate moved after the verdict ⇒ suspect, and the row
        // names which crate did it.
        let rows = rescan(
            [&doc],
            &b,
            &with(CrateState::LastCommit("2026-07-26T00:00:00Z".into())),
        );
        assert_eq!(class_of(&rows, "a.md#one"), RescanClass::Changed);
        assert_eq!(rows[0].crate_moved.as_deref(), Some("vibe-core"));

        // T−1: the crate's last commit predates the verdict ⇒ carry on.
        let rows = rescan(
            [&doc],
            &b,
            &with(CrateState::LastCommit("2026-07-24T00:00:00Z".into())),
        );
        assert_eq!(class_of(&rows, "a.md#one"), RescanClass::CarriedForward);
        assert!(rows[0].crate_moved.is_none());

        // Instants, not strings: `02:00+03:00` sorts *after* `00:00Z`
        // lexically but happens an hour *before* it.
        let rows = rescan(
            [&doc],
            &b,
            &with(CrateState::LastCommit("2026-07-25T02:00:00+03:00".into())),
        );
        assert_eq!(class_of(&rows, "a.md#one"), RescanClass::CarriedForward);

        // A crate that left the tree is the strongest possible move.
        let rows = rescan([&doc], &b, &with(CrateState::Gone));
        assert_eq!(class_of(&rows, "a.md#one"), RescanClass::Changed);
    }

    #[test]
    fn control_sample_is_deterministic() {
        let doc = wide_doc(20);
        let b = baseline_of(&doc, "2026-07-25T00:00:00Z", &[]);
        let sampled = |rate: f64| -> Vec<String> {
            let opts = RescanOptions {
                crate_states: BTreeMap::new(),
                control_rate: rate,
            };
            rescan([&doc], &b, &opts)
                .into_iter()
                .filter(|r| r.class == RescanClass::ControlSample)
                .map(|r| r.addr)
                .collect()
        };

        // Two runs over the same baseline pick the same units.
        let first = sampled(DEFAULT_CONTROL_RATE);
        assert_eq!(first.len(), 1, "5 % of 20 units, rounded up");
        assert_eq!(first, sampled(DEFAULT_CONTROL_RATE));

        // Rate 0 disables the sample outright.
        assert!(sampled(0.0).is_empty());

        // One carried-forward unit at 5 % still gets drawn — rounding up is
        // what keeps a small corpus sampled at all.
        let one = parse_document("a.md", "# One {#one}\n\nbody\n");
        let b1 = baseline_of(&one, "2026-07-25T00:00:00Z", &[]);
        let rows = rescan(
            [&one],
            &b1,
            &RescanOptions {
                crate_states: BTreeMap::new(),
                control_rate: DEFAULT_CONTROL_RATE,
            },
        );
        assert_eq!(class_of(&rows, "a.md#one"), RescanClass::ControlSample);
    }

    #[test]
    fn missing_git_skips_rule_without_failing() {
        let doc = wide_doc(3);
        let b = baseline_of(
            &doc,
            "2026-07-25T00:00:00Z",
            &["crates/vibe-core/src/x.rs:1"],
        );
        // The adapter could not ask anything about any crate — an empty map.
        // Everything the hash carried stays carried; nothing errors.
        let rows = rescan([&doc], &b, &plain());
        assert_eq!(rows.len(), 3);
        assert!(
            rows.iter().all(|r| r.class == RescanClass::CarriedForward),
            "no crate knowledge ⇒ the named-crate rule is simply skipped"
        );
        assert!(rows.iter().all(|r| r.crate_moved.is_none()));
    }
}
