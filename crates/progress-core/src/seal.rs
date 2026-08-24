//! Sealing — recording that a file's verdicts hold for its current text
//! (PROP-043 §7.1, DRIFT-026).
//!
//! The shape is [`crate::state::record_gate`]'s, and for the same reason:
//! the caller did the real work — re-derived every verdict against the
//! text that is on disk now — and this records that it did. Nothing here
//! computes a verdict, changes one, or invents one. The verb is `seal`
//! rather than `verify` because it verifies nothing, and a name that
//! claimed otherwise is how the next reader would mis-use it.
//!
//! Two hashes decide whether a campaign's staleness warning tells the
//! truth. `content_hash` is what the corpus says now; the campaign map's
//! `processed_hash` is the text the verdicts were formed against. Only a
//! real verify batch used to write the second one, so a campaign that
//! seals verdicts **by hand** — as every sync-from-code wave does — left
//! it pointing at superseded text and the warning fired hardest on the
//! freshest files in the corpus (DRIFT-026 §3).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-047#CMD-SEAL");

use crate::cache::Cache;
use crate::doc::ParsedDoc;
use serde_json::{Value, json};

/// What sealing one file claims: the size of the assertion and the digest
/// transition it records.
///
/// Printed *before* the write, because a seal that silently vouches for
/// three hundred verdicts reads like a no-op in a diff (DRIFT-026 §4.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealClaim {
    /// How many verdicts the record carries. Sealing asserts that **every
    /// one of them** is valid for the text `now` identifies.
    pub verdicts: usize,
    /// The `processed_hash` the record carried before this seal — the text
    /// those verdicts had been formed against. `None` for a record that
    /// never carried one.
    pub was: Option<String>,
    /// The digest of the document being sealed against.
    pub now: String,
}

/// What one [`seal`] decided about one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Seal {
    /// Recorded: every addressable marker carries a verdict, and the
    /// digest moved.
    Recorded(SealClaim),
    /// Nothing to seal — the record already stands for exactly these
    /// bytes. Not a failure: re-sealing must be a no-op, never a fresh
    /// timestamp on an unchanged claim (§4.2 refusal 4).
    Current(SealClaim),
    /// Refused: the file carries markers the campaign never judged. You
    /// cannot vouch for verdicts that do not exist, so the whole file
    /// stays flagged rather than being sealed to the part that was
    /// checked — `processed_hash` is a per-file field and there is no
    /// honest per-anchor half of this claim (§4.2, "considered and
    /// rejected").
    Unjudged {
        /// Verdicts the record does carry, for scale against the gap.
        judged: usize,
        /// Every unjudged anchor, in document order; the caller names the
        /// first few and the count.
        anchors: Vec<String>,
    },
    /// Refused: the campaign cache has no record for this path — it was
    /// never observed, or it left the observed scope. There is nothing
    /// here whose verdicts a seal could speak for.
    Unobserved,
}

