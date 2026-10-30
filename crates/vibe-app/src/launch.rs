//! Launch points: the one-pass template grammar, containment under
//! `app_root`, the `vibe app launchers` listing and native execution.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-paths");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-operations");

use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

use specmark::spec;
use vibe_core::manifest::LauncherDecl;
use vibe_wire::generated::application_launchers as wire;

use crate::error::LaunchError;
use crate::index::{Installation, Location};
use crate::resolve::strip_verbatim;
use crate::storage::{Availability, Storage};

/// Values a template may reference.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-paths")]
pub struct TemplateVars {
    pub app_root: PathBuf,
    pub vibe_home: PathBuf,
    pub project_root: Option<PathBuf>,
}

const PROJECT_REQUIRED: &str = "project_required";

/// Expand `$app_root`, `$app_bin`, `$project_root`, `$vibe_home` (braced or
/// not) and `$$` in one pass; never shell evaluation.
///
/// ```
/// use std::path::PathBuf;
/// use vibe_app::{TemplateVars, expand_template};
///
/// let vars = TemplateVars {
///     app_root: PathBuf::from("/apps/demo"),
///     vibe_home: PathBuf::from("/home/u/.vibe"),
///     project_root: None,
/// };
/// assert_eq!(expand_template("${app_bin}/demo", &vars).unwrap(), "/apps/demo/bin/demo");
/// assert_eq!(expand_template("cost: $$5", &vars).unwrap(), "cost: $5");
/// assert_eq!(expand_template("$project_root", &vars).unwrap_err(), "project_required");
/// assert!(expand_template("$home", &vars).is_err());
/// ```
pub fn expand_template(template: &str, vars: &TemplateVars) -> Result<String, String> {
    let mut result = String::new();
    let mut chars = template.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '$' {
            result.push(ch);
            continue;
        }
        if chars.peek() == Some(&'$') {
            chars.next();
            result.push('$');
            continue;
        }
        let mut name = String::new();
        if chars.peek() == Some(&'{') {
            chars.next();
            let mut closed = false;
            for next in chars.by_ref() {
                if next == '}' {
                    closed = true;
                    break;
                }
                name.push(next);
            }
            if !closed {
                return Err("unterminated template variable".into());
            }
        } else {
            while let Some(next) = chars.peek().copied() {
                if !next.is_ascii_alphanumeric() && next != '_' {
                    break;
                }
                chars.next();
                name.push(next);
            }
        }
        let value = match name.as_str() {
            "app_root" => vars.app_root.clone(),
            "app_bin" => vars.app_root.join("bin"),
            "vibe_home" => vars.vibe_home.clone(),
            "project_root" => vars.project_root.clone().ok_or(PROJECT_REQUIRED)?,
            other => return Err(format!("unknown template variable `${other}`")),
        };
        result.push_str(&value.to_string_lossy());
    }
    Ok(result)
}

/// The `app_root` of an installation, derived now: an owned root from its
/// library and directory, an external root as recorded.
pub fn app_root(installation: &Installation, storage: &Storage) -> Result<PathBuf, String> {
    match &installation.location {
        Location::External { app_root } => Ok(app_root.clone()),
        Location::Owned {
            library, directory, ..
        } => match storage.availability(library) {
            Availability::Available => storage
                .path_of(library)
                .map(|root| root.join(directory))
                .ok_or_else(|| format!("library `{library}` is not configured")),
            Availability::Unavailable(reason) => Err(reason),
        },
    }
}

/// A launch point resolved against the installed payload.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-paths")]
pub struct ResolvedLauncher {
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub working_directory: Option<PathBuf>,
    pub icon: Option<PathBuf>,
    pub icon_diagnostic: Option<String>,
}

/// Resolve one launcher; `Err("project_required")` marks a point that needs
/// a project while the others stay runnable.
pub fn resolve(launcher: &LauncherDecl, vars: &TemplateVars) -> Result<ResolvedLauncher, String> {
    let root = canonical(&vars.app_root, "app_root", true)?;
    let vars = TemplateVars {
        app_root: root.clone(),
        ..vars.clone()
    };
    let executable = canonical(
        &template_path(&launcher.executable, &vars)?,
        "executable",
        false,
    )?;
    if !executable.starts_with(&root) {
        return Err("executable escapes app_root".into());
    }
    runnable(&executable)?;
    let project = vars
        .project_root
        .as_deref()
        .map(|path| canonical(path, "project_root", true))
        .transpose()?;
    let working_directory = match &launcher.working_directory {
        Some(template) => {
            let directory = canonical(&template_path(template, &vars)?, "working_directory", true)?;
            if !directory.starts_with(&root)
                && !project
                    .as_ref()
                    .is_some_and(|project| directory.starts_with(project))
            {
                return Err("working_directory escapes app_root and project_root".into());
            }
            Some(directory)
        }
        None => None,
    };
    let args = launcher
        .args
        .iter()
        .map(|arg| expand_template(arg, &vars))
        .collect::<Result<Vec<_>, _>>()?;
    let (icon, icon_diagnostic) = match launcher.icon.as_deref().filter(|icon| !icon.is_empty()) {
        None => (None, None),
        Some(template) => {
            match template_path(template, &vars).and_then(|path| canonical(&path, "icon", false)) {
                Ok(path) if path.starts_with(&root) => (Some(path), None),
                Ok(_) => (None, Some("icon escapes app_root".into())),
                Err(reason) => (None, Some(reason)),
            }
        }
    };
    Ok(ResolvedLauncher {
        executable,
        args,
        working_directory,
        icon,
        icon_diagnostic,
    })
}

