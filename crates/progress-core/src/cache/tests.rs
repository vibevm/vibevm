//! The cache's own tests — what a record keeps, and what it refuses to
//! keep.
//!
//! File-backed submodule of [`super`] so both cells stay inside the
//! AI-Native file budget, the same split `baseline/project` already
//! carries. Nothing moved but the file: every assertion here is the one
//! that stood beside the code it tests, and the seam is the ordinary one
//! — the cache decides, and this decides whether it decided right.

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

/// A record exactly as `campaigns/packages-2026-09/run/cache.json`
/// carried it before DRIFT-033 — the campaign map with its stored
/// `summary` in it, and the surrounding shape copied from the live
/// file rather than rebuilt by this version's own writer, so a field
/// this version starts demanding fails here the way it would fail on
/// the real one.
const LEGACY: &str = r#"{
  "schema": 2,
  "updated_at": "2026-07-26T13:19:56Z",
  "files": {
    "spec/boot/00-core.md": {
      "content_hash": "27697df5871b7e4831d1ae9db525ff6a93c0b124fcc90afa3d02047dfee9c511",
      "rollup": {
        "computed": ["impl", "done"],
        "effective": ["impl", "done"],
        "marker_count": 3,
        "fact_count": 3,
        "unmarked_facts": 0
      },
      "marker_count": 3,
      "unit_count": 1,
      "issue_count": 0,
      "campaign": {
        "processed_hash": "78d18746e702bfbb3fe8a1ae98e39f4c5289d7d6104b017e99e9bfec6d3a312f",
        "summary": { "confirmed": 2, "unverifiable": 1 },
        "verdicts": {
          "a1": { "v": "confirmed", "ev": ["crates/vibe-core/src/lib.rs"] },
          "a2": { "v": "confirmed", "ev": [] },
          "a3": { "v": "unverifiable", "ev": [] }
        },
        "verified_at": "2026-07-26T10:23:56Z",
        "verify_batch": "c0-boot + d2g"
      }
    }
  }
}"#;

/// F-077 §4.6, on a real pre-change record: a cache written by the
/// previous version **loads**, and its stored `summary` is ignored
/// rather than rejected.
///
/// Ignoring it is the only safe direction. The 4 498 verdicts in that
/// file are the one thing in this crate nobody can redo, so a load
/// that failed on an obsolete key beside them would be refusing to
/// read a campaign's whole memory over a number it can recompute in a
/// microsecond — and the number it recomputes is asserted here to be
/// the one the old field claimed, which is what makes dropping it a
/// deletion rather than a loss.
#[test]
fn a_legacy_summary_loads_and_is_ignored() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("cache.json");
    std::fs::write(&path, LEGACY).expect("write the legacy cache");

    let cache = Cache::load(&path).expect("a pre-change cache still loads");
    let record = &cache.files["spec/boot/00-core.md"];

    assert!(
        !record.campaign.contains_key("summary"),
        "the stored count is gone: {:?}",
        record.campaign.keys().collect::<Vec<_>>()
    );
    // Everything the campaign cannot redo came through untouched.
    assert_eq!(
        record.campaign["verdicts"]["a1"]["ev"][0],
        serde_json::json!("crates/vibe-core/src/lib.rs")
    );
    assert_eq!(
        record.campaign["verdicts"].as_object().map(|m| m.len()),
        Some(3)
    );
    assert_eq!(
        record.campaign["verify_batch"],
        serde_json::json!("c0-boot + d2g")
    );
    assert_eq!(
        record.campaign["processed_hash"],
        serde_json::json!("78d18746e702bfbb3fe8a1ae98e39f4c5289d7d6104b017e99e9bfec6d3a312f")
    );

    // The dropped field said `{confirmed: 2, unverifiable: 1}`, and so
    // does the recount that replaces it.
    assert_eq!(
        record.verdict_summary(),
        Some(serde_json::json!({"confirmed": 2, "unverifiable": 1})),
    );

    // …and the next store writes the record without it.
    let rewritten = dir.path().join("rewritten.json");
    assert!(cache.store(&rewritten).expect("store"));
    let text = std::fs::read_to_string(&rewritten).expect("read back");
    assert!(!text.contains("\"summary\""), "no stored count survives");
    assert!(text.contains("\"unverifiable\""), "the verdicts do");
}

/// The count is a *read*, not a field (F-077): it comes out of the
/// verdicts every time, it goes back into no record, and a record with
/// no verdict map has no count rather than an empty one — the
/// difference between a file the campaign judged and found nothing in
/// and a file it never opened.
#[test]
fn the_summary_is_computed_and_never_stored() {
    let doc = crate::parse::parse_document("a.md", "@impl hello\n");
    let mut c = Cache::default();
    c.upsert(&doc, &crate::rollup::rollup_doc(&doc));
    let judge = |c: &mut Cache, verdicts: serde_json::Value| {
        c.files
            .get_mut("a.md")
            .expect("record")
            .campaign
            .insert("verdicts".into(), verdicts);
    };

    assert_eq!(c.files["a.md"].verdict_summary(), None, "never judged");
    assert!(!c.files["a.md"].campaign_view().contains_key("summary"));

    judge(&mut c, serde_json::json!({}));
    assert_eq!(
        c.files["a.md"].verdict_summary(),
        Some(serde_json::json!({})),
        "judged, and nothing in it"
    );

    judge(
        &mut c,
        serde_json::json!({
            "a1": {"v": "confirmed", "ev": []},
            "a2": {"v": "drift", "ev": []},
            "a3": "confirmed",
            "a4": 7
        }),
    );
    assert_eq!(
        c.files["a.md"].verdict_summary(),
        Some(serde_json::json!({"confirmed": 2, "drift": 1})),
        "both shapes counted; the shape that names no verdict invents none"
    );

    // The view carries it, the record does not, and the disk agrees
    // with the record.
    assert_eq!(
        c.files["a.md"].campaign_view()["summary"],
        serde_json::json!({"confirmed": 2, "drift": 1})
    );
    assert!(!c.files["a.md"].campaign.contains_key("summary"));
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("cache.json");
    c.store(&path).expect("store");
    assert!(
        !std::fs::read_to_string(&path)
            .expect("read")
            .contains("\"summary\""),
        "computing it left nothing behind"
    );
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
/// The three `#[serde(skip)]` fields are cleared on the freshly parsed
/// side before comparing, and that is the *whole* of the residue: they
/// are parser scratch (`Block::scan_text` is the blanked block text it
/// scans, `Block::source_text` is the verbatim construction input, and
/// `Fact::span` indexes into both), written and read inside `parse` and by
/// nothing downstream. Naming them here keeps the
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
        b.source_text = String::new();
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
