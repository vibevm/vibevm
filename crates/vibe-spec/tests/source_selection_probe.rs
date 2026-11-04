//! Public source-selection invariants, fixed after retained stage-one RED evidence.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use vibe_spec::{
    FileResolver, FsSectionSource, SectionSource, SelectedPackage, SelfCoordinate, SpecAddress,
    expand_embeds,
};

#[cfg(test)]
#[path = "source_selection/edges.rs"]
mod edges;

#[cfg(test)]
fn own() -> SelfCoordinate {
    SelfCoordinate::new(Some("org.host".into()), "self".into())
}

#[cfg(test)]
fn address(raw: &str) -> SpecAddress {
    SpecAddress::parse(raw).unwrap()
}

#[cfg(test)]
fn write(root: &Path, relative: &str, bytes: &str) -> PathBuf {
    let file = root.join(relative);
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, bytes).unwrap();
    file
}

#[cfg(test)]
fn slot(workspace: &Path, group: &str, name: &str, version: &str) -> PathBuf {
    workspace.join(format!("vibedeps/{group}.{name}/{version}"))
}

#[cfg(test)]
fn document(root: &Path, name: &str, body: &str) -> PathBuf {
    write(
        root,
        &format!("spec/common/{name}.md"),
        &format!("# Doc {{#root}}\n\n{body}\n"),
    )
}

#[cfg(test)]
fn observe(case: &str, data: Value) {
    println!("OBSERVATION {}", json!({"case":case,"observed":data}));
}

#[cfg(test)]
fn expansion(resolver: &FileResolver, raw: &str) -> Value {
    match resolver.expand_pattern(&address(raw)) {
        Ok(rows) => json!({"ok":rows.iter().map(|a| &a.raw).collect::<Vec<_>>()}),
        Err(error) => json!({"error":error.to_string(),"debug":format!("{error:?}")}),
    }
}

#[cfg(test)]
fn read(source: &FsSectionSource, raw: &str) -> Value {
    match source.section_text(&address(raw)) {
        Ok(text) => json!({"ok":text}),
        Err(error) => json!({"error":error}),
    }
}

#[test]
fn invariant_requested_duplicate_anchor_refuses_in_section_and_embed() {
    let mut accepted = Vec::new();
    for (suffix, text) in [
        (
            "md",
            "# First {#same}\n\nFIRST_ONLY\n\n# Second {#same}\n\nSECOND_ONLY\n",
        ),
        (
            "xml",
            "<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">D</title><section id=\"same\" title=\"First\"><p>FIRST_ONLY</p></section><section id=\"same\" title=\"Second\"><p>SECOND_ONLY</p></section></spec>",
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), &format!("spec/common/DUP.{suffix}"), text);
        let source = FsSectionSource::new(FileResolver::new(root.path(), own()));
        let raw = "spec://org.host/self/common/DUP#same";
        let section = source.section_text(&address(raw));
        let embedded = expand_embeds(&format!("before\n#embed {raw}\nafter\n"), &source);
        observe(
            "requested_duplicate",
            json!({"format":suffix,"section":format!("{section:?}"),"embed":format!("{embedded:?}")}),
        );
        if section.is_ok() {
            accepted.push(format!("{suffix}:section"));
        }
        if embedded.is_ok() {
            accepted.push(format!("{suffix}:embed"));
        }
    }
    assert!(
        accepted.is_empty(),
        "requested ambiguous anchor returned a body: {accepted:?}"
    );
}

