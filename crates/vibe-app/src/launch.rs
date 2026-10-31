//! Launch points: the one-pass template grammar, containment under
//! `app_root`, the `vibe app launchers` listing and native execution.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-paths");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-operations");

use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

use specmark::spec;
use vibe_core::manifest::{ARG_PLACEHOLDERS, IconDecl, LauncherDecl, LauncherKind};

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

pub(crate) const PROJECT_REQUIRED: &str = "project_required";

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
                .map(|root| library_path(&root, directory))
                .ok_or_else(|| format!("library `{library}` is not configured")),
            Availability::Unavailable(reason) => Err(reason),
        },
    }
}

/// The path of a library-relative directory (`apps/<slot>/<key dir>`, `/`
/// separated as the index records it) under a library root, joined one
/// component at a time so every platform prints native separators.
pub(crate) fn library_path(root: &Path, directory: &str) -> PathBuf {
    directory
        .split('/')
        .filter(|part| !part.is_empty())
        .fold(root.to_path_buf(), |path, part| path.join(part))
}

/// Why a derived `app_root` is not ready, or `None` when it is a directory.
pub(crate) fn unavailable_reason(root: &Result<PathBuf, String>) -> Option<String> {
    match root {
        Ok(root) if root.is_dir() => None,
        Ok(root) => Some(format!("app_root `{}` is missing", root.display())),
        Err(reason) => Some(reason.clone()),
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
    let short_form = match &launcher.icon {
        Some(IconDecl::Path(template)) if !template.is_empty() => Some(template.as_str()),
        _ => None,
    };
    let (icon, icon_diagnostic) = match short_form {
        None => (None, None),
        Some(template) => match locate_icon(template, &vars) {
            Ok(path) => (Some(path), None),
            Err(reason) => (None, Some(reason)),
        },
    };
    Ok(ResolvedLauncher {
        executable,
        args,
        working_directory,
        icon,
        icon_diagnostic,
    })
}

/// One icon file of a launcher under the canonical `app_root` of `vars`:
/// the file exists and stays inside the root.
pub(crate) fn locate_icon(template: &str, vars: &TemplateVars) -> Result<PathBuf, String> {
    let root = canonical(&vars.app_root, "app_root", true)?;
    match template_path(template, vars).and_then(|path| canonical(&path, "icon", false)) {
        Ok(path) if path.starts_with(&root) => Ok(path),
        Ok(_) => Err("icon escapes app_root".into()),
        Err(reason) => Err(reason),
    }
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
    // A full-screen terminal program needs the caller's terminal; a
    // `command` reads arguments and prints, so a pipe serves it as well.
    if launcher.effective_kind() == LauncherKind::Tui && !request.attended_terminal {
        return Err(unavailable("terminal_required".into()));
    }
    let mut command = Command::new(&resolved.executable);
    command
        .args(with_operands(&resolved.args, request.extra_args))
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

/// The argument array of one launch: `{file}` / `{url}` take the first
/// extra argument, `{files}` / `{urls}` all of them, an unfilled placeholder
/// argument is dropped, and extra arguments no placeholder took follow.
///
/// ```
/// use std::ffi::OsString;
/// use vibe_app::launch_arguments;
///
/// let extra = [OsString::from("a.txt"), OsString::from("b.txt")];
/// let args = ["--new".to_owned(), "{files}".to_owned()];
/// assert_eq!(launch_arguments(&args, &extra), ["--new", "a.txt", "b.txt"]);
/// assert_eq!(launch_arguments(&args, &[]), ["--new"]);
/// assert_eq!(launch_arguments(&["--x".to_owned()], &extra), ["--x", "a.txt", "b.txt"]);
/// ```
pub fn launch_arguments(args: &[String], extra: &[OsString]) -> Vec<OsString> {
    with_operands(args, extra)
}

fn with_operands(args: &[String], extra: &[OsString]) -> Vec<OsString> {
    let mut consumed = false;
    let mut out = Vec::new();
    for arg in args {
        let single = ["{file}", "{url}"]
            .into_iter()
            .find(|placeholder| arg.contains(placeholder));
        if arg == "{files}" || arg == "{urls}" {
            out.extend(extra.iter().cloned());
            consumed = true;
        } else if let Some(placeholder) = single {
            consumed = true;
            match extra.first() {
                Some(first) => out.push(OsString::from(
                    arg.replace(placeholder, &first.to_string_lossy()),
                )),
                None if arg == placeholder => {}
                None => out.push(OsString::from(arg.replace(placeholder, ""))),
            }
        } else if ARG_PLACEHOLDERS.contains(&arg.as_str()) {
            consumed = true;
        } else {
            out.push(OsString::from(arg));
        }
    }
    if !consumed {
        out.extend(extra.iter().cloned());
    }
    out
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

#[path = "launch/listing.rs"]
mod listing_form;

pub(crate) use listing_form::application_icon;
pub use listing_form::{icon_choice, listing, listing_all};

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn launcher(executable: &str) -> LauncherDecl {
        LauncherDecl {
            id: "main".into(),
            name: "Main".into(),
            executable: executable.into(),
            args: vec!["${app_root}".into(), "$$x".into()],
            working_directory: Some(".".into()),
            icon: Some(IconDecl::Path("missing.png".into())),
            kind: Some(LauncherKind::Gui),
            default: true,
            ..LauncherDecl::default()
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

#[cfg(test)]
mod library_path_tests {
    use std::path::{MAIN_SEPARATOR_STR, Path};

    use super::library_path;

    #[test]
    fn a_library_relative_directory_joins_with_native_separators() {
        let root = Path::new("library");
        let joined = library_path(root, "apps/vim/org.vibevm.apps.vim@1.1.0");
        assert_eq!(
            joined,
            root.join("apps")
                .join("vim")
                .join("org.vibevm.apps.vim@1.1.0")
        );
        assert_eq!(
            joined.to_string_lossy(),
            ["library", "apps", "vim", "org.vibevm.apps.vim@1.1.0"].join(MAIN_SEPARATOR_STR),
            "every separator is the platform's own"
        );
    }
}
