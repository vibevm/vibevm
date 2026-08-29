//! The active-transforms header payload grammar (R4 architecture §7.1) —
//! the value half, judged directly on built plans.
//!
//! The compile-level half (byte position in both lanes, empty-plan byte
//! identity, validator admission/refusal, decompile classification) lives in
//! `header_e2e_tests`; the two are separate because they answer different
//! questions and only one of them needs a compiler.

use specmark::verifies;

use vibe_specdoc::{decode_generated_xml_comment_payload, encode_generated_xml_comment};

use super::header::{TRANSFORMS_HEADER_PREFIX, observed_header_tokens, transforms_header_payload};
use super::plan::{TransformPlan, TransformStage};
use super::plan_test_support::{build_or_panic, dependency_seed};

/// A plan over the given keys, one document-stage entry each, in exactly the
/// order given — which is the order `build` assigns as dense effective order.
fn plan_over(keys: &[&str]) -> TransformPlan {
    build_or_panic(
        keys.iter()
            .map(|key| dependency_seed(key, TransformStage::Document))
            .collect(),
    )
}

/// The empty plan contributes NO payload — the whole of the active-only law.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn the_empty_plan_contributes_no_header_at_all() {
    assert_eq!(transforms_header_payload(&TransformPlan::empty()), None);
}

/// One token per entry, in dense effective order, each the entry's canonical
/// `ExtensionKey` spelling — and each recoverable through the SAME codec.
///
/// The round-trip is the assertion that matters: it says the token is not
/// merely "some encoding" but the reversible canonical one, so the header can
/// be read by a human and re-derived by a checker without a private table.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn every_entry_contributes_one_codec_round_tripping_token_in_effective_order() {
    let keys = [
        "org.demo/tools#announce",
        "org.demo/other#second",
        "__host__/my%20app#third",
    ];
    let payload = transforms_header_payload(&plan_over(&keys)).expect("a nonempty plan records");

    let tokens: Vec<&str> = observed_header_tokens(&payload)
        .expect("the payload opens with the reserved prefix")
        .collect();
    assert_eq!(tokens.len(), keys.len(), "one token per entry: {payload}");
    for (token, key) in tokens.iter().zip(keys) {
        assert_eq!(
            decode_generated_xml_comment_payload(token).expect(token),
            key,
            "each token round-trips to the entry's canonical key: {payload}"
        );
    }
    assert_eq!(
        payload,
        format!(
            "{TRANSFORMS_HEADER_PREFIX} org.demo/tools#announce org.demo/other#second \
             __host__/my%2520app#third"
        ),
        "the exact payload spelling, including the codec's `%` escape"
    );
}

/// A key carrying `--` — legal in a package name, and the corner §7.1 names
/// as the reason the codec is mandatory — is ENCODED, never raw, so the
/// comment can never be corrupted or forbidden by its own payload.
///
/// The two invariants asserted here are the codec's whole contract at this
/// call site: no `--` anywhere in the payload, and no terminal `-` that could
/// touch the closing `-->`.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn a_key_bearing_a_double_hyphen_or_a_trailing_hyphen_is_encoded_never_raw() {
    let keys = ["org.demo/a--b#x", "org.demo/tools#trailing-"];
    let payload = transforms_header_payload(&plan_over(&keys)).expect("a nonempty plan records");

    assert!(
        !payload.contains("--"),
        "no `--` may survive into the comment: {payload}"
    );
    assert!(
        !payload.ends_with('-'),
        "no terminal `-` may touch the comment close: {payload}"
    );
    assert!(
        payload.contains("a-%2Db#x") && payload.contains("trailing%2D"),
        "both dangerous shapes took the codec's escape: {payload}"
    );
    for (token, key) in observed_header_tokens(&payload)
        .expect("the payload opens with the reserved prefix")
        .zip(keys)
    {
        assert_eq!(
            decode_generated_xml_comment_payload(token).expect(token),
            key
        );
    }
}

/// The header cell spells NO percent escape of its own.
///
/// §7.1 rejects a second spelling of the codec by name, and this is that
/// rejection made mechanical at the one cell most tempted to inline it: a
/// literal `%25`/`%2D` in this source would be a private table drifting
/// against `vibe_specdoc`'s, and the fence catches it before a reviewer has
/// to.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn the_header_cell_carries_no_second_spelling_of_the_percent_rules() {
    let cell = include_str!("header.rs");
    for escape in ["%25", "%2D", "%2d"] {
        assert!(
            !cell.contains(escape),
            "the header cell must reach the ONE shared codec, never spell `{escape}` itself"
        );
    }
    assert!(
        cell.contains("encode_generated_xml_comment"),
        "the header cell reaches the shared codec by name"
    );
}

/// A payload that does not open with the reserved prefix is not a transforms
/// header, and the split half says so rather than guessing.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn a_foreign_payload_is_not_read_as_a_transforms_header() {
    assert!(observed_header_tokens("vibe:static org.demo/pkg — x").is_none());
    assert!(
        observed_header_tokens(&encode_generated_xml_comment("vibe:transform")).is_none(),
        "a shorter reserved-looking payload is not this header"
    );
    assert!(
        observed_header_tokens("vibe:transformsX y").is_none(),
        "the prefix is a whole token, not a string prefix"
    );
}
