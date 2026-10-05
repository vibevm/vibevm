use super::*;
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_core::progress::Progress;
use vibe_lifecycle::process::capture_package_tree_witness;

const PACKAGE_MANIFEST: &str =
    "[package]\ngroup = 'org.example'\nname = 'fixture'\nkind = 'flow'\nversion = '1.1.0'\n";

fn pin() -> PackageRef {
    PackageRef::parse("flow:org.example/fixture@=1.1.0").unwrap()
}

fn fixture() -> (tempfile::TempDir, PathBuf, PackageTreeWitness) {
    let dir = tempfile::tempdir().unwrap();
    let registry = dir.path().join("registry");
    let source = registry.join("org.example/fixture/v1.1.0");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(source.join("vibe.toml"), PACKAGE_MANIFEST).unwrap();
    std::fs::write(source.join("README.md"), b"source\n").unwrap();
    let witness =
        capture_package_tree_witness(&source, &pin(), &["vibe.toml", "README.md"]).unwrap();
    (dir, registry, witness)
}

#[test]
fn exact_local_factory_builds_only_pinned_offline_source() {
    assert!(vibe_test_support::isolated_home().is_some());
    let (dir, registry, witness) = fixture();
    let factory = PinnedLocalFactory::new(&registry, &pin(), &witness).unwrap();
    let binding = factory.checked_serving_root().unwrap();
    assert_eq!(binding.source_root, witness.root);
    assert_eq!(binding.source_identity, witness.root_identity);
    assert_eq!(binding.tree_sha256, witness.tree_sha256);
    let project = dir.path().join("consumer");
    std::fs::create_dir(&project).unwrap();
    let manifest = Manifest::parse_str(
        "[project]\nname='consumer'\nversion='1.1.0'\n[requires.packages]\n'flow:org.example/fixture'='=1.1.0'\n",
    )
    .unwrap();
    let hostile_global: vibe_core::GlobalRegistryConfig =
        toml::from_str("[[registry]]\nname='remote'\nurl='https://invalid.example.test/repo'\n")
            .unwrap();
    let progress = Progress::default();
    let source = factory
        .build(PackageSourceBuild {
            progress: &progress,
            manifest: &manifest,
            embedded_root: None,
            project_root: &project,
            global: &hostile_global,
            offline: true,
            locked: &[],
        })
        .unwrap();
    let lock = Lockfile::empty("vibe 1.1.0", "2026-09-30T00:00:00Z");
    assert_eq!(source.qualify(&pin(), &lock).unwrap(), pin());
    assert!(
        source
            .qualify(
                &PackageRef::parse("flow:org.example/other@=1.1.0").unwrap(),
                &lock
            )
            .is_err()
    );
    let graph = source.solve(&[pin()]).unwrap();
    assert_eq!(graph.packages.len(), 1);
    assert_eq!(graph.packages[0].name, "fixture");
    let store = dir.path().join("scratch-store");
    let cached = source.resolve_and_fetch(&pin(), &store, None).unwrap();
    assert_eq!(cached.resolved.source_dir, witness.root);
    assert!(cached.cache_dir.join("README.md").is_file());
}

#[test]
fn bound_source_port_accepts_only_its_original_witness_and_current_tree() {
    let (_dir, registry, witness) = fixture();
    let factory = PinnedLocalFactory::new(&registry, &pin(), &witness).unwrap();
    let port: &dyn BoundOnePackageSource = &factory;
    assert_eq!(port.source_witness(), &witness);
    port.recheck_source_binding(&witness).unwrap();

    let mut foreign = witness.clone();
    foreign.tree_sha256 = format!("sha256:{}", "0".repeat(64));
    assert!(port.recheck_source_binding(&foreign).is_err());
    std::fs::write(witness.root.join("README.md"), b"changed after pin\n").unwrap();
    assert!(port.recheck_source_binding(&witness).is_err());
}

#[test]
fn foreign_pin_and_changed_registry_root_refuse() {
    let (dir, registry, witness) = fixture();
    let foreign = PackageRef::parse("flow:org.example/other@=1.1.0").unwrap();
    assert!(PinnedLocalFactory::new(&registry, &foreign, &witness).is_err());
    let factory = PinnedLocalFactory::new(&registry, &pin(), &witness).unwrap();
    let displaced = dir.path().join("old-registry");
    std::fs::rename(&registry, &displaced).unwrap();
    std::fs::create_dir(&registry).unwrap();
    assert!(factory.checked_serving_root().is_err());
    let project = dir.path().join("consumer");
    std::fs::create_dir(&project).unwrap();
    let manifest = Manifest::parse_str("[project]\nname='consumer'\nversion='1.1.0'\n").unwrap();
    let global = vibe_core::GlobalRegistryConfig::default();
    let progress = Progress::default();
    assert!(
        factory
            .build(PackageSourceBuild {
                progress: &progress,
                manifest: &manifest,
                embedded_root: None,
                project_root: &project,
                global: &global,
                offline: true,
                locked: &[],
            })
            .is_err()
    );
}

#[cfg(unix)]
#[test]
fn symlinked_registry_root_is_refused_before_build() {
    use std::os::unix::fs::symlink;

    let (dir, registry, witness) = fixture();
    let link = dir.path().join("registry-link");
    symlink(&registry, &link).unwrap();
    let linked_source = link.join("org.example/fixture/v1.1.0");
    assert!(
        capture_package_tree_witness(&linked_source, &pin(), &["vibe.toml", "README.md"]).is_err()
    );
    assert!(PinnedLocalFactory::new(&link, &pin(), &witness).is_err());
}

#[test]
fn online_or_embedded_build_refuses_without_fallback() {
    let (dir, registry, witness) = fixture();
    let factory = PinnedLocalFactory::new(&registry, &pin(), &witness).unwrap();
    let project = dir.path().join("consumer");
    std::fs::create_dir(&project).unwrap();
    let manifest = Manifest::parse_str("[project]\nname='consumer'\nversion='1.1.0'\n").unwrap();
    let global = vibe_core::GlobalRegistryConfig::default();
    let progress = Progress::default();
    let input = |offline, embedded_root| PackageSourceBuild {
        progress: &progress,
        manifest: &manifest,
        embedded_root,
        project_root: &project,
        global: &global,
        offline,
        locked: &[],
    };
    assert!(factory.build(input(false, None)).is_err());
    assert!(factory.build(input(true, Some(&registry))).is_err());
    assert!(!project.join("vibe.lock").exists());
}
