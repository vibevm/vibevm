use super::*;
#[test]
fn member_preview_retains_inherited_foundation_and_authored_sources() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("vibevm/vibespecs/boot")).unwrap();
    std::fs::create_dir_all(root.path().join("member/vibevm/vibespecs/boot")).unwrap();
    std::fs::write(
        root.path().join("vibe.toml"),
        "[workspace]\nmembers=['member']\n",
    )
    .unwrap();
    std::fs::write(
        root.path().join("member/vibe.toml"),
        "[project]\ngroup='org.overlay'\nname='member'\nversion='1.0.0'\n",
    )
    .unwrap();
    std::fs::write(
        root.path().join("vibevm/vibespecs/boot/00-core.md"),
        "Foundation",
    )
    .unwrap();
    std::fs::write(
        root.path().join("vibevm/vibespecs/boot/10-root.md"),
        "Root-only",
    )
    .unwrap();
    std::fs::write(
        root.path().join("member/vibevm/vibespecs/boot/10-own.xml"),
        "<data/>",
    )
    .unwrap();
    std::fs::write(
        root.path().join("member/vibevm/vibespecs/boot/STATIC.md"),
        "GENERATED POISON",
    )
    .unwrap();
    let rows = vec![
        ResourceDeclaration {
            path: "vibe.toml".into(),
            role: ResourceRole::Manifest,
        },
        ResourceDeclaration {
            path: "member/vibe.toml".into(),
            role: ResourceRole::Manifest,
        },
        ResourceDeclaration {
            path: "vibevm/vibespecs/boot/00-core.md".into(),
            role: ResourceRole::Opaque,
        },
        ResourceDeclaration {
            path: "vibevm/vibespecs/boot/10-root.md".into(),
            role: ResourceRole::Opaque,
        },
        ResourceDeclaration {
            path: "member/vibevm/vibespecs/boot/10-own.xml".into(),
            role: ResourceRole::Opaque,
        },
        ResourceDeclaration {
            path: "member/vibevm/vibespecs/boot/STATIC.md".into(),
            role: ResourceRole::Opaque,
        },
    ];
    let host = DocumentAnalysisHost::new(
        root.path(),
        "member/vibe.toml",
        rows,
        AnalysisLimits {
            resource_bytes: 1024 * 1024,
            capture_bytes: 8 * 1024 * 1024,
            open_buffers: 8,
        },
    )
    .unwrap();
    let before = inventory(root.path());
    let capture = host.client().unwrap().freeze().unwrap();
    let preview = capture.configuration_preview(&vibe_resolver::FeatureRequest::default());
    assert!(preview.diagnostics.is_empty(), "{:?}", preview.diagnostics);
    let entries = &preview.boot.unwrap().entries;
    assert_eq!(entries.len(), 2);
    assert!(entries[0].path.ends_with("00-core.md"));
    assert!(entries[1].path.ends_with("10-own.xml"));
    assert!(
        entries
            .iter()
            .all(|e| !e.path.contains("STATIC") && !e.path.ends_with("10-root.md"))
    );
    assert_eq!(before, inventory(root.path()));
}

#[test]
fn inherited_source_is_admitted_to_actual_member_compiler() {
    let (root, original) = boot_fixture();
    std::fs::create_dir_all(root.path().join("member")).unwrap();
    let member = "[project]\ngroup='org.overlay'\nname='member'\nversion='1.0.0'\n[requires.packages]\n'flow:org.overlay/dep'='=1.0.0'\n";
    std::fs::write(root.path().join("member/vibe.toml"), member).unwrap();
    std::fs::write(
        root.path().join("vibevm/vibespecs/boot/00-core.md"),
        "# Foundation {#foundation}\n\n@fact:FOUND Inherited captured body. @status:impl/done\n",
    )
    .unwrap();
    std::fs::write(
        root.path().join("dep/vibevm/vibespecs/boot/dep.md"),
        "# Dep {#root}\n\n#embed spec://org.overlay/app/boot/00-core#foundation\n",
    )
    .unwrap();
    let mut rows: Vec<_> = original.declarations.values().cloned().collect();
    rows.push(ResourceDeclaration {
        path: "member/vibe.toml".into(),
        role: ResourceRole::Manifest,
    });
    rows.push(ResourceDeclaration {
        path: "vibevm/vibespecs/boot/00-core.md".into(),
        role: ResourceRole::Source {
            manifest: "vibe.toml".into(),
        },
    });
    let host = DocumentAnalysisHost::new(
        root.path(),
        "member/vibe.toml",
        rows,
        AnalysisLimits {
            resource_bytes: 1024 * 1024,
            capture_bytes: 8 * 1024 * 1024,
            open_buffers: 8,
        },
    )
    .unwrap();
    let capture = host.client().unwrap().freeze().unwrap();
    let before = inventory(root.path());
    let preview = capture.configuration_preview(&vibe_resolver::FeatureRequest::default());
    assert!(preview.diagnostics.is_empty(), "{:?}", preview.diagnostics);
    assert!(
        std::str::from_utf8(preview.compiled_boot.as_ref().unwrap())
            .unwrap()
            .contains("Inherited captured body.")
    );
    assert_eq!(before, inventory(root.path()));
}
