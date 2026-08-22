//! The incremental cache and atomic file IO (PROP-043 §7.1).
//!
//! Every write in this crate goes through `write_atomic` (tmp + rename),
//! so a killed process never leaves a torn JSON on disk.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-047#cache");

use crate::doc::ParsedDoc;
use crate::rollup::DocRollup;
use crate::sidecar::Payloads;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::path::Path;

/// Schema 2: the fact amendment — `DocRollup` counts facts
/// (paragraphs + list items + table cells), not paragraphs.
///
/// Neither DRIFT-010's `parsed` payload nor DRIFT-016's removal of it
/// bumped this, and for the same reason in both directions: the key was
/// additive going in and is additive going out. A record still carrying
/// one loads unchanged and the key is ignored; a record without one reads
/// as a miss, which is what a miss already meant. No record is re-keyed,
/// so no migration exists for a live campaign's verdict maps to survive —
/// which is the one thing in this file that could not be redone.
pub const CACHE_SCHEMA: u32 = 2;

/// One observed file's record — everything a cold reader needs to know
/// what was judged and what it was judged against (DRIFT-016 §4.1).
///
/// This file is tracked in git, so what it holds is chosen by what cannot
/// be recomputed: the content hash a verdict was formed against, the
/// rollup, and the campaign map. The parse those bytes produce is
/// regenerable, and since DRIFT-016 it lives in [`crate::sidecar`],
/// outside the repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    pub content_hash: String,
    pub rollup: DocRollup,
    pub marker_count: usize,
    pub unit_count: usize,
    pub issue_count: usize,
    /// Campaign fields (verdicts etc.) merge in during phases C–E; absent
    /// until then.
    ///
    /// What belongs here is what cannot be recomputed from what is beside
    /// it: the verdicts, the evidence, the batch id, the two hashes and
    /// the date. A count over those verdicts does not — see
    /// [`FileRecord::verdict_summary`] (DRIFT-033, F-077).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub campaign: BTreeMap<String, serde_json::Value>,
}

impl FileRecord {
    /// The tally this record's own verdict map adds up to —
    /// `{verdict → count}` — or `None` for a record no campaign judged.
    ///
    /// Until DRIFT-033 this number was *stored*, as a `summary` field
    /// sitting beside the map it counts. Two copies of one fact with
    /// nothing between them: the corpus happened to agree on all 58 of
    /// them the day it was measured, and no code made that true or would
    /// have said anything the day it stopped being. So the field is gone
    /// and the count is produced here, from the verdicts, every time
    /// somebody reads it — which is the only version of it that cannot be
    /// wrong.
    ///
    /// A verdict is the entry's `v` where the entry is an object and the
    /// entry itself where it is a bare string; those are the two shapes
    /// the campaign map has carried. An entry of any other shape names no
    /// verdict, so it counts toward none rather than being given a bucket
    /// of its own — a tally that invented `"?"` would be reporting this
    /// function's confusion as if it were a campaign's judgement.
    pub fn verdict_summary(&self) -> Option<serde_json::Value> {
        let verdicts = self.campaign.get("verdicts")?.as_object()?;
        let mut tally: BTreeMap<&str, usize> = BTreeMap::new();
        for entry in verdicts.values() {
            let verdict = match entry {
                serde_json::Value::Object(o) => o.get("v").and_then(serde_json::Value::as_str),
                serde_json::Value::String(s) => Some(s.as_str()),
                _ => None,
            };
            if let Some(v) = verdict {
                *tally.entry(v).or_default() += 1;
            }
        }
        Some(serde_json::json!(tally))
    }

