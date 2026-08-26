use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{self, UserScratch, fixture_registry};
use crate::support::{ANNOUNCER, STACK, TreeEntry, snapshot};
use vibe_core::manifest::{LockedPackage, Lockfile, Manifest, Materialization};

const COORDINATES: [(&str, &str, &str); 2] = [
    (
        "org.vibevm.fixture",
        "phase-announcer",
        "org.vibevm.fixture/phase-announcer/v0.1.0",
    ),
    (
        "org.vibevm.fixture",
        "lifecycle-rust-stack",
        "org.vibevm.fixture/lifecycle-rust-stack/v0.1.0",
    ),
];

const HOST_MANIFEST: &str = r#"[project]
name = "owner-scenario"
version = "0.0.1"

[requires.packages]
"org.vibevm.fixture/phase-announcer" = "=0.1.0"
"org.vibevm.fixture/lifecycle-rust-stack" = "=0.1.0"

[active]
stack = "lifecycle-rust-stack"
"#;

const CARGO_MANIFEST: &str =
    "[package]\nname = \"owner-scenario\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";
const MAIN_SOURCE: &str = "fn main() { println!(\"owner-scenario\"); }\n";
const TEST_SOURCE: &str = r#"#[test]
fn selected_stack_test_runs() {
    assert_eq!(2 + 2, 4);
    let marker = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/commissioning-test-ran.txt");
    std::fs::write(marker, "ran").unwrap();
}
"#;

pub struct EpochA {
    lock: Lockfile,
    registry_before: std::collections::BTreeMap<String, TreeEntry>,
}

pub fn assert_fresh_user(user: &UserScratch) {
    assert!(snapshot(&user.settings).is_empty());
    assert!(snapshot(&user.cache).is_empty());
    assert!(snapshot(&user.search_cache).is_empty());
}

pub fn append_manifest(project: &Path, text: &str) {
    let path = project.join("vibe.toml");
    let mut body = fs::read_to_string(&path).unwrap();
    body.push_str(text);
    fs::write(path, body).unwrap();
}

pub fn trusted_registry(scenario: &Path, name: &str) -> PathBuf {
    let registry = scenario.join(name);
    for (_, _, coordinate) in COORDINATES {
        common::copy_tree(
            &fixture_registry().join(coordinate),
            &registry.join(coordinate),
        );
    }
    registry
}

pub fn create_epoch_a(user: &UserScratch, scenario: &Path, registry: &Path) -> EpochA {
    let registry_before = snapshot(registry);
    let project = scenario.join("epoch-a/owner-scenario");
    fs::create_dir_all(project.parent().unwrap()).unwrap();
    user.vibe()
        .args(["init", "--no-registry", "--author", "Commissioning Test"])
        .arg("--path")
        .arg(&project)
        .assert()
        .success();
    assert!(
        Manifest::read(project.join("vibe.toml"))
            .unwrap()
            .project
            .unwrap()
            .group
            .is_none()
    );
    write_cargo_files(&project);

    let install = user
        .vibe()
        .args([
            "install",
            "tool:org.vibevm.fixture/phase-announcer",
            "stack:org.vibevm.fixture/lifecycle-rust-stack",
        ])
        .arg("--registry")
        .arg(registry)
        .arg("--path")
        .arg(&project)
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        install.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&install.stdout),
        String::from_utf8_lossy(&install.stderr)
    );
    let installed = String::from_utf8_lossy(&install.stdout);
    assert!(!installed.contains("hello from build"));
    assert!(!installed.contains("hello from test"));
    assert_eq!(snapshot(registry), registry_before);

    let raw = Lockfile::read(project.join("vibe.lock")).unwrap();
    assert_epoch_a_install(&project, &raw);
    EpochA {
        lock: sanitize_lock(&raw),
        registry_before,
    }
}

fn assert_epoch_a_install(project: &Path, lock: &Lockfile) {
    assert_eq!(lock.packages.len(), 2);
    for (group, name, _) in COORDINATES {
        let package = find_package(lock, group, name);
        assert_eq!(package.version.to_string(), "0.1.0");
        assert!(
            project
                .join(common::slot_dir(&format!("{group}.{name}"), "0.1.0"))
                .is_dir()
        );
    }
    let announcer = project.join(common::slot_dir(
        "org.vibevm.fixture.phase-announcer",
        "0.1.0",
    ));
    let manifest = Manifest::read(announcer.join("vibe.toml")).unwrap();
    assert_eq!(
        manifest
            .extensions
            .iter()
            .map(|row| row.id.as_str())
            .collect::<Vec<_>>(),
        ["announce", "announce-test"]
    );
}

fn sanitize_lock(raw: &Lockfile) -> Lockfile {
    let mut lock = Lockfile::empty("commissioning-fixture", "2000-01-01T00:00:00Z");
    lock.packages = COORDINATES
        .iter()
        .map(|(group, name, coordinate)| {
            let source = find_package(raw, group, name);
            let trusted = fixture_registry().join(coordinate);
            assert_eq!(
                vibe_registry::compute_content_hash(&trusted).unwrap(),
                source.content_hash.as_str()
            );
            assert_eq!(source.materialization, Materialization::Copy);
            LockedPackage {
                kind: source.kind,
                name: source.name.clone(),
                group: source.group.clone(),
                version: source.version.clone(),
                registry: None,
                source_url: source.source_url.clone(),
                source_ref: None,
                resolved_commit: None,
                content_hash: source.content_hash.clone(),
                boot_snippet: None,
                files_written: Vec::new(),
                dependencies: source.dependencies.clone(),
                admitted_by: None,
                via_override: None,
                overridden: false,
                source_kind: source.source_kind,
                via_redirect: None,
                features: Vec::new(),
                subskills_active: Vec::new(),
                describes: None,
                language: None,
                materialization: source.materialization,
            }
        })
        .collect();
    lock.meta.root_dependencies = lock
        .packages
        .iter()
        .map(|package| package.as_package_ref().unwrap())
        .collect();
    assert_sanitized(&lock);
    lock
}

