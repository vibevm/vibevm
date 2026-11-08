//! `vibe app describe` (`PROP-065#describe`): the whole choice of a package
//! as one document — the application icon, every option with its result,
//! launchers, feature tree, profiles, parameters, license and steps after
//! the sugar mapping. Nothing is resolved or run except the default
//! feature set, which is pure data.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#describe");

use std::path::Path;

use vibe_core::manifest::{
    Condition, IconDecl, InstallerOptionDecl, LauncherDecl, LauncherKind, ParameterType,
    RequirementDecl, RequirementScope, ShellInterpreter, StepDecl, StepKind, StepOrigin, StepPhase,
    result_of, steps_of,
};
use vibe_wire::generated::application_describe as wire;

use crate::icons::{ResolvedIcon, wire_shape};
use crate::installer::shell::interpreter_name;
use crate::resolve::ResolvedPackage;
use crate::steps::{FeatureRequest, resolve_features};

/// The unchanged legacy `vibe-app-describe/1` view, or an explicit refusal
/// when a common option definition cannot be represented by that document.
///
/// ```
/// use vibe_app::{describe, from_manifest, PackageCoordinate};
/// use vibe_core::manifest::Manifest;
/// let manifest = Manifest::parse_str("[package]\ngroup='org.example'\nname='demo'\nkind='app'\nversion='1.0.0'\n[application]\nid='demo'")?;
/// let resolved = from_manifest(manifest, "/tmp/demo".into(), PackageCoordinate {
///     group: "org.example".into(), name: "demo".into(), version: "1.0.0".into()
/// })?;
/// assert_eq!(describe(&resolved)?.protocol, "vibe-app-describe/1");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn describe(
    resolved: &ResolvedPackage,
) -> Result<wire::ApplicationDescribe, crate::error::AppError> {
    if resolved.configuration.is_some() {
        return Err(crate::ResolveError::ConfigurationUnsupported {
            package: resolved.package.key(),
            scope: "legacy package description; use --configuration".into(),
        }
        .into());
    }
    if let Some(option) = resolved
        .options
        .iter()
        .find(|option| option.configuration.is_some())
    {
        return Err(crate::error::ResolveError::ConfigurationUnsupported {
            package: resolved.package.key(),
            scope: format!("installer {}", option.id),
        }
        .into());
    }
    Ok(wire::ApplicationDescribe {
        source_hosting: vibe_registry::source_hosting(resolved.source_evidence.as_ref()),
        source_evidence: resolved.source_evidence.clone(),
        installed_source: None,
        installed_hosting: None,
        protocol: "vibe-app-describe/1".into(),
        package: wire::Package {
            group: resolved.package.group.clone(),
            name: resolved.package.name.clone(),
            version: resolved.package.version.clone(),
        },
        application: describe_application(resolved),
        options: resolved
            .options
            .iter()
            .map(|option| describe_option(resolved, option))
            .collect(),
    })
}

pub(crate) fn describe_option(
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
        launchers_from: option.launchers_from.clone(),
        launchers: option
            .launchers
            .iter()
            .map(|launcher| describe_launcher(&resolved.root, launcher))
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

/// An icon set as written, each package-relative file resolved to its
/// absolute path in the package tree when it exists; a template naming the
/// installed payload has no path before installation (`PROP-059#menu`).
fn package_icon(root: &Path, icon: &IconDecl) -> Option<wire::IconSet> {
    let resolved = ResolvedIcon::resolve(icon, |template| {
        if template.starts_with('$') {
            Err("the file is in the installed payload".into())
        } else {
            Ok(root.join(template))
        }
    });
    wire_shape(&resolved)
}

/// One launcher row as declared (`PROP-059#launch-points`, `#menu`,
/// `#desktop-entries`).
fn describe_launcher(root: &Path, launcher: &LauncherDecl) -> wire::Launcher {
    wire::Launcher {
        id: launcher.id.clone(),
        name: launcher.name.clone(),
        description: launcher.description.clone(),
        executable: launcher.executable.clone(),
        args: launcher.args.clone(),
        working_directory: launcher.working_directory.clone(),
        icon: match &launcher.icon {
            Some(IconDecl::Path(icon)) => Some(icon.clone()),
            _ => None,
        },
        icon_set: launcher
            .icon
            .as_ref()
            .and_then(|icon| package_icon(root, icon)),
        terminal: launcher.is_terminal(),
        default: launcher.default,
        kind: launcher.kind.map(|kind| match kind {
            LauncherKind::Gui => wire::LauncherKind::Gui,
            LauncherKind::Tui => wire::LauncherKind::Tui,
            LauncherKind::Command => wire::LauncherKind::Command,
        }),
        menu: launcher.shows_in_menu(),
        source: wire::LauncherSource::Authored,
        categories: launcher.categories.clone(),
        keywords: launcher.keywords.clone(),
        generic_name: launcher.generic_name.clone(),
        window_class: launcher.window_class.clone(),
        mime_types: launcher.mime_types.clone(),
        names: launcher.names.clone(),
        descriptions: launcher.descriptions.clone(),
        actions: launcher
            .actions
            .iter()
            .map(|action| wire::LauncherAction {
                id: action.id.clone(),
                name: action.name.clone(),
                args: action.args.clone(),
                icon_set: action
                    .icon
                    .as_ref()
                    .and_then(|icon| package_icon(root, icon)),
            })
            .collect(),
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
                    drop: source.drop.clone(),
                    file: source.file.clone(),
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

pub(crate) fn describe_application(resolved: &ResolvedPackage) -> wire::Application {
    wire::Application {
        id: resolved.application.id.clone(),
        portable: resolved.application.portable,
        icon: match &resolved.application.icon {
            Some(IconDecl::Path(icon)) => Some(resolved.root.join(icon).display().to_string()),
            _ => None,
        },
        icon_set: resolved
            .application
            .icon
            .as_ref()
            .and_then(|icon| package_icon(&resolved.root, icon)),
        menu_group: resolved.application.menu_group.clone(),
        categories: resolved.application.categories.clone(),
        keywords: resolved.application.keywords.clone(),
    }
}
