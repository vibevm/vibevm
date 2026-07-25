//! The incremental cache and atomic file IO (PROP-043 §7.1).
//!
//! Every write in this crate goes through `write_atomic` (tmp + rename),
//! so a killed process never leaves a torn JSON on disk.

specmark::scope!("spec://vibevm/modules/vibe-progress/PROP-043#cache");

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
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub campaign: BTreeMap<String, serde_json::Value>,
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
        serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
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
    #[specmark::spec(implements = "spec://vibevm/modules/vibe-progress/PROP-043#cache")]
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
mod tests {
    use super::*;

    /// A payload sidecar on disk under `dir` holding exactly `docs` — the
    /// shape a finished run leaves behind, built somewhere a test owns.
    fn stored(dir: &Path, docs: &[&ParsedDoc]) -> Payloads {
        let store = Payloads::load(Some(dir.to_path_buf()));
        store.store(docs.iter().copied());
        Payloads::load(Some(dir.to_path_buf()))
    }

    /// The whole of DRIFT-017's judgement call, stated on the seam that
    /// makes it: the stamp is not content, and everything else is.
    ///
    /// Reading (a) — `updated_at` says when the content last *changed* —
    /// is only true if a document that differs in nothing else is left
    /// alone, and only safe if a document that differs anywhere else is
    /// written. Both halves are asserted here, including the case the
    /// live corpus actually contains: a verdict whose own text names the
    /// field, which a looser search would mistake for the stamp.
    #[test]
    fn write_if_changed_skips_the_stamp_and_nothing_else() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("campaign.json");
        let doc = |stamp: &str, files: u32, note: &str| {
            format!(
                "{{\n  \"schema\": 1,\n  \"updated_at\": \"{stamp}\",\n  \"counters\": {{\n    \"files\": {files}\n  }},\n  \"note\": \"{note}\"\n}}"
            )
        };
        let first = doc("2026-07-25T00:00:00Z", 58, "quiet");

        // Absent ⇒ written; identical ⇒ skipped.
        assert!(write_if_changed(&path, &first).expect("create"), "absent");
        assert!(
            !write_if_changed(&path, &first).expect("same"),
            "the same bytes twice is one write"
        );

        // A later run of an unchanged corpus: only the clock moved.
        let later = doc("2026-07-26T12:34:56Z", 58, "quiet");
        assert!(!write_if_changed(&path, &later).expect("stamp only"));
        let on_disk = std::fs::read_to_string(&path).expect("read");
        assert_eq!(on_disk, first, "the old stamp stands, byte for byte");

        // Content moved under the same clock ⇒ written.
        let moved = doc("2026-07-25T00:00:00Z", 59, "quiet");
        assert!(write_if_changed(&path, &moved).expect("content"));
        assert_eq!(std::fs::read_to_string(&path).expect("read"), moved);

        // The trap `corpus.json` sets: a verdict's own text names the
        // field. It is a value, not the top-level key, and moving it must
        // still count as a change.
        let named = doc(
            "2026-07-25T00:00:00Z",
            59,
            "live campaign.json has updated_at",
        );
        assert!(write_if_changed(&path, &named).expect("nested mention"));
        assert!(!write_if_changed(&path, &named).expect("still identical"));

        // A file this function cannot read is replaced, never trusted.
        std::fs::write(&path, [0xff, 0xfe, 0x00]).expect("clobber");
        assert!(write_if_changed(&path, &named).expect("not utf-8"));

        // And a document with no stamp at all falls back to whole-file
        // equality — the sidecar's case.
        let flat = dir.path().join("payloads.json");
        let compact = r#"{"schema":1,"docs":{}}"#;
        assert!(write_if_changed(&flat, compact).expect("absent"));
        assert!(!write_if_changed(&flat, compact).expect("identical"));
        assert!(write_if_changed(&flat, r#"{"schema":1,"docs":{"a.md":1}}"#).expect("differs"));
    }

    #[test]
    fn corrupt_cache_degrades_to_empty_with_warning() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("cache.json");
        // A zero-length file — exactly what a power cut after rename but
        // before data flush leaves behind.
        std::fs::write(&path, b"").expect("write");
        let (c, warn) = Cache::load_tolerant(&path);
        assert!(c.files.is_empty());
        assert!(warn.expect("warning").contains("rebuilt from scratch"));
    }

