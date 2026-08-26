//! Two MANDATORY conversion gates — FOREST and EMIT IDENTITY — and ONE builtin
//! producer oracle, OPAQUE TAPE. The three are kept in one file because the
//! tape and the identity share a decoder; the labels below say which is which,
//! and they are not interchangeable.
//!
//! * GATE   FOREST (`x-conversion-gates`) — the node arena is a forest rooted
//!          at 0, checked totally and iteratively. Every decoded carrier owes
//!          it: a `children` cycle spins `DocTree::facts_under` forever
//!          instead of panicking, whoever produced the tree.
//! * GATE   EMIT IDENTITY (`x-conversion-gates`) — artifact/target/backend are
//!          one id, `producer` is `emit:<backend>`, a builtin tape is valid in
//!          its own framing while opaque bytes ride a custom target, and
//!          `bytes_digest` is RECOMPUTED. Says nothing about which bytes.
//! * ORACLE OPAQUE TAPE (`x-corpus-producer-oracles`) — the fixed three bytes
//!          `OpaqueTestBackend::new` returns. Characterization of THIS corpus,
//!          never a decode rule.
//!
//! The other two gates, PASS/SNAPSHOT and SET PROJECTION, are in
//! `compiler_ir_domain_invariants.rs`; the CLOSE ORDER and QUALIFY SPELLING
//! oracles in `compiler_ir_producer_laws.rs` and
//! `compiler_ir_qualify_oracle.rs`.

use std::path::PathBuf;

use serde::Serialize;
use serde::de::DeserializeOwned;
use vibe_wire::generated::compiler_ir::e1::ir::Ir;

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("formats/corpora/compiler_ir/e1")
}

fn valid_names() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(corpus().join("valid"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn raw(name: &str) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(corpus().join("valid").join(name)).unwrap()).unwrap()
}

fn typed<T: DeserializeOwned + Serialize>(value: &serde_json::Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}

