//! Resolve and run installed application launch points from the durable index.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::IsTerminal;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use specmark::spec;

mod status;

use super::model::{ApplicationRecord, ApplicationStatus, ManagementRuntime, UserLauncher};
use super::source::qualified_ref;
use super::store::installation_key;
use super::{ApplicationStore, binary, lease, record_for_package, settings_root};

struct ResolvedLauncher {
    executable: PathBuf,
    working_directory: Option<PathBuf>,
    args: Vec<String>,
    icon: Option<PathBuf>,
    icon_diagnostic: Option<String>,
}

enum Resolution {
    Ready(ResolvedLauncher),
    RequiresProject,
    Unavailable(String),
}

#[derive(Default)]
struct KnownValues {
    executable: Option<String>,
    working_directory: Option<String>,
    args: Option<Vec<String>>,
    icon: Option<String>,
    icon_diagnostic: Option<String>,
}

/// Installation-time projection for an OS PATH proxy. The proxy executes this
/// target itself; ordinary command invocation never reads Vibe's index.
#[derive(Debug)]
pub(super) struct PublicLaunchPlan {
    pub target: PathBuf,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
}

pub(super) fn plan_public_launcher(
    record: &ApplicationRecord,
    launcher_id: &str,
    settings: &Path,
) -> Result<PublicLaunchPlan> {
    if record.status != ApplicationStatus::Ready {
        bail!("public command target is not deployed");
    }
    let launcher = record
        .user_launchers
        .iter()
        .find(|entry| entry.id == launcher_id)
        .ok_or_else(|| anyhow::anyhow!("unknown public command launcher `{launcher_id}`"))?;
    let root = app_root(record)?;
    let app_bin = settings.join("opt").join("bin");
    let authored_path = resolve_path(&launcher.executable, &root, &app_bin, settings, None)
        .map_err(|error| {
            if error.to_string() == "project_required" {
                anyhow::anyhow!("public command cannot use $project_root")
            } else {
                error
            }
        })?;
    if authored_path.starts_with(&app_bin) {
        bail!("public command cannot target shared opt/bin");
    }
    let canonical_root = canonical_dir(&root, "app_root")?;
    let resolved = resolve_launcher_checked(launcher, record, &canonical_root, settings, None)
        .map_err(|error| {
            if error.to_string() == "project_required" {
                anyhow::anyhow!("public command cannot use $project_root")
            } else {
                error
            }
        })?;
    if !resolved.executable.starts_with(&canonical_root)
        || resolved.executable.starts_with(&app_bin)
    {
        bail!("public command target must stay within registered app_root");
    }
    #[cfg(windows)]
    if matches!(
        resolved
            .executable
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "cmd" | "bat" | "ps1"
    ) {
        bail!("Windows scripts cannot be public command targets");
    }
    Ok(PublicLaunchPlan {
        target: resolved.executable,
        args: resolved.args,
        cwd: resolved.working_directory,
    })
}