fn assert_sanitized(lock: &Lockfile) {
    assert_eq!(lock.meta.generated_by, "commissioning-fixture");
    assert_eq!(lock.meta.generated_at, "2000-01-01T00:00:00Z");
    assert!(lock.meta.solver.is_none());
    assert_eq!(lock.meta.root_dependencies.len(), 2);
    assert!(lock.meta.language_chain.is_empty());
    assert!(lock.meta.active_features.is_empty());
    assert!(lock.meta.virtual_capabilities.is_empty());
    assert_eq!(lock.packages.len(), 2);
    for package in &lock.packages {
        assert!(package.registry.is_none());
        assert!(package.source_ref.is_none());
        assert!(package.resolved_commit.is_none());
        assert!(package.boot_snippet.is_none());
        assert!(package.files_written.is_empty());
        assert!(package.dependencies.is_empty());
        assert!(package.admitted_by.is_none());
        assert!(package.via_override.is_none());
        assert!(!package.overridden);
        assert!(package.via_redirect.is_none());
        assert!(package.features.is_empty());
        assert!(package.subskills_active.is_empty());
        assert!(package.describes.is_none());
        assert!(package.language.is_none());
        assert_eq!(package.materialization, Materialization::Copy);
    }
}

fn find_package<'a>(lock: &'a Lockfile, group: &str, name: &str) -> &'a LockedPackage {
    lock.packages
        .iter()
        .find(|package| package.group.as_str() == group && package.name.as_str() == name)
        .unwrap_or_else(|| panic!("missing {group}/{name}: {lock:?}"))
}

pub fn fresh_epoch_b_registry(epoch_a: &EpochA, scenario: &Path) -> PathBuf {
    let registry = trusted_registry(scenario, "epoch-b-registry");
    assert_eq!(snapshot(&registry), epoch_a.registry_before);
    registry
}

pub fn create_epoch_b(epoch_a: &EpochA, scenario: &Path) -> PathBuf {
    let project = scenario.join("epoch-b/owner-scenario");
    let oracle = tempfile::tempdir().unwrap();
    seed_independent_world(&project, &epoch_a.lock);
    seed_independent_world(oracle.path(), &epoch_a.lock);
    assert_eq!(snapshot(&project), snapshot(oracle.path()));
    assert_eq!(
        fs::read_to_string(project.join("vibe.toml")).unwrap(),
        HOST_MANIFEST
    );
    assert_eq!(
        fs::read_to_string(project.join("Cargo.toml")).unwrap(),
        CARGO_MANIFEST
    );
    assert_eq!(
        fs::read_to_string(project.join("src/main.rs")).unwrap(),
        MAIN_SOURCE
    );
    assert_eq!(
        fs::read_to_string(project.join("tests/commissioning.rs")).unwrap(),
        TEST_SOURCE
    );
    assert_eq!(
        Lockfile::read(project.join("vibe.lock")).unwrap(),
        epoch_a.lock
    );
    assert_no_authority_genre(&project);
    assert!(!project.join(".vibe").exists());
    assert!(!project.join("target").exists());
    project
}

fn seed_independent_world(project: &Path, lock: &Lockfile) {
    fs::create_dir_all(project).unwrap();
    fs::write(project.join("vibe.toml"), HOST_MANIFEST).unwrap();
    write_cargo_files(project);
    lock.write(project.join("vibe.lock")).unwrap();
    for (group, name, coordinate) in COORDINATES {
        let package = find_package(lock, group, name);
        vibe_workspace::vibedeps::materialise(
            project,
            &package.group,
            package.name.as_str(),
            &package.version,
            &fixture_registry().join(coordinate),
            &package.content_hash,
        )
        .unwrap();
    }
}

fn write_cargo_files(project: &Path) {
    fs::create_dir_all(project.join("src")).unwrap();
    fs::create_dir_all(project.join("tests")).unwrap();
    fs::write(project.join("Cargo.toml"), CARGO_MANIFEST).unwrap();
    fs::write(project.join("src/main.rs"), MAIN_SOURCE).unwrap();
    fs::write(project.join("tests/commissioning.rs"), TEST_SOURCE).unwrap();
}

fn assert_no_authority_genre(project: &Path) {
    for (path, entry) in snapshot(project) {
        let path_lower = path.to_ascii_lowercase();
        assert!(
            ["consent", "allow", "trust"]
                .iter()
                .all(|word| !path_lower.contains(word)),
            "authority-genre path crossed into Epoch B: {path}"
        );
        if let TreeEntry::File(bytes) = entry {
            let text = String::from_utf8_lossy(&bytes).to_ascii_lowercase();
            assert!(
                ["consent", "allow", "trust"]
                    .iter()
                    .all(|word| !text.contains(word)),
                "authority-genre bytes crossed into Epoch B: {path}"
            );
        }
    }
    let manifest = Manifest::read(project.join("vibe.toml")).unwrap();
    assert!(manifest.project.unwrap().group.is_none());
    assert!(manifest.extension_controls.uses.is_empty());
    assert!(manifest.extension_controls.disable.is_empty());
    assert!(manifest.extensions.is_empty());
    assert_eq!(
        manifest
            .requires
            .packages
            .iter()
            .map(|package| package.qualified_name())
            .collect::<Vec<_>>(),
        [STACK, ANNOUNCER]
    );
}
