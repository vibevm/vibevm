//! Consumer declarations survive identity deduplication of the solved packages.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#INSTALL-CONSUMER-CONSTRAINTS"
);

use std::collections::BTreeSet;
use std::path::Path;
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_core::{PackageKind, PackageRef, VersionSpec};
use vibe_resolver::SolveError;
use vibe_workspace::Workspace;

use crate::error::{Error, Result};

pub(super) struct ConsumerRequirement {
    consumer: String,
    requirement: PackageRef,
    configuration: Option<vibe_core::manifest::ConfigurationIntentDecl>,
    registry_pin: Option<vibe_core::manifest::RegistryPin>,
    site: vibe_resolver::source_edges::RequirementSite,
}

pub(super) struct RootRequirements {
    declarations: Vec<ConsumerRequirement>,
}

/// Validate before the legacy migration can write to a selected manifest.
pub(super) fn selected_node(workspace: &Workspace, root: &Path) -> Result<String> {
    workspace
        .node_rel_of(root)
        .map(|rel| rel.to_string())
        .ok_or_else(|| Error::SelectedConsumerMissing {
            selected: root.display().to_string(),
            workspace: workspace.root.display().to_string(),
        })
}

pub(super) fn unique_requests(requests: &[PackageRef]) -> Result<Vec<PackageRef>> {
    let mut unique: Vec<PackageRef> = Vec::new();
    for request in requests {
        if let Some(prior) = unique.iter().find(|prior| same_package(prior, request)) {
            if prior != request {
                return Err(Error::ConflictingExplicitRoots {
                    first: prior.to_string(),
                    second: request.to_string(),
                });
            }
        } else {
            unique.push(request.clone());
        }
    }
    Ok(unique)
}

pub(super) fn has_declarations(workspace: &Workspace) -> bool {
    workspace.iter_nodes().any(|(_, node)| {
        let requires = &node.requires;
        !requires.packages.is_empty()
            || !requires.git_packages.is_empty()
            || !requires.path_packages.is_empty()
            || !requires.var_packages.is_empty()
            || !requires.capabilities.is_empty()
    })
}

pub(super) fn same_package(left: &PackageRef, right: &PackageRef) -> bool {
    left.group == right.group && left.name == right.name
}

impl RootRequirements {
    pub(super) fn collect(
        workspace: &Workspace,
        selected: &str,
        requests: &[PackageRef],
    ) -> Result<Self> {
        let mut declarations: Vec<_> = requests
            .iter()
            .enumerate()
            .map(|(index, requirement)| ConsumerRequirement {
                site: vibe_resolver::source_edges::RequirementSite::ExplicitRequest(index),
                registry_pin: workspace
                    .iter_nodes()
                    .find(|(rel, _)| *rel == selected)
                    .and_then(|(_, node)| registry_pin(&node.requires, requirement)),
                consumer: format!("{selected}/vibe.toml (explicit request)"),
                requirement: requirement.clone(),
                configuration: workspace
                    .iter_nodes()
                    .find(|(rel, _)| *rel == selected)
                    .and_then(|(_, node)| {
                        node.requires
                            .configurations
                            .get(&requirement.qualified_name())
                    })
                    .cloned(),
            })
            .collect();
        for (rel, node) in workspace.iter_nodes() {
            for (index, requirement) in node.requires.packages.iter().enumerate() {
                if rel == selected && requests.iter().any(|r| same_package(r, requirement)) {
                    continue;
                }
                declarations.push(ConsumerRequirement {
                    consumer: format!("{rel}/vibe.toml"),
                    site: vibe_resolver::source_edges::RequirementSite::Requires(index),
                    registry_pin: registry_pin(&node.requires, requirement),
                    requirement: requirement.clone(),
                    configuration: node
                        .requires
                        .configurations
                        .get(&requirement.qualified_name())
                        .cloned(),
                });
            }
            // Source routing remains the provider's owner. A plain version
            // request never removes or changes an existing source declaration.
            for git in &node.requires.git_packages {
                declarations.push(ConsumerRequirement {
                    consumer: format!("{rel}/vibe.toml (git source)"),
                    site: vibe_resolver::source_edges::RequirementSite::Requires(
                        declarations.len(),
                    ),
                    registry_pin: None,
                    configuration: node
                        .requires
                        .configurations
                        .get(&format!("{}/{}", git.group, git.name))
                        .cloned(),
                    requirement: PackageRef::new(
                        git.kind,
                        Some(git.group.clone()),
                        git.name.clone(),
                        VersionSpec::Latest,
                    )?,
                });
            }
        }
        Ok(Self { declarations })
    }

