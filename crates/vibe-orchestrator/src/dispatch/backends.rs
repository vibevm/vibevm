//! The two handler backends this dispatcher injects: project package-skill
//! bindings, and workspace-resolved binaries.
//!
//! Split from the dispatch loop because they answer a different question. The
//! loop decides WHICH contribution runs and what its row says; these decide
//! HOW a package binding or a binary handler is actually satisfied, and they
//! change for reasons that have nothing to do with row accumulation or failure
//! carriage.

use anyhow::Result;
use vibe_lifecycle::handlers::{
    BinaryBackend, PackageBindingArtifact, PackageBindingBackend, PackageBindingOutcome,
};
use vibe_lifecycle::native::{ArtifactNativeBackend, NativeBuildExecution, NativePlatform};

use crate::{RitualPlan, world};

pub(super) fn native_backend<'a>(
    plan: &'a RitualPlan,
    metadata: &'a vibe_lifecycle::RunMetadata,
    candidates: &'a [&'a vibe_lifecycle::ExtensionRegistryRow],
) -> Result<ArtifactNativeBackend<'a>> {
    Ok(ArtifactNativeBackend::new(NativeBuildExecution {
        candidates,
        selected_project_root: std::path::Path::new(&plan.project.root),
        registry: &plan.mechanisms,
        routes: &plan.mechanism_routes,
        platform: NativePlatform::current()?,
        offline: metadata.offline,
        created_at: &metadata.started,
    }))
}

pub(super) struct ProjectPackageBindingBackend<'a> {
    project_root: &'a std::path::Path,
    bindings: &'a std::collections::BTreeMap<
        String,
        vibe_agent_projection::pkgskill::ProjectSkillBinding,
    >,
    desired: &'a std::collections::BTreeSet<String>,
}

impl<'a> ProjectPackageBindingBackend<'a> {
    pub(super) fn new(plan: &'a RitualPlan) -> Self {
        Self {
            project_root: std::path::Path::new(&plan.project.root),
            bindings: &plan.package_bindings,
            desired: &plan.package_desired_keys,
        }
    }
}

impl PackageBindingBackend for ProjectPackageBindingBackend<'_> {
    fn probe(
        &self,
        key: &str,
        artifacts: &[vibe_wire::generated::lifecycle_state::StateArtifact],
    ) -> Result<bool, String> {
        if key == world::PACKAGE_SKILL_RECOVER_KEY {
            return vibe_agent_projection::pkgskill::probe_recovered_project_skill_bindings(
                self.project_root,
                artifacts,
            )
            .map_err(|error| error.to_string());
        }
        if key == world::PACKAGE_SKILL_RECONCILE_KEY {
            return vibe_agent_projection::pkgskill::probe_vanished_project_skill_bindings(
                self.project_root,
                self.desired,
                artifacts,
            )
            .map_err(|error| error.to_string());
        }
        let binding = self.bindings.get(key).ok_or_else(|| {
            format!("package binding `{key}` was not present in the prepared plan")
        })?;
        vibe_agent_projection::pkgskill::probe_project_skill_binding(
            self.project_root,
            binding,
            artifacts,
        )
        .map_err(|error| error.to_string())
    }

    fn execute(&self, key: &str) -> Result<PackageBindingOutcome, String> {
        if key == world::PACKAGE_SKILL_RECOVER_KEY {
            let reports =
                vibe_agent_projection::pkgskill::recover_project_skill_bindings(self.project_root)
                    .map_err(|error| error.to_string())?;
            return Ok(PackageBindingOutcome {
                artifacts: Vec::new(),
                message: Some(format!(
                    "recovered {} pending package-skill target(s)",
                    reports.len()
                )),
            });
        }
        if key == world::PACKAGE_SKILL_RECONCILE_KEY {
            let reports =
                vibe_agent_projection::pkgskill::reconcile_vanished_project_skill_bindings(
                    self.project_root,
                    self.desired,
                )
                .map_err(|error| error.to_string())?;
            return Ok(PackageBindingOutcome {
                artifacts: Vec::new(),
                message: Some(format!(
                    "reconciled {} vanished project skill target(s)",
                    reports.len()
                )),
            });
        }
        let binding = self.bindings.get(key).ok_or_else(|| {
            format!("package binding `{key}` was not present in the prepared plan")
        })?;
        let reports = vibe_agent_projection::pkgskill::reconcile_project_skill_binding(
            self.project_root,
            binding,
        )
        .map_err(|error| error.to_string())?;
        let artifacts = if binding.selected_files.is_some() {
            binding
                .targets
                .iter()
                .map(|target| PackageBindingArtifact {
                    id: binding.artifact_id(target.agent),
                    kind: "agent-skill".into(),
                    path: vibe_core::machine_json_path(&target.path),
                })
                .collect()
        } else {
            Vec::new()
        };
        let summary = if reports.is_empty() && binding.selected_files.is_none() {
            "source=missing, no receipt-owned target changed".to_string()
        } else {
            reports
                .iter()
                .map(|report| format!("{}={}", report.agent, report.status))
                .collect::<Vec<_>>()
                .join(", ")
        };
        Ok(PackageBindingOutcome {
            artifacts,
            message: Some(format!(
                "projected skill `{}` ({summary})",
                binding.skill.decl.name
            )),
        })
    }
}

