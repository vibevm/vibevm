//! Gate 15 on REAL built-in emitted artifacts: the value, provenance and
//! tape the production schedule actually produced. A tape is judged against
//! the engine's own prologue builders and the carried emission witnesses, so
//! arbitrary UTF-8 Markdown and generically well-formed XML are both refused.

use specmark::verifies;

use super::super::emitted::{digest_hex, encode_base64};
use super::super::{decode, encode_compact, framing};
use super::fixture::{
    both_targets, compatibility_emitted, emit, hoisted_plan, plan_for, two_root_plan,
};
use crate::compiler::emit::emitted_bytes_digest;
use crate::compiler::ir::{ArtifactTarget, EmittedArtifact, StaticCompileMode};
use crate::compiler::pass::AnyIr;

/// The real artifact's own wire bytes, with the tape replaced and the
/// manager's independent digest recomputed over the replacement — so nothing
/// but the framing law can be what refuses it.
fn retaped(emitted: &EmittedArtifact, bytes: &[u8]) -> Vec<u8> {
    let carrier = AnyIr::Emitted(emitted.clone());
    let mut document: serde_json::Value =
        serde_json::from_slice(&encode_compact(&carrier).unwrap()).unwrap();
    document["emitted"]["bytes_b64"] = serde_json::json!(encode_base64(bytes));
    document["emitted"]["provenance"]["bytes_digest"] =
        serde_json::json!(digest_hex(&emitted_bytes_digest(bytes)));
    serde_json::to_vec(&document).unwrap()
}

/// The refusal must be gate 15 AND must name the framing law that caught it,
/// so a red cannot pass for the wrong reason.
fn assert_emit_identity(name: &str, wire: &[u8], law: &str) {
    let error = decode(wire)
        .err()
        .unwrap_or_else(|| panic!("{name}: the tape must be refused"))
        .to_string();
    assert!(
        error.contains("gate `emit-identity`"),
        "{name}: expected `emit-identity`, got {error}"
    );
    assert!(error.contains(law), "{name}: expected `{law}`, got {error}");
}

const PROLOGUE: &str = "does not open with the context-owned header/preamble prologue";
const SEQUENCE: &str = "does not reconcile with the carried emission witnesses";

/// BOTH built-in backends: the real emitted artifact crosses domain→wire→
/// domain with its COMPLETE value and provenance intact, not merely
/// successfully.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn both_builtin_backends_round_trip_the_complete_emitted_value() {
    for target in both_targets() {
        let backend = target.backend_id().to_string();
        let emitted = emit(plan_for(target));
        let wire = encode_compact(&AnyIr::Emitted(emitted.clone())).unwrap();
        let back =
            decode(&wire).unwrap_or_else(|error| panic!("{backend} tape must decode: {error}"));
        let AnyIr::Emitted(back) = back else {
            panic!("{backend}: the emitted carrier decodes to the emitted level");
        };
        assert_eq!(
            emitted.provenance(),
            back.provenance(),
            "{backend}: the whole provenance"
        );
        assert_eq!(emitted.bytes(), back.bytes(), "{backend}: the whole tape");
        assert_eq!(emitted, back, "{backend}: the whole emitted value");
    }
}

/// A `static-md` carrier whose tape is plain UTF-8 Markdown with no engine
/// header or marker — the exact shape repair 1 still accepted — is red.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn plain_markdown_without_the_engine_prologue_is_red() {
    let emitted = emit(plan_for(ArtifactTarget::StaticMarkdown));
    let plain = b"# A title\n\nOrdinary prose, no engine header, no markers.\n";
    assert_emit_identity("plain markdown", &retaped(&emitted, plain), PROLOGUE);
}

/// A `static-xml` carrier whose tape is a generically well-formed document
/// with no engine header or marker is red: XML well-formedness is not this
/// backend's framing.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn generic_well_formed_xml_is_not_static_xml_framing() {
    let emitted = emit(plan_for(ArtifactTarget::StaticXml));
    assert_emit_identity("generic xml", &retaped(&emitted, b"<root/>"), PROLOGUE);
}

/// The real tape, minus its first header line: the context-owned prologue is
/// exact, so a removed header line is red for both backends.
#[test]
fn a_real_tape_missing_one_header_line_is_red() {
    for target in both_targets() {
        let backend = target.backend_id().to_string();
        let emitted = emit(plan_for(target));
        let tape = String::from_utf8(emitted.bytes().to_vec()).unwrap();
        let (_, rest) = tape.split_once('\n').expect("the tape opens with a header");
        assert_emit_identity(&backend, &retaped(&emitted, rest.as_bytes()), PROLOGUE);
    }
}

