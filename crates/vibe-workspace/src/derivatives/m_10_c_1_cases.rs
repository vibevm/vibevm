use super::*;
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/next/derivatives/service_fixture.rs"
));

fn lifecycle(family: &str) {
    for sealed in [false, true] {
        let fixture = Fixture::new(family);
        for file in &fixture.binding.base().files {
            let path = fixture.base.root_path().join(&file.path);
            let mut mode = fs::metadata(&path).unwrap().permissions();
            mode.set_readonly(sealed);
            fs::set_permissions(path, mode).unwrap();
        }
        let create = || {
            MaterializedDerivative::derive(
                &fixture.text,
                &fixture.base,
                &fixture.local,
                &fixture.output,
                "result",
            )
            .unwrap()
        };
        let result = create();
        let shown = result.show().unwrap();
        assert_eq!(shown.effects, Effects::NoExternalEffects);
        assert_eq!(shown.value.local.len(), fixture.expected.len());
        for file in &fixture.expected {
            assert_eq!(
                fs::read(result.tree().root_path().join(&file.path)).unwrap(),
                file.bytes
            );
            let row = shown
                .value
                .local
                .iter()
                .find(|r| r.path == file.path)
                .unwrap();
            assert_eq!(row.raw, file.bytes);
            assert_eq!(
                row.raw_sha256,
                format!("sha256:{}", crate::vibedeps::sha256_bytes(&file.bytes))
            );
            assert_eq!(row.vibe_projection.is_some(), family == "vibe-spec");
            if let Some(projection) = &row.vibe_projection {
                assert!(
                    projection
                        .as_ref()
                        .unwrap()
                        .0
                        .contains("Retry at most once")
                );
            }
        }
        let diff = result.diff().unwrap();
        assert_eq!(diff.effects, Effects::NoExternalEffects);
        assert_eq!(
            diff.value.len(),
            if family == "nested-tree" { 4 } else { 1 }
        );
        if family == "nested-tree" {
            assert!(
                diff.value
                    .iter()
                    .any(|d| d.path == "payload/docs/obsolete.txt" && d.after.is_none())
            );
            assert!(
                diff.value
                    .iter()
                    .any(|d| d.path == "payload/docs/local.txt" && d.before.is_none())
            );
        }
        let identity = shown.value.result_identity;
        assert_eq!(result.result().record().binding_toml, fixture.text);
        drop(result);
        fs::remove_dir_all(fixture.output.root_path().join("result")).unwrap();
        let retained = fs::read_to_string(fixture._home.path().join("authored.toml")).unwrap();
        assert_eq!(retained, fixture.text);
        let recreated = MaterializedDerivative::derive(
            &retained,
            &Project::open(fixture.base.root_path()).unwrap(),
            &Project::open(fixture.local.root_path()).unwrap(),
            &fixture.output,
            "result",
        )
        .unwrap();
        assert_eq!(recreated.result().record().result_identity, identity);
        assert_eq!(
            capture_retained(&fixture.base, fixture.binding.base())
                .unwrap()
                .retained_source("payload"),
            *fixture.binding.base()
        );
        assert_eq!(
            capture_retained(&fixture.local, fixture.binding.local())
                .unwrap()
                .retained_source("payload"),
            *fixture.binding.local()
        );
        assert!(!fixture._home.path().join("DO_NOT_EXECUTE").exists());
    }
}
#[test]
fn commonmark_real_result_recreated() {
    lifecycle("commonmark");
}
#[test]
fn xml_data_real_result_recreated() {
    lifecycle("xml-data");
}
#[test]
fn vibe_spec_real_result_recreated() {
    lifecycle("vibe-spec");
}
#[test]
fn opaque_file_real_result_recreated() {
    lifecycle("opaque-file");
}
#[test]
fn nested_tree_real_result_recreated() {
    lifecycle("nested-tree");
}

