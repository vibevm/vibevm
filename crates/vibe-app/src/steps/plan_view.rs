//! The two views of a plan: the `vibe-app-plan/1` document and the text a
//! terminal shows before the first step runs (`PROP-065#plan`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#plan");

use vibe_core::manifest::{RequirementScope, StepPhase};
use vibe_wire::generated::application_plan as wire;

use super::features::ImpliedRule;
use super::plan::{Plan, PlannedDependency};

impl Plan {
    /// The `vibe-app-plan/1` document; secret parameter values are omitted.
    pub fn to_wire(&self) -> wire::ApplicationPlan {
        wire::ApplicationPlan {
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
                })
                .collect(),
            warnings: self.warnings.clone(),
        }
    }

    /// The plan as text: the package, its fields aligned, then one
    /// numbered step per block with its title, reason, needs and
    /// requirements on indented lines.
    ///
    /// ```
    /// # use vibe_app::{Plan, PackageCoordinate};
    /// # use vibe_core::manifest::ResultDecl;
    /// let plan = Plan {
    ///     package: PackageCoordinate { group: "org.example".into(), name: "tool".into(), version: "1.0.0".into() },
    ///     option: "src".into(), platform: "linux".into(), arch: "x86_64".into(),
    ///     result: ResultDecl::owned(), declared: vec![], profile: None, features: vec![],
    ///     implied: vec![], parameters: vec![], license: None, download_bytes: 0, unknown_sizes: 0,
    ///     dependencies: vec![], steps: vec![], warnings: vec![],
    /// };
    /// assert!(plan.render().starts_with("plan for org.example/tool@1.0.0"));
    /// ```
    pub fn render(&self) -> String {
        let result = match self.result.external_root() {
            Some(root) => format!("external `{root}`"),
            None => "owned".into(),
        };
        let mut text = format!("plan for {}\n", self.package.key());
        let field = |text: &mut String, name: &str, value: &str| {
            text.push_str(&format!("  {name:<10} {value}\n"));
        };
        field(
            &mut text,
            "installer",
            &format!(
                "{} ({}/{}), result {result}",
                self.option, self.platform, self.arch
            ),
        );
        if let Some(license) = &self.license {
            field(
                &mut text,
                "license",
                &format!("{} (accepted)", license.name),
            );
        }
        if let Some(profile) = &self.profile {
            field(&mut text, "profile", profile);
        }
        if !self.declared.is_empty() {
            let features: Vec<String> = self
                .declared
                .iter()
                .map(|feature| {
                    let mark = if self.features.contains(&feature.id) {
                        "x"
                    } else {
                        " "
                    };
                    let why = self
                        .implied
                        .iter()
                        .find(|implied| implied.feature == feature.id)
                        .map(|implied| match &implied.by {
                            Some(by) => format!(" ({} of {by})", implied.rule.as_str()),
                            None => format!(" ({})", implied.rule.as_str()),
                        })
                        .unwrap_or_default();
                    format!("[{mark}] {}{why}", feature.id)
                })
                .collect();
            field(&mut text, "features", &features.join("  "));
        }
        for param in &self.parameters {
            let value = if param.secret {
                "(secret)"
            } else {
                &param.value
            };
            field(&mut text, "parameter", &format!("{} = {value}", param.id));
        }
        if self.download_bytes > 0 || self.unknown_sizes > 0 {
            let unknown = match self.unknown_sizes {
                0 => String::new(),
                count => format!(" plus {count} input(s) of unknown size"),
            };
            field(
                &mut text,
                "download",
                &format!("{} bytes{unknown}", self.download_bytes),
            );
        }
        for dependency in self.dependencies.iter().filter(|d| d.step.is_none()) {
            field(&mut text, "requires", &dependency_text(dependency));
        }
        if !self.steps.is_empty() {
            text.push_str("  steps\n");
        }
        for (index, planned) in self.steps.iter().enumerate() {
            let step = &planned.step;
            let action = if planned.run { "run " } else { "skip" };
            let phase = if step.phase == StepPhase::Finish {
                " [finish]"
            } else {
                ""
            };
            text.push_str(&format!(
                "    {:>2}. {action}  {} ({}){phase}\n",
                index + 1,
                step.id,
                step.kind.name()
            ));
            let detail = |text: &mut String, line: &str| {
                text.push_str(&format!("              {line}\n"));
            };
            detail(&mut text, &step.title);
            if let Some(reason) = &planned.reason {
                detail(&mut text, &format!("skipped: {reason}"));
            }
            if !step.needs.is_empty() {
                let needs: Vec<String> = step.needs.iter().map(ToString::to_string).collect();
                detail(&mut text, &format!("needs {}", needs.join(", ")));
            }
            for dependency in self
                .dependencies
                .iter()
                .filter(|dependency| dependency.step.as_deref() == Some(step.id.as_str()))
            {
                detail(
                    &mut text,
                    &format!("requires {}", dependency_text(dependency)),
                );
            }
        }
        for warning in &self.warnings {
            text.push_str(&format!("warning: {warning}\n"));
        }
        text
    }
}

/// One requirement of the text plan: `install|use <key> (<scope>[, hidden])
/// for \`<requirement>\``, shown under the step that needs it or among the
/// plan's fields when no step does (`PROP-065#app-dependencies`).
fn dependency_text(dependency: &PlannedDependency) -> String {
    let action = if dependency.installed {
        "use"
    } else {
        "install"
    };
    let exposure = if dependency.hidden { ", hidden" } else { "" };
    format!(
        "{action} {} ({}{exposure}) for `{}`",
        dependency.key,
        dependency.scope.as_str(),
        dependency.package
    )
}
