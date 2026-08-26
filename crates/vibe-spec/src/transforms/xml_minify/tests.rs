//! Exact byte oracles for the policy-free emitted-XML minifier.

use std::borrow::Cow;

use specmark::verifies;

use super::*;

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TEST-XML-MINIFY")]
fn pure_nested_and_empty_element_containers_lose_only_whitespace_text() {
    let input = "<root>\n <outer>\n  <a/>\n  <b></b>\n </outer>\n</root>";
    assert_eq!(
        minify_emitted_xml(input).unwrap(),
        "<root><outer><a/><b></b></outer></root>"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TEST-XML-MINIFY")]
fn leaf_whitespace_and_mixed_parent_direct_whitespace_are_preserved() {
    for input in [
        "<leaf> \r\n </leaf>",
        "<p>lead<b>x</b> \r\n </p>",
        "<p> \t lead <b>x</b> tail \r\n</p>",
    ] {
        let output = minify_emitted_xml(input).unwrap();
        assert!(matches!(output, Cow::Borrowed(_)), "{input:?}");
        assert_eq!(output, input);
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TEST-XML-MINIFY")]
fn lexical_spelling_comments_and_cdata_survive_exactly() {
    let input = "<?xml version='1.0'?>\r\n<root  a = '1'>\r\n  <!-- vibe:marker  -->\r\n  <leaf><![CDATA[x < y]]></leaf>\r\n  <empty q = 'v'/>\r\n</root>\r\n";
    let expected = "<?xml version='1.0'?>\r\n<root  a = '1'><!-- vibe:marker  --><leaf><![CDATA[x < y]]></leaf><empty q = 'v'/></root>\r\n";
    let output = minify_emitted_xml(input).unwrap();
    assert_eq!(output, expected);
    for exact in [
        "<?xml version='1.0'?>",
        "<root  a = '1'>",
        "<!-- vibe:marker  -->",
        "<![CDATA[x < y]]>",
        "<empty q = 'v'/>",
    ] {
        assert!(
            output.contains(exact),
            "lost lexical bytes `{exact}`: {output}"
        );
    }

    let cdata_mixed = "<root>\n<![CDATA[ prose ]]>\n<child/>\n</root>";
    assert!(matches!(
        minify_emitted_xml(cdata_mixed).unwrap(),
        Cow::Borrowed(_)
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TEST-XML-MINIFY")]
fn c1_comments_and_cdata_stay_exact_while_double_hyphen_names_are_untouched() {
    let logical = "vibe:static org.demo/a--b — dir/a--b-/x%2D&雪.xml";
    let comment = format!(
        "<!-- vibe:c1 {} -->",
        vibe_specdoc::encode_generated_xml_comment(logical)
    );
    let input = format!("<root>\n  {comment}\n  <a--b><![CDATA[x < y]]></a--b>\n</root>");
    let expected = format!("<root>{comment}<a--b><![CDATA[x < y]]></a--b></root>");

    assert_eq!(minify_emitted_xml(&input).unwrap(), expected);
    assert!(expected.contains("<a--b>"));
    assert_eq!(
        vibe_specdoc::decode_generated_xml_comment(&comment)
            .unwrap()
            .as_deref(),
        Some(logical)
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#STAGE-EMITTED")]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TEST-XML-MINIFY")]
fn emitted_stream_accepts_two_declarations_and_roots_while_pivot_rejects_it() {
    let input = "<!-- lane -->\n<?xml version=\"1.0\"?> <!-- before root --> <?lane probe?><one>\n<a/>\n</one>\n<?xml version='1.0'?><two>\t<b/>\r\n</two>\n";
    let expected = "<!-- lane -->\n<?xml version=\"1.0\"?> <!-- before root --> <?lane probe?><one><a/></one>\n<?xml version='1.0'?><two><b/></two>\n";

    assert!(vibe_specdoc::from_xml(input).is_err());
    assert_eq!(minify_emitted_xml(input).unwrap(), expected);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TEST-XML-MINIFY")]
fn element_names_and_attributes_are_fully_validated_before_editing() {
    let cases = [
        ("<root attr='&unknown;'/>", "attribute value"),
        ("<root attr=value/>", "malformed or duplicate attribute"),
        ("<root attr='one' attr='two'/>", "duplicate attribute"),
        ("<root attr=/>", "malformed or duplicate attribute"),
        ("<1root/>", "element name"),
        ("<root 1attr='value'/>", "attribute name"),
    ];
    for (input, expected) in cases {
        let error = minify_emitted_xml(input).expect_err(input);
        assert!(error.diagnostic().contains(expected), "{input:?}: {error}");
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TEST-XML-MINIFY")]
fn declarations_and_top_level_framing_are_strict_per_root() {
    let malformed_declarations = [
        "<?xml nope?><root/>",
        "<?xml encoding='UTF-8' version='1.0'?><root/>",
        "<?xml version='1.1'?><root/>",
        "<?xml version='1.0' bogus='x'?><root/>",
        "<?xml version='1.0' standalone='maybe'?><root/>",
        "<?xml version='1.0' encoding='9bad'?><root/>",
    ];
    for input in malformed_declarations {
        let error = minify_emitted_xml(input).expect_err(input);
        assert!(
            error.diagnostic().contains("declaration"),
            "{input:?}: {error}"
        );
    }

    for (input, expected) in [
        ("&#x20;<root/>", "references require an open element"),
        (
            "<![CDATA[ ]]><root/>",
            "CDATA sections require an open element",
        ),
        (
            "<root><?xml version='1.0'?><child/></root>",
            "declaration is legal only at top level",
        ),
        ("<?xml version='1.0'?>", "declaration is orphaned"),
        (
            "<?xml version='1.0'?><?xml version='1.0'?><root/>",
            "second XML declaration",
        ),
        ("<root/><?xml version='1.0'?>", "declaration is orphaned"),
    ] {
        let error = minify_emitted_xml(input).expect_err(input);
        assert!(error.diagnostic().contains(expected), "{input:?}: {error}");
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TEST-XML-MINIFY")]
fn xml_10_character_law_covers_text_cdata_attributes_and_numeric_references() {
    for input in [
        "<root>\u{1}<child/></root>",
        "<root><![CDATA[\u{1}]]><child/></root>",
        "<root attr='\u{1}'><child/></root>",
        "<root attr='&#1;'><child/></root>",
        "<root>&#1;<child/></root>",
    ] {
        let error = minify_emitted_xml(input).expect_err(input);
        assert!(
            error.diagnostic().contains("illegal XML 1.0 character"),
            "{input:?}: {error}"
        );
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TEST-XML-MINIFY")]
fn legal_unicode_names_and_characters_remain_byte_exact() {
    let input = "<?xml version='1.0' encoding='UTF-8' standalone='yes'?>\n<根 属性='值&#xA;'>合法😀<𐀀 名='值'/></根>";
    let output = minify_emitted_xml(input).unwrap();
    assert!(matches!(output, Cow::Borrowed(_)));
    assert_eq!(output, input);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TEST-XML-MINIFY")]
fn character_references_join_the_surrounding_logical_text_node() {
    let whitespace = "<root>\t&#xA;<a/>&#32;\r\n</root>";
    assert_eq!(minify_emitted_xml(whitespace).unwrap(), "<root><a/></root>");

    let mixed = "<root> \n&amp;<a/> \t</root>";
    let output = minify_emitted_xml(mixed).unwrap();
    assert!(matches!(output, Cow::Borrowed(_)));
    assert_eq!(output, mixed);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TEST-XML-MINIFY")]
fn unsafe_or_non_xml_inputs_fail_with_owned_typed_diagnostics() {
    let cases = [
        ("<!DOCTYPE root><root/>", "DTD declarations"),
        ("<root>&unknown;<a/></root>", "unknown entity `&unknown;`"),
        (
            "<root><!-- invalid--comment --><a/></root>",
            "not well formed",
        ),
        ("<root><a></root>", "not well formed"),
        ("<root><a/>", "unclosed element"),
        ("plain text", "outside an element"),
        ("  <!-- only a comment -->\n", "contains no element"),
    ];

    for (input, expected) in cases {
        let error = minify_emitted_xml(input).expect_err(input);
        assert!(error.diagnostic().contains(expected), "{input:?}: {error}");
        assert!(error.to_string().contains(MINIFY_SPEC), "{error}");
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TEST-XML-MINIFY")]
fn no_op_borrows_and_a_changed_stream_is_idempotent() {
    let no_op = "<p>prose <b>stays</b> intact</p>";
    assert!(matches!(
        minify_emitted_xml(no_op).unwrap(),
        Cow::Borrowed(_)
    ));

    let input = "<root>\n <a/>\n <b/>\n</root>";
    let first = minify_emitted_xml(input).unwrap();
    assert!(matches!(first, Cow::Owned(_)));
    let second = minify_emitted_xml(first.as_ref()).unwrap();
    assert!(matches!(second, Cow::Borrowed(_)));
    assert_eq!(first, second);
}
