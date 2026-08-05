//! The projection's own tests — the two grains meeting, asserted.
//!
//! File-backed submodule of [`super`] so both cells stay inside the
//! AI-Native file budget: everything here is about what the projection
//! *decides* (which unit carries which verdict, and which unit carries
//! none), and the round trip that proves the reader agrees with it.

use super::*;
use crate::baseline::{RescanClass, RescanOptions, rescan};
use crate::parse::parse_document;
use crate::rollup::rollup_doc;

/// A cache holding `doc` with the campaign map a verification pass
/// writes: `verified_at` plus `verdicts{anchor → {v, ev[]}}`.
///
/// Panics by name rather than by `.expect()`: a helper that dies has to
/// say which fixture died, and the ban on the terse form is what keeps
/// domain code from borrowing the habit.
fn cached(doc: &ParsedDoc, verified_at: &str, verdicts: serde_json::Value) -> Cache {
    let mut c = Cache::default();
    c.upsert(doc, &rollup_doc(doc));
    let campaign = &mut c
        .files
        .get_mut(&doc.path)
        .unwrap_or_else(|| panic!("upsert left no record for {}", doc.path))
        .campaign;
    if !verified_at.is_empty() {
        campaign.insert("verified_at".into(), serde_json::json!(verified_at));
    }
    campaign.insert("verdicts".into(), verdicts);
    c
}

/// A section whose facts disagree: one confirmed, one drift. §4.1.3 —
/// the unit records `drift`, because a unit carrying one drifting
/// fact is not a unit that may skip re-verification.
#[test]
fn worst_verdict_wins_over_the_units_facts() {
    let doc = parse_document(
        "a.md",
        "# One {#one}\n\n##a1 First. @impl/done\n\n##a2 Second. @impl/done\n\n\
         # Two {#two}\n\n##a3 Third. @impl/done\n",
    );
    let c = cached(
        &doc,
        "2026-07-25T00:00:00Z",
        serde_json::json!({
            "a1": {"v": "confirmed", "ev": ["one"]},
            "a2": {"v": "drift", "ev": ["two"]},
            "a3": {"v": "unverifiable", "ev": []},
        }),
    );
    let p = project([&doc], &c, "t");
    assert_eq!(p.baseline.units["a.md#one"].verdict, "drift");
    assert_eq!(
        p.baseline.units["a.md#one"].evidence,
        vec!["one".to_string(), "two".to_string()],
        "the union of the facts that voted, in document order"
    );
    assert_eq!(
        p.baseline.units["a.md#two"].verdict, "unverifiable",
        "…and unverifiable beats confirmed the same way"
    );
    assert_eq!(p.verdicts.get("drift"), Some(&1));

    // An unmodelled verdict string outranks every modelled one: it is
    // never quietly swallowed by the `confirmed` beside it.
    let c = cached(
        &doc,
        "2026-07-25T00:00:00Z",
        serde_json::json!({
            "a1": {"v": "confirmed", "ev": []},
            "a2": {"v": "drift-fixed", "ev": []},
        }),
    );
    let p = project([&doc], &c, "t");
    assert_eq!(p.baseline.units["a.md#one"].verdict, "drift-fixed");
}

/// §4.1's other half: a unit no judged fact reaches is left out and
/// counted, never given a verdict of its own.
#[test]
fn a_unit_without_judged_facts_is_omitted_not_invented() {
    let doc = parse_document(
        "a.md",
        "# One {#one}\n\n##a1 Judged. @impl/done\n\n\
         # Two {#two}\n\nUnanchored prose nobody judged.\n\n\
         # Three {#three}\n\n##a3 Unjudged. @impl/done\n",
    );
    let c = cached(
        &doc,
        "2026-07-25T00:00:00Z",
        serde_json::json!({"a1": {"v": "confirmed", "ev": []}, "_elements": {"v": "confirmed"}}),
    );
    let p = project([&doc], &c, "t");

    assert_eq!(p.baseline.units.len(), 1, "only the judged unit is written");
    assert!(p.baseline.units.contains_key("a.md#one"));
    assert_eq!(
        p.omitted,
        vec!["a.md#two".to_string(), "a.md#three".to_string()],
        "the units with nothing judged under them are reported, not filled in"
    );
    assert_eq!(
        p.unresolved,
        vec!["a.md#_elements".to_string()],
        "a verdict key naming no fact anchor is surfaced too"
    );
}