fn template_path(template: &str, vars: &TemplateVars) -> Result<PathBuf, String> {
    if Path::new(template).is_absolute() {
        return Err("an absolute literal path is refused; use a template variable".into());
    }
    let path = PathBuf::from(expand_template(template, vars)?);
    if path.components().any(|part| part == Component::ParentDir) {
        return Err("the path traverses with `..`".into());
    }
    Ok(if path.is_absolute() {
        path
    } else {
        vars.app_root.join(path)
    })
}

fn canonical(path: &Path, field: &str, directory: bool) -> Result<PathBuf, String> {
    let resolved = fs::canonicalize(path)
        .map(strip_verbatim)
        .map_err(|_| format!("{field} `{}` is unavailable", path.display()))?;
    if directory && !resolved.is_dir() {
        return Err(format!("{field} is not a directory"));
    }
    if !directory && !resolved.is_file() {
        return Err(format!("{field} is not a regular file"));
    }
    Ok(resolved)
}

fn runnable(executable: &Path) -> Result<(), String> {
    let extension = executable
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if extension == "exe" && !cfg!(windows) {
        return Err("a Windows executable cannot run on this platform".into());
    }
    if extension == "ps1"
        || (matches!(extension.as_str(), "cmd" | "bat") && !cfg!(windows))
        || (extension == "sh" && cfg!(windows))
    {
        return Err(format!(
            "script format `{extension}` cannot preserve literal arguments on this platform"
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(executable)
            .map_err(|error| error.to_string())?
            .permissions()
            .mode();
        if mode & 0o111 == 0 {
            return Err("the executable is not marked executable".into());
        }
    }
    Ok(())
}

/// The `vibe-app-launchers/1` listing of one installation.
pub fn listing(
    installation: &Installation,
    storage: &Storage,
    vibe_home: &Path,
    project_root: Option<&Path>,
) -> wire::ApplicationLaunchers {
    let root = app_root(installation, storage);
    let launchers = installation
        .launchers
        .iter()
        .map(|launcher| {
            let outcome = match &root {
                Ok(root) => resolve(
                    launcher,
                    &TemplateVars {
                        app_root: root.clone(),
                        vibe_home: vibe_home.to_path_buf(),
                        project_root: project_root.map(Path::to_path_buf),
                    },
                ),
                Err(reason) => Err(reason.clone()),
            };
            let (status, reason, resolved) = match outcome {
                Ok(resolved) => (wire::LauncherStatus::Available, None, Some(resolved)),
                Err(reason) if reason == PROJECT_REQUIRED => {
                    (wire::LauncherStatus::RequiresProject, Some(reason), None)
                }
                Err(reason) => (wire::LauncherStatus::Unavailable, Some(reason), None),
            };
            let text = |path: &PathBuf| path.display().to_string();
            wire::Launcher {
                id: launcher.id.clone(),
                name: launcher.name.clone(),
                description: launcher.description.clone(),
                default: launcher.default,
                terminal: launcher.terminal,
                executable: wire::PathValue {
                    template: Some(launcher.executable.clone()),
                    base: "app_root".into(),
                    resolved: resolved.as_ref().map(|value| text(&value.executable)),
                },
                args: wire::ArgsValue {
                    template: launcher.args.clone(),
                    resolved: resolved.as_ref().map(|value| value.args.clone()),
                },
                working_directory: wire::PathValue {
                    template: launcher.working_directory.clone(),
                    base: "app_root".into(),
                    resolved: resolved
                        .as_ref()
                        .and_then(|value| value.working_directory.as_ref().map(text)),
                },
                icon: wire::IconValue {
                    template: launcher.icon.clone(),
                    resolved: resolved
                        .as_ref()
                        .and_then(|value| value.icon.as_ref().map(text)),
                    diagnostic: resolved.and_then(|value| value.icon_diagnostic),
                },
                status,
                reason,
            }
        })
        .collect();
    wire::ApplicationLaunchers {
        protocol: "vibe-app-launchers/1".into(),
        application_id: installation.slot.clone(),
        package: wire::PackageIdentity {
            group: installation.package.group.clone(),
            name: installation.package.name.clone(),
            version: installation.package.version.clone(),
        },
        default_launcher_id: installation
            .launchers
            .iter()
            .find(|launcher| launcher.default)
            .map(|launcher| launcher.id.clone()),
        app_root: root.ok().map(|path| path.display().to_string()),
        launchers,
    }
}

/// Everything one launch needs besides the installation itself.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#launcher-operations")]
pub struct LaunchRequest<'a> {
    pub launcher_id: &'a str,
    pub extra_args: &'a [OsString],
    pub vibe_home: &'a Path,
    pub project_root: Option<&'a Path>,
    pub attended_terminal: bool,
}

/// Run one launch point with an argument array and return its exit status.
pub fn run(
    installation: &Installation,
    storage: &Storage,
    request: &LaunchRequest<'_>,
) -> Result<i32, LaunchError> {
    let launcher = installation
        .launchers
        .iter()
        .find(|launcher| launcher.id == request.launcher_id)
        .ok_or_else(|| LaunchError::UnknownLauncher {
            id: request.launcher_id.into(),
        })?;
    let unavailable = |reason: String| LaunchError::Unavailable { reason };
    let root = app_root(installation, storage).map_err(unavailable)?;
    let resolved = resolve(
        launcher,
        &TemplateVars {
            app_root: root,
            vibe_home: request.vibe_home.to_path_buf(),
            project_root: request.project_root.map(Path::to_path_buf),
        },
    )
    .map_err(unavailable)?;
    if launcher.terminal && !request.attended_terminal {
        return Err(unavailable("terminal_required".into()));
    }
    let mut command = Command::new(&resolved.executable);
    command
        .args(&resolved.args)
        .args(request.extra_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    if let Some(directory) = &resolved.working_directory {
        command.current_dir(directory);
    }
    let status = command.status().map_err(|error| LaunchError::Spawn {
        id: launcher.id.clone(),
        reason: error.to_string(),
    })?;
    Ok(exit_code(status))
}

/// The native exit status; a POSIX signal maps to `128 + signal`.
pub fn exit_code(status: ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return 128 + signal;
        }
    }
    1
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn launcher(executable: &str) -> LauncherDecl {
        LauncherDecl {
            id: "main".into(),
            name: "Main".into(),
            description: None,
            executable: executable.into(),
            args: vec!["${app_root}".into(), "$$x".into()],
            working_directory: Some(".".into()),
            icon: Some("missing.png".into()),
            terminal: false,
            default: true,
        }
    }

    fn vars(root: &Path) -> TemplateVars {
        TemplateVars {
            app_root: root.to_path_buf(),
            vibe_home: root.join("home"),
            project_root: None,
        }
    }

    #[test]
    fn resolution_stays_inside_app_root() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(dir.path()).unwrap();
        fs::create_dir_all(root.join("bin")).unwrap();
        fs::write(root.join("bin/tool"), "#!/bin/sh\n").unwrap();
        fs::set_permissions(root.join("bin/tool"), fs::Permissions::from_mode(0o755)).unwrap();
        let resolved = resolve(&launcher("$app_bin/tool"), &vars(&root)).unwrap();
        assert_eq!(resolved.executable, root.join("bin/tool"));
        assert_eq!(resolved.args, [root.display().to_string(), "$x".into()]);
        assert_eq!(resolved.working_directory.as_deref(), Some(root.as_path()));
        assert!(resolved.icon_diagnostic.is_some());
        assert!(resolve(&launcher("bin/tool"), &vars(&root)).is_ok());
        assert!(resolve(&launcher("/bin/sh"), &vars(&root)).is_err());
        assert!(resolve(&launcher("bin/../bin/tool"), &vars(&root)).is_err());
        let escape = resolve(&launcher("$vibe_home/tool"), &vars(&root));
        assert!(escape.is_err());
        let mut needs_project = launcher("bin/tool");
        needs_project.working_directory = Some("$project_root".into());
        assert_eq!(
            resolve(&needs_project, &vars(&root)).unwrap_err(),
            PROJECT_REQUIRED
        );
        fs::set_permissions(root.join("bin/tool"), fs::Permissions::from_mode(0o644)).unwrap();
        assert!(resolve(&launcher("bin/tool"), &vars(&root)).is_err());
    }
}
