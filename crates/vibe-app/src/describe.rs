//! `vibe app describe` (`PROP-065#describe`): the whole choice of a package
//! as one document — the application icon, every option with its result,
//! launchers, feature tree, profiles, parameters, license and steps after
//! the sugar mapping. Nothing is resolved or run except the default
//! feature set, which is pure data.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#describe");

use vibe_core::manifest::{
    Condition, InstallerOptionDecl, ParameterType, RequirementDecl, RequirementScope,
    ShellInterpreter, StepDecl, StepKind, StepOrigin, StepPhase, result_of, steps_of,
};
use vibe_wire::generated::application_describe as wire;

use crate::installer::shell::interpreter_name;
use crate::resolve::ResolvedPackage;
use crate::steps::{FeatureRequest, resolve_features};

/// The `vibe-app-describe/1` document of a resolved package.
pub fn describe(resolved: &ResolvedPackage) -> wire::ApplicationDescribe {
    wire::ApplicationDescribe {
        protocol: "vibe-app-describe/1".into(),
        package: wire::Package {
            group: resolved.package.group.clone(),
            name: resolved.package.name.clone(),
            version: resolved.package.version.clone(),
        },
        application: wire::Application {
            id: resolved.application.id.clone(),
            portable: resolved.application.portable,
            icon: resolved
                .application
                .icon
                .as_ref()
                .map(|icon| resolved.root.join(icon).display().to_string()),
        },
        options: resolved
            .options
            .iter()
            .map(|option| describe_option(resolved, option))
            .collect(),
    }
}

fn describe_option(
    resolved: &ResolvedPackage,
    option: &InstallerOptionDecl,
) -> wire::InstallerOption {
    let result = result_of(option);
    wire::InstallerOption {
        id: option.id.clone(),
        name: option.name.clone(),
        platform: option.platform.clone(),
        platform_name: option.platform_name.clone(),
        arch: option.arch.clone(),
        arch_name: option.arch_name.clone(),
        form: if option.installation.is_some() {
            wire::Form::Installation
        } else {
            wire::Form::Steps
        },
        result: if result.is_owned() {
            wire::Result::Owned
        } else {
            wire::Result::External
        },
        app_root: result.external_root().map(str::to_owned),
        portable: result.is_owned()
            && resolved.application.portable
            && option.portable.unwrap_or(true),
        launchers: option
            .launchers
            .iter()
            .map(|launcher| wire::Launcher {
                id: launcher.id.clone(),
                name: launcher.name.clone(),
                description: launcher.description.clone(),
                executable: launcher.executable.clone(),
                args: launcher.args.clone(),
                working_directory: launcher.working_directory.clone(),
                icon: launcher.icon.clone(),
                terminal: launcher.terminal,
                default: launcher.default,
            })
            .collect(),
        public_commands: option
            .public_commands
            .iter()
            .map(|command| wire::PublicCommand {
                name: command.name.clone(),
                launcher: command.launcher.clone(),
            })
            .collect(),
        features: option
            .features
            .iter()
            .map(|feature| wire::Feature {
                id: feature.id.clone(),
                name: feature.name.clone(),
                description: feature.description.clone(),
                default: feature.default,
                parent: feature.parent.clone(),
                required: feature.required,
                requires: feature.requires.clone(),
                exclusive_group: feature.exclusive_group.clone(),
            })
            .collect(),
        default_features: resolve_features(option, None, &FeatureRequest::default(), None, None)
            .map(|resolved| resolved.selected)
            .unwrap_or_default(),
        profiles: option
            .profiles
            .iter()
            .map(|profile| wire::Profile {
                id: profile.id.clone(),
                name: profile.name.clone(),
                description: profile.description.clone(),
                features: profile.features.clone(),
                default: profile.default,
            })
            .collect(),
        parameters: option
            .parameters
            .iter()
            .map(|parameter| wire::Parameter {
                id: parameter.id.clone(),
                name: parameter.name.clone(),
                description: parameter.description.clone(),
                type_: match parameter.kind {
                    ParameterType::String => wire::ParameterType::String,
                    ParameterType::Bool => wire::ParameterType::Bool,
                    ParameterType::Integer => wire::ParameterType::Integer,
                    ParameterType::Choice => wire::ParameterType::Choice,
                    ParameterType::Path => wire::ParameterType::Path,
                },
                default: parameter.default.as_ref().map(ToString::to_string),
                required: parameter.required,
                min: parameter.min,
                max: parameter.max,
                choices: parameter
                    .choices
                    .iter()
                    .map(|choice| wire::Choice {
                        value: choice.value.clone(),
                        name: choice.name.clone(),
                    })
                    .collect(),
                must_exist: parameter.must_exist,
                secret: parameter.secret,
            })
            .collect(),
        license: option.license.as_ref().map(|license| wire::License {
            file: resolved.root.join(&license.file).display().to_string(),
            name: license.name.clone(),
        }),
        requires: requirements(&option.requires, RequirementScope::Runtime),
        steps: steps_of(option).iter().map(describe_step).collect(),
    }
}

/// Requirements with their effective scope (`PROP-065#app-dependencies`).
fn requirements(declared: &[RequirementDecl], default: RequirementScope) -> Vec<wire::Requirement> {
    declared
        .iter()
        .map(|requirement| wire::Requirement {
            package: requirement.package().into(),
            scope: match requirement.scope_or(default) {
                RequirementScope::Build => wire::Scope::Build,
                RequirementScope::Runtime => wire::Scope::Runtime,
            },
        })
        .collect()
}

