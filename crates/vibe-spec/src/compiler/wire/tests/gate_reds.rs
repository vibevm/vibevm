//! One typed, non-panicking, non-hanging red for every named conversion gate
//! clause. Each red asserts the gate label the registry pins, so an
//! unimplemented gate cannot hide behind a neighbouring check.

use std::path::PathBuf;

use super::super::{CONVERSION_GATES, IrWireError, decode, encode_compact, widen};

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("formats/corpora/compiler_ir/e1")
}

fn raw(name: &str) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(corpus().join("valid").join(name)).unwrap()).unwrap()
}

fn gate_label(error: &IrWireError) -> Option<&'static str> {
    match error {
        IrWireError::Gate { gate, .. } => Some(gate),
        _ => None,
    }
}

fn assert_gate(value: &serde_json::Value, label: &'static str) {
    let bytes = serde_json::to_vec(value).unwrap();
    let error = decode(&bytes).unwrap_err();
    assert_eq!(
        gate_label(&error),
        Some(label),
        "the red must name its gate, got {error}"
    );
    assert!(
        CONVERSION_GATES.iter().any(|gate| gate.label == label),
        "the label must be in the registry"
    );
}

// ── 1. ir_schema ─────────────────────────────────────────────────────────────

#[test]
fn an_epoch_other_than_one_is_red() {
    for name in [
        "source_document.json",
        "closure_artifact.json",
        "lane_artifact.json",
        "emitted_artifact.json",
    ] {
        let mut document = raw(name);
        document["ir_schema"] = serde_json::json!(2);
        let bytes = serde_json::to_vec(&document).unwrap();
        assert!(
            matches!(decode(&bytes), Err(IrWireError::Schema(2))),
            "{name}"
        );
    }
}

// ── 2. scalar ids ────────────────────────────────────────────────────────────

#[test]
fn blank_or_newline_bearing_ids_are_red() {
    let mut document = raw("source_document.json");
    document["doc"]["format"] = serde_json::json!("  ");
    assert_gate(&document, "scalar-ids");

    let mut document = raw("closure_artifact.json");
    document["closure"]["contributions"][0]["meta"]["origin"] = serde_json::json!("org.demo/lib\n");
    assert_gate(&document, "scalar-ids");

    let mut document = raw("closure_artifact.json");
    document["closure"]["contributions"][1]["document"]["address"]["path"] = serde_json::json!("");
    assert_gate(&document, "scalar-ids");
}

/// The open custom-target vocabulary is owned by the BackendId charset — the
/// ordinary scalar gate, never an `UnsupportedCustomTarget` refusal.
#[test]
fn an_invalid_custom_backend_id_is_refused_by_the_scalar_gate() {
    let mut document = raw("closure_artifact_compat.json");
    document["closure"]["context"]["target"] = serde_json::json!("Demo_Backend!");
    document["closure"]["context"]["artifact"] = serde_json::json!("Demo_Backend!");
    assert_gate(&document, "scalar-ids");
}

// ── 3. the context tuple ─────────────────────────────────────────────────────

#[test]
fn a_context_tuple_that_is_no_row_is_red() {
    let mut document = raw("closure_artifact.json");
    document["closure"]["context"]["artifact"] = serde_json::json!("wrong-id");
    assert_gate(&document, "context-tuple");
}

#[test]
fn a_static_lane_path_without_its_targets_extension_is_red() {
    let mut document = raw("closure_artifact.json");
    document["closure"]["context"]["frame"]["generated_path"] =
        serde_json::json!("vibevm/vibespecs/boot/STATIC.txt");
    assert_gate(&document, "context-tuple");
}

// ── 4. origin/package relation ───────────────────────────────────────────────

#[test]
fn an_origin_that_contradicts_its_target_is_red() {
    let mut document = raw("closure_artifact.json");
    document["closure"]["contributions"][0]["meta"]["origin"] = serde_json::json!("org.demo/other");
    assert_gate(&document, "origin-package-relation");
}

