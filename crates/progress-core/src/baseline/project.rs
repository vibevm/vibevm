//! The fact → unit projection, and the writer it feeds (PROP-043 §7.3).
//!
//! The campaign's knowledge is per **fact anchor**: `run/cache.json` keys
//! its verdicts by `##FACT-ID`. The baseline contract is per **unit** —
//! §7.3 fixes that record and [`rescan`](super::rescan) reads it that way.
//! This module is the one place the two grains meet: every judged fact
//! rolls up into every unit whose body span carries it, the worst verdict
//! wins, and a unit no judged fact reaches is left out rather than
//! invented.
//!
//! Both halves of that shape follow from one property and point the same
//! way. A unit's identity is the hash of its **whole** body span, nested
//! subsections included (the PROP-035 §5 rule `parse::units` implements),
//! so a verdict carried on that hash has to answer for every fact inside
//! the span — not only for the prose between this heading and the next
//! one. And where no fact answers for it, the unit is absent: an absent
//! unit reads as `new` next time and costs one re-verification, while a
//! fabricated verdict carries forward a judgment nobody made. The
//! artifact fails toward re-verifying, never toward false confidence.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#baseline");

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::Result;

use super::{BASELINE_SCHEMA, Baseline, BaselineUnit, governing_marker, unit_addr};
use crate::cache::{Cache, FileRecord, now_utc, write_if_changed};
use crate::doc::ParsedDoc;

impl Baseline {
    /// Write the baseline back, and say whether that was necessary.
    ///
    /// The writer half of [`Baseline::load`](super::Baseline::load),
    /// shaped like [`Cache::store`](crate::cache::Cache::store) because it
    /// carries the same two obligations: the write is atomic (tmp, fsync,
    /// rename), and a run that has nothing new to say leaves the file
    /// alone — `false` is "the bytes on disk already said this"
    /// (DRIFT-017 §4.1). Only the `written_at` stamp is allowed to differ
    /// for that answer, which is what makes two runs over an unchanged
    /// tree leave a clean `git diff` rather than a fresh timestamp.
    pub fn store(&self, path: &Path) -> Result<bool> {
        let body = serde_json::to_string_pretty(self)?;
        write_if_changed(path, &body)
    }
}

/// What one projection produced, and everything about how it got there
/// that an operator has to be told rather than left to assume.
///
/// The counts are the point: a baseline is read next month by whoever
/// re-runs the campaign, and "918 units written, 2 omitted" is the
/// difference between a corpus that carries forward and one that quietly
/// re-verifies two sections nobody can name.
#[derive(Debug, Default)]
pub struct Projection {
    /// The artifact — ready to [`store`](Baseline::store).
    pub baseline: Baseline,
    /// Addresses of units no judged fact reached, in document order.
    /// Deliberately absent from `baseline.units` (§4.1).
    pub omitted: Vec<String>,
    /// verdict → how many written units carry it.
    pub verdicts: BTreeMap<String, usize>,
    /// `path#key` of every verdict the projection could not attach to a
    /// fact: a key naming no `##<ID>` anchor in the document that filed it
    /// (the campaign's per-file `_elements` bundles are the ordinary
    /// case), or an entry carrying no verdict string. Either way there is
    /// no judgment here to carry, and the facts stay unjudged.
    pub unresolved: Vec<String>,
    /// Addresses two units of the same file both claim — a duplicate
    /// heading anchor. One of them would silently overwrite the other in
    /// the map, so the collision is surfaced instead of absorbed.
    pub collisions: Vec<String>,
    /// Files carrying verdicts but no `verified_at` to date them by.
    /// Their units are omitted: an undated verdict cannot be compared
    /// against the code that moved under it, and a baseline that carries
    /// one is claiming knowledge it cannot defend.
    pub undated: Vec<String>,
    /// Files whose text moved after their verdicts were formed — the
    /// campaign map's own `processed_hash` disagrees with the document
    /// this run parsed.
    ///
    /// Their units are still projected, against the hash §4.1.2 fixes:
    /// the one `rescan` compares, which is the current one. But the
    /// verdict riding on it was formed against text that has since been
    /// edited — a Phase E drift fix, say — so a close-out that ships this
    /// carries those units forward on a judgment made about something
    /// else. The projection reports it rather than deciding for the
    /// campaign: the fix is to re-verify those files (or refresh their
    /// maps) before the baseline is sealed.
    pub stale: Vec<String>,
}