/// A parent heading carries what its subsections carry, because its
/// content hash does: a drift three levels down invalidates the hash
/// of every unit above it, so every unit above it must say `drift`.
#[test]
fn a_units_verdict_covers_its_nested_subsections() {
    let doc = parse_document(
        "a.md",
        "# Root {#root}\n\n##a1 Lead. @impl/done\n\n\
         ## Child {#child}\n\n##a2 Nested. @impl/done\n",
    );
    let c = cached(
        &doc,
        "2026-07-25T00:00:00Z",
        serde_json::json!({
            "a1": {"v": "confirmed", "ev": []},
            "a2": {"v": "drift", "ev": []},
        }),
    );
    let p = project([&doc], &c, "t");
    assert_eq!(
        p.baseline.units["a.md#root"].verdict, "drift",
        "the nested fact is inside the root unit's hashed body span"
    );
    assert_eq!(p.baseline.units["a.md#child"].verdict, "drift");
}

/// The round trip, which is the whole point of the projection: write
/// a baseline from a tree, read it back against that same tree, and
/// every unit carries forward with its marker snapshot intact.
#[test]
fn what_the_projection_writes_the_rescan_carries_forward() {
    let doc = parse_document(
        "a.md",
        "<status stage=\"impl\" state=\"work\"/>\n\n\
         # Root {#root}\n\n<status stage=\"spec\" state=\"done\"/>\n\n\
         ##a1 Lead. @impl/done\n\n\
         ## Child {#child}\n\n##a2 Nested. @impl/done\n\n\
         # Tail {#tail}\n\n##a3 Tail. @impl/done\n",
    );
    let c = cached(
        &doc,
        "2026-07-25T00:00:00Z",
        serde_json::json!({
            "a1": {"v": "confirmed", "ev": []},
            "a2": {"v": "confirmed", "ev": []},
            "a3": {"v": "confirmed", "ev": []},
        }),
    );
    let p = project([&doc], &c, "t");
    assert_eq!(p.baseline.units.len(), doc.units.len(), "every unit judged");
    assert_eq!(
        p.baseline.units["a.md#root"].marker.as_deref(),
        Some("spec/done"),
        "the section marker, resolved by the function rescan reads with"
    );
    assert_eq!(
        p.baseline.units["a.md#tail"].marker.as_deref(),
        Some("impl/work"),
        "…and the document marker where a section has none"
    );

    let rows = rescan(
        [&doc],
        &p.baseline,
        &RescanOptions {
            crate_states: BTreeMap::new(),
            control_rate: 0.0,
        },
    );
    assert_eq!(rows.len(), doc.units.len());
    assert!(
        rows.iter().all(|r| r.class == RescanClass::CarriedForward),
        "every row carries forward: {rows:?}"
    );
    assert!(
        rows.iter().all(|r| !r.marker_diverged),
        "and no marker reads as diverged"
    );

    // The negative control, on the same baseline: one unit's text
    // moves and exactly that row turns suspect.
    let edited = parse_document(
        "a.md",
        "<status stage=\"impl\" state=\"work\"/>\n\n\
         # Root {#root}\n\n<status stage=\"spec\" state=\"done\"/>\n\n\
         ##a1 Lead. @impl/done\n\n\
         ## Child {#child}\n\n##a2 Nested, and edited. @impl/done\n\n\
         # Tail {#tail}\n\n##a3 Tail. @impl/done\n",
    );
    let rows = rescan(
        [&edited],
        &p.baseline,
        &RescanOptions {
            crate_states: BTreeMap::new(),
            control_rate: 0.0,
        },
    );
    let class = |addr: &str| {
        rows.iter()
            .find(|r| r.addr == addr)
            .map(|r| r.class.clone())
            .expect("row")
    };
    assert_eq!(class("a.md#child"), RescanClass::Changed);
    assert_eq!(
        class("a.md#root"),
        RescanClass::Changed,
        "the parent's hash covers the child, so it is suspect too"
    );
    assert_eq!(class("a.md#tail"), RescanClass::CarriedForward);
}

/// Two units of one file claiming one address: the second would
/// silently overwrite the first in the map, so it is surfaced.
#[test]
fn a_colliding_unit_anchor_is_surfaced() {
    let doc = parse_document(
        "a.md",
        "# One {#dup}\n\n##a1 First. @impl/done\n\n# Two {#dup}\n\n##a2 Second. @impl/done\n",
    );
    let c = cached(
        &doc,
        "2026-07-25T00:00:00Z",
        serde_json::json!({
            "a1": {"v": "confirmed", "ev": []},
            "a2": {"v": "confirmed", "ev": []},
        }),
    );
    let p = project([&doc], &c, "t");
    assert_eq!(p.collisions, vec!["a.md#dup".to_string()]);
}

/// A file whose verdicts carry no date: its units are omitted rather
/// than written undated, and the file is named.
#[test]
fn undated_verdicts_are_omitted_and_reported() {
    let doc = parse_document("a.md", "# One {#one}\n\n##a1 Claim. @impl/done\n");
    let c = cached(&doc, "", serde_json::json!({"a1": {"v": "confirmed"}}));
    let p = project([&doc], &c, "t");
    assert!(p.baseline.units.is_empty());
    assert_eq!(p.undated, vec!["a.md".to_string()]);
    assert_eq!(p.omitted, vec!["a.md#one".to_string()]);
}

