//! Configuration admission applies to both lock8 and remap-capable lock9.
use super::*;
use crate::InstallSource;
use std::collections::BTreeSet;
use vibe_registry::{CachedPackage, InPlaceMaterialised, RegistryError};
use vibe_resolver::{ResolvedGraph, SolveError};
struct NoAcquisition;
impl InstallSource for NoAcquisition {
    fn source_solve_limits(
        &self,
    ) -> std::result::Result<vibe_resolver::SourceSolveLimits, SolveError> {
        panic!("admission does not solve")
    }
    fn solve_metadata(
        &self,
        _: &vibe_resolver::SourceSolveRequest<'_>,
    ) -> std::result::Result<vibe_resolver::SourceResolvedGraph, SolveError> {
        panic!("admission does not solve")
    }
    fn acquire_candidate_for_install(
        &self,
        _: &vibe_registry::source_candidates::RetainedSourceCandidate,
        _: &Path,
        _: Option<&str>,
        _: &Path,
        _: &vibe_core::progress::Progress,
    ) -> std::result::Result<vibe_registry::PackageAcquisition, RegistryError> {
        panic!("verified fixture cache must not acquire")
    }
    fn resolve_and_fetch(
        &self,
        _: &PackageRef,
        _: &Path,
        _: Option<&str>,
    ) -> std::result::Result<CachedPackage, RegistryError> {
        panic!("verified fixture cache must not acquire")
    }
    fn solve(&self, _: &[PackageRef]) -> std::result::Result<ResolvedGraph, SolveError> {
        panic!("admission does not solve")
    }
    fn manifest_of(&self, _: &PackageRef) -> std::result::Result<Manifest, SolveError> {
        panic!("admission reads retained bytes")
    }
    fn solve_masked(
        &self,
        _: &[PackageRef],
        _: &BTreeSet<(String, String)>,
    ) -> std::result::Result<ResolvedGraph, SolveError> {
        panic!("admission does not solve")
    }
    fn materialise_in_place(
        &self,
        _: &PackageRef,
        _: &Path,
    ) -> std::result::Result<InPlaceMaterialised, RegistryError> {
        panic!("admission never publishes")
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#LF-CONFIGURED-STATE"
)]
fn rich_lock8_and_lock9_reject_missing_receipt_and_corrupt_store_without_preview_writes() {
    for schema in [
        vibe_core::manifest::CONFIGURATION_SCHEMA_VERSION,
        vibe_core::manifest::CURRENT_SCHEMA_VERSION,
    ] {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("project");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(
            root.join("vibe.toml"),
            "[project]\nname='consumer'\nversion='1.0.0'\n",
        )
        .unwrap();
        let workspace = Workspace::discover(&root).unwrap();
        let store = temporary.path().join("read-only-store");
        let group = vibe_core::Group::parse("org.test").unwrap();
        let version = "1.0.0".parse().unwrap();
        let entry = vibe_registry::store::entry_dir(&store, &group, "rich", &version);
        std::fs::create_dir_all(&entry).unwrap();
        let body = "[package]\ngroup='org.test'\nname='rich'\nkind='flow'\nversion='1.0.0'\n[configuration]\n";
        std::fs::write(entry.join("vibe.toml"), body).unwrap();
        let hash = vibe_registry::compute_content_hash(&entry).unwrap();
        let lock_path = temporary.path().join("vibe.lock");
        std::fs::write(&lock_path,format!("[meta]\nschema_version={schema}\ngenerated_by='fixture'\ngenerated_at='fixed'\nroot_dependencies=[]\n[[package]]\ngroup='org.test'\nname='rich'\nkind='flow'\nversion='1.0.0'\nsource_url='file:///unused'\ncontent_hash='{hash}'\n")).unwrap();
        let mut lock = vibe_core::manifest::Lockfile::read(&lock_path).unwrap();
        let scratch = temporary.path().join("must-not-write");
        let storage = crate::PreviewSourceStorage {
            read_only_store: Some(&store),
            acquisition_store: &scratch,
        };
        let missing = retained_for_preview(&NoAcquisition, &workspace, &lock, storage).unwrap_err();
        assert!(
            missing.to_string().contains("has no lock record"),
            "{missing}"
        );
        assert!(!scratch.exists());
        let slot =
            vibe_workspace::vibedeps::slot_abs_path(&workspace.root, &group, "rich", &version);
        std::fs::create_dir_all(&slot).unwrap();
        std::fs::write(slot.join("vibe.toml"), body).unwrap();
        let apply_missing =
            retained_before_planning(&NoAcquisition, &workspace, &lock).unwrap_err();
        assert!(
            apply_missing.to_string().contains("has no lock record"),
            "{apply_missing}"
        );
        std::fs::remove_dir_all(&slot).unwrap();
        std::fs::write(entry.join("unexpected"), "tampered").unwrap();
        let corrupt = retained_for_preview(&NoAcquisition, &workspace, &lock, storage).unwrap_err();
        assert!(
            corrupt
                .to_string()
                .contains("disagrees with the exact lock pin"),
            "{corrupt}"
        );
        assert!(!scratch.exists());
        assert_eq!(
            std::fs::read_to_string(entry.join("vibe.toml")).unwrap(),
            body
        );
        // The deliberately legacy path keeps its existing pre-rich behavior.
        lock.meta.schema_version = vibe_core::manifest::LEGACY_SCHEMA_VERSION;
        retained_for_preview(&NoAcquisition, &workspace, &lock, storage).unwrap();
        assert!(!scratch.exists());
    }
}
