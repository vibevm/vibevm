//! Original requirement occurrences before package-reference flattening.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#solver");
use crate::DepProviderError;
use vibe_core::{
    PackageRef,
    manifest::{Manifest, Requires},
};

/// A site is interpreted only beside its exact retained candidate metadata.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RequirementSite {
    Capability(usize),
    Recommendation(usize),
    ExplicitRequest(usize),
    Requires(usize),
    InstallerOption {
        option: usize,
        requirement: usize,
    },
    InstallerStep {
        option: usize,
        step: usize,
        requirement: usize,
    },
    Alternative(usize),
    Conditional {
        predicate: String,
        requirement: usize,
    },
}
/// Eligibility is separate from the target coordinate and registry constraint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequirementActivation {
    Mandatory,
    Optional,
    Conditional { predicate: String, optional: bool },
}
#[derive(Debug, Clone)]
pub struct RequirementAlternative<P> {
    pub reference: PackageRef,
    pub registry_pin: Option<P>,
}
/// Distinct original sites are never deduplicated merely by target PackageRef.
#[derive(Debug, Clone)]
pub struct OriginalRequirement<P> {
    pub site: RequirementSite,
    pub activation: RequirementActivation,
    pub alternatives: Vec<RequirementAlternative<P>>,
}

/// Preserve original declaration ownership for edge-local pin projection.
/// The pin callback reads the supplied Requires owner, never a global name map.
///
/// ```
/// use vibe_core::manifest::Manifest;
/// use vibe_resolver::source_edges::original_requirements;
/// let manifest = Manifest::new_project("demo", "1.0.0");
/// let edges = original_requirements::<String>(&manifest, |_, _| None)?;
/// assert!(edges.is_empty());
/// # Ok::<(), vibe_resolver::DepProviderError>(())
/// ```
pub fn original_requirements<P>(
    manifest: &Manifest,
    pin: impl Fn(&Requires, &PackageRef) -> Option<P>,
) -> Result<Vec<OriginalRequirement<P>>, DepProviderError> {
    let mut result = Vec::new();
    let make = |site, activation, reference, registry_pin| OriginalRequirement {
        site,
        activation,
        alternatives: vec![RequirementAlternative {
            reference,
            registry_pin,
        }],
    };
    for (index, reference) in manifest.requires.packages.iter().enumerate() {
        let optional = manifest
            .requires
            .optional
            .get(&reference.qualified_name())
            .copied()
            .unwrap_or(false);
        result.push(make(
            RequirementSite::Requires(index),
            if optional {
                RequirementActivation::Optional
            } else {
                RequirementActivation::Mandatory
            },
            reference.clone(),
            pin(&manifest.requires, reference),
        ));
    }
    let parse_app = |text: &str| {
        PackageRef::parse(text).map_err(|error| {
            DepProviderError::Other(format!("invalid original application requirement: {error}"))
        })
    };
    for (option, declaration) in manifest.installer_options.iter().enumerate() {
        for (requirement, target) in declaration.requires.iter().enumerate() {
            result.push(make(
                RequirementSite::InstallerOption {
                    option,
                    requirement,
                },
                RequirementActivation::Mandatory,
                parse_app(target.package())?,
                None,
            ));
        }
        for (step, declaration) in declaration.steps.iter().enumerate() {
            for (requirement, target) in declaration.requires.iter().enumerate() {
                result.push(make(
                    RequirementSite::InstallerStep {
                        option,
                        step,
                        requirement,
                    },
                    RequirementActivation::Mandatory,
                    parse_app(target.package())?,
                    None,
                ));
            }
        }
    }
    for (index, choice) in manifest.requires_any.iter().enumerate() {
        result.push(OriginalRequirement {
            site: RequirementSite::Alternative(index),
            activation: RequirementActivation::Mandatory,
            alternatives: choice
                .one_of
                .iter()
                .cloned()
                .map(|reference| RequirementAlternative {
                    reference,
                    registry_pin: None,
                })
                .collect(),
        });
    }
    for (predicate, target) in &manifest.conditional_deps {
        for (requirement, reference) in target.dependencies.packages.iter().enumerate() {
            let optional = target
                .dependencies
                .optional
                .get(&reference.qualified_name())
                .copied()
                .unwrap_or(false);
            result.push(make(
                RequirementSite::Conditional {
                    predicate: predicate.clone(),
                    requirement,
                },
                RequirementActivation::Conditional {
                    predicate: predicate.clone(),
                    optional,
                },
                reference.clone(),
                pin(&target.dependencies, reference),
            ));
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn optional_pin_and_unpinned_installer_requirement_keep_separate_owners() {
        let manifest = Manifest::parse_str(
            r#"
[package]
name='demo'
group='org.example'
kind='app'
version='1.0.0'
[application]
id='demo'
[requires.packages]
'org.example/runtime'={version='2',optional=true}
[[installer_options]]
id='main'
name='Main'
platform='linux'
platform_name='Linux'
arch='x86_64'
requires=['org.example/runtime@2']
[[installer_options.steps]]
id='build'
title='Build'
kind='copy'
from='payload'
requires=['org.example/runtime@2']
"#,
        )
        .unwrap();
        let edges = original_requirements(&manifest, |_, _| Some("private-registry")).unwrap();
        assert_eq!(edges.len(), 3);
        assert_eq!(edges[0].activation, RequirementActivation::Optional);
        assert_eq!(
            edges[0].alternatives[0].registry_pin,
            Some("private-registry")
        );
        assert!(
            edges[1..]
                .iter()
                .all(|edge| edge.alternatives[0].registry_pin.is_none())
        );
        assert_eq!(
            edges[1].site,
            RequirementSite::InstallerOption {
                option: 0,
                requirement: 0
            }
        );
        assert_eq!(
            edges[2].site,
            RequirementSite::InstallerStep {
                option: 0,
                step: 0,
                requirement: 0
            }
        );
        assert_eq!(manifest.resolution_requires().len(), 1);
    }
}
