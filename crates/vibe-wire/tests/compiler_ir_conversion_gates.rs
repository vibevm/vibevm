//! The MANDATORY conversion gates the epoch-1 compiler IR wire NAMES but JTD
//! cannot express, plus the structural guards on the schema itself — including
//! the pin on BOTH metadata label sets. Strict domain conversion enforces them before allocating
//! or indexing, on every carrier it is handed; here they run over every corpus
//! document. The carrier and round-trip half lives in
//! `compiler_ir_wire_corpus.rs`; the remaining gates in
//! `compiler_ir_domain_invariants.rs`; the BUILTIN producer oracles, which
//! bind nothing a plugin returns, in `compiler_ir_producer_laws.rs`,
//! `compiler_ir_qualify_oracle.rs` and `compiler_ir_emit_and_forest.rs`.

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::Serialize;
use serde::de::DeserializeOwned;
use vibe_wire::generated::compiler_ir::e1::ir::{
    ClosureContribution, ClosureIr, EmissionContributionWitness, Ir, LinkState,
};

const SIX: [&str; 6] = [
    "source-document",
    "document-document",
    "documents-artifact",
    "closure-artifact",
    "lane-artifact",
    "emitted-artifact",
];

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("formats/corpora/compiler_ir/e1")
}

fn read_valid<T: DeserializeOwned + Serialize>(name: &str) -> T {
    let path = corpus().join("valid").join(name);
    let bytes = std::fs::read(&path).unwrap();
    let authored: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let value: T = serde_json::from_value(authored.clone()).unwrap();
    let round_trip = serde_json::to_value(&value).unwrap();
    assert_eq!(
        round_trip,
        authored,
        "{} loses data on generated round-trip",
        path.display()
    );
    value
}