/// Project the campaign's fact-grain verdicts onto the baseline's
/// unit-grain record (DRIFT-023 §4.1).
///
/// Reads the cache and nothing else — no re-verification happens here and
/// no verdict is invented; a fact the campaign never judged contributes
/// nothing, and a unit reached by no judged fact is reported rather than
/// filled in.
///
/// ```
/// use progress_core::baseline::project::project;
/// use progress_core::cache::Cache;
/// use progress_core::parse::parse_document;
///
/// let doc = parse_document("a.md", "# One {#one}\n\n##a1 A claim. @impl/done\n");
/// let mut cache = Cache::default();
/// cache.upsert(&doc, &progress_core::rollup::rollup_doc(&doc));
/// let campaign = &mut cache.files.get_mut("a.md").expect("record").campaign;
/// campaign.insert("verified_at".into(), serde_json::json!("2026-07-25T00:00:00Z"));
/// campaign.insert(
///     "verdicts".into(),
///     serde_json::json!({"a1": {"v": "confirmed", "ev": ["crates/vibe-cli/src/x.rs:1"]}}),
/// );
///
/// let p = project([&doc], &cache, "progress-2026-08");
/// let unit = p.baseline.units.get("a.md#one").expect("the unit carries the fact");
/// assert_eq!(unit.verdict, "confirmed");
/// assert_eq!(unit.crates, vec!["vibe-cli"], "derived from the evidence refs");
/// assert!(p.omitted.is_empty());
/// ```
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#baseline"
)]
pub fn project<'a>(
    docs: impl IntoIterator<Item = &'a ParsedDoc>,
    cache: &Cache,
    campaign_id: &str,
) -> Projection {
    let mut out = Projection {
        baseline: Baseline {
            schema: BASELINE_SCHEMA,
            written_at: now_utc(),
            campaign_id: campaign_id.to_string(),
            units: BTreeMap::new(),
        },
        ..Projection::default()
    };
    for doc in docs {
        let (verified_at, judged) = read_verdicts(doc, cache.files.get(&doc.path), &mut out);
        for (i, u) in doc.units.iter().enumerate() {
            let addr = unit_addr(doc, i);
            let carried = judged
                .iter()
                .filter(|j| j.line >= u.line_start && j.line <= u.line_end);
            let (Some(verdict), evidence) = roll_up(carried) else {
                out.omitted.push(addr);
                continue;
            };
            let unit = BaselineUnit::new(
                addr.clone(),
                u.content_hash.clone(),
                verdict,
                evidence,
                verified_at.as_str(),
                governing_marker(doc, u),
            );
            *out.verdicts.entry(unit.verdict.clone()).or_default() += 1;
            if out.baseline.units.insert(addr.clone(), unit).is_some() {
                out.collisions.push(addr);
            }
        }
    }
    out
}

/// One fact the campaign judged, located in the document that carries it.
struct Judged {
    line: usize,
    verdict: String,
    evidence: Vec<String>,
}

/// Worst verdict wins, and the evidence of everything that voted.
///
/// A unit carrying one drifting fact is not a unit that may skip
/// re-verification, so the roll-up takes the least reassuring answer in
/// the span rather than the commonest or the last one. Ties keep the
/// first fact in document order, and the evidence is the union in that
/// same order — deduplicated, so a batch-wide reference cited by forty
/// facts appears once.
fn roll_up<'a>(judged: impl Iterator<Item = &'a Judged>) -> (Option<String>, Vec<String>) {
    let mut worst: Option<(u8, &str)> = None;
    let mut evidence: Vec<String> = Vec::new();
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for j in judged {
        let rank = severity(&j.verdict);
        if worst.is_none_or(|(w, _)| rank > w) {
            worst = Some((rank, &j.verdict));
        }
        for e in &j.evidence {
            if seen.insert(e.as_str()) {
                evidence.push(e.clone());
            }
        }
    }
    (worst.map(|(_, v)| v.to_string()), evidence)
}

/// The verdict order §4.1.3 fixes: `drift` > `unverifiable` > `confirmed`.
///
/// Anything else outranks all three. A verdict vocabulary this code does
/// not model is one nobody here can judge the weight of, and the safe
/// direction is the one that gets the unit looked at again — never the
/// one where an unrecognised string is swallowed by a neighbouring
/// `confirmed`.
fn severity(verdict: &str) -> u8 {
    match verdict {
        "confirmed" => 0,
        "unverifiable" => 1,
        "drift" => 2,
        _ => 3,
    }
}

/// The judged facts of one file, in document order, with the date the
/// campaign judged them.
///
/// The `campaign` map is deliberately loose in the cache record (§7.1) —
/// it is written by whoever runs the verification pass — so everything
/// unreadable here is *reported*, never guessed at: a key naming no fact
/// anchor, an entry with no verdict, a file with no `verified_at`. The
/// worst outcome any of them can produce is a unit that gets re-verified
/// next month.
fn read_verdicts(
    doc: &ParsedDoc,
    record: Option<&FileRecord>,
    out: &mut Projection,
) -> (String, Vec<Judged>) {
    let nothing = || (String::new(), Vec::new());
    let Some(record) = record else {
        return nothing();
    };
    let Some(map) = record
        .campaign
        .get("verdicts")
        .and_then(serde_json::Value::as_object)
    else {
        return nothing();
    };
    let verified_at = record
        .campaign
        .get("verified_at")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if verified_at.is_empty() {
        out.undated.push(doc.path.clone());
        return nothing();
    }
    // The campaign's own note of what it judged, against what is here now.
    if record
        .campaign
        .get("processed_hash")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|h| h != doc.content_hash)
    {
        out.stale.push(doc.path.clone());
    }
    // Fact anchors are minted once per document (the duplicate-id law,
    // §3.8), so the first line wins and a document that broke the law
    // still projects rather than panicking.
    let mut lines: BTreeMap<&str, usize> = BTreeMap::new();
    for b in &doc.blocks {
        for f in &b.facts {
            if let Some(id) = &f.id {
                lines.entry(id.as_str()).or_insert(f.line);
            }
        }
    }
    let mut judged = Vec::new();
    for (key, value) in map {
        let verdict = value.get("v").and_then(serde_json::Value::as_str);
        match (lines.get(key.as_str()), verdict) {
            (Some(line), Some(verdict)) => judged.push(Judged {
                line: *line,
                verdict: verdict.to_string(),
                evidence: evidence_of(value),
            }),
            _ => out.unresolved.push(format!("{}#{key}", doc.path)),
        }
    }
    judged.sort_by_key(|j| j.line);
    (verified_at.to_string(), judged)
}

/// The evidence refs one verdict entry carries (`ev`), in the order it
/// wrote them. A non-string entry is dropped rather than rendered as
/// JSON: an evidence ref is provenance a human follows (§6), and
/// `{"file":…}` is not one.
fn evidence_of(value: &serde_json::Value) -> Vec<String> {
    value
        .get("ev")
        .and_then(serde_json::Value::as_array)
        .map(|refs| {
            refs.iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