#[test]
fn control_markdown_paths_and_characterize_xml_global_id_uniqueness() {
    for (suffix, text) in [
        (
            "md",
            "# D {#root}\n\n## Unique {#unique}\n\nUNIQUE_BODY\n\n## Left {#left}\n\n### Leaf {#leaf}\n\nLEFT_BODY\n\n## Right {#right}\n\n### Leaf {#leaf}\n\nRIGHT_BODY\n\n## First {#other}\nOTHER_ONE\n\n## Second {#other}\nOTHER_TWO\n",
        ),
        (
            "xml",
            "<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">D</title><section id=\"unique\" title=\"Unique\"><p>UNIQUE_BODY</p></section><section id=\"left\" title=\"Left\"><section id=\"leaf\" title=\"Leaf\"><p>LEFT_BODY</p></section></section><section id=\"right\" title=\"Right\"><section id=\"leaf\" title=\"Leaf\"><p>RIGHT_BODY</p></section></section><section id=\"other\" title=\"First\"><p>OTHER_ONE</p></section><section id=\"other\" title=\"Second\"><p>OTHER_TWO</p></section></spec>",
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), &format!("spec/common/CONTROL.{suffix}"), text);
        let source = FsSectionSource::new(FileResolver::new(root.path(), own()));
        for (anchor, wanted, unwanted) in [
            ("unique", "UNIQUE_BODY", "LEFT_BODY"),
            ("left.leaf", "LEFT_BODY", "RIGHT_BODY"),
            ("right.leaf", "RIGHT_BODY", "LEFT_BODY"),
        ] {
            let raw = format!("spec://org.host/self/common/CONTROL#{anchor}");
            let result = source.section_text(&address(&raw));
            observe(
                "unique_path_control",
                json!({"format":suffix,"anchor":anchor,"result":format!("{result:?}")}),
            );
            if suffix == "xml" {
                // Existing XML projection validates the whole document before
                // selection. This is characterization, not a new blanket rule.
                assert!(result.unwrap_err().contains("defined twice"));
                assert!(expand_embeds(&format!("#embed {raw}\n"), &source).is_err());
                continue;
            }
            let result = result.unwrap();
            assert!(
                result.contains(wanted) && !result.contains(unwanted),
                "{result}"
            );
            let embedded = expand_embeds(&format!("#embed {raw}\n"), &source).unwrap();
            assert!(embedded.contains(wanted) && !embedded.contains(unwanted));
        }
    }
}

#[test]
fn control_unique_xml_projection_remains_readable() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "spec/common/VALID.xml",
        "<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">D</title><section id=\"left\" title=\"Left\"><section id=\"leaf\" title=\"Leaf\"><p>VALID_XML_BODY</p></section></section></spec>",
    );
    let source = FsSectionSource::new(FileResolver::new(root.path(), own()));
    let raw = "spec://org.host/self/common/VALID#left.leaf";
    let result = source.section_text(&address(raw)).unwrap();
    observe("valid_xml_control", json!({"address":raw,"text":result}));
    assert!(result.contains("VALID_XML_BODY"));
    assert!(
        expand_embeds(&format!("#embed {raw}\n"), &source)
            .unwrap()
            .contains("VALID_XML_BODY")
    );
}

#[test]
fn invariant_matched_document_failures_do_not_disappear_from_pattern() {
    let mut hidden = Vec::new();
    for case in ["pair", "id_stems", "non_directory_document_parent"] {
        let ws = tempfile::tempdir().unwrap();
        let selected = slot(ws.path(), "org.demo", "plugin-a", "1.0.0");
        let doc = match case {
            "pair" => {
                document(&selected, "DOC", "MARKDOWN");
                write(
                    &selected,
                    "spec/common/DOC.xml",
                    "<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">XML</title></spec>",
                );
                "common/DOC"
            }
            "id_stems" => {
                document(&selected, "PROP-042-one", "ONE");
                document(&selected, "PROP-042-two", "TWO");
                "common/PROP-042"
            }
            _ => {
                write(&selected, "spec/common", "not a directory");
                "common/DOC"
            }
        };
        let resolver = FileResolver::new(ws.path(), own());
        let point = resolver.resolve_file(&address(&format!("spec://org.demo/plugin-a/{doc}")));
        let pattern = resolver.expand_pattern(&address(&format!("spec://org.demo/plugin-*/{doc}")));
        observe(
            "matched_failure",
            json!({"shape":case,"point":format!("{point:?}"),"pattern":format!("{pattern:?}")}),
        );
        assert!(point.is_err(), "fixture must exercise a real point refusal");
        if pattern.is_ok() {
            hidden.push(case);
        }
    }
    assert!(
        hidden.is_empty(),
        "matched point failures were hidden as set success: {hidden:?}"
    );
}

#[test]
fn control_absence_unmatched_names_and_empty_world_remain_empty() {
    let ws = tempfile::tempdir().unwrap();
    let resolver = FileResolver::new(ws.path(), own());
    assert!(
        resolver
            .expand_pattern(&address("spec://org.demo/plugin-*/common/DOC"))
            .unwrap()
            .is_empty()
    );
    let root = slot(ws.path(), "org.demo", "plugin-a", "1.0.0");
    document(&root, "OTHER", "OTHER");
    for raw in [
        "spec://org.demo/plugin-*/common/DOC",
        "spec://org.demo/absent-*/common/OTHER",
    ] {
        let value = expansion(&resolver, raw);
        observe("absence_control", json!({"address":raw,"result":value}));
        assert!(resolver.expand_pattern(&address(raw)).unwrap().is_empty());
    }
    assert!(
        resolver
            .resolve_file(&address("spec://org.demo/plugin-a/common/DOC"))
            .is_err()
    );
}