fn condition(condition: &Condition) -> wire::Condition {
    let mut row = wire::Condition {
        feature: None,
        param: None,
        param_equals: None,
        command_present: None,
        file_present: None,
        env_present: None,
        agent_configured: None,
        offline: None,
    };
    match condition {
        Condition::Feature(id) => row.feature = Some(id.clone()),
        Condition::Param(id) => row.param = Some(id.clone()),
        Condition::ParamEquals(test) => {
            row.param_equals = Some(wire::ParamEquals {
                id: test.id.clone(),
                value: test.value.clone(),
            });
        }
        Condition::CommandPresent(name) => row.command_present = Some(name.clone()),
        Condition::FilePresent(path) => row.file_present = Some(path.clone()),
        Condition::EnvPresent(name) => row.env_present = Some(name.clone()),
        Condition::AgentConfigured(value) => row.agent_configured = Some(*value),
        Condition::Offline(value) => row.offline = Some(*value),
    }
    row
}

/// The serde spelling of a unit enum value (`zip`, `msi`, `inherit`).
fn to_value(value: &impl serde::Serialize) -> Option<String> {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
}

fn describe_step(step: &StepDecl) -> wire::Step {
    let text = |path: &std::path::Path| Some(path.to_string_lossy().into_owned());
    let interpreter = |value: &Option<ShellInterpreter>| value.map(|v| interpreter_name(v).into());
    let mut row = wire::Step {
        id: step.id.clone(),
        title: step.title.clone(),
        kind: step.kind.name().into(),
        description: step.description.clone(),
        phase: match step.phase {
            StepPhase::Prepare => wire::Phase::Prepare,
            StepPhase::Finish => wire::Phase::Finish,
        },
        optional: step.optional,
        hook: match step.origin {
            StepOrigin::Authored => None,
            StepOrigin::PreInstall => Some("pre_install".into()),
            StepOrigin::PostInstall => Some("post_install".into()),
        },
        when: step.when.iter().map(condition).collect(),
        unless: step.unless.iter().map(condition).collect(),
        needs: step.needs.iter().map(ToString::to_string).collect(),
        sources: Vec::new(),
        format: None,
        source: None,
        sha256: None,
        size: None,
        payload_root: None,
        interpreter: None,
        script: None,
        command: None,
        force_install: None,
        update: None,
        uninstall: None,
        force_uninstall: None,
        arguments: Vec::new(),
        force_arguments: None,
        from: None,
        to: None,
        prompt: None,
        prompt_file: None,
        mode: None,
        required: None,
        set: Vec::new(),
        unset: Vec::new(),
        path_prepend: Vec::new(),
        requires: requirements(&step.requires, RequirementScope::Build),
    };
    match &step.kind {
        StepKind::Env {
            set,
            unset,
            path_prepend,
        } => {
            row.set = set
                .iter()
                .map(|(name, value)| wire::EnvSet {
                    name: name.clone(),
                    value: value.clone(),
                })
                .collect();
            row.unset = unset.clone();
            row.path_prepend = path_prepend.clone();
        }
        StepKind::Sources { sources } => {
            row.sources = sources
                .iter()
                .map(|source| wire::SourceInput {
                    name: source.name.clone(),
                    url: source.url.clone(),
                    path: source.path.as_deref().and_then(text),
                    sha256: source.sha256.clone(),
                    size: source.size,
                    extract: source.extract,
                })
                .collect();
        }
        StepKind::Archive {
            format,
            source,
            sha256,
            size,
            payload_root,
        } => {
            row.format = to_value(format);
            row.source = Some(source.clone());
            row.sha256 = Some(sha256.clone());
            row.size = *size;
            row.payload_root = payload_root.as_deref().and_then(text);
        }
        StepKind::Recipe {
            interpreter: chosen,
            script,
        } => {
            row.interpreter = interpreter(chosen);
            row.script = text(script);
        }
        StepKind::Shell {
            command,
            interpreter: chosen,
            force_install,
            update,
            uninstall,
            force_uninstall,
        } => {
            row.command = Some(command.clone());
            row.interpreter = interpreter(chosen);
            row.force_install = force_install.clone();
            row.update = update.clone();
            row.uninstall = uninstall.clone();
            row.force_uninstall = force_uninstall.clone();
        }
        StepKind::Native {
            format,
            source,
            sha256,
            arguments,
            force_arguments,
            uninstall,
            force_uninstall,
        } => {
            row.format = to_value(format);
            row.source = Some(source.clone());
            row.sha256 = Some(sha256.clone());
            row.arguments = arguments.clone();
            row.force_arguments = force_arguments.clone();
            row.uninstall = uninstall.clone();
            row.force_uninstall = force_uninstall.clone();
        }
        StepKind::Copy { from, to } => {
            row.from = text(from);
            row.to = Some(to.clone());
        }
        StepKind::Agent {
            prompt,
            prompt_file,
            mode,
            required,
        } => {
            row.prompt = prompt.clone();
            row.prompt_file = prompt_file.as_deref().and_then(text);
            row.mode = mode.as_ref().and_then(to_value);
            row.required = Some(*required);
        }
    }
    row
}
