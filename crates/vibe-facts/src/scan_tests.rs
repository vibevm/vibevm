//! Oracles for the dependency-clean root scanner and the adoption join.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-046#laws");

use std::fs;

use tempfile::tempdir;
use vibe_core::layout;

use crate::{
    AdoptionObservation, AuthoredFact, FactEntry, FactOrigin, FactStatus, Registry, RegistryError,
    SourceKind, join_adoption, scan_authored_facts,
};

pub(crate) const XML_NS: &str = "xmlns=\"https://vibevm.org/spec/1\"";

fn status(value: &str) -> FactStatus {
    FactStatus::parse(value).expect("test status")
}

fn package_fact(address: &str, value: &str) -> FactEntry {
    FactEntry {
        address: address.to_string(),
        origin: FactOrigin::Package,
        package: Some("org.example/pkg".to_string()),
        status: Some(status(value)),
        comment: None,
    }
}

/// A fixture path under the specs root, `/`-separated — routed through
/// the layout seam (PROP-052 L2) so the scaffold names whichever layout
/// is live.
pub(crate) fn spec_rel(rel: &str) -> String {
    layout::current_specs_root()
        .join(rel)
        .to_string_lossy()
        .replace('\\', "/")
}

pub(crate) fn write_spec(root: &std::path::Path, rel: &str, body: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("spec parent")).expect("spec dir");
    fs::write(path, body).expect("spec file");
}

#[test]
fn markdown_and_xml_forms_mint_the_same_canonical_address() {
    let markdown = tempdir().expect("tempdir");
    write_spec(
        markdown.path(),
        &spec_rel("RULE.md"),
        "# Rules\n\n@fact:FIRST First. @status:impl/done\n",
    );
    let xml = tempdir().expect("tempdir");
    write_spec(
        xml.path(),
        &spec_rel("RULE.xml"),
        format!(
            "<spec {XML_NS}><p><fact id=\"FIRST\" status=\"impl/done\">First.</fact></p></spec>"
        )
        .as_str(),
    );

    let from_md =
        scan_authored_facts(markdown.path(), "org.example/pkg", SourceKind::Package).unwrap();
    let from_xml = scan_authored_facts(xml.path(), "org.example/pkg", SourceKind::Package).unwrap();
    assert_eq!(from_md, from_xml);
    assert_eq!(
        from_md,
        vec![AuthoredFact {
            address: "spec://org.example/pkg/RULE#FIRST".to_string(),
            status: Some(status("impl/done")),
        }]
    );
}

#[test]
fn a_same_stem_pair_refuses_even_with_disjoint_fact_ids() {
    let root = tempdir().expect("tempdir");
    // Disjoint anchors: the duplicate-full-address arm cannot fire —
    // only the one-document/one-form law sees this split brain.
    write_spec(
        root.path(),
        &spec_rel("RULE.md"),
        "# Rules\n\n@fact:ONLY_MD The markdown anchor. @status:impl/done\n",
    );
    write_spec(
        root.path(),
        &spec_rel("RULE.xml"),
        format!(
            "<spec {XML_NS}><p><fact id=\"ONLY_XML\" status=\"impl/done\">the xml anchor</fact></p></spec>"
        )
        .as_str(),
    );

    let error = scan_authored_facts(root.path(), "org.example/pkg", SourceKind::Host).unwrap_err();
    assert!(
        matches!(&error, RegistryError::SpecParse { line: 1, message, .. }
            if message.contains("one document, one form")
                && message.contains(&spec_rel("RULE.md"))
                && message.contains(&spec_rel("RULE.xml"))),
        "{error}"
    );
}

#[test]
fn host_scans_the_specs_root_and_package_adds_the_root_readme() {
    let root = tempdir().expect("tempdir");
    write_spec(
        root.path(),
        &spec_rel("RULE.md"),
        "# Rules\n\n@fact:DOC The document.\n",
    );
    write_spec(
        root.path(),
        "README.md",
        "# Readme\n\n@fact:README The root readme.\n",
    );

    let host = scan_authored_facts(root.path(), "org.example/demo", SourceKind::Host).unwrap();
    assert_eq!(host.len(), 1);
    assert_eq!(host[0].address, "spec://org.example/demo/RULE#DOC");

    let package = scan_authored_facts(root.path(), "org.example/pkg", SourceKind::Package).unwrap();
    let addresses: Vec<&str> = package.iter().map(|fact| fact.address.as_str()).collect();
    assert_eq!(
        addresses,
        [
            "spec://org.example/pkg/README#README",
            "spec://org.example/pkg/RULE#DOC"
        ]
    );
}

