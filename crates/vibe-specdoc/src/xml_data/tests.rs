//! Exact raw equality under success/refusal; bounded XML event profile only.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-DATA-INSPECTION");
use super::*;
fn inspect(raw: &[u8]) -> PreservedData<'_> {
    inspect_xml_data(raw, InspectionLimits::unlimited())
}
fn refusal(raw: &[u8], reason: XmlDataIssue) {
    let view = inspect(raw);
    assert_eq!(view.raw(), raw);
    assert!(std::ptr::eq(view.raw(), raw));
    assert_eq!(
        view.inspection(),
        &DataInspection::Unavailable(reason),
        "{raw:?}"
    );
}
#[test]
fn exact_bom_crlf_comments_references_and_lexical_root_are_preserved() {
    let raw=b"\xef\xbb\xbf<?xml version='1.0' encoding='UTF-8' standalone='yes'?>\r\n<!--before-->\r\n<catalog note='a&amp;b'><item> x &amp; &#x41; y </item></catalog>\r\n<!--after-->";
    let view = inspect(raw);
    assert!(std::ptr::eq(view.raw(), raw));
    let DataInspection::Available(outline) = view.inspection() else {
        panic!("{:?}", view.inspection())
    };
    assert_eq!(
        outline,
        &XmlOutline {
            root: "catalog".into(),
            elements: 2,
            max_depth: 2,
            text_bytes: 24,
            comments: 2,
            processing_instructions: 0,
        }
    );
}
#[test]
fn opaque_binary_never_decodes_and_empty_element_is_real_structure() {
    for raw in [b"\xff\0<bad>".as_slice(), b"", b"<?xml broken"] {
        let view = preserve_opaque(raw);
        assert!(std::ptr::eq(view.raw(), raw));
        assert_eq!(view.inspection(), &DataInspection::NotRequested);
    }
    assert_eq!(
        inspect(b"<empty/>").inspection(),
        &DataInspection::Available(XmlOutline {
            root: "empty".into(),
            elements: 1,
            max_depth: 1,
            text_bytes: 0,
            comments: 0,
            processing_instructions: 0,
        })
    );
    refusal(b"", XmlDataIssue::MalformedEvents);
}
#[test]
fn all_encoding_and_version_refusals_retain_full_bytes() {
    for raw in [
        b"\xff\xfe<\0x\0/\0>\0".as_slice(),
        b"<?xml version='1.0' encoding='ISO-8859-1'?><x>\xe9</x>",
    ] {
        refusal(raw, XmlDataIssue::InvalidUtf8);
    }
    refusal(
        b"<?xml version='1.0' encoding='ISO-8859-1'?><x/>",
        XmlDataIssue::UnsupportedEncoding,
    );
    refusal(
        b"<?xml version='1.1'?><x/>",
        XmlDataIssue::UnsupportedXmlVersion,
    );
    assert!(matches!(
        inspect("<été>café</été>".as_bytes()).inspection(),
        DataInspection::Available(_)
    ));
}
#[test]
fn malformed_event_structure_attributes_comments_and_controls_are_unavailable() {
    for raw in [
        b"<root><x></root>".as_slice(),
        b"<root>",
        b"<a/><b/>",
        b"</root>",
        b"outside<root/>",
        b"<root/>outside",
        b"<![CDATA[x]]><root/>",
        b"<r a='1' a='2'/>",
        b"<r a=noquote/>",
        b"<r><!-- a--b --></r>",
        b"<r>\0</r>",
        b"<r a='\x01'/>",
        b"<r",
    ] {
        refusal(raw, XmlDataIssue::MalformedEvents);
    }
}
#[test]
fn declaration_profile_uses_upstream_attributes_not_first_value_only() {
    for raw in [
        b"<?xml?><r/>".as_slice(),
        b"<?xml encoding='utf-8'?><r/>",
        b"<?xml version='1.2'?><r/>",
        b"<?xml version='1.0' version='1.0'?><r/>",
        b"<?xml version='1.0' extra='x'?><r/>",
        b"<?xml version='1.0' standalone='maybe'?><r/>",
        b"<?xml version='1.0' standalone='yes' encoding='UTF-8'?><r/>",
        b"<!--before--><?xml version='1.0'?><r/>",
        b" <?xml version='1.0'?><r/>",
        b"<?xml version='1.0'?><?xml version='1.0'?><r/>",
    ] {
        refusal(raw, XmlDataIssue::InvalidDeclaration);
    }
}
#[test]
fn dtd_and_external_unknown_or_illegal_references_never_resolve() {
    for raw in [
        b"<!DOCTYPE root SYSTEM 'https://example.invalid/ext.dtd'><root/>".as_slice(),
        b"<!DOCTYPE r [<!ENTITY x SYSTEM 'file:///not-opened'>]><r>&x;</r>",
    ] {
        refusal(raw, XmlDataIssue::ForbiddenDtd);
    }
    for raw in [
        b"<r>&unknown;</r>".as_slice(),
        b"<r a='&unknown;'/>",
        b"<r>&#x0;</r>",
        b"<r>&#x1;</r>",
        b"<r>&#xD800;</r>",
        b"<r>&#1114112;</r>",
        b"<r a='&#x1;'/>",
        b"&amp;<r/>",
    ] {
        refusal(raw, XmlDataIssue::ForbiddenReference);
    }
    refusal(b"<r>&amp</r>", XmlDataIssue::MalformedEvents);
    assert!(matches!(
        inspect(b"<r>&lt;&gt;&amp;&apos;&quot;&#65;&#x41;<![CDATA[&not-a-reference;]]></r>")
            .inspection(),
        DataInspection::Available(_)
    ));
}
#[test]
fn pi_is_inert_and_names_are_not_a_validity_certificate() {
    for raw in [
        b"<?app arbitrary content?><r/>".as_slice(),
        b"<?1bad?><r/>",
        b"<?XML version='1.0'?><r/>",
        b"<r><?app data?></r>",
    ] {
        let view = inspect(raw);
        assert_eq!(view.raw(), raw);
        assert!(
            matches!(view.inspection(), DataInspection::Available(outline) if outline.processing_instructions==1)
        );
    }
    refusal(b"<?app unclosed", XmlDataIssue::MalformedEvents);
    // Upstream is a non-validating event reader: lexical Name/QName grammar and
    // namespace bindings are intentionally not certified by Available.
    for raw in [b"<1bad/>".as_slice(), b"<p:root/>", b"<a::b/>"] {
        let view = inspect(raw);
        assert_eq!(view.raw(), raw);
        assert!(
            matches!(view.inspection(), DataInspection::Available(_)),
            "{:?}",
            view.inspection()
        );
    }
}
#[test]
fn caller_budgets_only_disable_structure_with_exact_boundaries() {
    let raw = b"<r><x/>abc&amp;<![CDATA[z]]><!--c--></r>";
    let DataInspection::Available(outline) = inspect(raw).inspection().clone() else {
        panic!()
    };
    assert_eq!(outline.text_bytes, 9);
    for (limits, budget, limit) in [
        (
            InspectionLimits {
                max_input_bytes: Some(raw.len() - 1),
                ..InspectionLimits::unlimited()
            },
            InspectionBudget::InputBytes,
            raw.len() - 1,
        ),
        (
            InspectionLimits {
                max_depth: Some(1),
                ..InspectionLimits::unlimited()
            },
            InspectionBudget::Depth,
            1,
        ),
        (
            InspectionLimits {
                max_elements: Some(1),
                ..InspectionLimits::unlimited()
            },
            InspectionBudget::Elements,
            1,
        ),
        (
            InspectionLimits {
                max_text_bytes: Some(8),
                ..InspectionLimits::unlimited()
            },
            InspectionBudget::TextBytes,
            8,
        ),
        (
            InspectionLimits {
                max_events: Some(0),
                ..InspectionLimits::unlimited()
            },
            InspectionBudget::Events,
            0,
        ),
    ] {
        let view = inspect_xml_data(raw, limits);
        assert_eq!(view.raw(), raw);
        assert_eq!(
            view.inspection(),
            &DataInspection::Unavailable(XmlDataIssue::LimitExceeded { budget, limit })
        );
    }
    let exact = InspectionLimits {
        max_input_bytes: Some(raw.len()),
        max_depth: Some(2),
        max_elements: Some(2),
        max_text_bytes: Some(9),
        max_events: None,
    };
    assert!(matches!(
        inspect_xml_data(raw, exact).inspection(),
        DataInspection::Available(_)
    ));
    let deep = format!("{}{}", "<x>".repeat(100), "</x>".repeat(100));
    let view = inspect_xml_data(
        deep.as_bytes(),
        InspectionLimits {
            max_depth: Some(3),
            ..InspectionLimits::unlimited()
        },
    );
    assert_eq!(view.raw(), deep.as_bytes());
    assert!(matches!(
        view.inspection(),
        DataInspection::Unavailable(XmlDataIssue::LimitExceeded {
            budget: InspectionBudget::Depth,
            ..
        })
    ));
    assert!(matches!(
        inspect(deep.as_bytes()).inspection(),
        DataInspection::Available(_)
    ));
}