/// Decide what sealing `doc` would claim, and record it when the claim
/// holds.
///
/// Verdicts are **recorded, never computed here** — the caller re-derived
/// them against this text and reports that it did. This function re-judges
/// nothing and touches no verdict's `v` or `ev`; the only fields it moves
/// are the two hashes a staleness check compares and the date.
///
/// The digest it records is `doc.content_hash` — the number the caller's
/// **parse of the bytes on disk** produced — and never the `content_hash`
/// the cache already carries. That cached value is refreshed only by a
/// scan, so a seal that trusted it would be comparing one stale number
/// with another and could not see the disk at all: the file edited two
/// minutes ago would read as freshly sealed. That is the second bug
/// DRIFT-026 §3 records, and taking the document rather than the path is
/// what makes it unrepresentable here.
///
/// ```
/// use progress_core::cache::Cache;
/// use progress_core::parse::parse_document;
/// use progress_core::seal::{Seal, seal};
///
/// let doc = parse_document("a.md", "# One {#one}\n\n##a1 A claim. @impl/done\n");
/// let mut cache = Cache::default();
/// cache.upsert(&doc, &progress_core::rollup::rollup_doc(&doc));
/// cache
///     .files
///     .get_mut("a.md")
///     .expect("the record just upserted")
///     .campaign
///     .insert("verdicts".into(), serde_json::json!({"a1": {"v": "confirmed"}}));
///
/// // The caller re-derived that one verdict against this text, and says so.
/// let first = seal(&mut cache, &doc, "2026-07-26T00:00:00Z");
/// assert!(matches!(&first, Seal::Recorded(c) if c.verdicts == 1 && c.now == doc.content_hash));
///
/// // Saying it twice claims nothing new, so nothing is written.
/// let again = seal(&mut cache, &doc, "2026-07-27T00:00:00Z");
/// assert!(matches!(again, Seal::Current(_)));
/// assert_eq!(
///     cache.files["a.md"].campaign["verified_at"],
///     serde_json::json!("2026-07-26T00:00:00Z"),
///     "the first seal's date stands",
/// );
/// ```
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-047#CMD-SEAL"
)]
pub fn seal(cache: &mut Cache, doc: &ParsedDoc, verified_at: &str) -> Seal {
    let Some(record) = cache.files.get_mut(&doc.path) else {
        return Seal::Unobserved;
    };
    let (judged, unjudged) = {
        let map = record.campaign.get("verdicts").and_then(Value::as_object);
        let unjudged: Vec<String> = addressable(doc)
            .filter(|id| !map.is_some_and(|m| m.contains_key(*id)))
            .map(str::to_string)
            .collect();
        (map.map_or(0, serde_json::Map::len), unjudged)
    };
    if !unjudged.is_empty() {
        return Seal::Unjudged {
            judged,
            anchors: unjudged,
        };
    }
    let was = record
        .campaign
        .get("processed_hash")
        .and_then(Value::as_str)
        .map(str::to_string);
    let claim = SealClaim {
        verdicts: judged,
        was: was.clone(),
        now: doc.content_hash.clone(),
    };
    // Both halves are compared against the digest, never against each
    // other: `content_hash == processed_hash` is exactly the answer a
    // cache that has not been scanned since the edit keeps giving, and it
    // is the wrong one.
    if record.content_hash == doc.content_hash && was.as_deref() == Some(doc.content_hash.as_str())
    {
        return Seal::Current(claim);
    }
    record.content_hash = doc.content_hash.clone();
    record
        .campaign
        .insert("processed_hash".into(), json!(doc.content_hash));
    record
        .campaign
        .insert("verified_at".into(), json!(verified_at));
    Seal::Recorded(claim)
}