#[test]
fn generated_boot_lane_is_excluded_from_the_scan() {
    let root = tempdir().expect("tempdir");
    write_spec(
        root.path(),
        &spec_rel("RULE.md"),
        "# Rules\n\n@fact:KEPT The authored fact.\n",
    );
    write_spec(
        root.path(),
        &spec_rel("boot/INDEX.md"),
        "# generated boot manifest\n\n@fact:INDEX Generated.\n",
    );
    write_spec(
        root.path(),
        &spec_rel("boot/STATIC.md"),
        "# generated static lane\n\n@fact:STATIC Generated.\n",
    );
    write_spec(
        root.path(),
        &spec_rel("boot/STATIC.xml"),
        format!(
            "<spec {XML_NS}><p><fact id=\"GENERATED\" status=\"impl/done\">lane</fact></p></spec>"
        )
        .as_str(),
    );

    let scanned = scan_authored_facts(root.path(), "org.example/pkg", SourceKind::Package).unwrap();
    assert_eq!(scanned.len(), 1);
    assert_eq!(scanned[0].address, "spec://org.example/pkg/RULE#KEPT");
}

#[test]
fn invalid_coordinate_refuses_before_a_scan_result_is_minted() {
    // A malformed spec sits in the tree: had the walk run first, the
    // failure would be the parse error, not the coordinate refusal.
    let root = tempdir().expect("tempdir");
    write_spec(
        root.path(),
        &spec_rel("RULE.md"),
        "# Rule {#root}\n\n@fact:BROKEN Broken. @status:not-a-stage/done\n",
    );
    for bad in ["org.example", "org.example/pkg/extra", "org.example/Pkg"] {
        assert!(
            matches!(
                scan_authored_facts(root.path(), bad, SourceKind::Host),
                Err(RegistryError::InvalidCoordinate { .. })
            ),
            "`{bad}` must refuse as a coordinate"
        );
        assert!(
            matches!(
                join_adoption(&Registry::default(), bad, SourceKind::Host, &[]),
                Err(RegistryError::InvalidCoordinate { .. })
            ),
            "`{bad}` must refuse as a coordinate before joining"
        );
    }
}

#[test]
fn duplicate_full_address_across_documents_refuses_deterministically() {
    let root = tempdir().expect("tempdir");
    // Both filenames canonicalise to the `PROP-009` doc-path, so the two
    // `LAW` facts are one full address minted twice.
    write_spec(
        root.path(),
        &spec_rel("PROP-009-first.md"),
        "# First\n\n@fact:LAW The law. @status:impl/done\n",
    );
    write_spec(
        root.path(),
        &spec_rel("PROP-009-second.md"),
        "# Second\n\n@fact:LAW The law again. @status:impl/done\n",
    );

    let first = scan_authored_facts(root.path(), "org.example/pkg", SourceKind::Host);
    let second = scan_authored_facts(root.path(), "org.example/pkg", SourceKind::Host);
    let (first, second) = (first.unwrap_err(), second.unwrap_err());
    assert_eq!(first.to_string(), second.to_string());
    assert!(
        matches!(&first, RegistryError::SpecParse { line: 1, message, .. } if message.contains(
            "duplicate full fact address `spec://org.example/pkg/PROP-009#LAW`"
        )),
        "{first}"
    );
}