pub(super) struct WorkspaceBinaryBackend {
    pub(super) quiet: bool,
    pub(super) allow_build: bool,
}
impl BinaryBackend for WorkspaceBinaryBackend {
    fn resolve_or_build(
        &self,
        row: &vibe_lifecycle::ExtensionRegistryRow,
        name: &str,
    ) -> Result<std::path::PathBuf, String> {
        let (binary, home) = match row.provider() {
            vibe_lifecycle::ExtensionProvider::Dependency(provider) => (
                vibe_workspace::bins::find_binary_in_provider_slot(
                    &provider.root,
                    provider.id.group(),
                    provider.id.name().as_str(),
                    &provider.version,
                    name,
                ),
                vibe_workspace::bins::BinaryProviderHome::InstalledSlot,
            ),
            vibe_lifecycle::ExtensionProvider::Host(provider) => {
                let vibe_lifecycle::HostIdentity::Coordinate(id) = &provider.identity else {
                    return Err("binary handler host must be a package-role coordinate".into());
                };
                if provider.kind.is_none() {
                    return Err("binary handler host must be an authored package root".into());
                }
                (
                    vibe_workspace::bins::find_binary_in_authored_package_root(
                        &provider.root,
                        id.group(),
                        id.name().as_str(),
                        &provider.version,
                        name,
                    ),
                    vibe_workspace::bins::BinaryProviderHome::AuthoredPackageRoot,
                )
            }
        };
        let binary = binary.map_err(|error| error.to_string())?;
        if !binary.artifact().exists() {
            if !self.allow_build {
                return Err("preparation-required: ordinary verification cannot build a missing handler executable".into());
            }
            vibe_workspace::bins::build_binary_authorized_with_output(
                &binary,
                vibe_workspace::bins::BuildAuthorization::InstalledExtension { home },
                if self.quiet {
                    vibe_workspace::bins::BuildOutput::Quiet
                } else {
                    vibe_workspace::bins::BuildOutput::Inherit
                },
            )
            .map_err(|error| error.to_string())?;
        }
        Ok(binary.artifact())
    }
}

#[cfg(test)]
mod m_03_a_1_cases {
    use super::*;
    use std::{collections::BTreeMap, fs, path::Path};
    fn inventory(root: &Path) -> BTreeMap<String, Option<Vec<u8>>> {
        fn visit(root: &Path, path: &Path, rows: &mut BTreeMap<String, Option<Vec<u8>>>) {
            for entry in fs::read_dir(path).unwrap() {
                let path = entry.unwrap().path();
                let name = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                if path.is_dir() {
                    rows.insert(name, None);
                    visit(root, &path, rows);
                } else {
                    rows.insert(name, Some(fs::read(path).unwrap()));
                }
            }
        }
        let mut rows = BTreeMap::new();
        visit(root, root, &mut rows);
        rows
    }
    #[test]
    fn ordinary_verify_missing_tool_refuses_before_build_and_preserves_owned_tree() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("crates/missing/src")).unwrap();
        fs::write(root.path().join("vibe.toml"),"[package]\ngroup='org.demo'\nname='tool'\nkind='tool'\nversion='1.0.0'\n[[binary]]\nname='missing'\ncrate='crates/missing'\n[[extension]]\nid='observe'\npoint='phase:verify'\neffects=['evidence-observer']\ninputs=[]\nhandler={kind='binary',name='missing'}\n").unwrap();
        fs::write(
            root.path().join("crates/missing/Cargo.toml"),
            "[package]\nname='missing'\nversion='0.1.0'\nedition='2024'\n",
        )
        .unwrap();
        fs::write(
            root.path().join("crates/missing/src/main.rs"),
            "fn main(){}\n",
        )
        .unwrap();
        fs::write(
            root.path().join("crates/missing/build.rs"),
            "fn main(){std::fs::write(\"BUILD-CALLED\",b\"forbidden\").unwrap();}\n",
        )
        .unwrap();
        let loaded = crate::world::inspect(root.path()).unwrap();
        let row = loaded
            .registry
            .rows()
            .iter()
            .find(|row| row.declaration().id == "observe")
            .unwrap();
        let lookup = vibe_workspace::bins::find_binary_in_authored_package_root(
            root.path(),
            &"org.demo".parse().unwrap(),
            "tool",
            "1.0.0",
            "missing",
        )
        .unwrap();
        assert!(!lookup.artifact().exists());
        let before = inventory(root.path());
        let backend = WorkspaceBinaryBackend {
            quiet: true,
            allow_build: false,
        };
        let error = backend.resolve_or_build(row, "missing").unwrap_err();
        assert_eq!(
            error,
            "preparation-required: ordinary verification cannot build a missing handler executable"
        );
        assert_eq!(before, inventory(root.path()));
        assert!(!root.path().join("crates/missing/BUILD-CALLED").exists());
        assert!(!root.path().join("crates/missing/target").exists());
    }
}
