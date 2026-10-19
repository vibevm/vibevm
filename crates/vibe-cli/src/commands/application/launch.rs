//! Resolve and run installed application launch points from the durable index.

use std::fs;
use std::io::IsTerminal;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use super::model::{ApplicationRecord, ApplicationStatus, ManagementRuntime, UserLauncher};
use super::source::qualified_ref;
use super::{ApplicationStore, binary, record_for_package, settings_root};

struct ResolvedLauncher {
    executable: PathBuf,
    working_directory: PathBuf,
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

pub fn listing(package: &str, project_root: Option<&Path>) -> Result<Value> {
    let settings = settings_root()?;
    let store = ApplicationStore::open(&settings)?;
    let index = store.load()?;
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
                        Some(resolved.working_directory.display().to_string()),
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
    let working_directory = path(launcher.working_directory.as_deref().unwrap_or("."))
        .and_then(|candidate| canonical_dir(&candidate, "working_directory"))
        .ok()
        .filter(|dir| {
            dir.starts_with(app_root) || project_root.is_some_and(|root| dir.starts_with(root))
        })
        .map(|dir| dir.display().to_string());
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
    let settings = settings_root()?;
    let store = ApplicationStore::open(&settings)?;
    let index = store.load()?;
    let requested = qualified_ref(package)?;
    let record = record_for_package(&index, &requested)?.clone();
    drop(index);
    drop(store);
    let launcher = record
        .user_launchers
        .iter()
        .find(|entry| entry.id == launcher_id)
        .ok_or_else(|| anyhow::anyhow!("unknown_launcher: `{launcher_id}`"))?;
    if record.status != ApplicationStatus::Ready {
        bail!("launcher_unavailable: application is not deployed");
    }
    let root = app_root(&record)?;
    let resolved = match resolve_launcher(launcher, &record, &root, &settings, project_root) {
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
    let mut command = if extension == "sh" && !cfg!(windows) {
        let mut command = Command::new("sh");
        command.arg(&resolved.executable);
        command
    } else if matches!(extension.as_str(), "cmd" | "bat" | "ps1" | "sh") {
        bail!(
            "launcher_unavailable: script format `{extension}` cannot preserve literal arguments on this platform"
        );
    } else {
        Command::new(&resolved.executable)
    };
    let status = command
        .args(&resolved.args)
        .args(extra_args)
        .current_dir(&resolved.working_directory)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("launcher_unavailable: starting `{launcher_id}`"))?;
    Ok(status.code().unwrap_or(1))
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
    if matches!(extension.as_str(), "cmd" | "bat" | "ps1") || (extension == "sh" && cfg!(windows)) {
        bail!("script format `{extension}` cannot preserve literal arguments on this platform");
    }
    #[cfg(unix)]
    if extension != "sh" {
        use std::os::unix::fs::PermissionsExt;
        if fs::metadata(&executable)?.permissions().mode() & 0o111 == 0 {
            bail!("executable is not marked executable");
        }
    }
    if launcher.terminal && !(std::io::stdin().is_terminal() && std::io::stdout().is_terminal()) {
        bail!("terminal_required");
    }
    let directory_template = launcher.working_directory.as_deref().unwrap_or(".");
    let working_directory = resolve_path(
        directory_template,
        &app_root,
        &app_bin,
        settings,
        project_root,
    )?;
    let working_directory = canonical_dir(&working_directory, "working_directory")?;
    let project_root = project_root
        .map(|path| canonical_dir(path, "project_root"))
        .transpose()?;
    if !working_directory.starts_with(&app_root)
        && !project_root
            .as_ref()
            .is_some_and(|root| working_directory.starts_with(root))
    {
        bail!("working_directory escapes app_root and project_root");
    }
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