#[test]
fn changed_base_local_missing_and_extra_resources_refuse_before_staging() {
    for change in 0..6 {
        let f = Fixture::new("nested-tree");
        match change {
            0 => {
                f.base.write_atomic("payload/SKILL.md", b"changed").unwrap();
            }
            1 => {
                f.local
                    .write_atomic("payload/SKILL.md", b"changed")
                    .unwrap();
            }
            2 => {
                f.local.remove_file("payload/docs/keep.txt").unwrap();
            }
            3 => {
                f.local
                    .write_atomic("payload/docs/extra.txt", b"extra")
                    .unwrap();
            }
            4 => {
                f.base.remove_file("payload/docs/keep.txt").unwrap();
            }
            _ => {
                f.base
                    .write_atomic("payload/docs/extra.txt", b"extra")
                    .unwrap();
            }
        }
        let error = MaterializedDerivative::derive(&f.text, &f.base, &f.local, &f.output, "result")
            .unwrap_err();
        assert!(error.residue.is_none());
        assert_eq!(fs::read_dir(f.output.root_path()).unwrap().count(), 0);
    }
}
#[test]
fn traversal_duplicate_case_and_file_directory_collisions_refuse() {
    let f = Fixture::new("nested-tree");
    for bad_path in [
        "../escape",
        "payload/../escape",
        "payload/SKILL.md",
        "payload/skill.md",
        "payload/docs",
        "payload/.vibe-stage-evil",
    ] {
        let mut value = serde_json::to_value(&f.binding).unwrap();
        value["local"]["files"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"path":bad_path,"bytes":[0]}));
        // TOML owner parsing rejects malformed inventories before output creation.
        let bad = toml::to_string(&value).unwrap();
        assert!(
            MaterializedDerivative::derive(&bad, &f.base, &f.local, &f.output, "result").is_err(),
            "{bad_path}"
        );
    }
    assert_eq!(fs::read_dir(f.output.root_path()).unwrap().count(), 0);
}
#[test]
fn occupied_destination_and_overlapping_source_are_never_adopted() {
    let f = Fixture::new("nested-tree");
    fs::create_dir(f.output.root_path().join("result")).unwrap();
    fs::write(f.output.root_path().join("result/FOREIGN.txt"), b"neighbor").unwrap();
    assert!(
        MaterializedDerivative::derive(&f.text, &f.base, &f.local, &f.output, "result").is_err()
    );
    assert_eq!(
        fs::read(f.output.root_path().join("result/FOREIGN.txt")).unwrap(),
        b"neighbor"
    );
    assert!(
        MaterializedDerivative::derive(&f.text, &f.base, &f.local, &f.local, "result").is_err()
    );
}
#[test]
fn reader_refuses_unpublished_changed_and_extra_output() {
    for change in 0..4 {
        let f = Fixture::new("opaque-file");
        let result =
            MaterializedDerivative::derive(&f.text, &f.base, &f.local, &f.output, "result")
                .unwrap();
        let entry = f.output.subproject(&["result"]).unwrap();
        match change {
            0 => {
                entry.remove_file("result.json").unwrap();
            }
            1 => {
                result
                    .tree()
                    .write_atomic("payload.bin", b"changed")
                    .unwrap();
            }
            2 => {
                result.tree().write_atomic("EXTRA", b"extra").unwrap();
            }
            _ => {
                result.tree().remove_file("payload.bin").unwrap();
            }
        }
        assert!(MaterializedDerivative::read(&entry, &f.base, &f.local, 1 << 20).is_err());
    }
}

#[cfg(unix)]
#[test]
fn links_never_supply_source_bytes() {
    let f = Fixture::new("opaque-file");
    let path = f.local.root_path().join("payload.bin");
    fs::remove_file(&path).unwrap();
    std::os::unix::fs::symlink(f.base.root_path().join("payload.bin"), &path).unwrap();
    assert!(
        MaterializedDerivative::derive(&f.text, &f.base, &f.local, &f.output, "result").is_err()
    );
    assert_eq!(fs::read_dir(f.output.root_path()).unwrap().count(), 0);
}

#[test]
fn commit_record_failure_reports_unpublished_residue() {
    let f = Fixture::new("nested-tree");
    vibe_safefs::fail_before_publish(Some("result.json"));
    let failure = MaterializedDerivative::derive(&f.text, &f.base, &f.local, &f.output, "result");
    vibe_safefs::fail_before_publish(None);
    let failure = failure.unwrap_err();
    assert_eq!(failure.residue, Some(f.output.root_path().join("result")));
    assert!(!failure.record_may_be_published);
    let entry = f.output.subproject(&["result"]).unwrap();
    assert!(MaterializedDerivative::read(&entry, &f.base, &f.local, 1 << 20).is_err());
    assert!(!entry.root_path().join("result.json").exists());
    capture_retained(&f.local, f.binding.local()).unwrap();
}

#[test]
fn commonmark_named_export_is_bound_to_actual_result_and_keeps_siblings() {
    let f = Fixture::new("commonmark");
    let result =
        MaterializedDerivative::derive(&f.text, &f.base, &f.local, &f.output, "result").unwrap();
    let view = result.show().unwrap().value;
    let mut hashes = Vec::new();
    for file in [&view.base[0], &view.local[0]] {
        let raw = std::str::from_utf8(&file.raw).unwrap();
        let doc = vibe_specdoc::commonmark::CommonMarkDoc::parse(raw);
        let selected = doc
            .resolve_export("usage.v1", &["Guide".into(), "Usage".into()])
            .unwrap();
        assert!(raw[selected.span.clone()].contains("DO_NOT_EXECUTE"));
        assert!(!raw[selected.span].contains("Keep this section."));
        assert!(raw.ends_with("## Reference\nKeep this section.\n"));
        assert!(doc.resolve_export("missing", &["Absent".into()]).is_err());
        hashes.push(selected.source_sha256);
    }
    assert_ne!(hashes[0], hashes[1]);
}