#[test]
fn a_versioned_or_pinned_or_anchored_hoist_is_red() {
    // The raw spelling must stay coherent (that is the address gate's own
    // red), so the pin rides both fields and the HOIST law is what fires.
    let mut document = raw("closure_artifact.json");
    document["closure"]["contributions"][3]["target"]["anchor"] = serde_json::json!(["a"]);
    document["closure"]["contributions"][3]["target"]["pinned_r"] = serde_json::json!(2);
    document["closure"]["contributions"][3]["target"]["raw"] =
        serde_json::json!("spec://org.demo/lib/manual/part.md#a~r2");
    assert_gate(&document, "origin-package-relation");

    let mut document = raw("closure_artifact.json");
    document["closure"]["contributions"][3]["target"]["anchor"] = serde_json::json!(["a"]);
    document["closure"]["contributions"][3]["target"]["raw"] =
        serde_json::json!("spec://org.demo/lib/manual/part.md#a");
    assert_gate(&document, "origin-package-relation");
}

// ── 5. digests and canonical base64 ──────────────────────────────────────────

#[test]
fn digests_must_be_64_lowercase_hex() {
    for mutation in [
        serde_json::json!("ED73521B98C3CAB322C923AFD66C6A5ECBE81A2A24983071F3CA35DE314EA4F8"),
        serde_json::json!("ed73521b98c3cab322c923afd66c6a5ecbe81a2a24983071f3ca35de314ea4"),
        serde_json::json!("not hex at all but long enough to mislead if unchecked ok"),
    ] {
        let mut document = raw("lane_artifact.json");
        document["lane"]["source_link_digest"] = mutation;
        assert_gate(&document, "digest-base64-canonical");
    }
    let mut document = raw("closure_artifact.json");
    document["closure"]["link"]["result"]["input_digest"] =
        serde_json::json!("ed73521b98c3cab322c923afd66c6a5ecbe81a2a24983071f3ca35de314ea4G8");
    assert_gate(&document, "digest-base64-canonical");
}

#[test]
fn emitted_bytes_must_be_canonical_padded_standard_base64() {
    // missing padding / excess padding / interior padding / alphabet /
    // non-zero trailing bits in both padded remainders — every one is red
    // BEFORE any allocation from the decoded length.
    for spelling in ["AP8", "AP8K=", "AP8K==", "AP8=AP8K", "AP!K", "AP9=", "AR=="] {
        let mut document = raw("emitted_artifact.json");
        document["emitted"]["bytes_b64"] = serde_json::json!(spelling);
        assert_gate(&document, "digest-base64-canonical");
    }
}

/// The canonical spellings of the 0-, 1-, 2- and 3-byte remainders all ride,
/// each with the manager's own recomputed digest.
#[test]
fn the_canonical_remainder_spellings_all_ride() {
    use sha2::{Digest, Sha256};
    let frame = |bytes: &[u8]| -> String {
        let domain = b"vibe-spec/emitted-bytes/v1";
        let mut stream = Vec::new();
        stream.extend_from_slice(&(domain.len() as u64).to_le_bytes());
        stream.extend_from_slice(domain);
        stream.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        stream.extend_from_slice(bytes);
        Sha256::digest(&stream)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    };
    for (bytes, canonical) in [
        (Vec::<u8>::new(), ""),
        (vec![0x00], "AA=="),
        (vec![0x00, 0xff], "AP8="),
        (vec![0x00, 0xff, 0x0a], "AP8K"),
    ] {
        let mut document = raw("emitted_artifact.json");
        document["emitted"]["bytes_b64"] = serde_json::json!(String::from(canonical));
        document["emitted"]["provenance"]["bytes_digest"] = serde_json::json!(frame(&bytes));
        decode(&serde_json::to_vec(&document).unwrap())
            .unwrap_or_else(|error| panic!("`{canonical}` is canonical: {error}"));
    }
}

// ── 6. address reparse and fence delimiter ───────────────────────────────────

#[test]
fn raw_address_drift_is_red() {
    let mut document = raw("source_document.json");
    document["doc"]["address"]["address"]["raw"] =
        serde_json::json!("spec://org.demo/lib/manual/other.md#root");
    assert_gate(&document, "address-reparse");
}

#[test]
fn a_fence_delimiter_that_is_not_exactly_one_fence_character_is_red() {
    let mut document = raw("lane_artifact.json");
    document["lane"]["contributions"][0]["chunks"][1]["node"]["fence_after"]["delimiter"] =
        serde_json::json!("``");
    assert_gate(&document, "address-reparse");
}

// ── 7. arena bounds ──────────────────────────────────────────────────────────

