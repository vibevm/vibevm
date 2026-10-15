//! B1 source-boundary cases. These do not establish wire or client coverage.
use super::*;
use std::cell::Cell;
use vibe_core::manifest::SkillResourceDecl;

fn skill(root: &Path) -> DeclaredSkill {
    DeclaredSkill {
        decl: SkillDecl {
            name: "demo".into(),
            source: None,
            path: "skills/demo".into(),
            description: None,
            agents: vec!["claude".into()],
            include: vec![],
            resources: vec![],
        },
        source: root.join("skills/demo"),
        source_root: root.into(),
        origin: "project".into(),
        provider: DeclaredSkillProvider::Authored {
            group: Group::parse("org.example").unwrap(),
            name: PackageName::parse("fixture").unwrap(),
            version: "1.0.0".into(),
            kind: PackageKind::Tool,
            root: root.into(),
            embedded_sources: vec![],
        },
    }
}

fn inventory(root: &Path) -> BTreeMap<String, Option<Vec<u8>>> {
    fn walk(root: &Path, at: &Path, out: &mut BTreeMap<String, Option<Vec<u8>>>) {
        for entry in fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            let key = path
                .strip_prefix(root)
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();
            if path.is_dir() {
                out.insert(key, None);
                walk(root, &path, out);
            } else {
                out.insert(key, Some(fs::read(path).unwrap()));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

#[test]
fn empty_cache_embedded_query_never_enters_preparation() {
    let root = tempfile::tempdir().unwrap();
    let mut selected = skill(root.path());
    selected.decl.source = Some("upstream".into());
    let before = inventory(root.path());
    let calls = Cell::new(0);
    let error = binding::lower_skills_with(root.path(), vec![selected], None, |_, _| {
        calls.set(calls.get() + 1);
        bail!("preparation must not be entered")
    })
    .unwrap_err();
    assert!(error.downcast_ref::<SourcePreparationRequired>().is_some());
    assert_eq!(calls.get(), 0);
    assert_eq!(inventory(root.path()), before);
    assert!(!root.path().join("cache").exists());
}

#[test]
fn resource_query_refuses_before_composition() {
    let root = tempfile::tempdir().unwrap();
    let mut selected = skill(root.path());
    selected.decl.resources.push(SkillResourceDecl {
        embedded_source: "upstream".into(),
        path: "docs".into(),
        include: vec!["**".into()],
        target: "references/upstream".into(),
    });
    let before = inventory(root.path());
    let calls = Cell::new(0);
    let error = binding::lower_skills_with(root.path(), vec![selected], None, |_, _| {
        calls.set(calls.get() + 1);
        bail!("composition must not be entered")
    })
    .unwrap_err();
    assert!(error.downcast_ref::<SourcePreparationRequired>().is_some());
    assert_eq!(calls.get(), 0);
    assert_eq!(inventory(root.path()), before);
}

#[test]
fn explicit_preparation_forwards_both_offline_values_exactly() {
    let root = tempfile::tempdir().unwrap();
    for offline in [false, true] {
        let mut calls = vec![];
        let error = binding::lower_skills_with(
            root.path(),
            vec![skill(root.path())],
            Some(offline),
            |_, policy| {
                calls.push(policy);
                bail!("injected preparation failure")
            },
        )
        .unwrap_err();
        assert_eq!(calls, vec![offline]);
        assert_eq!(error.to_string(), "injected preparation failure");
    }
}

#[test]
fn local_lowering_preserves_ordered_selected_resources_without_preparation() {
    let root = tempfile::tempdir().unwrap();
    let mut selected = skill(root.path());
    fs::create_dir_all(selected.source.join("references")).unwrap();
    fs::write(selected.source.join("SKILL.md"), "body").unwrap();
    fs::write(selected.source.join("references/a.txt"), "resource").unwrap();
    fs::write(selected.source.join("ignored.txt"), "ignored").unwrap();
    selected.decl.include = vec!["SKILL.md".into(), "references/**".into()];
    let before = inventory(root.path());
    let calls = Cell::new(0);
    let plan = binding::lower_skills_with(root.path(), vec![selected], None, |_, _| {
        calls.set(calls.get() + 1);
        bail!("no preparation in query")
    })
    .unwrap();
    assert_eq!(calls.get(), 0);
    let files = plan[0].selected_files.as_ref().unwrap();
    assert_eq!(
        files.keys().map(String::as_str).collect::<Vec<_>>(),
        vec!["SKILL.md", "references/a.txt"]
    );
    assert_eq!(files["references/a.txt"], b"resource");
    assert_eq!(plan[0].targets.len(), 1);
    assert_eq!(
        plan[0].targets[0].path,
        root.path().join(".claude/skills/demo")
    );
    assert_eq!(inventory(root.path()), before);
}

#[test]
fn prepared_raw_closure_is_retained_and_membership_drift_refuses() {
    let root = tempfile::tempdir().unwrap();
    let selected = skill(root.path());
    fs::write(root.path().join("vibe.toml"), "retained manifest").unwrap();
    fs::create_dir_all(selected.source.join("references")).unwrap();
    fs::write(selected.source.join("SKILL.md"), "body").unwrap();
    fs::write(selected.source.join("references/a.txt"), "resource").unwrap();
    let calls = Cell::new(0);
    let capture = capture::prepare_with(
        root.path(),
        vec![ProjectSkillProviderInput {
            provider: selected.provider,
            declarations: vec![selected.decl],
        }],
        true,
        |_, _, _| {
            calls.set(calls.get() + 1);
            bail!("unexpected provider")
        },
    )
    .unwrap();
    let before = inventory(root.path());
    let first = capture.plan().unwrap();
    let second = capture.plan().unwrap();
    assert_eq!(first[0].selected_files, second[0].selected_files);
    assert_eq!(first[0].selected_files.as_ref().unwrap().len(), 2);
    assert_eq!(calls.get(), 0);
    assert_eq!(inventory(root.path()), before);
    fs::write(
        root.path().join("skills/demo/references/b.txt"),
        "new member",
    )
    .unwrap();
    let tampered = inventory(root.path());
    assert!(
        capture
            .plan()
            .unwrap_err()
            .downcast_ref::<SourceCaptureStale>()
            .is_some()
    );
    assert_eq!(inventory(root.path()), tampered);
    assert_eq!(first[0].selected_files.as_ref().unwrap().len(), 2);
}

#[test]
fn prepared_embedded_closure_uses_injected_provider_once_and_exact_offline() {
    let root = tempfile::tempdir().unwrap();
    let upstream = tempfile::tempdir().unwrap();
    fs::write(root.path().join("vibe.toml"), "retained manifest").unwrap();
    fs::create_dir_all(upstream.path().join("skills/demo/references")).unwrap();
    fs::write(upstream.path().join("skills/demo/SKILL.md"), "body").unwrap();
    fs::write(
        upstream.path().join("skills/demo/references/a.txt"),
        "upstream",
    )
    .unwrap();
    let mut selected = skill(root.path());
    selected.decl.source = Some("upstream".into());
    let mut policies = Vec::new();
    let capture = capture::prepare_with(
        root.path(),
        vec![ProjectSkillProviderInput {
            provider: selected.provider,
            declarations: vec![selected.decl],
        }],
        true,
        |_, name, policy| {
            assert_eq!(name, "upstream");
            policies.push(policy);
            Ok(upstream.path().to_owned())
        },
    )
    .unwrap();
    assert_eq!(policies, [true]);
    let before = (inventory(root.path()), inventory(upstream.path()));
    assert_eq!(
        capture.plan().unwrap()[0]
            .selected_files
            .as_ref()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        capture.plan().unwrap()[0]
            .selected_files
            .as_ref()
            .unwrap()
            .len(),
        2
    );
    assert_eq!((inventory(root.path()), inventory(upstream.path())), before);
    fs::write(
        upstream.path().join("skills/demo/references/a.txt"),
        "drift",
    )
    .unwrap();
    assert!(
        capture
            .plan()
            .unwrap_err()
            .downcast_ref::<SourceCaptureStale>()
            .is_some()
    );
}

#[test]
fn prepared_resources_preserve_includes_and_only_prepare_publishes_owned_cache() {
    let root = tempfile::tempdir().unwrap();
    let upstream = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let cache = scratch.path().join("absent-cache");
    fs::write(root.path().join("vibe.toml"), "manifest").unwrap();
    let mut selected = skill(root.path());
    fs::create_dir_all(&selected.source).unwrap();
    fs::write(
        selected.source.join("SKILL.md"),
        "---\nname: demo\ndescription: retained\n---\nbody",
    )
    .unwrap();
    fs::create_dir_all(upstream.path().join("docs/nested")).unwrap();
    fs::write(upstream.path().join("docs/nested/a.md"), "included").unwrap();
    fs::write(upstream.path().join("docs/omit.txt"), "excluded").unwrap();
    selected.decl.resources.push(SkillResourceDecl {
        embedded_source: "upstream".into(),
        path: "docs".into(),
        include: vec!["**/*.md".into()],
        target: "references/vendor".into(),
    });
    let resolutions = Cell::new(0);
    let publications = Cell::new(0);
    assert!(!cache.exists());
    let capture = capture::prepare_with_ports(
        root.path(),
        vec![ProjectSkillProviderInput {
            provider: selected.provider,
            declarations: vec![selected.decl],
        }],
        false,
        |_, name, offline| {
            assert_eq!(name, "upstream");
            assert!(!offline);
            resolutions.set(resolutions.get() + 1);
            Ok(upstream.path().into())
        },
        |_, _, files| {
            publications.set(publications.get() + 1);
            super::super::write_snapshot(&cache, files)?;
            Ok(cache.clone())
        },
    )
    .unwrap();
    assert_eq!((resolutions.get(), publications.get()), (1, 1));
    let before = (
        inventory(root.path()),
        inventory(upstream.path()),
        inventory(scratch.path()),
    );
    let plan = capture.plan().unwrap();
    let again = capture.plan().unwrap();
    let files = plan[0].selected_files.as_ref().unwrap();
    assert_eq!(files.len(), 2);
    assert_eq!(files["references/vendor/nested/a.md"], b"included");
    assert_eq!(plan[0].selected_files, again[0].selected_files);
    assert_eq!((resolutions.get(), publications.get()), (1, 1));
    assert_eq!(
        (
            inventory(root.path()),
            inventory(upstream.path()),
            inventory(scratch.path())
        ),
        before
    );
    fs::write(
        upstream.path().join("docs/nested/new.md"),
        "new selected member",
    )
    .unwrap();
    assert!(capture.plan().is_err());
    assert_eq!((resolutions.get(), publications.get()), (1, 1));
}

#[test]
fn installed_capture_keeps_selected_provider_and_refuses_manifest_drift() {
    let project = tempfile::tempdir().unwrap();
    let dependency = tempfile::tempdir().unwrap();
    let mut selected = skill(dependency.path());
    selected.provider = DeclaredSkillProvider::Installed {
        group: Group::parse("org.example").unwrap(),
        name: PackageName::parse("dependency").unwrap(),
        version: "1.0.0".into(),
        kind: PackageKind::Tool,
        root: dependency.path().into(),
        content_hash: ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap(),
        embedded_sources: vec![],
        declared_sources: vec![],
    };
    fs::write(dependency.path().join("vibe.toml"), "selected manifest").unwrap();
    fs::create_dir_all(&selected.source).unwrap();
    fs::write(selected.source.join("SKILL.md"), "body").unwrap();
    let capture = capture::prepare_with(
        project.path(),
        vec![ProjectSkillProviderInput {
            provider: selected.provider,
            declarations: vec![selected.decl],
        }],
        true,
        |_, _, _| bail!("local dependency must not hydrate"),
    )
    .unwrap();
    let before = (inventory(project.path()), inventory(dependency.path()));
    let first = capture.plan().unwrap();
    assert!(matches!(
        first[0].skill.provider,
        DeclaredSkillProvider::Installed { .. }
    ));
    assert_eq!(first[0].skill.provider.identity(), "org.example/dependency");
    assert_eq!(
        capture.plan().unwrap()[0].selected_files,
        first[0].selected_files
    );
    assert_eq!(
        (inventory(project.path()), inventory(dependency.path())),
        before
    );
    fs::write(dependency.path().join("vibe.toml"), "changed manifest").unwrap();
    assert!(capture.plan().is_err());
}
