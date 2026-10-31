//! The method forms as short spellings of step lists (`PROP-065#sugar`):
//! the installer engine only ever sees the steps this module returns.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#sugar");

use super::{
    ArchiveFormatDecl, InstallerMethod, InstallerOptionDecl, NativeFormatDecl, ResultDecl,
    ShellInterpreter, StepDecl, StepKind, StepOrigin, StepPhase,
};

/// The steps an option runs, in order: its authored `[[steps]]`, or the
/// mapping of its `installation` method, with `pre_install` as a leading
/// `prepare` shell step and `post_install` as a leading `finish` one.
///
/// ```
/// use vibe_core::manifest::{Manifest, steps_of};
///
/// let manifest = Manifest::parse_str(r#"
/// [package]
/// name = "demo"
/// group = "org.example"
/// kind = "app"
/// version = "1.0.0"
/// [application]
/// id = "demo"
/// [[installer_options]]
/// id = "src"
/// name = "Source"
/// platform = "linux"
/// platform_name = "Linux"
/// arch = "x86_64"
/// [installer_options.installation]
/// type = "source_build"
/// recipe = { script = "build.sh" }
/// "#).unwrap();
/// let kinds: Vec<&str> = steps_of(&manifest.installer_options[0])
///     .iter()
///     .map(|step| step.kind.name())
///     .collect();
/// assert_eq!(kinds, ["sources", "recipe"]);
/// ```
pub fn steps_of(option: &InstallerOptionDecl) -> Vec<StepDecl> {
    let hook_interpreter = match &option.installation {
        Some(InstallerMethod::Shell { interpreter, .. }) => *interpreter,
        _ => None,
    };
    let mut steps = Vec::new();
    if let Some(command) = &option.pre_install {
        steps.push(hook(
            "pre_install",
            command,
            hook_interpreter,
            StepPhase::Prepare,
        ));
    }
    match &option.installation {
        Some(method) => method_steps(method, &mut steps),
        // An env step always runs in the finish phase (`PROP-065#env-step`).
        None => steps.extend(option.steps.iter().cloned().map(|mut step| {
            if matches!(step.kind, StepKind::Env { .. }) {
                step.phase = StepPhase::Finish;
            }
            step
        })),
    }
    if let Some(command) = &option.post_install {
        // A leading finish step: before every authored finish step.
        let position = steps
            .iter()
            .position(|step| step.phase == StepPhase::Finish)
            .unwrap_or(steps.len());
        steps.insert(
            position,
            hook("post_install", command, hook_interpreter, StepPhase::Finish),
        );
    }
    steps
}

fn method_steps(method: &InstallerMethod, steps: &mut Vec<StepDecl>) {
    match method {
        InstallerMethod::Zip {
            source,
            sha256,
            payload_root,
        } => steps.push(archive(
            ArchiveFormatDecl::Zip,
            source,
            sha256,
            payload_root,
        )),
        InstallerMethod::TarGz {
            source,
            sha256,
            payload_root,
        } => steps.push(archive(
            ArchiveFormatDecl::TarGz,
            source,
            sha256,
            payload_root,
        )),
        InstallerMethod::Shell {
            install,
            force_install,
            update,
            uninstall,
            force_uninstall,
            interpreter,
            ..
        } => steps.push(StepDecl::new(
            "install",
            "Run the shell installer",
            StepKind::Shell {
                command: install.clone(),
                interpreter: *interpreter,
                force_install: Some(force_install.clone()),
                update: update.clone(),
                uninstall: Some(uninstall.clone()),
                force_uninstall: Some(force_uninstall.clone()),
            },
        )),
        InstallerMethod::Exe {
            source,
            sha256,
            arguments,
            force_arguments,
            uninstall,
            force_uninstall,
            ..
        }
        | InstallerMethod::Msi {
            source,
            sha256,
            arguments,
            force_arguments,
            uninstall,
            force_uninstall,
            ..
        } => steps.push(StepDecl::new(
            "install",
            "Run the native installer",
            StepKind::Native {
                format: if matches!(method, InstallerMethod::Exe { .. }) {
                    NativeFormatDecl::Exe
                } else {
                    NativeFormatDecl::Msi
                },
                source: source.clone(),
                sha256: sha256.clone(),
                arguments: arguments.clone(),
                force_arguments: force_arguments.clone(),
                uninstall: uninstall.clone(),
                force_uninstall: force_uninstall.clone(),
            },
        )),
        InstallerMethod::SourceBuild { sources, recipe } => {
            steps.push(StepDecl::new(
                "sources",
                "Fetch and lay out the sources",
                StepKind::Sources {
                    sources: sources.clone(),
                },
            ));
            steps.push(StepDecl::new(
                "recipe",
                "Run the build recipe",
                StepKind::Recipe {
                    interpreter: recipe.interpreter,
                    script: recipe.script.clone(),
                },
            ));
        }
    }
}

/// The result an option declares: the method's ownership for the short
/// spelling, otherwise `result` (owned when absent).
pub fn result_of(option: &InstallerOptionDecl) -> ResultDecl {
    match &option.installation {
        Some(
            InstallerMethod::Shell { app_root, .. }
            | InstallerMethod::Exe { app_root, .. }
            | InstallerMethod::Msi { app_root, .. },
        ) => ResultDecl::external(app_root),
        Some(_) => ResultDecl::owned(),
        None => option.result.clone().unwrap_or_else(ResultDecl::owned),
    }
}

fn archive(
    format: ArchiveFormatDecl,
    source: &str,
    sha256: &str,
    payload_root: &Option<std::path::PathBuf>,
) -> StepDecl {
    StepDecl::new(
        "archive",
        "Fetch, verify and extract the archive",
        StepKind::Archive {
            format,
            source: source.into(),
            sha256: sha256.into(),
            size: None,
            payload_root: payload_root.clone(),
        },
    )
}

fn hook(
    id: &str,
    command: &str,
    interpreter: Option<ShellInterpreter>,
    phase: StepPhase,
) -> StepDecl {
    let mut step = StepDecl::new(
        id,
        &format!("Run the {id} hook"),
        StepKind::Shell {
            command: command.into(),
            interpreter,
            force_install: None,
            update: None,
            uninstall: None,
            force_uninstall: None,
        },
    );
    step.phase = phase;
    step.origin = if phase == StepPhase::Prepare {
        StepOrigin::PreInstall
    } else {
        StepOrigin::PostInstall
    };
    step
}
