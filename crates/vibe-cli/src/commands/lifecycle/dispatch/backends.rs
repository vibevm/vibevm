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

use super::world;

pub(super) struct ProjectPackageBindingBackend<'a> {
    project_root: &'a std::path::Path,
    bindings: &'a std::collections::BTreeMap<String, vibe_mcp::pkgskill::ProjectSkillBinding>,
    desired: &'a std::collections::BTreeSet<String>,
}

impl<'a> ProjectPackageBindingBackend<'a> {
    pub(super) fn new(plan: &'a world::RitualPlan) -> Self {
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
            return vibe_mcp::pkgskill::probe_recovered_project_skill_bindings(
                self.project_root,
                artifacts,
            )
            .map_err(|error| error.to_string());
        }
        if key == world::PACKAGE_SKILL_RECONCILE_KEY {
            return vibe_mcp::pkgskill::probe_vanished_project_skill_bindings(
                self.project_root,
                self.desired,
                artifacts,
            )
            .map_err(|error| error.to_string());
        }
        let binding = self.bindings.get(key).ok_or_else(|| {
            format!("package binding `{key}` was not present in the prepared plan")
        })?;
        vibe_mcp::pkgskill::probe_project_skill_binding(self.project_root, binding, artifacts)
            .map_err(|error| error.to_string())
    }

    fn execute(&self, key: &str) -> Result<PackageBindingOutcome, String> {
        if key == world::PACKAGE_SKILL_RECOVER_KEY {
            let reports = vibe_mcp::pkgskill::recover_project_skill_bindings(self.project_root)
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
            let reports = vibe_mcp::pkgskill::reconcile_vanished_project_skill_bindings(
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
        let reports =
            vibe_mcp::pkgskill::reconcile_project_skill_binding(self.project_root, binding)
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