#[test]
fn out_of_range_indices_are_red_before_any_indexing() {
    let mut document = raw("document_document.json");
    document["doc"]["tree"]["nodes"][1]["children"] = serde_json::json!([2, 9]);
    assert_gate(&document, "arena-bounds");

    let mut document = raw("document_document.json");
    document["doc"]["tree"]["anchors"]["install"] = serde_json::json!(9);
    assert_gate(&document, "arena-bounds");

    let mut document = raw("closure_artifact.json");
    document["closure"]["edges"][0]["from"] = serde_json::json!(9);
    assert_gate(&document, "arena-bounds");

    let mut document = raw("closure_artifact.json");
    document["closure"]["contributions"][0]["emission_order"][0]["node"] = serde_json::json!(9);
    assert_gate(&document, "arena-bounds");

    let mut document = raw("lane_artifact.json");
    document["lane"]["contributions"][0]["chunks"][1]["node"]["node"] = serde_json::json!(9);
    assert_gate(&document, "arena-bounds");

    let mut document = raw("closure_artifact.json");
    document["closure"]["absorption"]["plan"]["contributions"][0]["seed"] = serde_json::json!(9);
    assert_gate(&document, "arena-bounds");
}

// ── 8. forest ────────────────────────────────────────────────────────────────

#[test]
fn an_empty_arena_is_a_typed_red_not_a_panic() {
    let mut document = raw("document_document.json");
    document["doc"]["tree"]["nodes"] = serde_json::json!([]);
    document["doc"]["tree"]["anchors"] = serde_json::json!({});
    document["doc"]["tree"]["duplicate_anchors"] = serde_json::json!([]);
    assert_gate(&document, "forest");
}

#[test]
fn a_children_cycle_is_reported_never_followed() {
    let mut document = raw("document_document.json");
    document["doc"]["tree"]["nodes"][1]["children"] = serde_json::json!([2, 3, 1]);
    document["doc"]["tree"]["nodes"][1]["parent"] = serde_json::json!(1);
    assert_gate(&document, "forest");
}

#[test]
fn a_detached_ring_that_passes_every_count_is_red() {
    let mut document = raw("document_document.json");
    document["doc"]["tree"]["nodes"][1]["children"] = serde_json::json!([2]);
    document["doc"]["tree"]["nodes"][2]["children"] = serde_json::json!([3]);
    document["doc"]["tree"]["nodes"][2]["parent"] = serde_json::json!(3);
    document["doc"]["tree"]["nodes"][3]["children"] = serde_json::json!([2]);
    document["doc"]["tree"]["nodes"][3]["parent"] = serde_json::json!(2);
    assert_gate(&document, "forest");
}

#[test]
fn a_root_with_a_parent_or_wrong_shape_is_red() {
    let mut document = raw("document_document.json");
    document["doc"]["tree"]["nodes"][0]["parent"] = serde_json::json!(0);
    assert_gate(&document, "forest");

    let mut document = raw("document_document.json");
    document["doc"]["tree"]["nodes"][0]["level"] = serde_json::json!(1);
    assert_gate(&document, "forest");
}

// ── 9. span bounds ───────────────────────────────────────────────────────────

#[test]
fn spans_that_leave_the_document_are_red_before_slicing() {
    let mut document = raw("document_document.json");
    document["doc"]["tree"]["nodes"][1]["span"]["end"] = serde_json::json!(99);
    assert_gate(&document, "span-bounds");

    let mut document = raw("document_document.json");
    document["doc"]["tree"]["nodes"][1]["span"]["start"] = serde_json::json!(11);
    document["doc"]["tree"]["nodes"][1]["span"]["end"] = serde_json::json!(2);
    assert_gate(&document, "span-bounds");

    let mut document = raw("document_document.json");
    document["doc"]["tree"]["nodes"][3]["heading_line"] = serde_json::json!(99);
    assert_gate(&document, "span-bounds");
}

// ── 10. anchor coherence ─────────────────────────────────────────────────────

#[test]
fn an_anchor_that_names_a_foreign_node_is_red() {
    let mut document = raw("document_document.json");
    document["doc"]["tree"]["anchors"]["install"] = serde_json::json!(2);
    assert_gate(&document, "anchor-coherence");
}

#[test]
fn a_duplicate_record_that_does_not_repeat_is_red() {
    let mut document = raw("document_document.json");
    document["doc"]["tree"]["duplicate_anchors"] = serde_json::json!(["nowhere"]);
    assert_gate(&document, "anchor-coherence");
}

// ── 11. set projection ───────────────────────────────────────────────────────