    #[test]
    fn cache_round_trips_and_tracks_currency() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("cache.json");
        let mut c = Cache {
            schema: CACHE_SCHEMA,
            ..Cache::default()
        };
        let doc = crate::parse::parse_document("a.md", "@impl hello\n");
        let r = crate::rollup::rollup_doc(&doc);
        c.upsert(&doc, &r);
        c.touch();
        c.store(&path).expect("store");
        let back = Cache::load(&path).expect("load");
        assert!(back.is_current("a.md", &doc.content_hash));
        assert!(!back.is_current("a.md", "deadbeef"));
    }

    /// The payload's whole claim: what comes back out of a stored run is
    /// the document that went in. Asserted on the struct, not on a few
    /// hand-picked counters — everything `ParsedDoc` persists must survive
    /// the JSON, or a warm run is quietly answering from a different
    /// document than a cold one.
    ///
    /// Since DRIFT-016 it also asserts *where*: the tracked `cache.json`
    /// carries the identity a verdict is formed against and none of the
    /// text that identity stands for.
    ///
    /// The two `#[serde(skip)]` fields are cleared on the freshly parsed
    /// side before comparing, and that is the *whole* of the residue: they
    /// are the marker scanner's scratch (`Block::scan_text` is the blanked
    /// block text it scans, `Fact::span` indexes into it), written and read
    /// inside `parse` and by nothing downstream. Naming them here keeps the
    /// day someone reaches for them from being silent.
    #[test]
    fn cached_doc_round_trips_the_parse() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("cache.json");
        let text = "<status stage=\"impl\" state=\"work\"/>\n\n\
                    # Title {#t}\n\n\
                    ##b1 @test/plan A paragraph.\n\n\
                    - ##i1 An item. @doc/done\n\
                    - ##i2 Another item. @impl/hold\n\n\
                    ```\ncode fence\n```\n\n\
                    ##b2 <status stage=\"spec\" state=\"done\" action=\"drift\">frag</status> tail.\n";
        let doc = crate::parse::parse_document("spec/x.md", text);
        assert!(doc.markers.len() >= 5, "a document worth round-tripping");

        let mut c = Cache::default();
        c.upsert(&doc, &crate::rollup::rollup_doc(&doc));
        c.store(&path).expect("store");
        let side = stored(&dir.path().join("payloads"), &[&doc]);
        let back = Cache::load(&path).expect("load");

        let got = back
            .cached_doc("spec/x.md", &doc.content_hash, &side)
            .expect("payload survives the JSON");

        let mut expected = doc.clone();
        for b in &mut expected.blocks {
            b.scan_text = String::new();
            for f in &mut b.facts {
                f.span = (0, 0);
            }
        }
        assert_eq!(got, &expected, "the parse comes back whole");

        // …and it came out of the sidecar, not out of git.
        let tracked = std::fs::read_to_string(&path).expect("read cache.json");
        assert!(tracked.contains(&doc.content_hash), "the identity stays");
        assert!(!tracked.contains("A paragraph"), "the payload does not");
    }

    /// Four ways to be stale, one answer: parse it. A cache is allowed to
    /// know nothing; it is never allowed to answer for the wrong bytes.
    #[test]
    fn cached_doc_misses_are_misses() {
        let dir = tempfile::tempdir().expect("tempdir");
        let doc = crate::parse::parse_document("a.md", "@impl hello\n");
        let mut c = Cache::default();
        c.upsert(&doc, &crate::rollup::rollup_doc(&doc));
        let side = stored(&dir.path().join("warm"), &[&doc]);

        assert!(
            c.cached_doc("b.md", &doc.content_hash, &side).is_none(),
            "no record"
        );
        assert!(
            c.cached_doc("a.md", "deadbeef", &side).is_none(),
            "stale hash"
        );

        // The sidecar erased between runs — the case DRIFT-016 exists to
        // make ordinary. The record is still current and still
        // authoritative; there is simply nothing to hand back.
        let erased = Payloads::load(Some(dir.path().join("gone")));
        assert!(c.is_current("a.md", &doc.content_hash), "still current");
        assert!(
            c.cached_doc("a.md", &doc.content_hash, &erased).is_none(),
            "an erased sidecar is a miss, not an empty document"
        );

        // A payload that disagrees with the record filing it.
        let other = crate::parse::parse_document("a.md", "@spec other\n");
        let lying = stored(&dir.path().join("lying"), &[&other]);
        assert!(
            c.cached_doc("a.md", &doc.content_hash, &lying).is_none(),
            "a payload whose identity disagrees is a miss"
        );
    }

    /// DRIFT-016 §4.3, stated on the seam: a payload whose hash disagrees
    /// with the record in git is ignored, not trusted. The record is the
    /// authority precisely because it is the half a verdict was formed
    /// against and the half that a clone still has.
    #[test]
    fn sidecar_stale_hash_is_a_miss() {
        let dir = tempfile::tempdir().expect("tempdir");
        // The file as the campaign judged it …
        let judged = crate::parse::parse_document("a.md", "@impl judged\n");
        let mut c = Cache::default();
        c.upsert(&judged, &crate::rollup::rollup_doc(&judged));
        // … and a sidecar left behind by an older state of that same file,
        // which a branch switch or a stale bucket produces for free.
        let stale = crate::parse::parse_document("a.md", "@impl an older draft\n");
        assert_ne!(judged.content_hash, stale.content_hash, "two states");
        let side = stored(dir.path(), &[&stale]);

        assert!(
            c.cached_doc("a.md", &judged.content_hash, &side).is_none(),
            "the stale payload is never handed back"
        );
        // And the store is not merely broken: asked for the bytes it does
        // hold, it still answers. Wrong-for-this-run, not corrupt.
        assert!(
            side.get("a.md", &stale.content_hash).is_some(),
            "the store is honest about what it has"
        );
    }

    /// The campaign field is load-bearing (DRIFT-010 §5): re-upserting the
    /// same file — which is what every warm run does — must carry the
    /// verdicts forward untouched.
    #[test]
    fn upsert_preserves_campaign_across_a_warm_write() {
        let dir = tempfile::tempdir().expect("tempdir");
        let doc = crate::parse::parse_document("a.md", "@impl hello\n");
        let r = crate::rollup::rollup_doc(&doc);
        let mut c = Cache::default();
        c.upsert(&doc, &r);
        c.files
            .get_mut("a.md")
            .expect("record")
            .campaign
            .insert("verdicts".into(), serde_json::json!({"x": "confirmed"}));
        let side = stored(dir.path(), &[&doc]);

        // The warm path: the payload comes back out and goes straight
        // back in, exactly as `ground` + `refresh_state` do it.
        let warm = c
            .cached_doc("a.md", &doc.content_hash, &side)
            .expect("hit")
            .clone();
        c.upsert(&warm, &r);

        assert_eq!(
            c.files["a.md"].campaign.get("verdicts"),
            Some(&serde_json::json!({"x": "confirmed"})),
            "a warm rewrite keeps the verdicts"
        );
    }

    #[test]
    fn retain_paths_prunes_out_of_scope_and_preserves_campaign() {
        use std::collections::BTreeSet;
        let mut c = Cache {
            schema: CACHE_SCHEMA,
            ..Cache::default()
        };
        let a = crate::parse::parse_document("a.md", "@impl keep\n");
        let b = crate::parse::parse_document("b.md", "@impl drop\n");
        c.upsert(&a, &crate::rollup::rollup_doc(&a));
        c.upsert(&b, &crate::rollup::rollup_doc(&b));
        // A campaign verdict on the survivor (must be preserved) and on the
        // record that leaves scope (its loss must be reported, not silent).
        c.files
            .get_mut("a.md")
            .expect("a record")
            .campaign
            .insert("verdict".into(), serde_json::json!("pass"));
        c.files
            .get_mut("b.md")
            .expect("b record")
            .campaign
            .insert("verdict".into(), serde_json::json!("fail"));

        let observed: BTreeSet<String> = ["a.md".to_string()].into_iter().collect();
        let dropped = c.retain_paths(&observed);

        // b.md left the scope: its record is gone …
        assert!(!c.files.contains_key("b.md"), "out-of-scope record pruned");
        // … and because it carried a verdict, the drop was reported.
        assert_eq!(dropped, vec!["b.md".to_string()]);
        // a.md stayed, its campaign map intact.
        let survivor = c.files.get("a.md").expect("survivor kept");
        assert_eq!(
            survivor.campaign.get("verdict"),
            Some(&serde_json::json!("pass")),
        );
    }
}