    /// Preserve exact consumer occurrence and pin ownership before flattening.
    pub(super) fn source_edges(&self) -> Vec<vibe_resolver::RootRequirement> {
        use vibe_resolver::source_candidates::{EdgeId, EdgeOwner};
        use vibe_resolver::source_edges::{
            OriginalRequirement, RequirementActivation, RequirementAlternative,
        };
        self.declarations
            .iter()
            .map(|declaration| {
                let site = declaration.site.clone();
                vibe_resolver::RootRequirement {
                    id: EdgeId {
                        owner: EdgeOwner::Root {
                            consumer: declaration.consumer.clone(),
                        },
                        site: site.clone(),
                    },
                    requirement: OriginalRequirement {
                        site,
                        // Root authority is the actual collected operation occurrence.
                        // Do not reinterpret the legacy collector's raw optional flag.
                        activation: RequirementActivation::Mandatory,
                        alternatives: vec![RequirementAlternative {
                            reference: declaration.requirement.clone(),
                            registry_pin: declaration.registry_pin.clone(),
                        }],
                    },
                }
            })
            .collect()
    }
    pub(super) fn check_metadata_kinds(
        &self,
        selected: &std::collections::BTreeMap<
            (vibe_core::Group, String),
            vibe_resolver::CandidateMetadata,
        >,
    ) -> Result<()> {
        for declaration in &self.declarations {
            if let Some(group) = &declaration.requirement.group
                && let Some(metadata) =
                    selected.get(&(group.clone(), declaration.requirement.name.to_string()))
            {
                self.check_kind(
                    &declaration.requirement,
                    metadata.candidate.manifest().require_package()?.kind,
                )?;
            }
        }
        Ok(())
    }

    pub(super) fn configuration_inputs(
        &self,
    ) -> vibe_resolver::configuration::PackageConfigurationInputs {
        let mut inputs = vibe_resolver::configuration::PackageConfigurationInputs::new();
        for declaration in &self.declarations {
            let consumer = declaration
                .consumer
                .strip_suffix(" (explicit request)")
                .unwrap_or(&declaration.consumer);
            inputs
                .entry(declaration.requirement.qualified_name())
                .or_default()
                .insert(consumer.to_owned(), declaration.configuration.clone());
        }
        inputs
    }

    pub(super) fn authored(&self) -> Vec<PackageRef> {
        self.declarations
            .iter()
            .map(|d| d.requirement.clone())
            .collect()
    }

    pub(super) fn package_count(&self) -> usize {
        self.declarations
            .iter()
            .map(|d| (&d.requirement.group, &d.requirement.name))
            .collect::<BTreeSet<_>>()
            .len()
    }

    /// Pins are a first-attempt optimization, never consumer intent.
    pub(super) fn held(&self, locked: &Lockfile, requests: &[PackageRef]) -> Vec<PackageRef> {
        let authored = self.authored();
        let mut held = vibe_workspace::freshness::hold_pins(&authored, locked);
        for (original, pinned) in authored.iter().zip(&mut held) {
            if requests
                .iter()
                .any(|request| same_package(original, request))
            {
                *pinned = original.clone();
            }
        }
        held
    }

    pub(super) fn solve_error(&self, source: SolveError) -> Error {
        Error::ConsumerConstraints {
            declarations: self
                .declarations
                .iter()
                .map(|d| format!("{}: {}", d.consumer, d.requirement))
                .collect(),
            source,
        }
    }

    pub(super) fn contextualize(&self, error: Error) -> Error {
        match error {
            Error::Solve(source) => self.solve_error(source),
            other => other,
        }
    }

    fn check_kind(&self, requirement: &PackageRef, actual: PackageKind) -> Result<()> {
        if let Some(requested) = requirement.kind
            && requested != actual
        {
            return Err(self.solve_error(SolveError::KindMismatch {
                package: requirement.qualified_name(),
                requested,
                actual,
            }));
        }
        Ok(())
    }

    pub(super) fn check_locked_kinds(&self, lock: &Lockfile) -> Result<()> {
        for d in &self.declarations {
            if let Some(group) = &d.requirement.group
                && let Some(locked) = lock.find(group, &d.requirement.name)
            {
                self.check_kind(&d.requirement, locked.kind)?;
            }
        }
        Ok(())
    }
}

/// Old snapshots have no consumer ownership. Only unambiguous duplicate rows
/// may seed one manifest; differing rows must not serialize with last-wins.
pub(super) fn migration_roots(lock: &Lockfile) -> Result<Vec<PackageRef>> {
    unique_requests(&lock.meta.root_dependencies).map_err(|_| Error::AmbiguousRootSnapshot)
}

pub(super) fn raw_has_declarations(manifest: &Manifest) -> bool {
    !manifest.requires.is_empty()
}

fn registry_pin(
    requires: &vibe_core::manifest::Requires,
    reference: &PackageRef,
) -> Option<vibe_core::manifest::RegistryPin> {
    reference
        .group
        .as_ref()
        .and_then(|group| requires.declared_registry(group, reference.name.as_str()))
        .cloned()
}
