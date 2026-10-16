use super::*;

fn fixture() -> (tempfile::TempDir, InstallResolver, PackageRef) {
    let dir = tempfile::tempdir().unwrap();
    let package = dir.path().join("org.example/app/v1.0.0");
    std::fs::create_dir_all(&package).unwrap();
    std::fs::write(package.join("vibe.toml"), "[package]\ngroup='org.example'\nname='app'\nkind='flow'\nversion='1.0.0'\n[features]\nfast=[]\n").unwrap();
    let source = InstallResolver::Local(LocalRegistry::new(dir.path()).unwrap(), None);
    (
        dir,
        source,
        PackageRef::parse("org.example/app@=1.0.0").unwrap(),
    )
}

#[test]
fn repeated_metadata_preserves_definition_and_capability_identity() {
    inspect_remote_receipt();
    let (_dir, source, root) = fixture();
    let first = source.begin_transaction(&[root.clone()]).unwrap().unwrap();
    let second = source.begin_transaction(&[root.clone()]).unwrap().unwrap();
    assert_eq!(first.catalog_inspection(), second.catalog_inspection());
    let inspection = first.catalog_inspection().unwrap();
    assert_eq!(
        inspection.digest,
        vibe_registry::catalog_digest(&inspection.candidates, &inspection.inventory)
    );
    assert_eq!(inspection.candidates.len(), 1);
    assert!(!inspection.candidates[0].1.definition.is_empty());
    assert_eq!(
        first.manifest_of(&root).unwrap(),
        first.manifest_of(&root).unwrap()
    );
    assert_eq!(first.solve(&[root]).unwrap().packages.len(), 1);
    println!("inspected catalog: {inspection:?}");
}

// Exercise the actual registry receipt path with a moving-ref backend. The
// backend refuses mutable-ref reads and install-store/clone operations.
fn inspect_remote_receipt() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use vibe_registry::{GitBackend, GitError, GitPerPackageRegistry};
    struct Backend(AtomicUsize);
    impl GitBackend for Backend {
        fn bootstrap(&self, _: &str, _: &str, _: &Path) -> Result<(), GitError> {
            panic!("discarded metadata cannot install or clone in this fixture")
        }
        fn update(&self, _: &Path, _: &str) -> Result<(), GitError> {
            panic!("no checkout")
        }
        fn list_tags(&self, _: &str) -> Result<Vec<String>, GitError> {
            Ok(vec!["v1.0.0".into()])
        }
        fn resolve_metadata_ref(&self, url: &str, reference: &str) -> Result<String, GitError> {
            assert_eq!(reference, "v1.0.0");
            if url.contains("primary") {
                return Err(GitError::NetworkUnreachable { url: url.into() });
            }
            let generation = self.0.fetch_add(1, Ordering::SeqCst);
            Ok(match generation {
                0 => "a".repeat(40),
                1 => "b".repeat(40),
                _ => String::new(),
            })
        }
        fn fetch_file_at_ref(
            &self,
            url: &str,
            reference: &str,
            _: &str,
        ) -> Result<Vec<u8>, GitError> {
            assert!(url.contains("mirror"));
            assert!(reference == "a".repeat(40) || reference == "b".repeat(40));
            Ok(format!("[package]\ngroup='org.example'\nname='app'\nversion='1.0.0'\nkind='flow'\ndescription='{reference}'\n").into_bytes())
        }
    }
    let cache = tempfile::tempdir().unwrap();
    let registry = GitPerPackageRegistry::open_with_mirrors(
        "fixture",
        "https://primary.example/org",
        "main",
        vibe_core::manifest::NamingConvention::Fqdn,
        vec!["https://mirror.example/org".into()],
        cache.path(),
        Arc::new(Backend(AtomicUsize::new(0))),
        0,
    )
    .unwrap();
    let group = Group::parse("org.example").unwrap();
    let first = registry
        .fetch_candidate_at_ref(&group, "app", "v1.0.0")
        .unwrap();
    let second = registry
        .fetch_candidate_at_ref(&group, "app", "v1.0.0")
        .unwrap();
    assert!(first.identity.source.contains("mirror.example"));
    assert_eq!(first.identity.content, format!("git:{}", "a".repeat(40)));
    assert_ne!(first.identity, second.identity);
    assert!(
        registry
            .fetch_candidate_at_ref(&group, "app", "v1.0.0")
            .is_err()
    );
    assert_eq!(
        first
            .manifest
            .require_package()
            .unwrap()
            .description
            .as_deref(),
        Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
    );
}

#[test]
fn same_version_replacement_keeps_frozen_snapshot() {
    let (dir, source, root) = fixture();
    let frozen = source.begin_transaction(&[root.clone()]).unwrap().unwrap();
    let before = frozen.manifest_of(&root).unwrap();
    let identity = frozen.catalog_inspection().unwrap().clone();
    let path = dir.path().join("org.example/app/v1.0.0/vibe.toml");
    std::fs::write(
        &path,
        std::fs::read_to_string(&path)
            .unwrap()
            .replace("fast=[]", "slow=[]"),
    )
    .unwrap();
    assert_eq!(frozen.manifest_of(&root).unwrap(), before);
    assert_eq!(frozen.catalog_inspection().unwrap(), &identity);
    assert!(frozen.validate_catalog_candidate(&root).is_err());
    let store = tempfile::tempdir().unwrap();
    assert!(frozen.resolve_and_fetch(&root, store.path(), None).is_err());
    assert_eq!(std::fs::read_dir(store.path()).unwrap().count(), 0);
    let fresh = source.begin_transaction(&[root]).unwrap().unwrap();
    assert_ne!(fresh.catalog_inspection().unwrap(), &identity);
}

#[test]
fn missing_and_corrupt_catalog_differ_from_missing_option() {
    let (dir, source, root) = fixture();
    let frozen = source.begin_transaction(&[root.clone()]).unwrap().unwrap();
    assert!(
        !frozen
            .manifest_of(&root)
            .unwrap()
            .features
            .features
            .contains_key("absent")
    );
    assert!(
        frozen
            .manifest_of(&PackageRef::parse("org.example/uncaptured").unwrap())
            .is_err()
    );
    let path = dir.path().join("org.example/app/v1.0.0/vibe.toml");
    std::fs::write(&path, "broken = [").unwrap();
    assert!(source.begin_transaction(&[root.clone()]).is_err());
    std::fs::remove_file(path).unwrap();
    assert!(source.begin_transaction(&[root]).is_err());
}
