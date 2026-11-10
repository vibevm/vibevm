//! The machine view of a plan: the `vibe-app-plan/1` document
//! (`PROP-065#plan`). The text a terminal shows is the CLI's, drawn in its
//! display language (PROP-067).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#plan");

use vibe_core::manifest::{RequirementScope, StepPhase};
use vibe_wire::generated::application_plan as wire;

use super::features::ImpliedRule;
use super::plan::Plan;
use crate::operations::{Recommendation, RecommendationOutcome};

/// One node of the recommendation subtree the plan prints
/// (`PROP-077#APP-RECOMMENDS-MECHANICS`).
fn recommendation_to_wire(line: &Recommendation) -> wire::Recommendation {
    wire::Recommendation {
        package: line.package.clone(),
        by: line.by.clone(),
        key: line.key.clone(),
        outcome: match line.outcome {
            RecommendationOutcome::Installed => wire::RecommendationOutcome::Installed,
            RecommendationOutcome::Satisfied => wire::RecommendationOutcome::Satisfied,
            RecommendationOutcome::Skipped => wire::RecommendationOutcome::Skipped,
            RecommendationOutcome::Planned => wire::RecommendationOutcome::Planned,
        },
        reason: line.reason.clone(),
    }
}

impl Plan {
    /// The `vibe-app-plan/1` document; secret parameter values are omitted.
    pub fn to_wire(&self) -> Result<wire::ApplicationPlan, crate::AppError> {
        if self.configuration.is_some() {
            return Err(crate::ResolveError::ConfigurationUnsupported {
                package: self.package.key(),
                scope: format!(
                    "legacy plan view of installer {}; use --configuration",
                    self.option
                ),
            }
            .into());
        }
        Ok(wire::ApplicationPlan {
            source_hosting: vibe_registry::source_hosting(self.source_evidence.as_ref()),
            source_evidence: self.source_evidence.clone(),
            // The CLI aggregates contacted hostings from this evidence.
            hostings: None,
            protocol: "vibe-app-plan/1".into(),
            package: wire::Package {
                group: self.package.group.clone(),
                name: self.package.name.clone(),
                version: self.package.version.clone(),
            },
            option: self.option.clone(),
            platform: self.platform.clone(),
            arch: self.arch.clone(),
            result: if self.result.is_owned() {
                wire::Result::Owned
            } else {
                wire::Result::External
            },
            app_root: self.result.external_root().map(str::to_owned),
            profile: self.profile.clone(),
            features: self
                .declared
                .iter()
                .map(|feature| wire::Feature {
                    id: feature.id.clone(),
                    name: feature.name.clone(),
                    default: feature.default,
                    chosen: self.features.contains(&feature.id),
                    parent: feature.parent.clone(),
                    implied: self
                        .implied
                        .iter()
                        .find(|implied| implied.feature == feature.id)
                        .map(|implied| wire::Implication {
                            rule: match implied.rule {
                                ImpliedRule::Parent => wire::Rule::Parent,
                                ImpliedRule::Requires => wire::Rule::Requires,
                                ImpliedRule::Required => wire::Rule::Required,
                                ImpliedRule::ExclusiveDefault => wire::Rule::ExclusiveDefault,
                            },
                            by: implied.by.clone(),
                        }),
                })
                .collect(),
            parameters: self
                .parameters
                .iter()
                .map(|param| wire::Parameter {
                    id: param.id.clone(),
                    value: (!param.secret).then(|| param.value.clone()),
                    secret: param.secret,
                })
                .collect(),
            license: self.license.as_ref().map(|license| wire::License {
                name: license.name.clone(),
                file: license.file.clone(),
                accepted: true,
            }),
            download_bytes: self.download_bytes,
            unknown_sizes: self.unknown_sizes,
            dependencies: self
                .dependencies
                .iter()
                .map(|dependency| wire::Dependency {
                    package: dependency.package.clone(),
                    key: dependency.key.clone(),
                    scope: match dependency.scope {
                        RequirementScope::Build => wire::Scope::Build,
                        RequirementScope::Runtime => wire::Scope::Runtime,
                    },
                    step: dependency.step.clone(),
                    action: if dependency.installed {
                        wire::DependencyAction::Use
                    } else {
                        wire::DependencyAction::Install
                    },
                    hidden: dependency.hidden,
                    commands: dependency.commands.clone(),
                })
                .collect(),
            recommended: self
                .recommended
                .iter()
                .map(recommendation_to_wire)
                .collect(),
            suggested: self.suggested.clone(),
            steps: self
                .steps
                .iter()
                .map(|planned| wire::Step {
                    id: planned.step.id.clone(),
                    title: planned.step.title.clone(),
                    kind: planned.step.kind.name().into(),
                    phase: match planned.step.phase {
                        StepPhase::Prepare => wire::Phase::Prepare,
                        StepPhase::Finish => wire::Phase::Finish,
                    },
                    needs: planned.step.needs.iter().map(ToString::to_string).collect(),
                    optional: planned.step.optional,
                    action: if planned.run {
                        wire::Action::Run
                    } else {
                        wire::Action::Skip
                    },
                    reason: planned.reason.clone(),
                    note: planned.note.clone(),
                })
                .collect(),
            warnings: self.warnings.clone(),
        })
    }
}
