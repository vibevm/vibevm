use super::*;

#[test]
fn utf8_xml_data_keeps_exact_bom_comments_whitespace_and_entities() {
    let raw = b"\xef\xbb\xbf<?xml version='1.0' encoding='UTF-8'?>\n<!-- before -->\n<catalog note='a&amp;b'>\n  <item> x &amp; &#x41; y </item>\n</catalog>\n<!-- after -->";
    let observed = inspect_xml_data(raw);
    assert_eq!(observed.raw(), raw);
    assert!(
        matches!(observed.inspection, DataInspection::Xml(XmlDataOutline {
        ref root, elements: 2, max_depth: 2, comments: 2, ..
    }) if root == "catalog")
    );
}

#[test]
fn empty_element_is_valid_structure_but_empty_bytes_are_invalid_and_retrievable() {
    let valid = inspect_xml_data(b"<empty/>");
    assert!(
        matches!(valid.inspection, DataInspection::Xml(XmlDataOutline {
        ref root, elements: 1, max_depth: 1, text_bytes: 0, ..
    }) if root == "empty")
    );
    let invalid = inspect_xml_data(b"");
    assert_eq!(invalid.raw(), b"");
    assert_eq!(
        invalid.inspection,
        DataInspection::Unavailable(XmlDataIssue::MalformedXml)
    );
    let opaque = preserve_opaque(b"\xff<not-xml");
    assert_eq!(opaque.raw(), b"\xff<not-xml");
    assert_eq!(opaque.inspection, DataInspection::NotRequested);
}

#[test]
fn unsupported_or_invalid_encoding_never_rewrites_raw_bytes() {
    let non_utf8 = b"<?xml version='1.0' encoding='ISO-8859-1'?><x>\xe9</x>";
    let observed = inspect_xml_data(non_utf8);
    assert_eq!(observed.raw(), non_utf8);
    assert_eq!(
        observed.inspection,
        DataInspection::Unavailable(XmlDataIssue::InvalidEncoding)
    );
    let declared_other = b"<?xml version='1.0' encoding='ISO-8859-1'?><x/>";
    let observed = inspect_xml_data(declared_other);
    assert_eq!(observed.raw(), declared_other);
    assert_eq!(
        observed.inspection,
        DataInspection::Unavailable(XmlDataIssue::UnsupportedEncoding)
    );
}

#[test]
fn invalid_xml_and_external_entities_refuse_structure_without_hiding_bytes() {
    for raw in [
        b"<root><x></root>".as_slice(),
        b"<root>".as_slice(),
        b"<a/><b/>".as_slice(),
    ] {
        let observed = inspect_xml_data(raw);
        assert_eq!(observed.raw(), raw);
        assert_eq!(
            observed.inspection,
            DataInspection::Unavailable(XmlDataIssue::MalformedXml)
        );
    }
    for raw in [
        b"<!DOCTYPE root SYSTEM 'https://example.invalid/ext.dtd'><root/>".as_slice(),
        b"<root>&outside;</root>".as_slice(),
        b"<root x='&outside;'/>".as_slice(),
        b"<root>&#x0;</root>".as_slice(),
    ] {
        let observed = inspect_xml_data(raw);
        assert_eq!(observed.raw(), raw);
        assert_eq!(
            observed.inspection,
            DataInspection::Unavailable(XmlDataIssue::ForbiddenEntity)
        );
    }
}

#[test]
fn illegal_xml_controls_and_late_declaration_keep_raw_without_structure() {
    for raw in [
        b"<root>before\0after</root>".as_slice(),
        b"<root note='before\x01after'/>".as_slice(),
        b"<!-- earlier --><?xml version='1.0'?><root/>".as_slice(),
    ] {
        let observed = inspect_xml_data(raw);
        assert_eq!(observed.raw(), raw);
        assert_eq!(
            observed.inspection,
            DataInspection::Unavailable(XmlDataIssue::MalformedXml),
            "accepted malformed XML: {raw:?}"
        );
    }
}

#[test]
fn inspection_limits_refuse_only_the_optional_tree() {
    let deep = format!(
        "{}{}",
        "<x>".repeat(MAX_XML_DATA_DEPTH + 1),
        "</x>".repeat(MAX_XML_DATA_DEPTH + 1)
    );
    let observed = inspect_xml_data(deep.as_bytes());
    assert_eq!(observed.raw(), deep.as_bytes());
    assert_eq!(
        observed.inspection,
        DataInspection::Unavailable(XmlDataIssue::LimitExceeded("depth"))
    );

    let many = format!("<r>{}</r>", "<x/>".repeat(MAX_XML_DATA_ELEMENTS));
    assert_eq!(
        inspect_xml_data(many.as_bytes()).inspection,
        DataInspection::Unavailable(XmlDataIssue::LimitExceeded("elements"))
    );

    let text = format!("<r>{}</r>", "a".repeat(MAX_XML_DATA_TEXT_BYTES + 1));
    assert_eq!(
        inspect_xml_data(text.as_bytes()).inspection,
        DataInspection::Unavailable(XmlDataIssue::LimitExceeded("text"))
    );
}
