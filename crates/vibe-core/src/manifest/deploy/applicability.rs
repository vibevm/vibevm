//! Pure forward and inverse profile projection.

use std::collections::{BTreeMap, BTreeSet};

use super::{DeployError, DeploySection, DeployTarget};
use crate::manifest::{TargetApplicability, TargetOs};

/// One forward profile after its authored members were evaluated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployProfileProjection {
    profile: String,
    active_targets: Vec<String>,
    decisions: Vec<TargetApplicability>,
}

impl DeployProfileProjection {
    #[must_use]
    pub fn profile(&self) -> &str {
        &self.profile
    }

    #[must_use]
    pub fn active_targets(&self) -> &[String] {
        &self.active_targets
    }

    #[must_use]
    pub fn decisions(&self) -> &[TargetApplicability] {
        &self.decisions
    }
}

impl DeploySection {
    /// Apply one injected OS before provider, artifact, receipt or collision work.
    pub fn project_profile(
        &self,
        name: &str,
        os: TargetOs,
    ) -> Result<DeployProfileProjection, DeployError> {
        let profile =
            self.profiles
                .get(name)
                .ok_or_else(|| DeployError::MissingProjectedProfile {
                    name: name.to_owned(),
                })?;
        let targets = self.target_index();
        let decisions = profile
            .targets
            .iter()
            .map(|id| {
                let target = targets
                    .get(id.as_str())
                    .expect("validated profiles reference declared targets");
                TargetApplicability::decide(id, target.when.as_ref(), os)
            })
            .collect::<Vec<_>>();
        let selected = profile
            .targets
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let active = decisions
            .iter()
            .filter(|decision| decision.is_active())
            .map(|decision| decision.target())
            .collect::<BTreeSet<_>>();
        for decision in decisions.iter().filter(|decision| decision.is_active()) {
            let target = targets[decision.target()];
            for dependency in target.depends_on.iter().flatten() {
                if !selected.contains(dependency.as_str()) {
                    return Err(DeployError::MissingDependencyInProfile {
                        name: name.to_owned(),
                        target: target.id.clone(),
                        dependency: dependency.clone(),
                    });
                }
                if !active.contains(dependency.as_str()) {
                    return Err(DeployError::ActiveDependencyInactive {
                        name: name.to_owned(),
                        target: target.id.clone(),
                        dependency: dependency.clone(),
                        os: os.to_string(),
                    });
                }
            }
        }
        let active_targets = decisions
            .iter()
            .filter(|decision| decision.is_active())
            .map(|decision| decision.target().to_owned())
            .collect::<Vec<_>>();
        if active_targets.is_empty() {
            return Err(DeployError::NoApplicableTargets {
                name: name.to_owned(),
                os: os.to_string(),
                targets: profile.targets.join(", "),
            });
        }
        Ok(DeployProfileProjection {
            profile: name.to_owned(),
            active_targets,
            decisions,
        })
    }

    /// Explicit inverse uses every authored profile member, irrespective of OS.
    pub fn inverse_profile_targets(&self, name: &str) -> Result<Vec<String>, DeployError> {
        let profile =
            self.profiles
                .get(name)
                .ok_or_else(|| DeployError::MissingProjectedProfile {
                    name: name.to_owned(),
                })?;
        let targets = self.target_index();
        let selected = profile
            .targets
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        for id in &profile.targets {
            let target = targets
                .get(id.as_str())
                .expect("validated profiles reference declared targets");
            for dependency in target.depends_on.iter().flatten() {
                if !selected.contains(dependency.as_str()) {
                    return Err(DeployError::MissingDependencyInProfile {
                        name: name.to_owned(),
                        target: target.id.clone(),
                        dependency: dependency.clone(),
                    });
                }
            }
        }
        Ok(profile.targets.clone())
    }

    fn target_index(&self) -> BTreeMap<&str, &DeployTarget> {
        self.targets
            .iter()
            .map(|target| (target.id.as_str(), target))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::manifest::{Manifest, TargetOs};

    fn manifest() -> Manifest {
        Manifest::parse_str(
            "[project]\nname='demo'\nversion='0.1.0'\n\
             [[artifacts.build]]\nid='build'\nmechanism='build:cargo'\noutputs=[{id='app',kind='file'}]\n\
             [[deploy.target]]\nid='win'\nartifact='app'\nmechanism='deploy:vibe-bin'\nwhen={os=['windows']}\n\
             [[deploy.target]]\nid='posix'\nartifact='app'\nmechanism='deploy:vibe-bin'\nwhen={os=['linux','macos']}\n\
             [deploy]\ndefault_profile='local'\n[deploy.profiles.local]\ntargets=['win','posix']\n",
        )
        .unwrap()
    }

    #[test]
    fn one_profile_projects_different_sets_without_changing_profile() {
        let deploy = manifest().deploy.unwrap();
        let windows = deploy.project_profile("local", TargetOs::Windows).unwrap();
        let linux = deploy.project_profile("local", TargetOs::Linux).unwrap();
        assert_eq!(windows.active_targets(), ["win"]);
        assert_eq!(linux.active_targets(), ["posix"]);
        assert_eq!(windows.profile(), linux.profile());
        assert_eq!(windows.decisions()[1].status(), "skipped");
    }

    #[test]
    fn active_to_inactive_dependency_refuses_and_inverse_keeps_both() {
        let mut manifest = manifest();
        manifest.deploy.as_mut().unwrap().targets[1].depends_on = Some(vec!["win".into()]);
        let deploy = manifest.deploy.unwrap();
        let error = deploy
            .project_profile("local", TargetOs::Linux)
            .unwrap_err()
            .to_string();
        for expected in ["posix", "win", "linux"] {
            assert!(error.contains(expected), "{error}");
        }
        assert_eq!(
            deploy.inverse_profile_targets("local").unwrap(),
            ["win", "posix"]
        );
    }

    #[test]
    fn inactive_dependencies_pull_nothing_and_zero_active_is_typed() {
        let mut manifest = manifest();
        let deploy = manifest.deploy.as_mut().unwrap();
        deploy.profiles["local"].targets = vec!["win".into()];
        deploy.targets[0].depends_on = Some(vec!["posix".into()]);
        let error = deploy
            .project_profile("local", TargetOs::Linux)
            .unwrap_err()
            .to_string();
        assert!(error.contains("NO_APPLICABLE_TARGETS"), "{error}");
        assert!(!error.contains("dependency"), "{error}");
    }
}