fn read_invalid(name: &str) -> serde_json::Value {
    let bytes = std::fs::read(corpus().join("invalid").join(name)).unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn valid_names() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(corpus().join("valid"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn reader_rejects_redundancy_and_unknown_fields_and_shapes() {
    for name in [
        "level_mismatch.json",
        "cardinality_mismatch.json",
        "unknown_shape.json",
        "unknown_field.json",
    ] {
        let value = read_invalid(name);
        assert!(
            serde_json::from_value::<Ir>(value).is_err(),
            "{name} unexpectedly parsed: level/cardinality/shape/unknown-field must be red"
        );
    }
}

// ── ARENA and SPAN BOUNDS, over every tree in every corpus document ─────────

fn trees(value: &serde_json::Value, out: &mut Vec<serde_json::Value>) {
    match value {
        serde_json::Value::Object(map) => {
            if [
                "nodes",
                "anchors",
                "duplicate_anchors",
                "lines",
                "directives",
            ]
            .iter()
            .all(|key| map.contains_key(*key))
            {
                out.push(value.clone());
            }
            for child in map.values() {
                trees(child, out);
            }
        }
        serde_json::Value::Array(items) => items.iter().for_each(|item| trees(item, out)),
        _ => {}
    }
}

#[test]
fn every_corpus_tree_satisfies_the_arena_and_span_gates() {
    let mut total = 0;
    for name in valid_names() {
        let document: serde_json::Value =
            serde_json::from_slice(&std::fs::read(corpus().join("valid").join(&name)).unwrap())
                .unwrap();
        let mut found = Vec::new();
        trees(&document, &mut found);
        for tree in &found {
            total += 1;
            let nodes = tree["nodes"].as_array().unwrap();
            let lines = tree["lines"].as_array().unwrap().len() as u64;
            for (index, node) in nodes.iter().enumerate() {
                let (start, end) = (
                    node["span"]["start"].as_u64().unwrap(),
                    node["span"]["end"].as_u64().unwrap(),
                );
                assert!(start <= end && end <= lines, "{name}: span out of range");
                if index > 0 {
                    assert!(lines > 0 && node["heading_line"].as_u64().unwrap() < lines);
                    assert!((node["parent"].as_u64().unwrap() as usize) < nodes.len());
                }
                for child in node["children"].as_array().unwrap() {
                    let child = child.as_u64().unwrap() as usize;
                    assert!(child > 0 && child < nodes.len(), "{name}: child index");
                }
            }
            for (anchor, index) in tree["anchors"].as_object().unwrap() {
                let index = index.as_u64().unwrap() as usize;
                assert!(index < nodes.len(), "{name}: anchor index");
                assert_eq!(nodes[index]["id"], serde_json::json!(anchor));
            }
            let ids: BTreeSet<&str> = nodes
                .iter()
                .filter_map(|node| node["id"].as_str())
                .collect();
            for duplicate in tree["duplicate_anchors"].as_array().unwrap() {
                assert!(
                    ids.contains(duplicate.as_str().unwrap()),
                    "{name}: duplicate"
                );
            }
        }
    }
    assert!(
        total >= 8,
        "the gates ran over every corpus tree, not a few"
    );
}

fn assert_digest(value: &str) {
    assert_eq!(value.len(), 64, "digest is 64 hex chars: {value}");
    assert!(
        value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "digest is lowercase hex: {value}"
    );
}

fn assert_id(value: &str) {
    assert!(!value.trim().is_empty(), "an id must not be blank");
    assert!(
        !value.contains(['\n', '\r', '\0']),
        "an id must not carry a newline or NUL: {value}"
    );
}

fn assert_closure_indices(closure: &ClosureIr) {
    let nodes = closure.nodes.len() as u32;
    for edge in &closure.edges {
        assert!(
            edge.from < nodes && edge.to < nodes,
            "edge indexes the arena"
        );
    }
    for contribution in &closure.contributions {
        match contribution {
            ClosureContribution::Normal(normal) => {
                assert!(normal.seed < nodes);
                for entry in &normal.emission_order {
                    assert!(entry.node < nodes, "occurrence indexes the arena");
                }
                assert_id(&normal.meta.origin);
            }
            ClosureContribution::Simple(simple) => assert_id(&simple.document.origin),
            ClosureContribution::Elided(elided) => assert_id(&elided.meta.path),
            ClosureContribution::Hoisted(hoisted) => {
                assert!(hoisted.target.anchor.is_empty() && hoisted.target.pinned_r.is_none())
            }
        }
    }
    if let LinkState::Linked(link) = &closure.link {
        assert_digest(&link.result.input_digest);
    }
}

#[test]
fn digests_ids_and_indices_pass_the_named_conversion_gates() {
    for name in valid_names() {
        match read_valid::<Ir>(&name) {
            Ir::ClosureArtifact(arm) => assert_closure_indices(&arm.closure),
            Ir::LaneArtifact(arm) => {
                assert_digest(&arm.lane.source_link_digest);
                assert_id(arm.lane.context.artifact.as_str());
            }
            Ir::EmittedArtifact(arm) => {
                let provenance = &arm.emitted.provenance;
                // Opaque bytes ride the custom backend; `emit.rs::prepare_target`
                // gives a builtin a Markdown/XML tape the pass re-reads.
                assert_eq!(provenance.backend, "opaque-test");
                assert_eq!(provenance.producer, "emit:opaque-test");
                assert_id(&provenance.backend);
                assert_digest(&provenance.source_lane_digest);
                assert_digest(&provenance.bytes_digest);
                for witness in &provenance.contributions {
                    match witness {
                        EmissionContributionWitness::Normal(inner) => {
                            assert_digest(&inner.chunk_digest)
                        }
                        EmissionContributionWitness::Simple(inner) => {
                            assert_digest(&inner.chunk_digest)
                        }
                        _ => {}
                    }
                }
            }
            Ir::SourceDocument(arm) => assert_id(&arm.doc.format),
            Ir::DocumentDocument(arm) => assert_id(&arm.doc.source.format),
            Ir::DocumentsArtifact(arm) => {
                arm.documents
                    .iter()
                    .for_each(|entry| assert_id(&entry.source.format));
            }
        }
    }
}

// ── Canonical base64 for opaque emitted bytes ────────────────────────────────
// The house precedent is the tiny codec in `vibe-registry`'s full_scan, and
// `--locked` keeps the tree free of a new direct dependency. Decode is strict;
// canonicality is proven by re-encoding.

fn b64_val(c: u8) -> Result<u32, &'static str> {
    match c {
        b'A'..=b'Z' => Ok((c - b'A') as u32),
        b'a'..=b'z' => Ok((c - b'a' + 26) as u32),
        b'0'..=b'9' => Ok((c - b'0' + 52) as u32),
        b'+' => Ok(62),
        b'/' => Ok(63),
        _ => Err("invalid character"),
    }
}

fn decode_base64(input: &str) -> Result<Vec<u8>, &'static str> {
    let bytes = input.as_bytes();
    if !bytes.len().is_multiple_of(4) {
        return Err("length not multiple of 4");
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    let mut i = 0;
    while i < bytes.len() {
        let q = &bytes[i..i + 4];
        let pad0 = q[2] == b'=';
        let pad1 = q[3] == b'=';
        if (pad0 || pad1) && i + 4 != bytes.len() {
            return Err("padding before the final quad is not canonical");
        }
        let v0 = b64_val(q[0])?;
        let v1 = b64_val(q[1])?;
        let v2 = if pad0 { 0 } else { b64_val(q[2])? };
        let v3 = if pad1 { 0 } else { b64_val(q[3])? };
        if pad0 && !pad1 {
            return Err("one-character padding is not canonical");
        }
        let n = (v0 << 18) | (v1 << 12) | (v2 << 6) | v3;
        out.push((n >> 16) as u8);
        if !pad0 {
            out.push(((n >> 8) & 0xff) as u8);
        }
        if !pad1 {
            out.push((n & 0xff) as u8);
        }
        i += 4;
    }
    Ok(out)
}

fn encode_base64(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

#[test]
fn emitted_bytes_are_canonical_base64_and_round_trip_non_utf8() {
    let ir = read_valid::<Ir>("emitted_artifact.json");
    let Ir::EmittedArtifact(arm) = &ir else {
        panic!("emitted_artifact.json must be the emitted-artifact shape");
    };
    let authored = &arm.emitted.bytes_b64;
    let bytes = decode_base64(authored).expect("corpus bytes_b64 is canonical base64");
    assert_eq!(
        encode_base64(&bytes),
        *authored,
        "base64 must be canonical standard with padding"
    );
    assert!(
        std::str::from_utf8(&bytes).is_err(),
        "corpus bytes deliberately include non-UTF-8 sequences"
    );

    // A non-UTF-8 payload built here survives the typed wire in both
    // directions: encode → serialise the generated type → parse → decode.
    let payload: Vec<u8> = vec![0xC3, 0x28, 0xFF, 0xFE, 0x00, 0x7F, 0x0D, 0x0A];
    let encoded = encode_base64(&payload);
    let mut value = serde_json::to_value(&ir).unwrap();
    value["emitted"]["bytes_b64"] = serde_json::Value::String(encoded);
    let round: Ir = serde_json::from_value(value).unwrap();
    let Ir::EmittedArtifact(round_arm) = &round else {
        panic!("the emitted shape survives the round-trip");
    };
    assert_eq!(
        decode_base64(&round_arm.emitted.bytes_b64).unwrap(),
        payload
    );
}

#[test]
fn emitted_bytes_that_are_not_base64_are_red_at_the_bytes_gate() {
    // A string is a legal JTD value, so the generated reader accepts it —
    // the red is the base64 gate the corpus reader (and R6.2b conversion)
    // applies to the opaque-bytes field.
    let value = read_invalid("emitted_bytes_not_base64.json");
    let parsed = serde_json::from_value::<Ir>(value)
        .expect("the wire type itself parses; the bytes gate is separate");
    let Ir::EmittedArtifact(arm) = &parsed else {
        panic!("the not-base64 fixture must be the emitted shape");
    };
    assert!(decode_base64(&arm.emitted.bytes_b64).is_err());
}

// ── The schema is one closed contract ────────────────────────────────────────

/// The leading ALL-CAPS label of each metadata entry that carries one, sorted.
fn labels_of(entries: &[serde_json::Value]) -> Vec<String> {
    let mut labels: Vec<String> = entries
        .iter()
        .filter_map(|entry| {
            let label: String = entry
                .as_str()
                .unwrap()
                .chars()
                .take_while(|c| c.is_ascii_uppercase() || *c == ' ' || *c == '/')
                .collect();
            let label = label.trim_end().to_string();
            (label.len() > 2).then_some(label)
        })
        .collect();
    labels.sort();
    labels
}

fn schema_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("schemas/compiler_ir/e1/ir.jtd.json")
}

#[test]
fn schema_pins_one_root_six_carriers_and_named_conversion_gates() {
    let text = std::fs::read_to_string(schema_path()).unwrap();
    let doc: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(doc["discriminator"], "shape", "one root discriminator");
    let mut keys: Vec<&str> = doc["mapping"]
        .as_object()
        .expect("mapping is an object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    let mut expected = SIX;
    expected.sort_unstable();
    assert_eq!(keys, expected, "exactly the six carrier mappings");

    assert!(!text.contains("additionalProperties"), "not a JTD key");
    assert!(!text.contains("\"values\": {}"), "no untyped catch-all map");
    assert!(!text.contains("\"type\": {}"), "no anything scalar");
    assert!(
        !text.contains("nullable"),
        "absence is optionalProperties, never null"
    );

    // The semantic gates JTD cannot express are NAMED, not dropped — and the
    // ones whose absence is a panic rather than an error are named too. The
    // gate list IS the strict conversion contract, so the labelled set is pinned
    // EXACTLY: dropping or renaming one silently narrows what conversion owes.
    // The BUILTIN producer characterization is a second, separately pinned set
    // — a decoder that enforced it would reject a verifier-valid closure a
    // plugin transformed and returned.
    let gates = doc["metadata"]["x-conversion-gates"]
        .as_array()
        .expect("the root names its conversion gates");
    let joined = gates
        .iter()
        .map(|gate| gate.as_str().unwrap())
        .collect::<Vec<_>>()
        .join(" | ");
    let oracles = doc["metadata"]["x-corpus-producer-oracles"]
        .as_array()
        .expect("the root names its corpus producer oracles separately");
    assert_eq!(
        labels_of(gates),
        [
            "ARENA BOUNDS",
            "EMIT IDENTITY",
            "FOREST",
            "PASS/SNAPSHOT",
            "SET PROJECTION",
            "SPAN BOUNDS",
        ],
        "the named gate set is what the decoder OWES on every carrier"
    );
    assert_eq!(gates.len(), 15, "and the unlabelled gates are still there");
    assert_eq!(
        labels_of(oracles),
        ["CLOSE ORDER", "OPAQUE TAPE", "QUALIFY SPELLING"],
        "the oracle set is characterization of THIS corpus's builtin passes"
    );
    assert_eq!(oracles.len(), 3);
    for required in ["heading_line", "base64", "64 lowercase hex", "non-blank"] {
        assert!(joined.contains(required), "a gate names {required}");
    }
    // A gate must not read as a producer claim, and an oracle must both cite
    // the builtin it characterises and say that it does — the layering is the
    // point of this split, so it is asserted rather than assumed.
    let text_of = |set: &[serde_json::Value], label: &str| -> String {
        set.iter()
            .map(|entry| entry.as_str().unwrap())
            .find(|entry| entry.starts_with(label))
            .unwrap_or_else(|| panic!("{label} is missing"))
            .to_string()
    };
    for (label, cite) in [
        ("FOREST", "LOOP FOREVER"),
        ("PASS/SNAPSHOT", "pending_embeds"),
        ("EMIT IDENTITY", "independent_bytes_digest"),
        ("SET PROJECTION", "BTreeSet<String>"),
    ] {
        assert!(text_of(gates, label).contains(cite));
    }
    for (label, cite) in [
        ("CLOSE ORDER", "topology::order_by"),
        ("QUALIFY SPELLING", "node_qualification_origin"),
        ("OPAQUE TAPE", "opaque_test_vehicle"),
    ] {
        let entry = text_of(oracles, label);
        assert!(entry.contains(cite), "oracle {label} must cite {cite}");
        assert!(
            entry.contains("BUILTIN") || entry.contains("THIS corpus"),
            "oracle {label} must say it characterises the builtin, not the decoder"
        );
    }
    assert!(
        doc["metadata"]["description"]
            .as_str()
            .unwrap()
            .contains("must not be confused"),
        "the root says why the two metadata sets are separate"
    );
    assert_eq!(
        doc["definitions"]["span"]["metadata"]["x-conversion"]
            .as_str()
            .map(|note| note.contains("panics")),
        Some(true),
        "the span gate says why it must run before slicing"
    );

    // Every enum site is annotated open/closed; the only open vocabulary is
    // the artifact target (a registered custom backend names itself).
    fn walk(value: &serde_json::Value, enums: &mut usize, open_sites: &mut usize) {
        let Some(object) = value.as_object() else {
            return;
        };
        if object.contains_key("enum") {
            *enums += 1;
            let annotation = &object["metadata"]["x-vocabulary"];
            assert!(
                annotation == "open" || annotation == "closed",
                "every enum site carries x-vocabulary"
            );
            if annotation == "open" {
                *open_sites += 1;
            }
        }
        for child in object.values() {
            walk(child, enums, open_sites);
        }
    }
    let mut enums = 0;
    let mut open_sites = 0;
    walk(&doc, &mut enums, &mut open_sites);
    // 5 level + 2 cardinality definitions, compile_mode, artifact_target,
    // and the three inline kind vocabularies (doc node, directive, edge).
    assert_eq!(
        enums, 12,
        "the closed vocabularies are exactly the known set"
    );
    assert_eq!(
        open_sites, 1,
        "exactly one open vocabulary: artifact_target"
    );
    let target_contract = doc["definitions"]["artifact_target"]["metadata"]["description"]
        .as_str()
        .unwrap();
    assert!(
        target_contract.contains("R6.2b") && target_contract.contains("R6.3"),
        "open target identity lands with strict conversion; registration and invocation remain R6.3"
    );

    // Every collection member declares its empty-collection policy.
    fn walk_collections(value: &serde_json::Value) {
        let Some(object) = value.as_object() else {
            return;
        };
        for block in ["properties", "optionalProperties"] {
            if let Some(members) = object.get(block).and_then(|v| v.as_object()) {
                for member in members.values() {
                    let Some(form) = member.as_object() else {
                        continue;
                    };
                    if form.contains_key("elements") || form.contains_key("values") {
                        assert_eq!(
                            form["metadata"]["x-empty"], "emit",
                            "a collection member declares x-empty"
                        );
                    }
                }
            }
        }
        for child in object.values() {
            walk_collections(child);
        }
    }
    walk_collections(&doc);
}

#[test]
fn generated_reader_is_strict_with_no_open_catch_all() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/generated/compiler_ir/e1/ir/mod.rs");
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(
        text.contains("#[serde(tag = \"shape\")]"),
        "the root is the tagged union"
    );
    assert!(
        text.contains("#[serde(deny_unknown_fields)]"),
        "the reader is strict"
    );
    assert!(
        !text.contains("serde_json"),
        "no JSON-value parsing in generated types"
    );
    assert!(!text.contains("Value"), "no open catch-all field");
    assert!(
        text.contains("pub enum ArtifactTarget") && text.contains("Unknown(String)"),
        "the one open vocabulary is the typed structural opening"
    );
}

// ── Small kind projections, so the assertions above read as tables ──────────