// ── GATE · EMIT IDENTITY, with the OPAQUE TAPE oracle beside it ──────────────

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// SHA-256, spelled out here rather than added as a dependency: `--locked`
/// keeps the tree free of a new dev-dependency edge, and the vector test below
/// proves this implementation before any assertion rests on it.
fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut message = data.to_vec();
    let bits = (data.len() as u64) * 8;
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bits.to_be_bytes());
    for block in message.chunks(64) {
        let mut w = [0u32; 64];
        for (index, word) in block.chunks(4).enumerate() {
            w[index] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for index in 16..64 {
            let x = w[index - 15];
            let y = w[index - 2];
            let s0 = x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3);
            let s1 = y.rotate_right(17) ^ y.rotate_right(19) ^ (y >> 10);
            w[index] = w[index - 16]
                .wrapping_add(s0)
                .wrapping_add(w[index - 7])
                .wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d) = (h[0], h[1], h[2], h[3]);
        let (mut e, mut f, mut g, mut acc) = (h[4], h[5], h[6], h[7]);
        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choose = (e & f) ^ ((!e) & g);
            let t1 = acc
                .wrapping_add(s1)
                .wrapping_add(choose)
                .wrapping_add(K[index])
                .wrapping_add(w[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let major = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(major);
            acc = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (slot, value) in h.iter_mut().zip([a, b, c, d, e, f, g, acc]) {
            *slot = slot.wrapping_add(value);
        }
    }
    let mut out = [0u8; 32];
    for (index, word) in h.iter().enumerate() {
        out[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// `emit/validate.rs::independent_bytes_digest` — the manager's own,
/// domain-separated digest of the emitted tape, recomputed at the backend
/// boundary and refused on mismatch (`common_current`, `:373`).
fn independent_bytes_digest(bytes: &[u8]) -> String {
    let domain = b"vibe-spec/emitted-bytes/v1";
    let mut framed = Vec::new();
    framed.extend_from_slice(&(domain.len() as u64).to_le_bytes());
    framed.extend_from_slice(domain);
    framed.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    framed.extend_from_slice(bytes);
    hex(&sha256(&framed))
}

fn decode_base64(input: &str) -> Vec<u8> {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = input.as_bytes();
    assert!(bytes.len().is_multiple_of(4), "base64 length");
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for quad in bytes.chunks(4) {
        let pad = quad.iter().filter(|byte| **byte == b'=').count();
        let mut value = 0u32;
        for byte in quad {
            let index = if *byte == b'=' {
                0
            } else {
                ALPHABET
                    .iter()
                    .position(|c| c == byte)
                    .expect("base64 char") as u32
            };
            value = (value << 6) | index;
        }
        out.push((value >> 16) as u8);
        if pad < 2 {
            out.push(((value >> 8) & 0xff) as u8);
        }
        if pad < 1 {
            out.push((value & 0xff) as u8);
        }
    }
    out
}

#[test]
fn the_local_sha256_matches_the_published_vectors() {
    assert_eq!(
        hex(&sha256(b"")),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        hex(&sha256(b"abc")),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    // 56 bytes forces the extra padding block; 1000 exercises many blocks.
    assert_eq!(
        hex(&sha256(&[b'a'; 56])),
        "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a"
    );
    assert_eq!(
        hex(&sha256(&[b'a'; 1000])),
        "41edece42d63e8d9bf515a9ba6932e1c20cbc9f5a5d134645adb5db1b9737ea3"
    );
}

/// ORACLE (`x-corpus-producer-oracles` · OPAQUE TAPE) — the EXACT tape
/// `OpaqueTestBackend::new` emits (`emit/opaque_test_vehicle.rs` —
/// `vec![0x00, 0xff, b'\n']`), its identity, and the manager digest of those
/// bytes. Pinned as constants so the golden is checked against the PRODUCER,
/// not merely against itself: a self-consistent `base64` + digest-of-whatever
/// pair would pass a recompute-only test while carrying bytes no backend emits.
///
/// This is characterization of THIS corpus, never a decode rule. Any tape whose
/// EMIT IDENTITY coherence holds — one id tuple, `emit:<backend>`, and
/// `bytes_digest == independent_bytes_digest(bytes)` — is a valid carrier, and
/// that coherence is the conversion gate asserted separately below.
const OPAQUE_BACKEND: &str = "opaque-test";
const OPAQUE_TAPE: [u8; 3] = [0x00, 0xff, 0x0a];
const OPAQUE_DIGEST: &str = "b7f8278a784ffdd290fe40ad7615d0f7d577e72fdfe120d0aa529bfc93c7ef83";

#[test]
fn the_emitted_tape_is_the_landed_backends_own_output() {
    let document = raw("emitted_artifact.json");
    let Ir::EmittedArtifact(arm) = typed::<Ir>(&document) else {
        panic!("emitted_artifact.json must be the emitted-artifact shape");
    };
    let provenance = &arm.emitted.provenance;
    assert_eq!(provenance.backend, OPAQUE_BACKEND);
    assert_eq!(provenance.producer, format!("emit:{OPAQUE_BACKEND}"));
    assert_eq!(decode_base64(&arm.emitted.bytes_b64), OPAQUE_TAPE.to_vec());
    assert_eq!(provenance.bytes_digest, OPAQUE_DIGEST);
    // The pinned constant and the recomputed digest agree, so neither can
    // drift alone.
    assert_eq!(independent_bytes_digest(&OPAQUE_TAPE), OPAQUE_DIGEST);
}

/// GATE (`x-conversion-gates` · EMIT IDENTITY) — the semantic coherence every
/// decoded carrier owes, whoever produced the bytes: one artifact/target/
/// backend id, `producer` = `emit:<backend>`, opaque bytes only under a custom
/// target's compatibility frame, and a `bytes_digest` the decoder RECOMPUTES
/// rather than trusts. Says nothing about which bytes.
#[test]
fn emitted_identity_is_one_tuple_and_the_digest_is_the_managers_own() {
    let document = raw("emitted_artifact.json");
    let Ir::EmittedArtifact(arm) = typed::<Ir>(&document) else {
        panic!("emitted_artifact.json must be the emitted-artifact shape");
    };
    let provenance = &arm.emitted.provenance;
    let backend = provenance.backend.as_str();
    let context = &document["emitted"]["provenance"]["context"];
    assert_eq!(provenance.producer, format!("emit:{backend}"));
    assert_eq!(context["artifact"].as_str(), Some(backend));
    assert_eq!(context["target"].as_str(), Some(backend));

    let bytes = decode_base64(&arm.emitted.bytes_b64);
    // Opaque bytes ride a CUSTOM target under the compatibility frame: a
    // builtin's tape is re-read by `markdown_observation` / `xml::observation`,
    // and this one is not even UTF-8.
    assert!(backend != "static-md" && backend != "static-xml");
    assert_eq!(
        context["frame"]["kind"].as_str(),
        Some("compatibility-fragment")
    );
    assert!(std::str::from_utf8(&bytes).is_err());
    assert_eq!(provenance.bytes_digest, independent_bytes_digest(&bytes));
}

#[test]
fn a_mutated_tape_or_digest_is_red() {
    let document = raw("emitted_artifact.json");
    let Ir::EmittedArtifact(arm) = typed::<Ir>(&document) else {
        panic!("emitted_artifact.json must be the emitted-artifact shape");
    };
    let bytes = decode_base64(&arm.emitted.bytes_b64);

    // One flipped tape byte, digest untouched: the manager recomputes and refuses.
    let mut flipped = bytes.clone();
    flipped[0] ^= 0x01;
    assert_ne!(
        independent_bytes_digest(&flipped),
        arm.emitted.provenance.bytes_digest
    );

    // One flipped digest nibble, tape untouched: the same refusal.
    let mut digest: Vec<char> = arm.emitted.provenance.bytes_digest.chars().collect();
    digest[0] = if digest[0] == 'a' { 'b' } else { 'a' };
    let mutated: String = digest.into_iter().collect();
    assert_ne!(mutated, independent_bytes_digest(&bytes));

    // The placeholder the repaired corpus replaced was not this digest either.
    assert_ne!(
        "a5ff3eccaf7b6233944f9e01d609e9e7b1c2d0d1eb32953717c71c227f86b54c",
        independent_bytes_digest(&bytes),
        "the retired placeholder must stay red"
    );

    // A foreign opaque tape — one byte off the landed backend's own — is red
    // against the pinned digest even though it is perfectly good base64.
    assert_ne!(independent_bytes_digest(&[0x00, 0xfe, 0x0a]), OPAQUE_DIGEST);
}

// ── The DocTree forest, checked iteratively ──────────────────────────────────

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
            map.values().for_each(|child| trees(child, out));
        }
        serde_json::Value::Array(items) => items.iter().for_each(|item| trees(item, out)),
        _ => {}
    }
}

/// Bounds are not enough, and the law is TOTAL — no input reaches an index.
/// An EMPTY arena is a typed violation here rather than the panic
/// `DocTree::root()` would take (`parse` always mints the synthetic root);
/// node 0 must BE that root — level 0, kind `heading`, no `parent`, no
/// incoming edge; every other node has exactly one incoming edge, claims that
/// parent back, and is reachable. `DocTree::facts_under` pops a stack of
/// `children`, so a cycle makes it LOOP FOREVER rather than panic — the walk
/// below carries its own step bound and a visited set, and a detached ring
/// passes every count while no walk can reach it.
fn forest_violations(tree: &serde_json::Value) -> Vec<String> {
    let nodes = tree["nodes"].as_array().unwrap();
    let mut out = Vec::new();
    if nodes.is_empty() {
        return vec!["the node arena is empty; `DocTree` always carries its root".to_string()];
    }
    if nodes[0].get("parent").is_some() {
        out.push("the root carries a parent".to_string());
    }
    if nodes[0]["level"].as_u64() != Some(0) || nodes[0]["kind"].as_str() != Some("heading") {
        out.push("the root is not the synthetic level-0 heading".to_string());
    }
    let mut incoming = vec![0usize; nodes.len()];
    for (index, node) in nodes.iter().enumerate() {
        for child in node["children"].as_array().unwrap() {
            let child = child.as_u64().unwrap() as usize;
            if child == 0 || child >= nodes.len() {
                out.push(format!("child index {child} out of range"));
                continue;
            }
            if nodes[child]["parent"].as_u64() != Some(index as u64) {
                out.push(format!("child {child} does not claim parent {index}"));
            }
            incoming[child] += 1;
        }
    }
    if incoming[0] != 0 {
        out.push("the root has an incoming child edge".to_string());
    }
    for (index, count) in incoming.iter().enumerate().skip(1) {
        if *count != 1 {
            out.push(format!("node {index} has {count} incoming edges, not one"));
        }
    }
    let mut seen = vec![false; nodes.len()];
    let mut stack = vec![0usize];
    let mut steps = 0usize;
    while let Some(index) = stack.pop() {
        steps += 1;
        if steps > nodes.len() + 1 {
            out.push("the forest walk did not terminate".to_string());
            break;
        }
        if seen[index] {
            out.push(format!("child cycle at node {index}"));
            break;
        }
        seen[index] = true;
        for child in nodes[index]["children"].as_array().unwrap() {
            let child = child.as_u64().unwrap() as usize;
            if child < nodes.len() {
                stack.push(child);
            }
        }
    }
    if out.is_empty() && seen.iter().any(|reached| !reached) {
        out.push("an arena node is unreachable from the root".to_string());
    }
    out
}

#[test]
fn every_corpus_doc_tree_is_a_forest() {
    let mut total = 0;
    for name in valid_names() {
        let mut found = Vec::new();
        trees(&raw(&name), &mut found);
        for tree in &found {
            total += 1;
            assert_eq!(forest_violations(tree), Vec::<String>::new(), "{name}");
        }
    }
    assert!(total >= 8, "the forest law ran over every corpus tree");
}

#[test]
fn a_child_cycle_is_red_and_does_not_hang() {
    let mut found = Vec::new();
    trees(&raw("document_document.json"), &mut found);
    let tree = &found[0];

    // Node 1's child list points back at its own parent: bounds-legal, and
    // exactly the shape that spins `facts_under` forever.
    let mut cyclic = tree.clone();
    cyclic["nodes"][1]["children"] = serde_json::json!([2, 3, 1]);
    let violations = forest_violations(&cyclic);
    assert!(
        violations.iter().any(|entry| entry.contains("cycle")
            || entry.contains("incoming edges")
            || entry.contains("claim parent")),
        "a child cycle must be red, got {violations:?}"
    );

    // A child that does not claim its parent back is red on its own.
    let mut orphaned = tree.clone();
    orphaned["nodes"][2]["parent"] = serde_json::json!(0);
    assert!(
        forest_violations(&orphaned)
            .iter()
            .any(|entry| entry.contains("claim parent")),
        "a broken parent back-reference must be red"
    );

    // A node no walk reaches is red even with every index in range.
    let mut detached = tree.clone();
    detached["nodes"][1]["children"] = serde_json::json!([2]);
    assert!(
        !forest_violations(&detached).is_empty(),
        "an unreachable arena node must be red"
    );
}

#[test]
fn the_forest_law_is_total() {
    let mut found = Vec::new();
    trees(&raw("document_document.json"), &mut found);
    let tree = &found[0];

    // An EMPTY arena is a typed violation, not the panic `DocTree::root()`
    // would take on `nodes[0]` — the gate must answer before any index.
    let mut empty = tree.clone();
    empty["nodes"] = serde_json::json!([]);
    assert!(
        forest_violations(&empty)
            .iter()
            .any(|entry| entry.contains("arena is empty")),
        "an empty node arena must be red"
    );

    // The synthetic root claiming a parent — index 0 pointing at itself is
    // bounds-legal and every count still balances.
    let mut rooted = tree.clone();
    rooted["nodes"][0]["parent"] = serde_json::json!(0);
    assert!(
        forest_violations(&rooted)
            .iter()
            .any(|entry| entry.contains("root carries a parent")),
        "a root with a parent must be red"
    );

    // A root that is not the synthetic level-0 heading.
    let mut promoted = tree.clone();
    promoted["nodes"][0]["level"] = serde_json::json!(1);
    assert!(
        forest_violations(&promoted)
            .iter()
            .any(|entry| entry.contains("synthetic level-0")),
        "a non-synthetic root must be red"
    );

    // A ring the root cannot reach: every incoming count and back-reference
    // is locally perfect, so only reachability catches it.
    let mut ring = tree.clone();
    ring["nodes"][1]["children"] = serde_json::json!([]);
    ring["nodes"][2]["children"] = serde_json::json!([3]);
    ring["nodes"][2]["parent"] = serde_json::json!(3);
    ring["nodes"][3]["children"] = serde_json::json!([2]);
    ring["nodes"][3]["parent"] = serde_json::json!(2);
    assert!(
        forest_violations(&ring)
            .iter()
            .any(|entry| entry.contains("unreachable")),
        "a detached ring must be red: {:?}",
        forest_violations(&ring)
    );
}
