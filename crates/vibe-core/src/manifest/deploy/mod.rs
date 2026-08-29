//! `[[deploy.target]]` and `[deploy.profiles]` — named destination
//! selections, in the amended A1 spelling (freeze of 2026-08-29, amended at
//! A1 acceptance the same day).
//!
//! A deploy target reconciles exactly one produced artifact into a
//! destination through a deploy-role mechanism. Profiles are named ordered
//! selections of targets, not overlays of the manifest; the optional
//! `default_profile` under `[deploy]` is the only defaulting surface —
//! environment and secrets never choose a profile.
//!
//! Pure grammar and validation: this cell plans nothing, installs nothing,
//! and writes no destination state. Ids obey the mechanism plane's one
//! grammar, the portable token; an exact `provider` pin uses the
//! ExtensionKey spelling (`<group>/<package>#<id>`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS");

mod error;
mod wire;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use std::collections::{BTreeMap, BTreeSet};

use indexmap::IndexMap;

use super::extension::ExtensionConfig;
use super::mechanism::{MechanismKey, MechanismRole, ProviderPin, is_portable_token};
use super::plane::{assert_acyclic, bounded_value};

pub use error::DeployError;
pub(crate) use wire::DeploySectionWire;

/// One `[[deploy.target]]` row.
///
/// ```
/// use vibe_core::manifest::Manifest;
///
/// let manifest = Manifest::parse_str(concat!(
///     "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n",
///     "[[artifacts.build]]\nid = \"vibe-helper\"\nmechanism = \"build:cargo\"\n",
///     "inputs = [{ path = \"Cargo.toml\" }]\n",
///     "outputs = [{ id = \"vibe-helper.exe\", kind = \"executable\" }]\n\n",
///     "[[deploy.target]]\nid = \"local-helper\"\nartifact = \"vibe-helper.exe\"\n",
///     "mechanism = \"deploy:vibe-bin\"\ndepends_on = []\n",
///     "config = { command = \"vibe-helper\" }\n",
/// )).unwrap();
/// assert_eq!(manifest.deploy.as_ref().unwrap().targets[0].id, "local-helper");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployTarget {
    pub id: String,
    pub artifact: String,
    pub mechanism: MechanismKey,
    /// Optional exact provider pin (`<group>/<package>#<id>`); selection
    /// lands later.
    pub provider: Option<ProviderPin>,
    /// Authored presence is preserved: absent and `depends_on = []` both
    /// mean "no dependencies", but the distinction survives the round-trip.
    pub depends_on: Option<Vec<String>>,
    pub config: Option<ExtensionConfig>,
}

impl DeployTarget {
    /// Validate the row's own shape (no graph context).
    pub fn validate(&self) -> Result<(), DeployError> {
        if !is_portable_token(&self.id) {
            return Err(DeployError::TargetIdNotPortable {
                value: bounded_value(&self.id),
            });
        }
        if !is_portable_token(&self.artifact) {
            return Err(DeployError::ArtifactIdNotPortable {
                target: self.id.clone(),
                value: bounded_value(&self.artifact),
            });
        }
        if self.mechanism.role() != MechanismRole::Deploy {
            return Err(DeployError::MechanismFamily {
                target: self.id.clone(),
                key: self.mechanism.to_string(),
                actual: self.mechanism.role().to_string(),
            });
        }
        Ok(())
    }
}

/// One `[deploy.profiles.<name>]` row — an ordered selection of target ids.
///
/// ```
/// use vibe_core::manifest::DeployProfile;
///
/// let profile = DeployProfile { targets: vec!["local-helper".into()] };
/// assert_eq!(profile.targets, ["local-helper"]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployProfile {
    pub targets: Vec<String>,
}

/// The whole `[deploy]` section.
///
/// ```
/// use vibe_core::manifest::DeploySection;
///
/// assert!(DeploySection::default().is_empty());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeploySection {
    /// Optional explicit default under `[deploy]`. Never inferred from
    /// environment or secrets.
    pub default_profile: Option<String>,
    /// Targets in authored order — the vector *is* the declaration, and a
    /// rewrite must hand it back unshuffled.
    pub targets: Vec<DeployTarget>,
    /// Named profiles. A profile table is a **map**: one name answers one
    /// selection and none shadows another, so the order the names were
    /// written in carries no meaning, is not preserved by a rewrite (map
    /// keys render sorted), and is ignored by equality. The order *inside*
    /// [`DeployProfile::targets`] is authored and does survive — that vector
    /// is the selection.
    pub profiles: IndexMap<String, DeployProfile>,
}

