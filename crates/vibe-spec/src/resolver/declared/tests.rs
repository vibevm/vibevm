use super::*;
use crate::source_policy::observe_package_policy_in;
use crate::{FsSectionSource, SectionSource};
use sha2::{Digest, Sha256};
use std::fs;

fn fixture() -> (tempfile::TempDir, FileResolver, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let project = Arc::new(Project::open(tmp.path()).unwrap());
    let relative = "vibevm/vibedeps/org.example.native/1.0.0";
    let root = tmp.path().join(relative);
    fs::create_dir_all(root.join("vibevm/vibespecs")).unwrap();
    let manifest = "[package]\nname='native'\ngroup='org.example'\nkind='flow'\nversion='1.0.0'\n[[package.sources]]\nfile='vibevm/vibespecs/start.md'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n";
    fs::write(root.join("vibe.toml"), manifest).unwrap();
    fs::write(root.join("record.bytes"), "opaque host validated metadata").unwrap();
    let bytes=b"<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">Hello</title><p>body</p></spec>";
    fs::write(root.join("vibevm/vibespecs/start.md"), bytes).unwrap();
    let slot = project
        .dir(&relative.split('/').collect::<Vec<_>>(), false)
        .unwrap();
    let policy = observe_package_policy_in(&project, &slot).unwrap();
    let binding = PreparedInstalledSource::from_host_checked(
        relative.into(),
        InstalledSourceExpectation {
            kind: PackageKind::Flow,
            group: Group::parse("org.example").unwrap(),
            name: "native".into(),
            version: "1.0.0".into(),
            source_hash: ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap(),
            materialized_format: SpecFormat::Mixed,
        },
        policy,
        "record.bytes".into(),
        Arc::from(b"opaque host validated metadata".as_slice()),
        vec![PreparedOutputRow {
            source_path: "vibevm/vibespecs/start.md".into(),
            output_path: "vibevm/vibespecs/start.md".into(),
            interpretation: Some(SourceInterpretation::VibeSpec),
            media: "application/xml".into(),
            sha256: format!("{:x}", Sha256::digest(bytes)),
            native_address: Some("start".into()),
        }],
    )
    .unwrap();
    let selected = BTreeMap::from([(
        ("org.example".into(), "native".into()),
        SelectedPackage::new("1.0.0", &root),
    )]);
    let resolver = FileResolver::with_selected_world(
        tmp.path(),
        tmp.path(),
        selected,
        SelfCoordinate::new(Some("org.example".into()), "host".into()),
    )
    .with_installed_source_bindings(project, vec![binding], None)
    .unwrap();
    (tmp, resolver, root)
}

#[test]
fn declared_media_controls_native_projection_and_anchor() {
    let (_tmp, resolver, _) = fixture();
    let source = FsSectionSource::new(resolver);
    let address = SpecAddress::parse("spec://org.example/native@1.0.0/start#root").unwrap();
    let text = source.section_text(&address).unwrap();
    assert!(text.contains("Hello"));
    assert!(!text.contains("<spec"));
}

#[test]
fn bound_read_refuses_manifest_record_and_output_drift() {
    for path in ["vibe.toml", "record.bytes", "vibevm/vibespecs/start.md"] {
        let (_tmp, resolver, root) = fixture();
        let address = SpecAddress::parse("spec://org.example/native/start").unwrap();
        let location = resolver.locate_source(&address, &[]).unwrap();
        fs::write(root.join(path), "changed").unwrap();
        assert!(location.read_with_overlay(None).is_err(), "{path}");
    }
}

#[test]
fn bad_exact_overlay_never_falls_back_to_valid_file() {
    let (_tmp, resolver, _) = fixture();
    let address = SpecAddress::parse("spec://org.example/native/start").unwrap();
    let location = resolver.locate_source(&address, &[]).unwrap();
    assert!(location.read_with_overlay(Some(b"wrong overlay")).is_err());
    assert!(location.read_with_overlay(None).is_ok());
}

#[test]
fn active_custom_frontends_do_not_replace_declared_builtin_media() {
    let (_tmp, resolver, _) = fixture();
    let address = SpecAddress::parse("spec://org.example/native/start").unwrap();
    let location = resolver
        .locate_source(&address, &["custom".into()])
        .unwrap();
    assert_eq!(location.resolved.format, "markdown");
    assert_eq!(location.declared_media().unwrap().1, "application/xml");
}

