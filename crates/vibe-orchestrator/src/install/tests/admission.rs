//! The real Fresh branch must admit providers before publishing a source delta.
#![cfg(test)]
use super::*;
use std::collections::BTreeSet;
use vibe_core::{
    Group, PackageRef,
    manifest::{Lockfile, Manifest, SpecFormat},
};
use vibe_registry::{CachedPackage, RegistryError};
use vibe_resolver::{ResolvedGraph, SolveError};

struct FreshSource;
impl vibe_install::InstallSource for FreshSource {
    fn resolve_and_fetch(
        &self,
        _: &PackageRef,
        _: &Path,
        _: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        panic!("Fresh must not fetch")
    }
    fn solve(&self, _: &[PackageRef]) -> Result<ResolvedGraph, SolveError> {
        panic!("Fresh must not solve")
    }
    fn manifest_of(&self, _: &PackageRef) -> Result<Manifest, SolveError> {
        panic!("Fresh must consume its materialized snapshot")
    }
    fn solve_masked(
        &self,
        _: &[PackageRef],
        _: &BTreeSet<(String, String)>,
    ) -> Result<ResolvedGraph, SolveError> {
        panic!("Fresh must not solve")
    }
    fn materialise_in_place(
        &self,
        _: &PackageRef,
        _: &Path,
    ) -> Result<vibe_registry::InPlaceMaterialised, RegistryError> {
        panic!("Fresh must not place")
    }
}
impl PackageSource for FreshSource {
    fn qualify(&self, p: &PackageRef, _: &Lockfile) -> anyhow::Result<PackageRef> {
        Ok(p.clone())
    }
}
struct FreshFactory;
impl PackageSourceFactory for FreshFactory {
    fn build(&self, _: PackageSourceBuild<'_>) -> anyhow::Result<Box<dyn PackageSource>> {
        Ok(Box::new(FreshSource))
    }
}
struct Rename;
impl InstallManifestMutation for Rename {
    fn apply(
        &self,
        manifest: &mut Manifest,
        workspace: &mut vibe_workspace::Workspace,
        _: &Path,
    ) -> anyhow::Result<()> {
        manifest.project.as_mut().unwrap().name = "after-source-delta".into();
        workspace.root_manifest.project.as_mut().unwrap().name = "after-source-delta".into();
        Ok(())
    }
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
fn fresh_source_delta_waits_for_actual_provider_admission() {
    for variable in [false, true] {
        for common in [true, false] {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().join("project");
            let source = temp.path().join("source");
            std::fs::create_dir_all(&root).unwrap();
            std::fs::create_dir_all(&source).unwrap();
            let before = if variable {
                "[project]\nname='before'\nversion='1.0.0'\n[workspace.versions]\nprovider='=1.0.0'\n[requires.packages]\n'org.example/provider'={version.var='provider'}\n"
            } else {
                "[project]\nname='before'\nversion='1.0.0'\n[requires.packages]\n'org.example/provider'='=1.0.0'\n"
            };
            std::fs::write(root.join("vibe.toml"), before).unwrap();
            std::fs::write(
                source.join("vibe.toml"),
                "[package]\ngroup='org.example'\nname='provider'\nkind='flow'\nversion='1.0.0'\n",
            )
            .unwrap();
            let hash = vibe_registry::compute_content_hash(&source).unwrap();
            let group = Group::parse("org.example").unwrap();
            let version = "1.0.0".parse().unwrap();
            vibe_workspace::vibedeps::materialise_with_spec_format(
                &root,
                &group,
                "provider",
                &version,
                &source,
                vibe_workspace::vibedeps::CopyMode::Copy,
                SpecFormat::Mixed,
                &vibe_core::ContentHash::from_validated(hash.clone()),
            )
            .unwrap();
            let slot = vibe_workspace::vibedeps::slot_abs_path(&root, &group, "provider", &version);
            let mut declaration = std::fs::read_to_string(slot.join("vibe.toml")).unwrap();
            if common {
                declaration.push_str("\n[configuration]\n");
            }
            std::fs::write(slot.join("vibe.toml"), declaration).unwrap();
            std::fs::write(root.join("vibe.lock"),format!("[meta]\ngenerated_by='fixture'\ngenerated_at='2026-11-05T00:00:00Z'\nschema_version=7\nroot_dependencies=['org.example/provider@=1.0.0']\n[[package]]\nkind='flow'\ngroup='org.example'\nname='provider'\nversion='1.0.0'\nsource_url='https://example.invalid/provider'\ncontent_hash='{hash}'\n")).unwrap();
            let root = resolve_project_root(&root).unwrap();
            let selection = SelectedManifest::read(&root).prepare();
            let inspected = vibe_workspace::Workspace::discover(&root).unwrap();
            let lock = Lockfile::read(root.join("vibe.lock")).unwrap();
            assert!(vibe_workspace::freshness::check(&inspected, &lock).is_fresh());
            let environment = CountingEnvironment::default();
            let mut stage = CountingStage::default();
            let result = execute_prepared(
                InstallExecution {
                    args: InstallInputs::default(),
                    environment: &environment,
                    policy: InstallPolicy {
                        offline: true,
                        ..Default::default()
                    },
                    lease: lease_for(&root),
                    manifest_mutation: &Rename,
                    selection,
                    metadata: metadata(&root),
                    sources: &FreshFactory,
                    confirm_gate: &AlwaysYes,
                    observer: &SilentInstall,
                    agent: Arc::new(vibe_lifecycle::NoAgentBackend),
                    trace: None,
                },
                &mut stage,
            );
            if common {
                let Err(error) = result else {
                    panic!("unsupported provider must refuse")
                };
                assert!(
                error.chain().any(|e| matches!(
                    e.downcast_ref::<vibe_install::Error>(),
                    Some(vibe_install::Error::Core(vibe_core::Error::InvalidLockfile {reason})) if reason.contains("org.example/provider") && reason.contains("no lock record")
                )),
                "{error:#}"
            );
                assert_eq!(
                    std::fs::read_to_string(root.join("vibe.toml")).unwrap(),
                    before,
                    "Fresh publication must follow provider admission"
                );
                assert_eq!(stage.calls(), 0);
            } else {
                let run = result.unwrap();
                assert!(matches!(run.disposition, InstallDisposition::Fresh));
                assert_eq!(
                    Manifest::read(root.join("vibe.toml"))
                        .unwrap()
                        .project
                        .unwrap()
                        .name,
                    "after-source-delta"
                );
                assert_eq!(stage.calls(), 1);
            }
        }
    }
}