#[test]
fn matching_non_directory_slot_refuses_as_a_source_observation() {
    let ws = tempfile::tempdir().unwrap();
    write(ws.path(), "vibedeps/org.demo.plugin-a", "ordinary file");
    let resolver = FileResolver::new(ws.path(), own());
    let point = resolver.resolve_file(&address("spec://org.demo/plugin-a/common/DOC"));
    let pattern = resolver.expand_pattern(&address("spec://org.demo/plugin-*/common/DOC"));
    observe(
        "non_directory_slot_policy",
        json!({"point":format!("{point:?}"),"pattern":format!("{pattern:?}"),"policy":"matching malformed slot refuses; nonmatching names are not candidates"}),
    );
    assert!(point.is_err());
    assert!(pattern.is_err());
}

#[test]
fn invariant_selected_world_drives_membership_instead_of_unselected_disk_slots() {
    let ws = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let a = external.path().join("selected-a");
    let only = external.path().join("selected-only");
    document(&a, "DOC", "SELECTED_A_V1");
    document(&only, "DOC", "SELECTED_ONLY_V3");
    let installed_doc = document(
        &slot(ws.path(), "org.demo", "plugin-a", "9.0.0"),
        "DOC",
        "UNSELECTED_A_V9",
    );
    document(
        &slot(ws.path(), "org.demo", "plugin-stray", "7.0.0"),
        "DOC",
        "UNSELECTED_STRAY",
    );
    let resolver = FileResolver::with_selected_world(
        ws.path(),
        ws.path(),
        BTreeMap::from([
            (
                ("org.demo".into(), "plugin-a".into()),
                SelectedPackage::new("1.0.0", &a),
            ),
            (
                ("org.demo".into(), "plugin-only".into()),
                SelectedPackage::new("3.0.0", &only),
            ),
        ]),
        own(),
    );
    let source = FsSectionSource::new(resolver.clone());
    let raw = "spec://org.demo/plugin-*/common/DOC#root";
    let assert_selected_bodies = |members: &[SpecAddress]| {
        assert_eq!(members.len(), 2);
        for (name, expected) in [
            ("plugin-a", "SELECTED_A_V1"),
            ("plugin-only", "SELECTED_ONLY_V3"),
        ] {
            let raw = format!("spec://org.demo/{name}/common/DOC#root");
            let emitted = members.iter().find(|a| a.raw == raw).unwrap();
            let body = source.section_text(emitted).unwrap();
            assert!(body.contains(expected), "{raw}: {body:?}");
            assert!(!body.contains("UNSELECTED"), "{raw}: {body:?}");
        }
    };
    let before = resolver.expand_pattern(&address(raw)).unwrap();
    assert_selected_bodies(&before);
    let reads: Vec<_> = before
        .iter()
        .map(|a| json!({"address":a.raw,"read":read(&source,&a.raw)}))
        .collect();
    let point_a = read(&source, "spec://org.demo/plugin-a@1.0.0/common/DOC#root");
    let point_only = read(&source, "spec://org.demo/plugin-only@3.0.0/common/DOC#root");
    fs::remove_file(installed_doc).unwrap();
    let after = resolver.expand_pattern(&address(raw)).unwrap();
    assert_selected_bodies(&after);
    observe(
        "selected_world_membership",
        json!({"before":before.iter().map(|a|&a.raw).collect::<Vec<_>>(),"emitted_reads":reads,"selected_point_a":point_a,"selected_point_only":point_only,"after_removing_only_unselected_doc":after.iter().map(|a|&a.raw).collect::<Vec<_>>() }),
    );
    let mut violations = Vec::new();
    if before.iter().any(|a| a.raw.contains("plugin-stray")) {
        violations.push("unselected slot promoted into expansion");
    }
    if !before.iter().any(|a| a.raw.contains("plugin-only")) {
        violations.push("selected outside-workspace source omitted");
    }
    if before != after {
        violations.push("unselected disk mutation changes selected-world membership");
    }
    assert!(
        violations.is_empty(),
        "selected-world invariants failed: {violations:?}"
    );
}