pub fn listing(package: &str, project_root: Option<&Path>) -> Result<Value> {
    let settings = settings_root()?;
    let index = ApplicationStore::load_snapshot(&settings)?;
    let requested = qualified_ref(package)?;
    let record = record_for_package(&index, &requested)?;
    let app_root = app_root(record);
    let entries: Vec<_> = record
        .user_launchers
        .iter()
        .map(|launcher| {
            let known = app_root
                .as_ref()
                .map(|root| known_values(launcher, record, root, &settings, project_root))
                .unwrap_or_default();
            let resolution = if record.status != ApplicationStatus::Ready {
                Resolution::Unavailable("application is not deployed".into())
            } else {
                match &app_root {
                    Ok(root) => resolve_launcher(launcher, record, root, &settings, project_root),
                    Err(error) => Resolution::Unavailable(error.to_string()),
                }
            };
            let (status, reason, executable, working_directory, args, icon, icon_status) =
                match resolution {
                    Resolution::Ready(resolved) => (
                        "available",
                        None,
                        Some(resolved.executable.display().to_string()),
                        resolved
                            .working_directory
                            .as_ref()
                            .map(|path| path.display().to_string()),
                        Some(resolved.args),
                        resolved.icon.map(|path| path.display().to_string()),
                        resolved.icon_diagnostic,
                    ),
                    Resolution::RequiresProject => (
                        "requires_project",
                        Some("project_required".to_string()),
                        known.executable,
                        known.working_directory,
                        known.args,
                        known.icon,
                        known.icon_diagnostic,
                    ),
                    Resolution::Unavailable(reason) => (
                        "unavailable",
                        Some(reason),
                        known.executable,
                        known.working_directory,
                        known.args,
                        known.icon,
                        known.icon_diagnostic,
                    ),
                };
            json!({
                "id": launcher.id,
                "name": launcher.name,
                "description": launcher.description,
                "default": launcher.default,
                "terminal": launcher.terminal,
                "executable": { "template": launcher.executable, "base": "app_root", "resolved": executable },
                "args": { "template": launcher.args, "resolved": args },
                "working_directory": { "template": launcher.working_directory, "base": "app_root", "resolved": working_directory },
                "icon": { "template": launcher.icon, "resolved": icon, "diagnostic": icon_status },
                "status": status,
                "reason": reason,
            })
        })
        .collect();
    Ok(json!({
        "protocol": "vibe-app-launchers/1",
        "application_id": record.application.id,
        "package": {
            "group": record.application.package.group,
            "name": record.application.package.name,
            "version": record.application.package.version,
        },
        "default_launcher_id": record.user_launchers.iter().find(|entry| entry.default).map(|entry| &entry.id),
        "app_root": app_root.ok().map(|path| path.display().to_string()),
        "launchers": entries,
    }))
}

fn known_values(
    launcher: &UserLauncher,
    record: &ApplicationRecord,
    app_root: &Path,
    settings: &Path,
    project_root: Option<&Path>,
) -> KnownValues {
    let app_bin = settings.join("opt").join("bin");
    let path = |template: &str| resolve_path(template, app_root, &app_bin, settings, project_root);
    let owned_file = |file: &Path| {
        file.starts_with(app_root)
            || record
                .launchers
                .iter()
                .any(|entry| entry.destination == file)
    };
    let executable = path(&launcher.executable)
        .and_then(|candidate| canonical_file(&candidate, "executable"))
        .ok()
        .filter(|file| owned_file(file))
        .map(|file| file.display().to_string());
    let working_directory = launcher.working_directory.as_deref().and_then(|template| {
        path(template)
            .and_then(|candidate| canonical_dir(&candidate, "working_directory"))
            .ok()
            .filter(|dir| {
                dir.starts_with(app_root) || project_root.is_some_and(|root| dir.starts_with(root))
            })
            .map(|dir| dir.display().to_string())
    });
    let args = launcher
        .args
        .iter()
        .map(|arg| expand(arg, app_root, &app_bin, settings, project_root))
        .collect::<Result<Vec<_>>>()
        .ok();
    let (icon, icon_diagnostic) = match launcher.icon.as_deref().filter(|value| !value.is_empty()) {
        None => (None, None),
        Some(template) => {
            match path(template).and_then(|candidate| canonical_file(&candidate, "icon")) {
                Ok(file) if owned_file(&file) => (Some(file.display().to_string()), None),
                Ok(_) => (
                    None,
                    Some("icon escapes registered app_root and runners".into()),
                ),
                Err(error) => (None, Some(error.to_string())),
            }
        }
    };
    KnownValues {
        executable,
        working_directory,
        args,
        icon,
        icon_diagnostic,
    }
}

pub fn run(
    package: &str,
    launcher_id: &str,
    extra_args: &[String],
    project_root: Option<&Path>,
) -> Result<i32> {
    let args: Vec<OsString> = extra_args.iter().map(OsString::from).collect();
    run_os(package, launcher_id, &args, project_root)
}

#[spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "Capture caller PATH once at the CLI app-launch boundary for the child process."
)]
pub fn run_os(
    package: &str,
    launcher_id: &str,
    extra_args: &[OsString],
    project_root: Option<&Path>,
) -> Result<i32> {
    let settings = settings_root()?;
    let index = ApplicationStore::load_snapshot(&settings)?;
    let requested = qualified_ref(package)?;
    let key = installation_key(&record_for_package(&index, &requested)?.application.package);
    let _lease = lease::acquire_shared(&settings, &key)?;
    let index = ApplicationStore::load_snapshot(&settings)?;
    let record = record_for_package(&index, &requested)?;
    if installation_key(&record.application.package) != key {
        bail!("launcher_unavailable: installation changed during launch");
    }
    let inherited_path = std::env::var_os("PATH");
    run_selected(
        record,
        launcher_id,
        extra_args,
        project_root,
        &settings,
        inherited_path.as_deref(),
    )
}