#[test]
fn an_unsorted_or_duplicated_set_projection_is_red() {
    for mutation in [
        serde_json::json!(["spec://z.demo/z/z", "spec://a.demo/a/a"]),
        serde_json::json!(["spec://a.demo/a/a", "spec://a.demo/a/a"]),
    ] {
        let mut document = raw("closure_artifact_compat.json");
        document["closure"]["pending_sources"]["explicit_use_keys"] = mutation;
        assert_gate(&document, "set-projection");
    }
}

// ── 12. absorption witness ───────────────────────────────────────────────────

#[test]
fn a_misaligned_absorption_plan_is_red() {
    let mut document = raw("closure_artifact.json");
    let mut plan = document["closure"]["absorption"]["plan"]["contributions"].take();
    plan.as_array_mut().unwrap().pop();
    document["closure"]["absorption"]["plan"]["contributions"] = plan;
    assert_gate(&document, "absorption-witness");
}

#[test]
fn an_applied_absorption_with_a_pending_snapshot_is_red() {
    let mut document = raw("closure_artifact.json");
    document["closure"]["pending_sources"] = serde_json::json!({
        "discovery_order": [],
        "documents": {},
        "expansions": {},
        "explicit_use_keys": []
    });
    assert_gate(&document, "absorption-witness");
}

// ── 13. link witness and lane bracketing ─────────────────────────────────────

#[test]
fn a_miscounted_link_witness_list_is_red() {
    let mut document = raw("closure_artifact.json");
    let mut witnesses = document["closure"]["link"]["result"]["contributions"].take();
    witnesses.as_array_mut().unwrap().pop();
    document["closure"]["link"]["result"]["contributions"] = witnesses;
    assert_gate(&document, "link-witness-lane");
}

/// A lane whose bracketing breaks is red through the verifier the decode
/// boundary invokes — typed refusal, never a panic.
#[test]
fn a_lane_with_broken_bracketing_is_red() {
    let mut document = raw("lane_artifact.json");
    let mut chunks = document["lane"]["contributions"][0]["chunks"].take();
    chunks.as_array_mut().unwrap().remove(0);
    document["lane"]["contributions"][0]["chunks"] = chunks;
    let bytes = serde_json::to_vec(&document).unwrap();
    let error = decode(&bytes).unwrap_err();
    assert!(
        matches!(
            error,
            IrWireError::Verification(_) | IrWireError::Gate { .. }
        ),
        "a broken bracket must be a typed refusal, got {error}"
    );
}

// ── 14. pass/snapshot ────────────────────────────────────────────────────────

#[test]
fn an_edge_kind_beside_its_own_pending_snapshot_is_red() {
    for kind in ["embed", "source"] {
        let mut document = raw("closure_artifact_compat.json");
        document["closure"]["edges"][0]["kind"] = serde_json::json!(kind);
        assert_gate(&document, "pass-snapshot");
    }
}

// ── 15. emit identity ────────────────────────────────────────────────────────

#[test]
fn an_emit_identity_that_is_not_one_id_is_red() {
    // The tuple row is satisfied (static-md lane), the producer matches the
    // backend, and it is the ONE-ID law that catches the drift: the artifact
    // says `static-md` while the backend says `static-xml`.
    let mut document = raw("emitted_artifact.json");
    document["emitted"]["provenance"]["context"] = serde_json::json!({
        "artifact": "static-md",
        "target": "static-md",
        "frame": {"kind": "static-lane", "generated_path": "vibevm/vibespecs/boot/STATIC.md", "source_root": "vibevm/vibespecs"},
        "mode": "qualify-per-node"
    });
    document["emitted"]["provenance"]["backend"] = serde_json::json!("static-xml");
    document["emitted"]["provenance"]["producer"] = serde_json::json!("emit:static-xml");
    assert_gate(&document, "emit-identity");
}

#[test]
fn a_producer_that_is_not_the_backends_pass_is_red() {
    let mut document = raw("emitted_artifact.json");
    document["emitted"]["provenance"]["producer"] = serde_json::json!("emit:other-test");
    assert_gate(&document, "emit-identity");
}