#[test]
fn invariant_selected_document_absence_cannot_borrow_unselected_newer_presence() {
    let ws = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    document(external.path(), "OTHER", "SELECTED_HAS_NO_DOC");
    document(
        &slot(ws.path(), "org.demo", "plugin-a", "9.0.0"),
        "DOC",
        "UNSELECTED_HAS_DOC",
    );
    let resolver = FileResolver::with_selected_world(
        ws.path(),
        ws.path(),
        BTreeMap::from([(
            ("org.demo".into(), "plugin-a".into()),
            SelectedPackage::new("1.0.0", external.path()),
        )]),
        own(),
    );
    let expanded = resolver
        .expand_pattern(&address("spec://org.demo/plugin-*/common/DOC#root"))
        .unwrap();
    let source = FsSectionSource::new(resolver);
    let reads: Vec<_> = expanded
        .iter()
        .map(|a| json!({"address":a.raw,"read":read(&source,&a.raw)}))
        .collect();
    observe(
        "selected_absence_membership",
        json!({"emitted":reads,"point":read(&source,"spec://org.demo/plugin-a@1.0.0/common/DOC#root")}),
    );
    assert!(
        expanded.is_empty(),
        "selected document was absent but unselected bytes supplied membership"
    );
}

#[test]
fn pattern_group_is_fixed_and_other_groups_cannot_control_membership() {
    let ws = tempfile::tempdir().unwrap();
    document(
        &slot(ws.path(), "org.foreign", "plugin-a", "1.0.0"),
        "DOC",
        "FOREIGN_BODY",
    );
    let resolver = FileResolver::new(ws.path(), own());
    let source = FsSectionSource::new(resolver.clone());
    assert!(
        resolver
            .expand_pattern(&address("spec://org.demo/plugin-*/common/DOC#root"))
            .unwrap()
            .is_empty()
    );
    let foreign = resolver
        .expand_pattern(&address("spec://org.foreign/plugin-*/common/DOC#root"))
        .unwrap();
    assert_eq!(foreign.len(), 1);
    assert!(
        source
            .section_text(&foreign[0])
            .unwrap()
            .contains("FOREIGN_BODY")
    );

    let ws = tempfile::tempdir().unwrap();
    let alpha = slot(ws.path(), "org.alpha", "plugin-a", "1.0.0");
    let zeta = slot(ws.path(), "org.zeta", "plugin-a", "1.0.0");
    document(&alpha, "OTHER", "ALPHA_NO_DOC");
    document(&zeta, "DOC", "ZETA_BODY");
    let resolver = FileResolver::new(ws.path(), own());
    let source = FsSectionSource::new(resolver.clone());
    let pattern = address("spec://org.zeta/plugin-*/common/DOC#root");
    let before = resolver.expand_pattern(&pattern).unwrap();
    assert_eq!(before.len(), 1);
    document(&alpha, "DOC", "ALPHA_BODY");
    let after = resolver.expand_pattern(&pattern).unwrap();
    assert_eq!(before, after);
    assert!(
        source
            .section_text(&after[0])
            .unwrap()
            .contains("ZETA_BODY")
    );
    let alpha = resolver
        .expand_pattern(&address("spec://org.alpha/plugin-*/common/DOC#root"))
        .unwrap();
    assert!(
        source
            .section_text(&alpha[0])
            .unwrap()
            .contains("ALPHA_BODY")
    );
}

#[test]
fn explicit_pattern_pin_is_honored_and_missing_pin_does_not_fall_back() {
    let ws = tempfile::tempdir().unwrap();
    document(
        &slot(ws.path(), "org.demo", "plugin-a", "1.0.0"),
        "DOC",
        "EXACT_V1",
    );
    document(
        &slot(ws.path(), "org.demo", "plugin-a", "2.0.0"),
        "DOC",
        "NEWEST_V2",
    );
    let resolver = FileResolver::new(ws.path(), own());
    let source = FsSectionSource::new(resolver.clone());
    let exact = resolver
        .expand_pattern(&address(
            "spec://org.demo/plugin-*@1.0.0/common/DOC#root~r2",
        ))
        .unwrap();
    assert_eq!(exact.len(), 1);
    assert_eq!(
        exact[0].raw,
        "spec://org.demo/plugin-a@1.0.0/common/DOC#root~r2"
    );
    assert_eq!(exact[0].pinned_r, Some(2)); // retained data, not revision verification
    assert!(source.section_text(&exact[0]).unwrap().contains("EXACT_V1"));
    assert!(
        resolver
            .expand_pattern(&address("spec://org.demo/plugin-*@99.0.0/common/DOC#root"))
            .unwrap()
            .is_empty()
    );
    let newest = resolver
        .expand_pattern(&address("spec://org.demo/plugin-*/common/DOC#root"))
        .unwrap();
    assert_eq!(newest.len(), 1);
    assert!(!newest[0].raw.contains('@'));
    assert!(
        source
            .section_text(&newest[0])
            .unwrap()
            .contains("NEWEST_V2")
    );
}