/// The real tape with its two ordered contribution markers swapped: the
/// marker sequence must reconcile with the carried emission witnesses, so a
/// reorder that keeps every byte otherwise identical is red.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn a_real_tape_with_reordered_contribution_markers_is_red() {
    for target in both_targets() {
        let backend = target.backend_id().to_string();
        let emitted = emit(two_root_plan(target));
        let tape = String::from_utf8(emitted.bytes().to_vec()).unwrap();
        let body = framing::tape_body(emitted.provenance(), &tape);
        let prologue_len = tape.len() - body.len();
        let mut lines: Vec<String> = body.lines().map(str::to_string).collect();
        let markers: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| framing::is_contribution_marker(emitted.provenance(), line))
            .map(|(index, _)| index)
            .collect();
        assert_eq!(
            markers.len(),
            2,
            "{backend}: the two-root tape carries two contribution markers"
        );
        lines.swap(markers[0], markers[1]);
        let reordered = format!("{}{}\n", &tape[..prologue_len], lines.join("\n"));
        assert_emit_identity(&backend, &retaped(&emitted, reordered.as_bytes()), SEQUENCE);
    }
}

/// The BUILTIN COMPATIBILITY row is a first-class carrier, not a shape the
/// gate happens to reject: the artifact the real
/// `compile_compatibility_artifact` produces crosses domain→wire→domain with
/// its COMPLETE value and provenance intact. Its artifact id is
/// `static-fragment` beside the `static-md` target/backend, and its tape is
/// flattened reversible Markdown with no static-lane prologue or contribution
/// marker at all.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE")]
fn the_real_compatibility_artifact_round_trips_the_complete_emitted_value() {
    for mode in [StaticCompileMode::Plain, StaticCompileMode::QualifyPerNode] {
        let emitted = compatibility_emitted(mode);
        let context = emitted.provenance().context.clone();
        assert_eq!(context.artifact().as_str(), "static-fragment");
        assert_eq!(emitted.provenance().backend.as_str(), "static-md");
        let tape = String::from_utf8(emitted.bytes().to_vec()).unwrap();
        assert!(
            !tape.contains("<!-- vibe:static ") && !tape.contains("generated by vibe"),
            "{mode:?}: the compatibility emitter writes no lane prologue or marker: {tape}"
        );

        let wire = encode_compact(&AnyIr::Emitted(emitted.clone())).unwrap();
        let AnyIr::Emitted(back) = decode(&wire).unwrap_or_else(|error| {
            panic!("{mode:?}: the compatibility tape must decode: {error}")
        }) else {
            panic!("{mode:?}: the emitted carrier decodes to the emitted level");
        };
        assert_eq!(
            emitted.provenance(),
            back.provenance(),
            "{mode:?}: the whole provenance"
        );
        assert_eq!(emitted.bytes(), back.bytes(), "{mode:?}: the whole tape");
        assert_eq!(emitted, back, "{mode:?}: the whole emitted value");
    }
}

/// The compatibility row keeps an honest law of its own: a fragment whose
/// reversible marker block never closes is still red.
#[test]
fn a_compatibility_fragment_with_an_unclosed_marker_block_is_red() {
    let emitted = compatibility_emitted(StaticCompileMode::QualifyPerNode);
    let unclosed = b"<!-- vibe:begin spec://org.demo/alpha/boot/entry#root -->\nbody\n";
    assert_emit_identity(
        "compatibility fragment",
        &retaped(&emitted, unclosed),
        "close every reversible marker block",
    );
}

/// The emitter's OWN hoisted marker spelling differs from its static one; the
/// decoder reconciles the real sequence rather than a static-only guess.
#[test]
fn a_real_hoisted_contribution_reconciles_for_both_builtin_backends() {
    for target in both_targets() {
        let backend = target.backend_id().to_string();
        let emitted = emit(hoisted_plan(target));
        let wire = encode_compact(&AnyIr::Emitted(emitted)).unwrap();
        decode(&wire)
            .unwrap_or_else(|error| panic!("{backend}: a hoisted tape must decode: {error}"));
    }
}