/// The markers a verdict map can speak about: every fact the marker scan
/// marked that carries an `##<ID>` fact anchor, in document order.
///
/// A standalone document- or section-level marker sits in a block with no
/// facts at all — campaigns file those under a per-file `_elements` bundle,
/// which is why the baseline projection reports such keys as matching no
/// anchor. Marked countable units, including table cells, are addressable by
/// the anchored-when-marked law (`parse::anchors`).
fn addressable(doc: &ParsedDoc) -> impl Iterator<Item = &str> {
    doc.blocks
        .iter()
        .flat_map(|b| b.facts.iter())
        .filter(|f| f.marked)
        .filter_map(|f| f.id.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_document;

    /// Two anchored, marked facts and a campaign map naming `judged`.
    fn fixture(judged: &[&str]) -> (Cache, ParsedDoc) {
        let doc = parse_document(
            "a.md",
            "# One {#one}\n\n##a1 First claim. @impl/done\n\n##a2 Second claim. @impl/done\n",
        );
        assert_eq!(doc.error_count(), 0, "a fixture that obeys the anchor laws");
        let mut cache = Cache::default();
        cache.upsert(&doc, &crate::rollup::rollup_doc(&doc));
        let verdicts: serde_json::Map<String, Value> = judged
            .iter()
            .map(|id| ((*id).to_string(), json!({"v": "confirmed"})))
            .collect();
        cache
            .files
            .get_mut("a.md")
            .expect("the record just upserted")
            .campaign
            .insert("verdicts".into(), Value::Object(verdicts));
        (cache, doc)
    }

    /// §4.2 refusal 1: a marker with no verdict is a verdict that does
    /// not exist, so the file is not sealed and the gap is *named* —
    /// a count alone would leave the operator grepping for which anchor.
    #[test]
    fn an_unjudged_marker_refuses_and_names_it() {
        let (mut cache, doc) = fixture(&["a1"]);
        let before = cache.files["a.md"].clone();

        let Seal::Unjudged { judged, anchors } = seal(&mut cache, &doc, "2026-07-26T00:00:00Z")
        else {
            unreachable!("`a2` carries a marker and no verdict");
        };
        assert_eq!(judged, 1, "the verdicts it does carry, for scale");
        assert_eq!(anchors, vec!["a2".to_string()], "named, not merely counted");

        // A refusal writes nothing at all.
        let after = &cache.files["a.md"];
        assert_eq!(after.content_hash, before.content_hash);
        assert_eq!(after.campaign, before.campaign);
    }

    /// §4.2 refusal 4: the second seal of unchanged text records nothing,
    /// and in particular does not stamp a fresh `verified_at` — a date
    /// that advances while nothing was re-verified is a forged
    /// re-verification.
    #[test]
    fn sealing_twice_is_a_no_op_and_leaves_the_date_alone() {
        let (mut cache, doc) = fixture(&["a1", "a2"]);
        assert!(matches!(
            seal(&mut cache, &doc, "2026-07-26T00:00:00Z"),
            Seal::Recorded(_)
        ));
        let sealed = cache.files["a.md"].clone();

        let again = seal(&mut cache, &doc, "2026-07-27T23:59:59Z");
        assert!(matches!(again, Seal::Current(_)), "{again:?}");
        assert_eq!(
            cache.files["a.md"].campaign, sealed.campaign,
            "the second seal moved neither hash nor date"
        );
    }

    /// The disk test (§6), stated on the seam that decides it: after the
    /// file is edited and **not** rescanned, the cache's own two hashes
    /// still agree with each other and disagree with the disk. A seal
    /// that read `content_hash` would answer "already sealed"; one that
    /// takes the digest from the parse sees the new text.
    #[test]
    fn an_edit_without_a_rescan_is_still_seen() {
        let (mut cache, doc) = fixture(&["a1", "a2"]);
        assert!(matches!(
            seal(&mut cache, &doc, "2026-07-26T00:00:00Z"),
            Seal::Recorded(_)
        ));
        let stale = cache.files["a.md"].content_hash.clone();

        // The file moves; no scan runs, so the record keeps both hashes.
        let edited = parse_document(
            "a.md",
            "# One {#one}\n\n##a1 First claim, reworded. @impl/done\n\n\
             ##a2 Second claim. @impl/done\n",
        );
        assert_ne!(edited.content_hash, stale, "the bytes really moved");
        assert_eq!(
            cache.files["a.md"].content_hash, stale,
            "and the cache has not noticed"
        );

        let Seal::Recorded(claim) = seal(&mut cache, &edited, "2026-07-26T01:00:00Z") else {
            unreachable!("the digest on disk moved, so there is something to seal");
        };
        assert_eq!(claim.was.as_deref(), Some(stale.as_str()));
        assert_eq!(claim.now, edited.content_hash);
        assert_eq!(
            cache.files["a.md"].campaign["verified_at"],
            json!("2026-07-26T01:00:00Z")
        );
    }

    /// The staleness evidence, asserted on the **record** rather than on
    /// the claim returned beside it (F-075).
    ///
    /// The claim is this function's report of what it did; the field is
    /// what a staleness check reads back tomorrow, and the two are only
    /// the same thing for as long as somebody checks. Until DRIFT-026 the
    /// field was written by a verify batch and by nothing else, so a
    /// campaign that hand-seals — as every sync-from-code wave does —
    /// left every file it touched with no recency evidence at all.
    #[test]
    fn seal_writes_the_processed_hash() {
        let (mut cache, doc) = fixture(&["a1", "a2"]);
        assert!(
            !cache.files["a.md"].campaign.contains_key("processed_hash"),
            "the fixture starts with no recency evidence to inherit"
        );

        assert!(matches!(
            seal(&mut cache, &doc, "2026-07-26T00:00:00Z"),
            Seal::Recorded(_)
        ));

        let written = cache.files["a.md"].campaign["processed_hash"]
            .as_str()
            .expect("`processed_hash` is a string");
        assert!(!written.is_empty(), "written, and not written empty");
        assert_eq!(written, doc.content_hash, "the digest of the text sealed");
    }

    /// §4 step 2: the same content, recorded two ways, ends up with the
    /// same digest — a file a batch verified and a file a hand sealed.
    ///
    /// Both numbers come from [`crate::parse::content_hash`], which is the
    /// point and is what this asserts: the third equality names that
    /// function directly, so a second hash implementation growing inside
    /// either path moves one of these values and not the others. Two
    /// digests that agree today and are computed twice are the defect this
    /// field exists to detect, wearing the field's own clothes.
    #[test]
    fn a_hand_seal_and_a_batch_agree_on_the_digest() {
        const TEXT: &str =
            "# One {#one}\n\n##a1 First claim. @impl/done\n\n##a2 Second claim. @impl/done\n";
        let mut cache = Cache::default();
        for path in ["hand.md", "batch.md"] {
            let doc = parse_document(path, TEXT);
            cache.upsert(&doc, &crate::rollup::rollup_doc(&doc));
            let verdicts: serde_json::Map<String, Value> = ["a1", "a2"]
                .iter()
                .map(|id| ((*id).to_string(), json!({"v": "confirmed"})))
                .collect();
            cache
                .files
                .get_mut(path)
                .expect("the record just upserted")
                .campaign
                .insert("verdicts".into(), Value::Object(verdicts));
        }

        // The batch's half: it read the file and recorded the digest of
        // what it read.
        cache
            .files
            .get_mut("batch.md")
            .expect("the batch record")
            .campaign
            .insert(
                "processed_hash".into(),
                json!(crate::parse::content_hash(TEXT)),
            );

        // The hand seal's half, over the same bytes.
        assert!(matches!(
            seal(
                &mut cache,
                &parse_document("hand.md", TEXT),
                "2026-07-26T00:00:00Z"
            ),
            Seal::Recorded(_)
        ));

        assert_eq!(
            cache.files["hand.md"].campaign["processed_hash"],
            cache.files["batch.md"].campaign["processed_hash"],
            "same content, same digest, whichever path recorded it"
        );
        assert_eq!(
            cache.files["hand.md"].campaign["processed_hash"],
            json!(crate::parse::content_hash(TEXT)),
            "and it is the crate's one hash, not a second implementation"
        );
    }

    /// §4.2 refusal 2: a path the campaign never observed has no record,
    /// so there are no verdicts here to speak for — including the case
    /// the wording calls out, a path outside the observed scope.
    #[test]
    fn a_path_with_no_record_refuses() {
        let (mut cache, _) = fixture(&["a1", "a2"]);
        let other = parse_document("elsewhere.md", "# Other {#o}\n\n##b1 Claim. @impl/done\n");
        assert_eq!(
            seal(&mut cache, &other, "2026-07-26T00:00:00Z"),
            Seal::Unobserved
        );
        assert!(
            !cache.files.contains_key("elsewhere.md"),
            "and none is minted"
        );
    }

    /// A standalone document marker has no fact key and does not block a
    /// seal; every marked countable unit, including the cell, is addressable.
    #[test]
    fn standalone_markers_do_not_block_a_seal() {
        let doc = parse_document(
            "a.md",
            "<status stage=\"impl\" state=\"work\"/>\n\n\
             # One {#one}\n\n\
             ##r1 A row. @impl/done\n\n\
             | h |\n| --- |\n| ##c1 cell @impl/done | \n",
        );
        let mut cache = Cache::default();
        cache.upsert(&doc, &crate::rollup::rollup_doc(&doc));
        let judged: Vec<&str> = addressable(&doc).collect();
        assert!(
            judged.iter().all(|id| *id != "_elements"),
            "the document marker is not an addressable anchor: {judged:?}"
        );
        let verdicts: serde_json::Map<String, Value> = judged
            .iter()
            .map(|id| ((*id).to_string(), json!({"v": "confirmed"})))
            .collect();
        cache
            .files
            .get_mut("a.md")
            .expect("the record just upserted")
            .campaign
            .insert("verdicts".into(), Value::Object(verdicts));

        assert!(matches!(
            seal(&mut cache, &doc, "2026-07-26T00:00:00Z"),
            Seal::Recorded(_)
        ));
    }
}
