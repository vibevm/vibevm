//! Mutation-sensitive oracles for the generated-XML-comment wire.

use crate::{
    decode_generated_xml_comment, decode_generated_xml_comment_payload,
    encode_generated_xml_comment,
};

fn wrapped(payload: &str) -> String {
    format!("<!-- vibe:c1 {} -->", encode_generated_xml_comment(payload))
}

#[test]
fn exact_selective_encoding_table_is_canonical_and_round_trips() {
    let cases = [
        ("plain", "plain"),
        ("a-b", "a-b"),
        ("a--b", "a-%2Db"),
        ("a---b", "a-%2D-b"),
        ("a----b", "a-%2D-%2Db"),
        ("tail-", "tail%2D"),
        ("%", "%25"),
        ("%2D", "%252D"),
        ("&#45; &amp; <tag>", "&#45; &amp; <tag>"),
        ("雪/путь/😀", "雪/путь/😀"),
        (
            "\0\u{1}\u{b}\u{fffe}\u{ffff}",
            "%00%01%0B%EF%BF%BE%EF%BF%BF",
        ),
        ("line\r\nnext", "line\r\nnext"),
        ("dir/a--b-/x%2D&雪.xml", "dir/a-%2Db-/x%252D&雪.xml"),
        ("x-->injected<!--y", "x-%2D>injected<!-%2Dy"),
    ];

    for (logical, encoded) in cases {
        assert_eq!(
            encode_generated_xml_comment(logical),
            encoded,
            "{logical:?}"
        );
        let comment = format!("<!-- vibe:c1 {encoded} -->");
        assert!(!encoded.contains("--"), "{comment}");
        assert!(!encoded.ends_with('-'), "{comment}");
        assert_eq!(
            decode_generated_xml_comment(&comment).unwrap().as_deref(),
            Some(logical),
            "{comment}"
        );
    }
}

#[test]
fn decoder_is_one_pass_and_legacy_comments_are_distinct() {
    assert_eq!(
        decode_generated_xml_comment("<!-- vibe:c1 %252D -->")
            .unwrap()
            .as_deref(),
        Some("%2D")
    );
    assert_eq!(
        decode_generated_xml_comment("<!-- vibe:static org.demo/pkg — x -->").unwrap(),
        None
    );
}

#[test]
fn decoder_refuses_unknown_malformed_noncanonical_and_extra_input() {
    let cases = [
        ("<!-- vibe:c0 x -->", "unsupported"),
        ("<!-- vibe:c2 x -->", "unsupported"),
        ("<!-- vibe:c x -->", "version"),
        ("<!--vibe:c1 x -->", "framing"),
        ("<!--  vibe:c1 x -->", "framing"),
        ("<!-- vibe:c1 x-->", "framing"),
        ("<!-- vibe:c1 % -->", "escape"),
        ("<!-- vibe:c1 %2 -->", "escape"),
        ("<!-- vibe:c1 %2d -->", "uppercase"),
        ("<!-- vibe:c1 %GG -->", "hex"),
        ("<!-- vibe:c1 %FF -->", "UTF-8"),
        ("<!-- vibe:c1 %41 -->", "canonical"),
        ("<!-- vibe:c1 x%2Dy -->", "canonical"),
        ("<!-- vibe:c1 plain --> trailing", "complete"),
        ("<!-- vibe:c1 plain --><!-- second -->", "complete"),
        ("vibe:c1 plain", "complete"),
    ];

    for (wire, expected) in cases {
        let error = decode_generated_xml_comment(wire).expect_err(wire);
        assert!(
            error.to_string().contains(expected),
            "{wire:?}: expected {expected:?} in {error}"
        );
    }
}

/// The kind-free payload entry is the exact inverse of the encoder, and it is
/// the SAME law the c1 entry applies — not a second, looser one.
///
/// It exists because a generated comment of another kind (the active-transforms
/// header) spells its payload with these rules while carrying its own framing,
/// and must be able to refuse a malformed payload with THIS codec's error
/// rather than a private re-spelling of the percent table.
#[test]
fn the_payload_entry_is_the_encoders_exact_inverse_under_the_same_law() {
    for (logical, encoded) in [
        ("plain", "plain"),
        ("a-b", "a-b"),
        ("org.demo/a--b#x", "org.demo/a-%2Db#x"),
        ("tail-", "tail%2D"),
        ("100%", "100%25"),
        ("雪", "雪"),
    ] {
        assert_eq!(
            encode_generated_xml_comment(logical),
            encoded,
            "{logical:?}"
        );
        assert_eq!(
            decode_generated_xml_comment_payload(encoded).expect(encoded),
            logical,
            "{encoded:?}"
        );
    }

    // The same refusal family the framed entry raises, at payload offsets.
    for (payload, expected) in [
        ("%", "escape"),
        ("%2", "escape"),
        ("%2d", "uppercase"),
        ("%GG", "hex"),
        ("%FF", "UTF-8"),
        ("%41", "canonical"),
        ("x%2Dy", "canonical"),
    ] {
        let error = decode_generated_xml_comment_payload(payload).expect_err(payload);
        assert!(
            error.to_string().contains(expected),
            "{payload:?}: expected {expected:?} in {error}"
        );
    }
}

#[test]
fn bounded_string_space_round_trips_and_is_injective() {
    use std::collections::BTreeMap;

    let alphabet = ['a', '-', '%', '&', '<', '雪', '😀', '\0', '\u{1}'];
    let mut seen = BTreeMap::<String, String>::new();
    for logical in closed_alphabet_strings(&alphabet, 4) {
        let encoded = encode_generated_xml_comment(&logical);
        assert!(!encoded.contains("--"), "{logical:?} => {encoded:?}");
        assert!(!encoded.ends_with('-'), "{logical:?} => {encoded:?}");
        assert_eq!(
            decode_generated_xml_comment(&wrapped(&logical)).unwrap(),
            Some(logical.clone())
        );
        assert_eq!(seen.insert(encoded, logical.clone()), None, "{logical:?}");
    }
}

fn closed_alphabet_strings(alphabet: &[char], max_len: usize) -> Vec<String> {
    fn extend(prefix: &mut String, alphabet: &[char], remaining: usize, cases: &mut Vec<String>) {
        cases.push(prefix.clone());
        if remaining == 0 {
            return;
        }
        for character in alphabet {
            prefix.push(*character);
            extend(prefix, alphabet, remaining - 1, cases);
            prefix.pop();
        }
    }

    let mut cases = Vec::new();
    extend(&mut String::new(), alphabet, max_len, &mut cases);
    cases
}