fn run_selected(
    record: &ApplicationRecord,
    launcher_id: &str,
    extra_args: &[OsString],
    project_root: Option<&Path>,
    settings: &Path,
    inherited_path: Option<&OsStr>,
) -> Result<i32> {
    let launcher = record
        .user_launchers
        .iter()
        .find(|entry| entry.id == launcher_id)
        .ok_or_else(|| anyhow::anyhow!("unknown_launcher: `{launcher_id}`"))?;
    if record.status != ApplicationStatus::Ready {
        bail!("launcher_unavailable: application is not deployed");
    }
    let root = app_root(record)?;
    let resolved = match resolve_launcher(launcher, record, &root, settings, project_root) {
        Resolution::Ready(value) => value,
        Resolution::RequiresProject => bail!("launcher_unavailable: project_required"),
        Resolution::Unavailable(reason) => bail!("launcher_unavailable: {reason}"),
    };
    if launcher.terminal && !(std::io::stdin().is_terminal() && std::io::stdout().is_terminal()) {
        bail!("launcher_unavailable: terminal_required");
    }
    let extension = resolved
        .executable
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if matches!(extension.as_str(), "ps1" | "cmd" | "bat") || (cfg!(windows) && extension == "sh") {
        bail!("launcher_unavailable: unsupported script format `{extension}`");
    }
    let mut command = Command::new(&resolved.executable);
    command
        .args(&resolved.args)
        .args(extra_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let key = installation_key(&record.application.package);
    let view = super::alternatives::projection::view_bin(settings, &record.application.id, &key);
    if !view.is_dir() {
        bail!("launcher_unavailable: installation PATH view is missing");
    }
    let mut paths = vec![view];
    if let Some(inherited) = inherited_path {
        paths.extend(std::env::split_paths(inherited));
    }
    command.env(
        "PATH",
        std::env::join_paths(paths).context("building application PATH")?,
    );
    if let Some(directory) = &resolved.working_directory {
        command.current_dir(directory);
    }
    let status = command
        .status()
        .with_context(|| format!("launcher_unavailable: starting `{launcher_id}`"))?;
    Ok(status::code(status))
}

fn app_root(record: &ApplicationRecord) -> Result<PathBuf> {
    if let Some(path) = &record.app_root {
        return Ok(path.clone());
    }
    if record.management.runtime == ManagementRuntime::Builtin {
        return binary::payload_root(&record.management);
    }
    Ok(record.host_root.clone())
}

fn resolve_launcher(
    launcher: &UserLauncher,
    record: &ApplicationRecord,
    app_root: &Path,
    settings: &Path,
    project_root: Option<&Path>,
) -> Resolution {
    match resolve_launcher_checked(launcher, record, app_root, settings, project_root) {
        Ok(value) => Resolution::Ready(value),
        Err(error) if error.to_string() == "project_required" => Resolution::RequiresProject,
        Err(error) => Resolution::Unavailable(error.to_string()),
    }
}

fn resolve_launcher_checked(
    launcher: &UserLauncher,
    record: &ApplicationRecord,
    app_root: &Path,
    settings: &Path,
    project_root: Option<&Path>,
) -> Result<ResolvedLauncher> {
    let app_root = canonical_dir(app_root, "app_root")?;
    let app_bin = settings.join("opt").join("bin");
    let executable = resolve_path(
        &launcher.executable,
        &app_root,
        &app_bin,
        settings,
        project_root,
    )?;
    let executable = canonical_file(&executable, "executable")?;
    if !executable.starts_with(&app_root)
        && !record
            .launchers
            .iter()
            .any(|owned| owned.destination == executable)
    {
        bail!("executable escapes registered app_root and runners");
    }
    let extension = executable
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if extension == "exe" && !cfg!(windows) {
        bail!("Windows executable cannot run on this platform");
    }
    if extension == "ps1"
        || (matches!(extension.as_str(), "cmd" | "bat") && !cfg!(windows))
        || (extension == "sh" && cfg!(windows))
    {
        bail!("script format `{extension}` cannot preserve literal arguments on this platform");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if fs::metadata(&executable)?.permissions().mode() & 0o111 == 0 {
            bail!("executable is not marked executable");
        }
    }
    let project_root = project_root
        .map(|path| canonical_dir(path, "project_root"))
        .transpose()?;
    let working_directory = launcher
        .working_directory
        .as_deref()
        .map(|template| {
            let directory = resolve_path(
                template,
                &app_root,
                &app_bin,
                settings,
                project_root.as_deref(),
            )?;
            let directory = canonical_dir(&directory, "working_directory")?;
            if !directory.starts_with(&app_root)
                && !project_root
                    .as_ref()
                    .is_some_and(|root| directory.starts_with(root))
            {
                bail!("working_directory escapes app_root and project_root");
            }
            Ok(directory)
        })
        .transpose()?;
    let args = launcher
        .args
        .iter()
        .map(|arg| expand(arg, &app_root, &app_bin, settings, project_root.as_deref()))
        .collect::<Result<Vec<_>>>()?;
    let mut icon = None;
    let mut icon_diagnostic = None;
    if let Some(template) = launcher.icon.as_deref().filter(|value| !value.is_empty()) {
        match resolve_path(
            template,
            &app_root,
            &app_bin,
            settings,
            project_root.as_deref(),
        )
        .and_then(|path| canonical_file(&path, "icon"))
        {
            Ok(path)
                if path.starts_with(&app_root)
                    || record
                        .launchers
                        .iter()
                        .any(|owned| owned.destination == path) =>
            {
                icon = Some(path);
            }
            Ok(_) => icon_diagnostic = Some("icon escapes registered app_root and runners".into()),
            Err(error) => icon_diagnostic = Some(error.to_string()),
        }
    }
    Ok(ResolvedLauncher {
        executable,
        working_directory,
        args,
        icon,
        icon_diagnostic,
    })
}

fn resolve_path(
    template: &str,
    app_root: &Path,
    app_bin: &Path,
    vibe_home: &Path,
    project_root: Option<&Path>,
) -> Result<PathBuf> {
    if Path::new(template).is_absolute() {
        bail!("absolute manifest path is forbidden; use a Vibe path variable");
    }
    let expanded = expand(template, app_root, app_bin, vibe_home, project_root)?;
    let path = PathBuf::from(expanded);
    if path
        .components()
        .any(|component| component == Component::ParentDir)
    {
        bail!("manifest path contains `..`");
    }
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(app_root.join(path))
    }
}

