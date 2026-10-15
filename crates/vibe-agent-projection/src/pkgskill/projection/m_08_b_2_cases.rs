//! Root-selection characterization; preparation effects are outside query intervals.
use super::*;
use sha2::{Digest, Sha256};
use std::cell::Cell;
use vibe_core::manifest::SkillResourceDecl;

fn put(root: &Path, path: &str, bytes: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}
fn inventory(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn walk(root: &Path, at: &Path, rows: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for entry in fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            rows.insert(
                path.strip_prefix(root).unwrap().into(),
                if path.is_dir() {
                    None
                } else {
                    Some(fs::read(&path).unwrap())
                },
            );
            if path.is_dir() {
                walk(root, &path, rows);
            }
        }
    }
    let mut rows = BTreeMap::new();
    walk(root, root, &mut rows);
    rows
}
fn digest(files: &BTreeMap<String, Vec<u8>>) -> String {
    let mut h = Sha256::new();
    h.update(b"vibe-package-skill-snapshot\0epoch=1\0");
    for (path, bytes) in files {
        h.update((path.len() as u64).to_be_bytes());
        h.update(path);
        h.update((bytes.len() as u64).to_be_bytes());
        h.update(bytes);
    }
    format!("sha256:{:x}", h.finalize())
}
struct Fixture {
    owned: tempfile::TempDir,
    project: PathBuf,
    provider: PathBuf,
    upstream: PathBuf,
    composed: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let owned = tempfile::tempdir().unwrap();
        let project = owned.path().join("selected");
        let provider = project.join("dependency");
        let upstream = owned.path().join("authenticated");
        let composed = owned.path().join("composition");
        for root in [&project, &provider] {
            put(root, "vibe.toml", "retained provider manifest");
            put(root, "skills/demo/SKILL.md", "provider decoy");
            put(root, "skills/demo/references/deep/a.md", "wrong root");
            put(root, "docs/deep/resource.md", "wrong resource");
        }
        put(&provider, "adapters/demo/SKILL.md", "body");
        put(
            &provider,
            "adapters/demo/references/deep/a.md",
            "nested source",
        );
        put(&upstream, "skills/demo/SKILL.md", "body");
        put(
            &upstream,
            "skills/demo/references/deep/a.md",
            "nested source",
        );
        put(&upstream, "skills/demo/omit.txt", "excluded");
        put(&upstream, "docs/deep/resource.md", "nested resource");
        put(&upstream, "docs/omit.txt", "excluded resource");
        Self {
            owned,
            project,
            provider,
            upstream,
            composed,
        }
    }
    fn skill(&self, installed: bool, composed: bool) -> DeclaredSkill {
        let provider = if installed {
            DeclaredSkillProvider::Installed {
                group: "org.example".parse().unwrap(),
                name: "bridge".parse().unwrap(),
                version: "2.3.4".into(),
                kind: PackageKind::Tool,
                root: self.provider.clone(),
                content_hash: ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap(),
                embedded_sources: vec![],
                declared_sources: vec![],
            }
        } else {
            DeclaredSkillProvider::Authored {
                group: "org.example".parse().unwrap(),
                name: "bridge".parse().unwrap(),
                version: "2.3.4".into(),
                kind: PackageKind::Tool,
                root: self.provider.clone(),
                embedded_sources: vec![],
            }
        };
        DeclaredSkill {
            decl: SkillDecl {
                name: "demo".into(),
                source: (!composed).then(|| "upstream".into()),
                path: if composed {
                    "adapters/demo"
                } else {
                    "skills/demo"
                }
                .into(),
                description: None,
                agents: vec!["claude".into()],
                include: vec!["SKILL.md".into(), "references/**".into()],
                resources: if composed {
                    vec![SkillResourceDecl {
                        embedded_source: "upstream".into(),
                        path: "docs".into(),
                        include: vec!["**/*.md".into()],
                        target: "references/vendor".into(),
                    }]
                } else {
                    vec![]
                },
            },
            source: self.provider.join(if composed {
                "adapters/demo"
            } else {
                "skills/demo"
            }),
            source_root: self.provider.clone(),
            origin: "fixture".into(),
            provider,
        }
    }
    fn expected(&self, composed: bool) -> BTreeMap<String, Vec<u8>> {
        let mut files = BTreeMap::from([
            ("SKILL.md".into(), b"body".to_vec()),
            ("references/deep/a.md".into(), b"nested source".to_vec()),
        ]);
        if composed {
            files.insert(
                "references/vendor/deep/resource.md".into(),
                b"nested resource".to_vec(),
            );
        }
        files
    }
    fn assert_binding(&self, b: &ProjectSkillBinding, composed: bool) {
        let expected = self.expected(composed);
        assert_eq!(b.selected_files.as_ref(), Some(&expected));
        assert_eq!(b.source_snapshot, digest(&expected));
        assert_eq!(b.skill.provider.root(), self.provider);
        assert_ne!(b.skill.provider.root(), b.skill.source_root);
        assert_eq!(
            b.skill.source_root,
            if composed {
                &self.composed
            } else {
                &self.upstream
            }
            .to_path_buf()
        );
        assert_eq!(
            b.skill.source,
            if composed {
                self.composed.clone()
            } else {
                self.upstream.join("skills/demo")
            }
        );
        assert_eq!(b.targets.len(), 1);
        assert_eq!(b.targets[0].path, self.project.join(".claude/skills/demo"));
    }
    fn prepare(
        &self,
        composed: bool,
        resolutions: &Cell<usize>,
        publications: &Cell<usize>,
    ) -> PreparedProjectSkills {
        let skill = self.skill(true, composed);
        capture::prepare_with_ports(
            &self.project,
            vec![ProjectSkillProviderInput {
                provider: skill.provider,
                declarations: vec![skill.decl],
            }],
            true,
            |provider, name, offline| {
                assert!(offline);
                assert_eq!(name, "upstream");
                assert_eq!(provider.root(), self.provider);
                resolutions.set(resolutions.get() + 1);
                Ok(self.upstream.clone())
            },
            |provider, name, files| {
                assert_eq!(provider, "org.example/bridge");
                assert_eq!(name, "demo");
                assert_eq!(*files, self.expected(true));
                publications.set(publications.get() + 1);
                super::super::write_snapshot(&self.composed, files)?;
                Ok(self.composed.clone())
            },
        )
        .unwrap()
    }
}
#[test]
fn direct_embedded_capture_ignores_provider_decoy_and_retains_full_membership() {
    let f = Fixture::new();
    let mut skill = f.skill(false, false);
    hydrate_declared_skill_using(
        &mut skill,
        true,
        |_, _, offline| {
            assert!(offline);
            Ok(f.upstream.clone())
        },
        |_, _, _| bail!("direct source cannot compose"),
    )
    .unwrap();
    let before = inventory(f.owned.path());
    let b = binding::lower_one_binding(&f.project, skill).unwrap();
    f.assert_binding(&b, false);
    assert_eq!(b.skill.decl.include, ["SKILL.md", "references/**"]);
    assert_eq!(inventory(f.owned.path()), before);
}
#[test]
fn composed_resources_keep_exact_roots_includes_and_digest() {
    let f = Fixture::new();
    let resolutions = Cell::new(0);
    let publications = Cell::new(0);
    let before_setup = inventory(f.owned.path());
    let capture = f.prepare(true, &resolutions, &publications);
    assert_ne!(inventory(f.owned.path()), before_setup);
    assert_eq!((resolutions.get(), publications.get()), (1, 1));
    let before = inventory(f.owned.path());
    let a = capture.plan().unwrap();
    let b = capture.plan().unwrap();
    f.assert_binding(&a[0], true);
    assert!(a[0].skill.decl.include.is_empty());
    assert!(a[0].skill.decl.resources.is_empty());
    assert_eq!(a[0].selected_files, b[0].selected_files);
    assert_eq!((resolutions.get(), publications.get()), (1, 1));
    assert_eq!(inventory(f.owned.path()), before);
}
#[test]
fn missing_or_changed_selected_closure_refuses_without_fallback() {
    for composed in [false, true] {
        for change in ["missing", "changed", "member", "output"] {
            let f = Fixture::new();
            let resolutions = Cell::new(0);
            let publications = Cell::new(0);
            let capture = f.prepare(composed, &resolutions, &publications);
            let root = if change == "output" && composed {
                &f.composed
            } else {
                &f.upstream
            };
            let member = if composed {
                "docs/deep/resource.md"
            } else {
                "skills/demo/references/deep/a.md"
            };
            match change {
                "missing" => fs::remove_file(root.join(member)).unwrap(),
                "changed" => put(root, member, "drift"),
                "member" => put(
                    root,
                    if composed {
                        "docs/deep/new.md"
                    } else {
                        "skills/demo/references/deep/new.md"
                    },
                    "new member",
                ),
                _ if composed => put(root, "references/vendor/deep/resource.md", "output drift"),
                _ => put(root, "skills/demo/SKILL.md", "output drift"),
            }
            let before = inventory(f.owned.path());
            let counters = (resolutions.get(), publications.get());
            for _ in 0..2 {
                assert!(
                    capture
                        .plan()
                        .unwrap_err()
                        .downcast_ref::<SourceCaptureStale>()
                        .is_some()
                );
            }
            assert_eq!(inventory(f.owned.path()), before);
            assert_eq!((resolutions.get(), publications.get()), counters);
        }
    }
}
#[test]
fn installed_provider_identity_survives_hydration() {
    let f = Fixture::new();
    let mut skill = f.skill(true, false);
    hydrate_declared_skill_using(
        &mut skill,
        true,
        |_, _, _| Ok(f.upstream.clone()),
        |_, _, _| bail!("unexpected compose"),
    )
    .unwrap();
    let b = binding::lower_one_binding(&f.project, skill).unwrap();
    f.assert_binding(&b, false);
    let DeclaredSkillProvider::Installed {
        group,
        name,
        version,
        content_hash,
        root,
        ..
    } = b.skill.provider
    else {
        panic!("lost installed identity")
    };
    assert_eq!(group.as_str(), "org.example");
    assert_eq!(name.as_str(), "bridge");
    assert_eq!(version, "2.3.4");
    assert_eq!(root, f.provider);
    assert_eq!(content_hash.as_str(), format!("sha256:{}", "a".repeat(64)));
}
