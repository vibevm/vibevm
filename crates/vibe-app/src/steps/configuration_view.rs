//! The common machine plan: execution fields and canonical configuration evaluation
//! (`PROP-065#plan`). The text a terminal shows is the CLI's, drawn in its
//! display language (PROP-067).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#plan");

use vibe_core::manifest::{RequirementScope, StepPhase};
use vibe_wire::generated::configuration::e1::app_plan as wire;

use super::plan::Plan;

impl Plan {
    /// The common app-plan document; secret values are absent from its evaluation.
    pub fn to_configuration_wire(&self) -> Result<wire::AppPlan, crate::AppError> {
        let configuration = self
            .configuration
            .as_ref()
            .ok_or_else(|| crate::configuration::ConfigurationError::Invalid {
                option: self.option.clone(),
                reason: "this installer uses the legacy plan vocabulary".into(),
            })?
            .evaluation
            .clone();
        Ok(wire::AppPlan {
            configuration,
            protocol: "vibe-app-configuration-plan/1".into(),
            package: wire::Package {
                group: self.package.group.clone(),
                name: self.package.name.clone(),
                version: self.package.version.clone(),
            },
            option: self.option.clone(),
            platform: self.platform.clone(),
            arch: self.arch.clone(),
            execution: Some(wire::Execution {
                result: if self.result.is_owned() {
                    wire::Result::Owned
                } else {
                    wire::Result::External
                },
                app_root: self.result.external_root().map(str::to_owned),
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
                steps: self
                    .steps
                    .iter()
                    .map(|planned| wire::Step {
                        agent_mode: planned.agent_mode.map(|m| match m {
                            crate::RunMode::Interactive => {
                                vibe_wire::generated::shared::ApplicationAgentMode::Interactive
                            }
                            crate::RunMode::Unattended => {
                                vibe_wire::generated::shared::ApplicationAgentMode::Unattended
                            }
                        }),
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
            }),
        })
    }
}