fn expand(
    template: &str,
    app_root: &Path,
    app_bin: &Path,
    vibe_home: &Path,
    project_root: Option<&Path>,
) -> Result<String> {
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
        let name = if chars.peek() == Some(&'{') {
            chars.next();
            let mut name = String::new();
            let mut closed = false;
            for next in chars.by_ref() {
                if next == '}' {
                    closed = true;
                    break;
                }
                name.push(next);
            }
            if !closed {
                bail!("unterminated Vibe path variable");
            }
            name
        } else {
            let mut name = String::new();
            while let Some(ch) = chars.peek().copied() {
                if !ch.is_ascii_alphanumeric() && ch != '_' {
                    break;
                }
                chars.next();
                name.push(ch);
            }
            name
        };
        let value = match name.as_str() {
            "app_root" => app_root,
            "app_bin" => app_bin,
            "vibe_home" => vibe_home,
            "project_root" => project_root.ok_or_else(|| anyhow::anyhow!("project_required"))?,
            _ => bail!("unknown Vibe path variable `${name}`"),
        };
        result.push_str(&value.to_string_lossy());
    }
    Ok(result)
}

fn canonical_dir(path: &Path, field: &str) -> Result<PathBuf> {
    let resolved = fs::canonicalize(path).with_context(|| format!("{field} is unavailable"))?;
    if !resolved.is_dir() {
        bail!("{field} is not a directory");
    }
    Ok(resolved)
}

fn canonical_file(path: &Path, field: &str) -> Result<PathBuf> {
    let resolved = fs::canonicalize(path).with_context(|| format!("{field} is unavailable"))?;
    if !resolved.is_file() {
        bail!("{field} is not a regular file");
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests;