#[test]
fn selected_pin_mismatch_refuses_without_touching_unrelated_broken_slots() {
    let ws = tempfile::tempdir().unwrap();
    let chosen = tempfile::tempdir().unwrap();
    document(chosen.path(), "DOC", "SELECTED");
    write(
        ws.path(),
        "vibedeps",
        "this entire installed layout is not a directory",
    );
    let resolver = FileResolver::with_selected_world(
        ws.path(),
        ws.path(),
        BTreeMap::from([
            (
                ("org.demo".into(), "plugin-a".into()),
                SelectedPackage::new("1.0.0", chosen.path()),
            ),
            (
                ("org.foreign".into(), "plugin-b".into()),
                SelectedPackage::new("7.0.0", "/absent/foreign"),
            ),
        ]),
        own(),
    );
    let exact = resolver
        .expand_pattern(&address("spec://org.demo/plugin-*@1.0.0/common/DOC"))
        .unwrap();
    assert_eq!(exact.len(), 1);
    assert!(matches!(
        resolver.expand_pattern(&address("spec://org.demo/plugin-*@2.0.0/common/DOC")),
        Err(vibe_spec::ResolveError::SelectedVersionMismatch { .. })
    ));
}

#[test]
fn requested_child_ambiguity_is_local_while_whole_and_parent_remain_readable() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "spec/common/DOC.md",
        "# Parent {#parent}\n## First {#child}\nONE\n## Second {#child}\nTWO\n",
    );
    let source = FsSectionSource::new(FileResolver::new(root.path(), own()));
    let error = source
        .section_text(&address("spec://org.host/self/common/DOC#parent.child"))
        .unwrap_err();
    assert!(
        error.contains("child") && error.contains("segment 1") && error.contains("ambiguous"),
        "{error}"
    );
    for raw in [
        "spec://org.host/self/common/DOC",
        "spec://org.host/self/common/DOC#parent",
    ] {
        let text = source.section_text(&address(raw)).unwrap();
        assert!(text.contains("ONE") && text.contains("TWO"));
    }
}

#[cfg(unix)]
#[test]
fn matching_symlink_slots_refuse_while_other_names_and_groups_are_not_opened() {
    use std::os::unix::fs::symlink;
    let ws = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    document(&outside.path().join("1.0.0"), "DOC", "OUTSIDE");
    document(
        &slot(ws.path(), "org.demo", "plugin-a", "1.0.0"),
        "DOC",
        "ALLOWED",
    );
    symlink(
        outside.path(),
        ws.path().join("vibedeps/org.other.plugin-b"),
    )
    .unwrap();
    write(
        ws.path(),
        "vibedeps/org.demo.unmatched",
        "not a package directory",
    );
    let resolver = FileResolver::new(ws.path(), own());
    assert_eq!(
        resolver
            .expand_pattern(&address("spec://org.demo/plugin-*/common/DOC"))
            .unwrap()
            .len(),
        1
    );
    symlink(outside.path(), ws.path().join("vibedeps/org.demo.plugin-b")).unwrap();
    assert!(matches!(
        resolver.expand_pattern(&address("spec://org.demo/plugin-*/common/DOC")),
        Err(vibe_spec::ResolveError::SourceObservation { .. })
    ));
}

#[test]
fn empty_versions_missing_version_and_ordinary_file_layout_fallback_remain_nonerrors() {
    let ws = tempfile::tempdir().unwrap();
    write(
        ws.path(),
        "vibevm",
        "ordinary file leaves legacy layout available",
    );
    fs::create_dir_all(slot(ws.path(), "org.demo", "plugin-empty", "notes")).unwrap();
    document(
        &slot(ws.path(), "org.demo", "plugin-a", "1.0.0"),
        "DOC",
        "LEGACY",
    );
    let resolver = FileResolver::new(ws.path(), own());
    assert_eq!(
        resolver
            .expand_pattern(&address("spec://org.demo/plugin-*/common/DOC"))
            .unwrap()
            .len(),
        1
    );
    assert!(
        resolver
            .expand_pattern(&address("spec://org.demo/plugin-*@99.0.0/common/DOC"))
            .unwrap()
            .is_empty()
    );
}