#[test]
fn a_bytes_digest_that_is_not_the_managers_own_is_red() {
    let mut document = raw("emitted_artifact.json");
    let mut digest: Vec<char> = document["emitted"]["provenance"]["bytes_digest"]
        .as_str()
        .unwrap()
        .chars()
        .collect();
    digest[0] = if digest[0] == 'a' { 'b' } else { 'a' };
    document["emitted"]["provenance"]["bytes_digest"] =
        serde_json::Value::String(digest.into_iter().collect());
    assert_gate(&document, "emit-identity");
}

/// A builtin tape is re-read by its backend in its OWN framing: UTF-8 alone
/// is not enough. static-md must close every reversible marker block it
/// opens; static-xml must be well-formed XML.
#[test]
fn a_utf8_builtin_tape_with_wrong_framing_is_red() {
    use sha2::{Digest, Sha256};
    let frame = |bytes: &[u8]| -> String {
        let domain = b"vibe-spec/emitted-bytes/v1";
        let mut stream = Vec::new();
        stream.extend_from_slice(&(domain.len() as u64).to_le_bytes());
        stream.extend_from_slice(domain);
        stream.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        stream.extend_from_slice(bytes);
        Sha256::digest(&stream)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    };
    let builtin = |target: &str, path: &str, bytes: &[u8], document: &mut serde_json::Value| {
        document["emitted"]["provenance"]["context"] = serde_json::json!({
            "artifact": target,
            "target": target,
            "frame": {"kind": "static-lane", "generated_path": path, "source_root": "vibevm/vibespecs"},
            "mode": "qualify-per-node"
        });
        document["emitted"]["provenance"]["backend"] = serde_json::json!(target);
        document["emitted"]["provenance"]["producer"] = serde_json::json!(format!("emit:{target}"));
        document["emitted"]["bytes_b64"] =
            serde_json::json!(super::super::emitted::encode_base64(bytes));
        document["emitted"]["provenance"]["bytes_digest"] = serde_json::json!(frame(bytes));
    };

    // static-md: an unclosed reversible marker block.
    let mut document = raw("emitted_artifact.json");
    let unclosed: Vec<u8> = b"<!-- vibe:begin spec://org.demo/lib/manual/guide.md#root -->
body
"
    .to_vec();
    builtin(
        "static-md",
        "vibevm/vibespecs/boot/STATIC.md",
        &unclosed,
        &mut document,
    );
    assert_gate(&document, "emit-identity");

    // static-xml: UTF-8, but not well-formed XML.
    let mut document = raw("emitted_artifact.json");
    let not_xml: Vec<u8> = b"not xml at all <".to_vec();
    builtin(
        "static-xml",
        "vibevm/vibespecs/boot/STATIC.xml",
        &not_xml,
        &mut document,
    );
    assert_gate(&document, "emit-identity");
}

/// A builtin tape is re-read by its backend, so it must at least be UTF-8;
/// opaque bytes ride a custom target instead.
#[test]
fn a_builtin_tape_that_is_not_utf8_is_red() {
    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD;
    let mut document = raw("emitted_artifact.json");
    document["emitted"]["provenance"]["context"] = serde_json::json!({
        "artifact": "static-md",
        "target": "static-md",
        "frame": {"kind": "static-lane", "generated_path": "vibevm/vibespecs/boot/STATIC.md", "source_root": "vibevm/vibespecs"},
        "mode": "qualify-per-node"
    });
    document["emitted"]["provenance"]["backend"] = serde_json::json!("static-md");
    document["emitted"]["provenance"]["producer"] = serde_json::json!("emit:static-md");
    document["emitted"]["bytes_b64"] = serde_json::json!(STANDARD.encode([0xC3, 0x28]));
    assert_gate(&document, "emit-identity");
}

// ── encode-side reds ─────────────────────────────────────────────────────────

/// A domain index that cannot fit a u32 refuses rather than truncating.
#[test]
fn an_encode_index_that_cannot_fit_epoch_one_refuses() {
    assert!(widen("test index", usize::MAX).is_err());
    assert!(widen("test index", u32::MAX as usize).is_ok());
}

/// Round-tripping a corpus document twice is stable: the second encode of the
/// first decode re-decodes to the same carrier.
#[test]
fn a_double_round_trip_is_stable() {
    let bytes = std::fs::read(corpus().join("valid").join("closure_artifact_compat.json")).unwrap();
    let first = decode(&bytes).unwrap();
    let encoded = encode_compact(&first).unwrap();
    let second = decode(&encoded).unwrap();
    let re_encoded = encode_compact(&second).unwrap();
    assert_eq!(encoded, re_encoded);
}
