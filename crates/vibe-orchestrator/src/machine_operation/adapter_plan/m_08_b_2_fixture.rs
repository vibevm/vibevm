//! Offline cache input, verified by the existing registry owner, never by this seed writer.
use super::*;
use std::{
    collections::BTreeMap,
    fs,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use vibe_agent_projection::pkgskill::DeclaredSkillProvider;
use vibe_core::{
    ContentHash,
    manifest::{LockedEmbeddedSource, Lockfile, Manifest, SpecFormat},
};

pub fn isolated(case: &str, run: impl FnOnce(&Path)) {
    if std::env::var("VIBE_B2_CHILD").as_deref() == Ok(case) {
        let root = PathBuf::from(std::env::var_os("VIBE_B2_ROOT").unwrap());
        run(&root);
        return;
    }
    let owned = tempfile::tempdir().unwrap();
    for dir in ["home", "settings", "tmp", "cwd"] {
        fs::create_dir(owned.path().join(dir)).unwrap();
    }
    // The child alone has an unrelated cwd and private environment. No runner-wide mutation.
    put(
        &owned.path().join("cwd"),
        "vibe.toml",
        &package("cwd-decoy"),
    );
    let out = fs::File::create(owned.path().join("stdout")).unwrap();
    let err = fs::File::create(owned.path().join("stderr")).unwrap();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            &format!("machine_operation::adapter_plan::tests::{case}"),
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("VIBE_B2_CHILD", case)
        .env("VIBE_B2_ROOT", owned.path())
        .env("VIBE_SETTINGS", owned.path().join("settings"))
        .env("HOME", owned.path().join("home"))
        .env("TMPDIR", owned.path().join("tmp"))
        .env("XDG_CACHE_HOME", owned.path().join("home/cache"))
        .env("XDG_CONFIG_HOME", owned.path().join("home/config"))
        .env("PATH", owned.path().join("no-executables"))
        .current_dir(owned.path().join("cwd"))
        .stdin(Stdio::null())
        .stdout(out)
        .stderr(err)
        .spawn()
        .unwrap();
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if start.elapsed() > Duration::from_secs(45) {
            child.kill().unwrap();
            let _ = child.wait();
            panic!("bounded inert child timeout");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    println!(
        "{}",
        fs::read_to_string(owned.path().join("stdout")).unwrap()
    );
    println!(
        "{}",
        fs::read_to_string(owned.path().join("stderr")).unwrap()
    );
    assert!(status.success(), "inert fixture child: {status}");
}
fn put(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}
fn package(name: &str) -> String {
    format!(
        "[package]\ngroup='org.example'\nname='{name}'\nkind='tool'\nversion='2.3.4'\nauthors=['Fixture']\nlicense='MIT'\ndescription='Fixture'\nkeywords=['fixture']\n"
    )
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
fn snapshot(files: &BTreeMap<String, Vec<u8>>) -> String {
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
pub struct Fixture {
    pub root: PathBuf,
    pub selected: PathBuf,
    pub dependency: PathBuf,
    pub tree: PathBuf,
    pub route: AdapterPlanRoute,
    pub compositions: Vec<PathBuf>,
}
impl Fixture {
    pub fn new(owned: &Path) -> Self {
        // vibe-test-support's pre-thread constructor may relocate VIBE_SETTINGS again.
        // Consume that existing owner setting and prove it stays within this child fixture.
        let settings = vibe_core::settings::settings_dir().unwrap();
        assert!(settings.starts_with(owned));
        let composition_cache = settings.join("cache/skill-compositions");
        if composition_cache.exists() {
            fs::remove_dir_all(&composition_cache).unwrap();
        }
        // Each scenario gets fresh retained objects; deliberate tampering never repairs a session.
        let scenario = tempfile::Builder::new()
            .prefix("scenario-")
            .tempdir_in(owned.join("tmp"))
            .unwrap()
            .keep();
        let selected = scenario.join("selected");
        let original = scenario.join("original-dependency");
        let upstream = scenario.join("upstream-input");
        put(&upstream, "LICENSE", "MIT fixture\n");
        put(&upstream, "skills/demo/SKILL.md", "body");
        put(
            &upstream,
            "skills/demo/references/deep/a.md",
            "nested source",
        );
        put(&upstream, "skills/demo/omit.txt", "excluded");
        put(&upstream, "docs/deep/resource.md", "nested resource");
        put(&upstream, "docs/omit.txt", "excluded resource");
        let content_hash =
            vibe_registry::compute_content_hash_with(vibe_registry::RecipeId::Tree1, &upstream)
                .unwrap();
        assert_eq!(
            content_hash,
            "sha256-tree/1:754fe606cb3d22d9b92a22d42fcf8e21af15c254693816c588ac5cd6376b6ec1"
        );
        let commit = "0123456789abcdef0123456789abcdef01234567";
        let url = "https://example.invalid/b2.git";
        let mut key = Sha256::new();
        key.update(b"vibe-embedded-source-v1\0");
        key.update(url);
        key.update([0]);
        key.update(commit);
        key.update([0]);
        key.update(&content_hash);
        let entry = settings
            .join("cache/embedded/git/v1")
            .join(format!("{:x}", key.finalize()));
        // Literal cache-layout fixture input only. No validator or fallback is implemented here.
        let tree = entry.join("tree");
        if entry.exists() {
            fs::remove_dir_all(&entry).unwrap();
        }
        for (path, bytes) in inventory(&upstream) {
            if let Some(bytes) = bytes {
                let to = tree.join(path);
                fs::create_dir_all(to.parent().unwrap()).unwrap();
                fs::write(to, bytes).unwrap();
            }
        }
        put(
            &entry,
            "source.toml",
            &format!(
                "source_url='{url}'\nresolved_commit='{commit}'\ntree_oid='{commit}'\ncontent_hash='{content_hash}'\n"
            ),
        );
        put(&entry, "complete", "ok\n");
        let embedded = format!(
            "\n[[embedded_source]]\nname='upstream'\nkind='git'\nurl='{url}'\ncommit='{commit}'\ncontent_hash='{content_hash}'\nauth='none'\nupstream_authors=['Fixture']\nupstream_license='MIT'\nlicense_path='LICENSE'\nlicense_url='https://example.invalid/b2/blob/{commit}/LICENSE'\n"
        );
        for (root, name) in [(&selected, "selected"), (&original, "dependency")] {
            let mut text = package(name) + &embedded;
            put(root, "adapters/demo/SKILL.md", "body");
            put(root, "adapters/demo/references/deep/a.md", "nested source");
            put(root, "adapters/demo/omit.txt", "excluded local");
            for mode in ["direct", "composed"] {
                let source = if mode == "direct" {
                    "source='upstream'\npath='skills/demo'"
                } else {
                    "path='adapters/demo'"
                };
                text += &format!(
                    "\n[[skill]]\nname='{name}-{mode}'\n{source}\nagents=['claude']\ninclude=['SKILL.md','references/**']\n"
                );
                if mode == "composed" {
                    text += "[[skill.resource]]\nembedded_source='upstream'\npath='docs'\ninclude=['**/*.md']\ntarget='references/vendor'\n";
                }
            }
            if name == "selected" {
                text += "\n[requires.packages]\n'tool:org.example/dependency'='=2.3.4'\n";
            }
            put(root, "vibe.toml", &text);
            put(root, "skills/demo/SKILL.md", "provider decoy");
            put(root, "skills/demo/references/deep/a.md", "wrong root");
            put(root, "docs/deep/resource.md", "wrong resource");
        }
        let manifest = Manifest::read(&original.join("vibe.toml")).unwrap();
        let declaration = &manifest.embedded_sources[0];
        // This call is the real cache verifier and must succeed before host preparation.
        let verified = vibe_registry::cache_embedded_source_with(declaration, true).unwrap();
        assert_eq!(verified.tree, tree);
        assert_eq!(verified.content_hash.as_str(), content_hash);
        let pin = LockedEmbeddedSource {
            name: declaration.name.clone(),
            source_url: vibe_core::SourceUrl::new(url),
            source_ref: None,
            resolved_commit: commit.into(),
            tree_oid: commit.into(),
            content_hash: verified.content_hash,
            upstream_authors: declaration.upstream_authors.clone(),
            upstream_license: declaration.upstream_license.clone(),
            license_path: declaration.license_path.clone(),
            license_url: declaration.license_url.clone(),
            license_file_sha256: ContentHash::parse(&hash(b"MIT fixture\n")).unwrap(),
        };
        assert!(pin.matches_declaration(declaration));
        let package_hash = ContentHash::parse(
            &vibe_registry::compute_content_hash_with(vibe_registry::RecipeId::Tree1, &original)
                .unwrap(),
        )
        .unwrap();
        let group = "org.example".parse().unwrap();
        let version = "2.3.4".parse().unwrap();
        vibe_workspace::vibedeps::materialise_with_spec_format(
            &selected,
            &group,
            "dependency",
            &version,
            &original,
            vibe_workspace::vibedeps::CopyMode::Copy,
            SpecFormat::Mixed,
            &package_hash,
        )
        .unwrap();
        let dependency =
            vibe_workspace::vibedeps::slot_abs_path(&selected, &group, "dependency", &version);
        let mut lock = Lockfile::empty("b2-fixture", "2026-10-15T00:00:00Z");
        let mut row: vibe_core::manifest::LockedPackage = toml::from_str(&format!("kind='tool'\ngroup='org.example'\nname='dependency'\nversion='2.3.4'\nsource_url='https://example.invalid/bridge.git'\ncontent_hash='{package_hash}'\n")).unwrap();
        row.embedded_sources.push(pin);
        lock.packages.push(row);
        lock.write(selected.join("vibe.lock")).unwrap();
        let host = host(&selected);
        let before = inventory(&settings);
        let prepared = host.prepare(true).unwrap();
        let after = inventory(&settings);
        assert_ne!(before, after, "explicit preparation publishes compositions");
        let bindings = prepared.skills.plan().unwrap();
        assert_eq!(bindings.len(), 4);
        let mut compositions = Vec::new();
        for b in &bindings {
            let composed = b.skill.decl.name.ends_with("-composed");
            match &b.skill.provider {
                DeclaredSkillProvider::Authored {
                    group,
                    name,
                    version,
                    root,
                    ..
                } => {
                    assert_eq!(group.as_str(), "org.example");
                    assert_eq!(name.as_str(), "selected");
                    assert_eq!(version, "2.3.4");
                    assert_eq!(root, &selected);
                }
                DeclaredSkillProvider::Installed {
                    group,
                    name,
                    version,
                    root,
                    content_hash,
                    embedded_sources,
                    ..
                } => {
                    assert_eq!(group.as_str(), "org.example");
                    assert_eq!(name.as_str(), "dependency");
                    assert_eq!(version, "2.3.4");
                    assert_eq!(root, &dependency);
                    assert_eq!(content_hash, &package_hash);
                    assert_eq!(embedded_sources.len(), 1);
                }
            }
            if composed {
                assert!(
                    b.skill
                        .source_root
                        .starts_with(settings.join("cache/skill-compositions/v1"))
                );
                assert_eq!(b.skill.source, b.skill.source_root);
                compositions.push(b.skill.source_root.clone());
            } else {
                assert_eq!(b.skill.source_root, tree);
                assert_eq!(b.skill.source, tree.join("skills/demo"));
            }
            assert_ne!(b.skill.source_root, selected);
            assert_ne!(b.skill.source_root, dependency);
            assert_eq!(b.selected_files.as_ref(), Some(&Self::expected(composed)));
            assert_eq!(b.source_snapshot, snapshot(&Self::expected(composed)));
        }
        let route = AdapterPlanRoute::new(host, Some(prepared)).unwrap();
        assert_eq!(route.root(), selected);
        assert_ne!(std::env::current_dir().unwrap(), selected);
        println!(
            "B2_PREPARATION {}",
            serde_json::json!({"selected":selected,"dependency":dependency,"embedded":tree,"compositions":compositions,"package_hash":package_hash.to_string(),"cache_hash":content_hash,"settings":settings,"settings_before_entries":before.len(),"settings_after_entries":after.len(),"settings_before_digest":hash(&serde_json::to_vec(&before).unwrap()),"settings_after_digest":hash(&serde_json::to_vec(&after).unwrap()),"effects":"fixture writes, materialisation, two composition publications"})
        );
        Self {
            root: scenario,
            selected,
            dependency,
            tree,
            route,
            compositions,
        }
    }
    fn expected(composed: bool) -> BTreeMap<String, Vec<u8>> {
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
    pub fn query(&self, label: &str) -> Answer<r::Result> {
        let settings = PathBuf::from(std::env::var_os("VIBE_SETTINGS").unwrap());
        let home = PathBuf::from(std::env::var_os("HOME").unwrap());
        let before = (
            inventory(&self.root),
            inventory(&settings),
            inventory(&home),
            inventory(&std::env::current_dir().unwrap()),
        );
        eprintln!("B2_QUERY_BEGIN {label}");
        let answer = self.route.request(plan(&self.route));
        eprintln!("B2_QUERY_END {label}");
        let after = (
            inventory(&self.root),
            inventory(&settings),
            inventory(&home),
            inventory(&std::env::current_dir().unwrap()),
        );
        assert_eq!(after, before);
        println!(
            "B2_QUERY_CENSUS {}",
            serde_json::json!({"label":label,"before":hash(&serde_json::to_vec(&before).unwrap()),"after":hash(&serde_json::to_vec(&after).unwrap()),"roots":[self.root.clone(),settings,home,std::env::current_dir().unwrap()],"result":if answer.is_ok() { "complete" } else { "refused" }})
        );
        answer
    }
    pub fn positive(&self) -> r::Result {
        let a = self.query("positive").unwrap();
        assert_eq!(self.query("repeat").unwrap(), a);
        let r::Body::Plan(body) = &a.body else {
            panic!()
        };
        assert!(body.complete);
        assert_eq!(body.rows.len(), 4);
        for row in &body.rows {
            let r::Row::Skill(skill) = row else { panic!() };
            let provider = if skill.declaration.starts_with("selected-") {
                "selected"
            } else {
                "dependency"
            };
            assert_eq!(skill.provider, format!("org.example/{provider}"));
            assert_eq!(skill.proof, r::RowSkillProof::Observed);
            assert_eq!(skill.ownership, r::RowSkillOwnership::Unavailable);
            assert_eq!(skill.scope, r::RowSkillScope::Project);
            assert_eq!(skill.agent, "claude");
            assert_eq!(
                skill.target,
                format!(".claude/skills/{}", skill.declaration)
            );
            let expected = Self::expected(skill.declaration.ends_with("-composed"));
            assert_eq!(skill.source_digest, snapshot(&expected));
            assert_eq!(skill.files.len(), expected.len());
            for (file, (path, bytes)) in skill.files.iter().zip(&expected) {
                assert_eq!(&file.path, path);
                assert_eq!(file.bytes, bytes.len().to_string());
                assert_eq!(file.sha256, format!("{:x}", Sha256::digest(bytes)));
            }
        }
        a
    }
    pub fn tamper(&self, kind: &str) {
        match kind {
            "absent-cache" => fs::remove_dir_all(&self.tree).unwrap(),
            "missing" => {
                fs::remove_file(self.tree.join("skills/demo/references/deep/a.md")).unwrap()
            }
            "changed" => put(&self.tree, "docs/deep/resource.md", "changed resource"),
            "member" => put(&self.tree, "docs/deep/new.md", "new included member"),
            "composed" => put(
                &self.compositions[0],
                "references/vendor/deep/resource.md",
                "changed output",
            ),
            "lock" => put(&self.selected, "vibe.lock", "changed selected world"),
            "manifest" => put(&self.dependency, "vibe.toml", "changed provider"),
            _ => panic!("unknown tamper"),
        }
    }
    pub fn decoys(&self) {
        put(&self.tree, "skills/demo/omit.txt", "excluded bytes changed");
        put(&self.tree, "docs/omit.txt", "excluded resource changed");
        for root in [
            &self.selected,
            &self.dependency,
            &std::env::current_dir().unwrap(),
        ] {
            put(root, "skills/demo/SKILL.md", "new same-path decoy");
            put(root, "skills/demo/references/deep/a.md", "new wrong root");
            put(root, "docs/deep/resource.md", "new wrong resource");
        }
    }
}