#[test]
fn bound_world_retains_complete_legacy_selection_and_version_anchor_laws() {
    let (tmp, resolver, _) = fixture();
    let bound = resolver.installed_sources.as_ref().unwrap();
    let mut selected = resolver.selected.clone().unwrap();
    let legacy = tmp.path().join("vibevm/vibedeps/org.example.legacy");
    for (version, text) in [("1.0.0", "selected old"), ("2.0.0", "wrong newer")] {
        let dir = legacy.join(version).join("vibevm/vibespecs");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("doc.md"), format!("# Title {{#root}}\n{text}\n")).unwrap();
    }
    selected.insert(
        ("org.example".into(), "legacy".into()),
        SelectedPackage::new("1.0.0", legacy.join("1.0.0")),
    );
    let resolver = FileResolver::with_selected_world(
        tmp.path(),
        tmp.path(),
        selected,
        SelfCoordinate::new(Some("org.example".into()), "host".into()),
    )
    .with_installed_source_bindings(
        bound.project.clone(),
        bound.bindings.values().cloned().collect(),
        None,
    )
    .unwrap();
    let address = SpecAddress::parse("spec://org.example/legacy@1.0.0/doc#root").unwrap();
    assert!(
        FsSectionSource::new(resolver.clone())
            .section_text(&address)
            .unwrap()
            .contains("selected old")
    );
    assert!(matches!(
        resolver.resolve_file(&SpecAddress::parse("spec://org.example/legacy@2.0.0/doc").unwrap()),
        Err(ResolveError::SelectedVersionMismatch { .. })
    ));
    let expanded = resolver
        .expand_pattern(&SpecAddress::parse("spec://org.example/*/doc#root~r3").unwrap())
        .unwrap();
    assert_eq!(expanded.len(), 1);
    assert_eq!(expanded[0].pinned_r, Some(3));
    assert_eq!(expanded[0].anchor, vec!["root"]);
}

#[cfg(unix)]
#[test]
fn workspace_alias_retarget_and_installed_self_symlink_refuse() {
    let (tmp, resolver, root) = fixture();
    let scope = tempfile::tempdir().unwrap();
    let alias = scope.path().join("workspace");
    std::os::unix::fs::symlink(tmp.path(), &alias).unwrap();
    let bound = resolver.installed_sources.as_ref().unwrap();
    let self_resolver = FileResolver::with_selected_world(
        &root,
        &alias,
        resolver.selected.clone().unwrap(),
        SelfCoordinate::new(Some("org.example".into()), "native".into()),
    )
    .with_installed_source_bindings(
        bound.project.clone(),
        bound.bindings.values().cloned().collect(),
        Some(("org.example".into(), "native".into())),
    )
    .unwrap();
    let address = SpecAddress::parse("spec://org.example/native/start").unwrap();
    assert!(self_resolver.resolve_file(&address).is_ok());
    assert!(matches!(
        self_resolver
            .resolve_file(&SpecAddress::parse("spec://org.example/native@1.0.0/start").unwrap()),
        Err(ResolveError::SelfCoordinateVersioned { .. })
    ));
    fs::remove_file(&alias).unwrap();
    std::os::unix::fs::symlink(scope.path(), &alias).unwrap();
    assert!(self_resolver.resolve_file(&address).is_err());
    fs::remove_file(&alias).unwrap();
    std::os::unix::fs::symlink(tmp.path(), &alias).unwrap();
    let moved = tmp.path().join("moved-self");
    fs::rename(&root, &moved).unwrap();
    std::os::unix::fs::symlink(&moved, &root).unwrap();
    assert!(self_resolver.resolve_file(&address).is_err());
    let legacy = FileResolver::with_roots(
        &root,
        tmp.path(),
        SelfCoordinate::new(Some("org.example".into()), "native".into()),
    );
    assert!(
        legacy.resolve_file(&address).is_err(),
        "recognizable installed self is never a fresh root-alias grant"
    );
}

#[test]
fn empty_pattern_rechecks_opaque_record() {
    let (_tmp, resolver, root) = fixture();
    let pattern = SpecAddress::parse("spec://org.example/absent-*/none").unwrap();
    assert!(resolver.expand_pattern(&pattern).unwrap().is_empty());
    fs::write(root.join("record.bytes"), "changed").unwrap();
    assert!(resolver.expand_pattern(&pattern).is_err());
}

#[cfg(unix)]
#[test]
fn substituted_slot_refuses_but_shared_hardlink_bytes_remain_readable() {
    let (tmp, resolver, root) = fixture();
    let address = SpecAddress::parse("spec://org.example/native/start").unwrap();
    fs::hard_link(
        root.join("vibevm/vibespecs/start.md"),
        tmp.path().join("shared"),
    )
    .unwrap();
    assert!(
        resolver
            .locate_source(&address, &[])
            .unwrap()
            .read_with_overlay(None)
            .is_ok()
    );
    let moved = tmp.path().join("moved");
    fs::rename(&root, &moved).unwrap();
    std::os::unix::fs::symlink(&moved, &root).unwrap();
    assert!(resolver.locate_source(&address, &[]).is_err());
}