    /// This record's campaign fields **as a reader is handed them**: every
    /// stored field, plus the `summary` computed from the verdicts beside
    /// them.
    ///
    /// The projection is where the count is produced and it is produced
    /// again on every write, so what a consumer reads can never be older
    /// than the map it counts. Nothing here is written back into
    /// [`Cache`] — see [`Cache::store`], which serialises the record and
    /// not this view.
    pub fn campaign_view(&self) -> BTreeMap<String, serde_json::Value> {
        let mut view = self.campaign.clone();
        if let Some(summary) = self.verdict_summary() {
            view.insert("summary".into(), summary);
        }
        view
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cache {
    pub schema: u32,
    pub updated_at: String,
    /// repo-relative `/`-separated path → record; BTreeMap keeps the
    /// serialized form stably sorted (clean diffs).
    pub files: BTreeMap<String, FileRecord>,
}

/// An empty cache is a *current-schema* cache: there is no state in this
/// crate that means "schema 0", and a default that claimed one would be a
/// forgery waiting to be stored.
impl Default for Cache {
    fn default() -> Self {
        Cache {
            schema: CACHE_SCHEMA,
            updated_at: String::new(),
            files: BTreeMap::new(),
        }
    }
}

impl Cache {
    pub fn load(path: &Path) -> Result<Cache> {
        if !path.exists() {
            return Ok(Cache::default());
        }
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let mut cache: Cache =
            serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        cache.drop_derived();
        Ok(cache)
    }

    /// Forget the campaign fields a previous version stored that this one
    /// computes (DRIFT-033, F-077).
    ///
    /// A cache written before that change carries a per-file `summary`
    /// beside the verdicts it counted. Such a file **loads** — a key this
    /// version has no use for is data, never an error, and the record's
    /// verdicts are the one thing in this crate that could not be redone
    /// — and the key is dropped here, so the next store writes the record
    /// without it and no reader is ever handed a count that nothing keeps
    /// honest. The count itself is still available, from
    /// [`FileRecord::verdict_summary`], where it cannot go stale.
    fn drop_derived(&mut self) {
        for record in self.files.values_mut() {
            record.campaign.remove("summary");
        }
    }

    /// Crash-tolerant load: a torn or corrupt cache (power loss mid-write)
    /// degrades to an empty cache plus a warning — the cache is derived
    /// acceleration (PROP-043 §7.5) and must never kill a session.
    pub fn load_tolerant(path: &Path) -> (Cache, Option<String>) {
        match Cache::load(path) {
            Ok(c) => (c, None),
            Err(e) => (
                Cache::default(),
                Some(format!(
                    "cache at {} was unreadable ({e:#}); rebuilt from scratch",
                    path.display()
                )),
            ),
        }
    }

    /// Write the cache back, and say whether that was necessary.
    ///
    /// `false` is a run whose records — including every campaign verdict
    /// map — are already exactly what is on disk, so nothing was written
    /// and nothing was fsync'd (DRIFT-017 §4.1). The verdicts are safe in
    /// that answer for the reason [`write_if_changed`] gives: identity is
    /// decided on the serialised bytes those maps are part of, and
    /// anything short of proof writes.
    pub fn store(&self, path: &Path) -> Result<bool> {
        let body = serde_json::to_string_pretty(self)?;
        write_if_changed(path, &body)
    }

    /// True when the cached record for `path` is current for `hash`.
    pub fn is_current(&self, path: &str, hash: &str) -> bool {
        self.files
            .get(path)
            .map(|r| r.content_hash == hash)
            .unwrap_or(false)
    }

    /// The cached parse for `path`, when the record **in git** is current
    /// for `hash` and the sidecar holds a payload that agrees with it.
    ///
    /// Everything else is a miss and the caller parses (PROP-043 §7.1,
    /// DRIFT-010 §4, DRIFT-016 §4.3): no record, a stale hash, a sidecar
    /// that was erased or never written, or — the case a hand-edited store
    /// creates — a payload whose own `path`/`content_hash` disagree with
    /// the record filing it. The cache is allowed to be *empty*; it is
    /// never allowed to be *wrong*.
    ///
    /// The asymmetry is deliberate and is the whole design: the record is
    /// the authority and it is in the repository; the payload is an
    /// accelerator and it is not. A run whose sidecar is gone is a slow
    /// run, and nothing else about it differs.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-047#cache"
    )]
    pub fn cached_doc<'p>(
        &self,
        path: &str,
        hash: &str,
        payloads: &'p Payloads,
    ) -> Option<&'p ParsedDoc> {
        let record = self.files.get(path)?;
        if record.content_hash != hash {
            return None;
        }
        payloads.get(path, hash)
    }

    pub fn upsert(&mut self, doc: &ParsedDoc, rollup: &DocRollup) {
        let campaign = self
            .files
            .get(&doc.path)
            .map(|r| r.campaign.clone())
            .unwrap_or_default();
        self.files.insert(
            doc.path.clone(),
            FileRecord {
                content_hash: doc.content_hash.clone(),
                rollup: rollup.clone(),
                marker_count: doc.markers.len(),
                unit_count: doc.units.len(),
                issue_count: doc.issues.len(),
                campaign,
            },
        );
    }

    /// Drop every record whose path is **not** in `observed`; the records
    /// that stay keep their `campaign` maps untouched. A file outside the
    /// observed set has no contract right to a record (PROP-043 §7.1), so
    /// scope-narrowing must not leave stale rows inflating the projections
    /// (DRIFT-001).
    ///
    /// Returns the paths of any dropped records that carried a **non-empty**
    /// `campaign` map — campaign verdicts that left the observed scope. The
    /// prune is never *silent* about that loss: the caller surfaces the
    /// list (DRIFT-001 §5). An empty return means no verdict data was lost.
    pub fn retain_paths(&mut self, observed: &BTreeSet<String>) -> Vec<String> {
        let mut dropped_with_campaign = Vec::new();
        self.files.retain(|path, record| {
            let keep = observed.contains(path);
            if !keep && !record.campaign.is_empty() {
                dropped_with_campaign.push(path.clone());
            }
            keep
        });
        dropped_with_campaign
    }

    pub fn touch(&mut self) {
        self.updated_at = now_utc();
    }
}

