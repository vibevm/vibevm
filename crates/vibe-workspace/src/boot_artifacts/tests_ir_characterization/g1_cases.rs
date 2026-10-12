//! B-224 whole-lane and M-06-G.1 normal directive characterizations.

use super::*;

#[test]
fn xml_normal_source_before_section_and_mid_section_both_fold() {
    let ws = TempDir::new().unwrap();
    let contract = write_normal_doc(
        ws.path(),
        "org.demo.contract",
        "contract/root.md",
        "#source spec://org.demo/contract/source/first#first\n# Contract {#root}\nCONTRACT_BEFORE\n#source spec://org.demo/contract/source/second#second\nCONTRACT_AFTER\n",
    );
    write_normal_doc(
        ws.path(),
        "org.demo.contract",
        "source/first.md",
        "# First {#first}\nFIRST_SOURCE_BODY\n",
    );
    write_normal_doc(
        ws.path(),
        "org.demo.contract",
        "source/second.md",
        "# Second {#second}\nSECOND_SOURCE_BODY\n",
    );

    let xml = render_static_with_spec_format(
        &boot(vec![entry_normal(&contract, "org.demo/contract")]),
        ws.path(),
        &coord(),
        SpecFormat::Xml,
    )
    .unwrap()
    .unwrap();
    for body in [
        "CONTRACT_BEFORE",
        "CONTRACT_AFTER",
        "FIRST_SOURCE_BODY",
        "SECOND_SOURCE_BODY",
    ] {
        assert_eq!(xml.matches(body).count(), 1, "{body}: {xml}");
    }
    assert!(!xml.contains("#source spec://"), "{xml}");
    vibe_spec::minify_emitted_xml(&xml).expect("the generated static lane is strict XML");
}

#[test]
fn xml_cross_package_normal_units_and_consumer_lane_close_once() {
    let ws = TempDir::new().unwrap();
    let effective = cross_package_normal_fixture(ws.path());
    let bodies = [
        "UMBRELLA_BODY",
        "ALPHA_ROOT_BODY",
        "BETA_CONTRACT_BODY",
        "BETA_SOURCE_BODY",
    ];
    let expected_units = [[1, 1, 1, 1], [0, 1, 1, 1], [0, 0, 1, 1]];
    for (entry, expected) in effective.entries.iter().zip(expected_units) {
        let unit = render_static_with_spec_format(
            &boot(vec![(*entry).clone()]),
            ws.path(),
            &coord(),
            SpecFormat::Xml,
        )
        .unwrap()
        .unwrap();
        for (body, count) in bodies.into_iter().zip(expected) {
            assert_eq!(unit.matches(body).count(), count, "{body}: {unit}");
        }
    }

    let lane = render_static_with_spec_format(&effective, ws.path(), &coord(), SpecFormat::Xml)
        .unwrap()
        .unwrap();
    for (body, count) in bodies.into_iter().zip([1, 1, 1, 1]) {
        assert_eq!(lane.matches(body).count(), count, "{body}: {lane}");
    }
}

#[test]
fn xml_direct_normal_unused_sibling_survives_once() {
    let ws = TempDir::new().unwrap();
    let effective = cross_package_normal_fixture(ws.path());
    let xml = render_static_with_spec_format(&effective, ws.path(), &coord(), SpecFormat::Xml)
        .unwrap()
        .unwrap();
    assert_eq!(xml.matches("ALPHA_UNUSED_BODY").count(), 1, "{xml}");
    assert!(xml.contains("org-demo--alpha--unused"), "{xml}");
}

#[test]
fn xml_equal_prose_from_distinct_authored_documents_is_not_deduplicated() {
    let ws = TempDir::new().unwrap();
    let first = write_normal_doc(
        ws.path(),
        "org.demo.first",
        "boot/entry.md",
        "# Shared {#root}\nSAME_AUTHORED_PROSE\n",
    );
    let second = write_normal_doc(
        ws.path(),
        "org.demo.second",
        "boot/entry.md",
        "# Shared {#root}\nSAME_AUTHORED_PROSE\n",
    );
    let xml = render_static_with_spec_format(
        &boot(vec![
            entry_normal(&first, "org.demo/first"),
            entry_normal(&second, "org.demo/second"),
        ]),
        ws.path(),
        &coord(),
        SpecFormat::Xml,
    )
    .unwrap()
    .unwrap();
    assert_eq!(xml.matches("SAME_AUTHORED_PROSE").count(), 2, "{xml}");
    assert!(xml.contains("org-demo--first--root"), "{xml}");
    assert!(xml.contains("org-demo--second--root"), "{xml}");
}

#[test]
fn xml_nested_use_and_direct_normal_entry_emit_solution_and_unused_sibling_once() {
    let ws = TempDir::new().unwrap();
    let umbrella = write_normal_doc(
        ws.path(),
        "org.demo.umbrella",
        "boot/entry.md",
        "# Umbrella {#root}\n#use spec://org.demo/beta/contract/root#solution\nUMBRELLA_NESTED_BODY\n",
    );
    let beta = write_normal_doc(
        ws.path(),
        "org.demo.beta",
        "contract/root.md",
        "# Beta {#root}\n## Solution {#solution}\nBETA_NESTED_SOLUTION\n## Unused {#unused}\nBETA_NESTED_UNUSED\n",
    );
    let xml = render_static_with_spec_format(
        &boot(vec![
            entry_normal(&umbrella, "org.demo/umbrella"),
            entry_normal(&beta, "org.demo/beta"),
        ]),
        ws.path(),
        &coord(),
        SpecFormat::Xml,
    )
    .unwrap()
    .unwrap();
    for body in [
        "UMBRELLA_NESTED_BODY",
        "BETA_NESTED_SOLUTION",
        "BETA_NESTED_UNUSED",
    ] {
        assert_eq!(xml.matches(body).count(), 1, "{body}: {xml}");
    }
}

#[test]
fn xml_partial_then_whole_then_direct_normal_keeps_preamble_once() {
    let ws = TempDir::new().unwrap();
    let a = write_normal_doc(
        ws.path(),
        "org.demo.a",
        "boot/entry.md",
        "# A {#root}\n#use spec://org.demo/beta/contract/root#solution\nA_CONSUMER_BODY\n",
    );
    let b = write_normal_doc(
        ws.path(),
        "org.demo.b",
        "boot/entry.md",
        "# B {#root}\n#use spec://org.demo/beta/contract/root\nB_CONSUMER_BODY\n",
    );
    let beta = write_normal_doc(
        ws.path(),
        "org.demo.beta",
        "contract/root.md",
        "BETA_PREAMBLE\n# Beta {#root}\n## Solution {#solution}\nBETA_SOLUTION\n## Unused {#unused}\nBETA_UNUSED\n",
    );
    let xml = render_static_with_spec_format(
        &boot(vec![
            entry_normal(&a, "org.demo/a"),
            entry_normal(&b, "org.demo/b"),
            entry_normal(&beta, "org.demo/beta"),
        ]),
        ws.path(),
        &coord(),
        SpecFormat::Xml,
    )
    .unwrap()
    .unwrap();
    for body in [
        "BETA_PREAMBLE",
        "BETA_SOLUTION",
        "BETA_UNUSED",
        "A_CONSUMER_BODY",
        "B_CONSUMER_BODY",
    ] {
        assert_eq!(xml.matches(body).count(), 1, "{body}: {xml}");
    }
}