impl DeploySection {
    /// Whether the section can be omitted entirely.
    pub fn is_empty(&self) -> bool {
        self.default_profile.is_none() && self.targets.is_empty() && self.profiles.is_empty()
    }

    /// Validate targets, the depends_on graph, and the profiles against the
    /// artifact ids the `[artifacts]` section produces.
    pub fn validate(&self, artifact_ids: &BTreeSet<String>) -> Result<(), DeployError> {
        let mut target_ids: BTreeSet<&str> = BTreeSet::new();
        for target in &self.targets {
            target.validate()?;
            if !target_ids.insert(target.id.as_str()) {
                return Err(DeployError::DuplicateTargetId {
                    value: bounded_value(&target.id),
                });
            }
            if !artifact_ids.contains(&target.artifact) {
                return Err(DeployError::UnknownArtifact {
                    target: target.id.clone(),
                    artifact: bounded_value(&target.artifact),
                });
            }
        }
        // Second pass: depends_on is judged against the complete id set, so
        // forward references are legal.
        for target in &self.targets {
            let Some(depends_on) = &target.depends_on else {
                continue;
            };
            let mut seen: BTreeSet<&str> = BTreeSet::new();
            for dependency in depends_on {
                if !is_portable_token(dependency) {
                    return Err(DeployError::DependencyIdNotPortable {
                        target: target.id.clone(),
                        value: bounded_value(dependency),
                    });
                }
                if !seen.insert(dependency.as_str()) {
                    return Err(DeployError::DuplicateDependency {
                        target: target.id.clone(),
                        dependency: dependency.clone(),
                    });
                }
                if dependency == &target.id {
                    return Err(DeployError::SelfDependency {
                        target: target.id.clone(),
                    });
                }
                if !target_ids.contains(dependency.as_str()) {
                    return Err(DeployError::UnknownDependency {
                        target: target.id.clone(),
                        dependency: dependency.clone(),
                    });
                }
            }
        }
        self.validate_target_graph_acyclic()?;
        self.validate_profiles(&target_ids)?;
        Ok(())
    }

    fn validate_target_graph_acyclic(&self) -> Result<(), DeployError> {
        let mut edges: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for target in &self.targets {
            if let Some(depends_on) = &target.depends_on {
                edges.insert(
                    target.id.as_str(),
                    depends_on.iter().map(String::as_str).collect(),
                );
            }
        }
        assert_acyclic(&edges).map_err(|cycle| DeployError::DependsOnCycle {
            cycle: cycle.join(" -> "),
        })
    }

    fn validate_profiles(&self, target_ids: &BTreeSet<&str>) -> Result<(), DeployError> {
        for (name, profile) in &self.profiles {
            if !is_portable_token(name) {
                return Err(DeployError::ProfileNameNotPortable { name: name.clone() });
            }
            if profile.targets.is_empty() {
                return Err(DeployError::EmptyProfileTargets { name: name.clone() });
            }
            let mut seen: BTreeSet<&str> = BTreeSet::new();
            for target in &profile.targets {
                if !seen.insert(target.as_str()) {
                    return Err(DeployError::DuplicateProfileTarget {
                        name: name.clone(),
                        target: target.clone(),
                    });
                }
                if !target_ids.contains(target.as_str()) {
                    return Err(DeployError::UnknownProfileTarget {
                        name: name.clone(),
                        target: target.clone(),
                    });
                }
            }
            // Every dependency of every selected target must be included in
            // the selected profile — reachable transitively only through
            // targets the profile itself selects, in authored order. An
            // implicit auto-include would make a profile silently deploy more
            // than it names, so the narrow reading is enforced: the selection
            // is the whole truth.
            let selected: BTreeSet<&str> = profile.targets.iter().map(String::as_str).collect();
            for target in &profile.targets {
                let Some(depends_on) = self
                    .targets
                    .iter()
                    .find(|candidate| candidate.id == *target)
                    .and_then(|candidate| candidate.depends_on.as_ref())
                else {
                    continue;
                };
                for dependency in depends_on {
                    if !selected.contains(dependency.as_str()) {
                        return Err(DeployError::MissingDependencyInProfile {
                            name: name.clone(),
                            target: target.clone(),
                            dependency: dependency.clone(),
                        });
                    }
                }
            }
        }
        if let Some(default) = &self.default_profile
            && !self.profiles.contains_key(default)
        {
            return Err(DeployError::UnknownDefaultProfile {
                name: default.clone(),
            });
        }
        Ok(())
    }
}