/// RFC-3339 UTC timestamp (seconds precision) for `updated_at` fields.
pub fn now_utc() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// tmp + fsync + rename: the poller never sees half a file, and a power
/// cut never leaves a zero-length rename (the fsync-before-rename lesson,
/// paid for live on 2026-07-24).
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write as _;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let tmp = path.with_extension("tmp~");
    {
        let mut f =
            std::fs::File::create(&tmp).with_context(|| format!("creating {}", tmp.display()))?;
        f.write_all(bytes)
            .with_context(|| format!("writing {}", tmp.display()))?;
        f.sync_all()
            .with_context(|| format!("syncing {}", tmp.display()))?;
    }
    std::fs::rename(&tmp, path)
        .with_context(|| format!("renaming {} → {}", tmp.display(), path.display()))?;
    Ok(())
}

/// Atomic write that first asks whether it is needed: a run with nothing
/// new to say leaves the file alone — untouched, unfsync'd, mtime and all
/// (DRIFT-017 §4.1). Returns whether it wrote.
///
/// Two documents count as the same when they are byte-identical outside
/// the top-level `updated_at` value they carry, because that stamp is not
/// content. It records when the content last **changed**, not when the
/// tool last looked (DRIFT-017 §4.2, reading (a)), so a run that changes
/// nothing leaves the stamp already on disk standing — a freshness plaque
/// that advances while nothing moved is not freshness. A document with no
/// such stamp is compared byte for byte.
///
/// Every way of failing to *prove* the file already says this writes: an
/// absent file, an unreadable one, bytes that are not UTF-8, a document
/// that keeps its stamp somewhere this function does not look. The safe
/// direction costs one fsync; the other loses state, and one of the files
/// on this path holds the campaign's verdicts (DRIFT-017 §5).
pub fn write_if_changed(path: &Path, body: &str) -> Result<bool> {
    if std::fs::read_to_string(path).is_ok_and(|current| same_but_for_stamp(&current, body)) {
        return Ok(false);
    }
    write_atomic(path, body.as_bytes())?;
    Ok(true)
}

/// True when `current` and `body` say the same thing — everything outside
/// the one wall clock each of them carries.
///
/// A side with no stamp where this crate puts one is compared whole, so
/// the fallback is the strictest reading rather than the loosest.
fn same_but_for_stamp(current: &str, body: &str) -> bool {
    match (stamp_span(current), stamp_span(body)) {
        (Some(c), Some(b)) => {
            current[..c.start] == body[..b.start] && current[c.end..] == body[b.end..]
        }
        _ => current == body,
    }
}

/// The byte range of the top-level wall-clock **value** in a document
/// `serde_json::to_string_pretty` produced, or `None` when there is none.
///
/// The needle is a newline, two spaces and the key. The pretty printer
/// indents two spaces per level, so that sequence is a key at depth 1 and
/// nothing else: a key one level deeper carries four spaces, and inside a
/// string value a newline is escaped, so it cannot occur there at all.
/// That is why this reads the bytes instead of parsing them — `corpus.json`
/// carries the word `updated_at` inside a campaign verdict's own text, and
/// a looser search would find it.
///
/// Two keys name that clock in this crate: `updated_at` on the cache and
/// the state projections, `written_at` on the baseline (DRIFT-023 §4.2).
/// They mean one thing under two names — when the content behind them
/// last moved — so both are recognised here. A writer that knew only the
/// first would rewrite a byte-identical baseline on every run, which is
/// exactly the fsync DRIFT-017 exists to skip.
fn stamp_span(json: &str) -> Option<Range<usize>> {
    ["\n  \"updated_at\": \"", "\n  \"written_at\": \""]
        .into_iter()
        .find_map(|key| {
            let start = json.find(key)? + key.len();
            let end = start + json[start..].find('"')?;
            Some(start..end)
        })
}

#[cfg(test)]
mod tests;