#[test]
fn join_reaches_all_four_adoption_states_and_keeps_host_rows_pure() {
    let temp = tempdir().expect("tempdir");
    let mut registry = Registry::default();
    registry
        .upsert(
            temp.path(),
            package_fact("spec://org.example/pkg/RULE#RECORDED", "impl/done"),
        )
        .unwrap();
    let mut indeterminate = package_fact("spec://org.example/pkg/RULE#MARKED", "spec/work");
    indeterminate.status = None;
    registry.upsert(temp.path(), indeterminate).unwrap();
    // The registry also holds a host-origin record with a status — host
    // rows must stay `NotApplicable` regardless of registry content.
    registry
        .upsert(
            temp.path(),
            FactEntry::for_host(
                "spec://org.example/demo/common/RULE#HOST",
                "org.example/demo",
                Some(status("impl/done")),
                None,
            )
            .unwrap(),
        )
        .unwrap();

    let authored = vec![
        AuthoredFact {
            address: "spec://org.example/pkg/RULE#ABSENT".to_string(),
            status: Some(status("spec/plan")),
        },
        AuthoredFact {
            address: "spec://org.example/pkg/RULE#MARKED".to_string(),
            status: Some(status("spec/work")),
        },
        AuthoredFact {
            address: "spec://org.example/pkg/RULE#RECORDED".to_string(),
            status: Some(status("spec/plan")),
        },
        AuthoredFact {
            address: "spec://org.example/demo/common/RULE#HOST".to_string(),
            status: Some(status("spec/work")),
        },
    ];

    let package_rows =
        join_adoption(&registry, "org.example/pkg", SourceKind::Package, &authored).unwrap();
    let by_address = |address: &str| {
        package_rows
            .iter()
            .find(|row| row.address == address)
            .unwrap_or_else(|| panic!("missing row for {address}"))
    };
    assert_eq!(
        by_address("spec://org.example/pkg/RULE#ABSENT").adoption,
        AdoptionObservation::Absent
    );
    assert_eq!(
        by_address("spec://org.example/pkg/RULE#MARKED").adoption,
        AdoptionObservation::Indeterminate
    );
    assert_eq!(
        by_address("spec://org.example/pkg/RULE#RECORDED").adoption,
        AdoptionObservation::Recorded(status("impl/done"))
    );
    // The recorded status is the registry's exact value, not the
    // authored one (`spec/plan` above).
    assert_eq!(
        by_address("spec://org.example/pkg/RULE#RECORDED").authored_status,
        Some(status("spec/plan"))
    );

    let host_rows =
        join_adoption(&registry, "org.example/demo", SourceKind::Host, &authored).unwrap();
    assert!(
        host_rows
            .iter()
            .all(|row| row.adoption == AdoptionObservation::NotApplicable),
        "no host row may acquire a package adoption state: {host_rows:?}"
    );
    assert_eq!(host_rows.len(), authored.len());
}

#[test]
fn shuffled_authored_input_cannot_change_joined_output_order() {
    let temp = tempdir().expect("tempdir");
    let mut registry = Registry::default();
    registry
        .upsert(
            temp.path(),
            package_fact("spec://org.example/pkg/RULE#B", "impl/done"),
        )
        .unwrap();
    let authored = vec![
        AuthoredFact {
            address: "spec://org.example/pkg/RULE#C".to_string(),
            status: None,
        },
        AuthoredFact {
            address: "spec://org.example/pkg/RULE#A".to_string(),
            status: None,
        },
        AuthoredFact {
            address: "spec://org.example/pkg/RULE#B".to_string(),
            status: None,
        },
    ];
    let straight =
        join_adoption(&registry, "org.example/pkg", SourceKind::Package, &authored).unwrap();
    let mut shuffled = authored.clone();
    shuffled.reverse();
    let reversed =
        join_adoption(&registry, "org.example/pkg", SourceKind::Package, &shuffled).unwrap();
    assert_eq!(straight, reversed);
    let addresses: Vec<&str> = straight.iter().map(|row| row.address.as_str()).collect();
    assert_eq!(
        addresses,
        [
            "spec://org.example/pkg/RULE#A",
            "spec://org.example/pkg/RULE#B",
            "spec://org.example/pkg/RULE#C",
        ]
    );
}

#[test]
fn an_explicit_symlink_document_is_not_followed() {
    let root = tempdir().expect("tempdir");
    write_spec(
        root.path(),
        &spec_rel("RULE.md"),
        "# Rules\n\n@fact:REAL The real document.\n",
    );
    let target = root.path().join(spec_rel("RULE.md"));
    let link = root.path().join(spec_rel("LINK.md"));
    #[cfg(unix)]
    let linked = std::os::unix::fs::symlink(&target, &link).is_ok();
    #[cfg(windows)]
    let linked = std::os::windows::fs::symlink_file(&target, &link).is_ok();
    if !linked {
        // No symlink privilege on this host (Win32 1314 without
        // Developer Mode): the oracle cannot be planted, and a silent
        // pass would lie — say so and skip.
        eprintln!("skipping: this host cannot create the symlink oracle");
        return;
    }

    let scanned = scan_authored_facts(root.path(), "org.example/pkg", SourceKind::Host).unwrap();
    assert_eq!(scanned.len(), 1);
    assert_eq!(scanned[0].address, "spec://org.example/pkg/RULE#REAL");
}