/// §4.2's writer: identical content is not written, a different
/// `written_at` alone is not content, and a moved verdict is.
#[test]
fn store_writes_only_when_something_moved() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("baseline.json");
    let doc = parse_document("a.md", "# One {#one}\n\n##a1 Claim. @impl/done\n");
    let confirmed = cached(
        &doc,
        "2026-07-25T00:00:00Z",
        serde_json::json!({"a1": {"v": "confirmed", "ev": []}}),
    );

    let first = project([&doc], &confirmed, "t");
    assert!(first.baseline.store(&path).expect("absent"), "absent");
    let on_disk = std::fs::read_to_string(&path).expect("read");

    // A later run of an unchanged tree: only the stamp moved.
    let mut again = project([&doc], &confirmed, "t");
    again.baseline.written_at = "2099-01-01T00:00:00Z".into();
    assert!(!again.baseline.store(&path).expect("stamp only"));
    assert_eq!(
        std::fs::read_to_string(&path).expect("read"),
        on_disk,
        "the old stamp stands, byte for byte"
    );

    // The verdict moves ⇒ the file does.
    let drifted = cached(
        &doc,
        "2026-07-25T00:00:00Z",
        serde_json::json!({"a1": {"v": "drift", "ev": []}}),
    );
    let moved = project([&doc], &drifted, "t");
    assert!(moved.baseline.store(&path).expect("content"));
    assert!(
        std::fs::read_to_string(&path)
            .expect("read")
            .contains("drift")
    );
}

/// A file edited after it was judged is projected — §4.1.2 fixes the
/// hash as the one `rescan` compares — and reported, so a close-out
/// cannot seal a verdict formed against text that has since moved
/// without anyone being told.
#[test]
fn a_file_that_moved_after_its_verdict_is_reported() {
    let doc = parse_document("a.md", "# One {#one}\n\n##a1 Claim. @impl/done\n");
    let mut c = cached(
        &doc,
        "2026-07-25T00:00:00Z",
        serde_json::json!({"a1": {"v": "confirmed", "ev": []}}),
    );
    let campaign = &mut c.files.get_mut("a.md").expect("record").campaign;
    campaign.insert("processed_hash".into(), serde_json::json!("an older state"));

    let p = project([&doc], &c, "t");
    assert_eq!(p.stale, vec!["a.md".to_string()]);
    assert_eq!(
        p.baseline.units["a.md#one"].unit_hash, doc.units[0].content_hash,
        "the hash written is the one rescan compares — the current one"
    );

    // The same file, judged against what is actually there: silence.
    let campaign = &mut c.files.get_mut("a.md").expect("record").campaign;
    campaign.insert(
        "processed_hash".into(),
        serde_json::json!(doc.content_hash.clone()),
    );
    assert!(project([&doc], &c, "t").stale.is_empty());
}

/// A record that carries verdicts and NO `processed_hash` is reported too.
/// The note of what was judged is missing, so the verdicts stand against
/// text nobody wrote down — the same debt as a stale file, and the read
/// must not mistake the absence for a match. It did: `is_some_and` returned
/// false on a missing key and the file passed as fresh, which is the one
/// direction this projection must never fail in.
#[test]
fn a_record_with_no_processed_hash_is_reported_rather_than_assumed_fresh() {
    let doc = parse_document("a.md", "# One {#one}\n\n##a1 Claim. @impl/done\n");
    let c = cached(
        &doc,
        "2026-07-25T00:00:00Z",
        serde_json::json!({"a1": {"v": "confirmed", "ev": []}}),
    );
    assert!(
        !c.files["a.md"].campaign.contains_key("processed_hash"),
        "the fixture must not carry the key this test is about"
    );

    let p = project([&doc], &c, "t");
    assert_eq!(p.stale, vec!["a.md".to_string()]);
    assert!(
        p.undated.is_empty(),
        "it is dated — the missing field is the hash, and the two are reported apart"
    );
}

/// An empty campaign map is an empty baseline, not a failure — and
/// not a baseline of invented verdicts either.
#[test]
fn a_campaign_with_no_verdicts_projects_an_empty_baseline() {
    let doc = parse_document("a.md", "# One {#one}\n\n##a1 Claim. @impl/done\n");
    let p = project([&doc], &Cache::default(), "t");
    assert!(p.baseline.units.is_empty());
    assert_eq!(p.omitted, vec!["a.md#one".to_string()]);
    assert_eq!(p.baseline.schema, BASELINE_SCHEMA);
    assert_eq!(p.baseline.campaign_id, "t");
}
