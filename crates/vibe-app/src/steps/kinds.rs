//! One step of each kind, run by reusing the installer modules
//! (`PROP-065#kinds`). Every child streams its output live.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#kinds");

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use vibe_core::manifest::{
    ArchiveFormatDecl, NativeFormatDecl, StepDecl, StepKind, StepOrigin, StepPhase,
};

use crate::error::{AppError, InstallerError, StepError};
use crate::index::AuthoredCommands;
use crate::installer::archive::{self, ArchiveFormat};
use crate::installer::native::{self, NativeKind};
use crate::installer::source_build::{self, digest_of};
use crate::installer::{self, ChildIo, FetchWatch, io, run_child, shell};

use super::template::{self, Vars};

/// What a step sees: its directories, variables and the operation flags.
pub(crate) struct StepContext<'a> {
    pub work: &'a Path,
    pub package_root: &'a Path,
    pub cache_root: &'a Path,
    pub vars: Vars<'a>,
    pub platform: &'a str,
    pub arch: &'a str,
    /// The placed or registered root for `finish` steps, the intended one
    /// for `prepare` shell steps (`VIBE_APP_ROOT`).
    pub app_root: &'a Path,
    pub phase: StepPhase,
    pub offline: bool,
    pub force: bool,
    pub update_in_place: bool,
    /// The step's run log, progress task, relay and interrupt.
    pub io: ChildIo<'a>,
    /// `VIBE_FEATURE_<ID>` and `VIBE_PARAM_<ID>` (`PROP-065#templates`).
    pub env: &'a [(String, String)],
}

impl StepContext<'_> {
    fn watch(&self, size: Option<u64>) -> FetchWatch<'_> {
        FetchWatch {
            size,
            task: self.io.task,
            interrupt: self.io.interrupt,
        }
    }
}

/// The kept verified input of an `archive` or `native` step.
pub(crate) fn input_file(work: &Path, step: &StepDecl) -> PathBuf {
    let extension = match &step.kind {
        StepKind::Native {
            format: NativeFormatDecl::Exe,
            ..
        } => ".exe",
        StepKind::Native {
            format: NativeFormatDecl::Msi,
            ..
        } => ".msi",
        _ => "",
    };
    work.join("inputs").join(format!("{}{extension}", step.id))
}

/// Run one step; a shell or native step returns its uninstall data.
pub(crate) fn run(cx: &StepContext<'_>, step: &StepDecl) -> Result<AuthoredCommands, AppError> {
    let what = match step.origin {
        StepOrigin::PreInstall => "pre_install".to_owned(),
        StepOrigin::PostInstall => "post_install".to_owned(),
        StepOrigin::Authored => format!("{} step `{}`", step.kind.name(), step.id),
    };
    let out = cx.work.join("out");
    match &step.kind {
        StepKind::Sources { sources } => {
            let fetched = sources
                .iter()
                .map(|source| {
                    source_build::fetch(
                        cx.cache_root,
                        cx.package_root,
                        source,
                        cx.offline,
                        &cx.watch(None),
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            source_build::lay_out(&cx.work.join("sources"), &fetched)?;
        }
        StepKind::Archive {
            format,
            source,
            sha256,
            size,
            payload_root,
        } => {
            let file = verified_input(cx, step, source, sha256, *size)?;
            let format = match format {
                ArchiveFormatDecl::Zip => ArchiveFormat::Zip,
                ArchiveFormatDecl::TarGz => ArchiveFormat::TarGz,
            };
            extract_into(
                cx.work,
                &step.id,
                &file,
                format,
                payload_root.as_deref(),
                &out,
            )?;
        }
        StepKind::Recipe {
            interpreter,
            script,
        } => {
            let mut command = source_build::script_command(*interpreter, cx.package_root, script)?;
            command.current_dir(cx.work);
            environment(cx, &mut command);
            run_child(&mut command, &what, &cx.io)?;
        }
        StepKind::Shell {
            command,
            interpreter,
            force_install,
            update,
            uninstall,
            force_uninstall,
        } => {
            let script = if cx.force {
                force_install.as_deref().unwrap_or(command)
            } else if cx.update_in_place {
                update.as_deref().unwrap_or(command)
            } else {
                command
            };
            let current_dir = match cx.phase {
                StepPhase::Prepare => cx.package_root,
                StepPhase::Finish => cx.app_root,
            };
            let mut child = shell::command(*interpreter, script, current_dir, cx.app_root)?;
            environment(cx, &mut child);
            run_child(&mut child, &what, &cx.io)?;
            return Ok(AuthoredCommands {
                uninstall: uninstall.clone(),
                force_uninstall: force_uninstall.clone(),
                update: None,
                interpreter: *interpreter,
            });
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
            let kind = match format {
                NativeFormatDecl::Exe => NativeKind::Exe,
                NativeFormatDecl::Msi => NativeKind::Msi,
            };
            if !cfg!(windows) {
                return Err(InstallerError::WindowsOnly {
                    method: kind.as_str(),
                }
                .into());
            }
            let file = verified_input(cx, step, source, sha256, None)?;
            let arguments = match (cx.force, force_arguments) {
                (true, Some(forced)) => forced.as_slice(),
                _ => arguments.as_slice(),
            };
            native::run(kind, &file, arguments, cx.package_root, &cx.io)?;
            return Ok(AuthoredCommands {
                uninstall: uninstall.clone(),
                force_uninstall: force_uninstall.clone(),
                update: None,
                interpreter: None,
            });
        }
        StepKind::Copy { from, to } => {
            let base = match cx.phase {
                StepPhase::Prepare => out.as_path(),
                StepPhase::Finish => cx.app_root,
            };
            let to = template::expand(to, &cx.vars, &step.id, "to")?;
            copy(cx.package_root, from, base, &to, &step.id)?;
        }
        // The runner delivers agent steps itself (`super::agent`,
        // `PROP-064`): they need the plan, the hooks and the run record;
        // env steps (`super::env`) need the user environment and the key.
        StepKind::Agent { .. } | StepKind::Env { .. } => {}
    }
    Ok(AuthoredCommands::default())
}

/// `VIBE_WORK`, `VIBE_SOURCES`, `VIBE_OUT`, `VIBE_PACKAGE`, `VIBE_PLATFORM`,
/// `VIBE_ARCH`, `VIBE_FEATURE_<ID>`, `VIBE_PARAM_<ID>`, and `VIBE_APP_ROOT`
/// for a finish step.
fn environment(cx: &StepContext<'_>, command: &mut Command) {
    command
        .env("VIBE_WORK", cx.work)
        .env("VIBE_SOURCES", cx.work.join("sources"))
        .env("VIBE_OUT", cx.work.join("out"))
        .env("VIBE_PACKAGE", cx.package_root)
        .env("VIBE_PLATFORM", cx.platform)
        .env("VIBE_ARCH", cx.arch)
        .envs(cx.env.iter().map(|(name, value)| (name, value)));
    if cx.phase == StepPhase::Finish {
        command.env("VIBE_APP_ROOT", cx.app_root);
    }
}

/// The step's input under `<work>/inputs/`, fetched only when no verified
/// copy is already in place (`PROP-065#resume`).
fn verified_input(
    cx: &StepContext<'_>,
    step: &StepDecl,
    source: &str,
    sha256: &str,
    size: Option<u64>,
) -> Result<PathBuf, AppError> {
    let file = input_file(cx.work, step);
    if digest_of(&file).ok().as_deref() != Some(sha256.to_ascii_lowercase().as_str()) {
        installer::fetch_verified(
            source,
            sha256,
            cx.package_root,
            &file,
            cx.offline,
            &cx.watch(size),
        )?;
    }
    Ok(file)
}

/// The first archive becomes `$VIBE_OUT`; a later one merges into it and
/// may not overwrite an existing entry.
fn extract_into(
    work: &Path,
    id: &str,
    file: &Path,
    format: ArchiveFormat,
    payload_root: Option<&Path>,
    out: &Path,
) -> Result<(), InstallerError> {
    let empty = fs::read_dir(out).map_or(true, |mut entries| entries.next().is_none());
    if empty {
        if out.exists() {
            fs::remove_dir(out).map_err(|source| io("clearing", out, source))?;
        }
        return archive::extract(file, format, payload_root, out);
    }
    let unpacked = work.join(format!("unpack-{id}"));
    if unpacked.exists() {
        fs::remove_dir_all(&unpacked).map_err(|source| io("clearing", &unpacked, source))?;
    }
    archive::extract(file, format, payload_root, &unpacked)?;
    let entries = fs::read_dir(&unpacked).map_err(|source| io("reading", &unpacked, source))?;
    for entry in entries {
        let entry = entry.map_err(|source| io("reading", &unpacked, source))?;
        let target = out.join(entry.file_name());
        if target.exists() {
            return Err(InstallerError::Archive {
                reason: format!("`{}` already exists in $VIBE_OUT", target.display()),
            });
        }
        fs::rename(entry.path(), &target).map_err(|source| io("placing", &target, source))?;
    }
    fs::remove_dir_all(&unpacked).map_err(|source| io("clearing", &unpacked, source))
}

/// Copy a package-relative file or directory to `<base>/<to>`, both
/// contained.
fn copy(
    package_root: &Path,
    from: &Path,
    base: &Path,
    to: &str,
    step: &str,
) -> Result<(), AppError> {
    let refuse = |reason: String| -> AppError {
        StepError::Copy {
            step: step.into(),
            reason,
        }
        .into()
    };
    let root = fs::canonicalize(package_root).map_err(|error| refuse(error.to_string()))?;
    let source = fs::canonicalize(root.join(from))
        .map_err(|error| refuse(format!("`{}`: {error}", from.display())))?;
    if !source.starts_with(&root) {
        return Err(refuse(format!(
            "`{}` escapes the package tree",
            from.display()
        )));
    }
    let to = Path::new(to);
    if to.components().any(|part| part == Component::ParentDir) {
        return Err(refuse("the destination traverses with `..`".into()));
    }
    let target = match to.strip_prefix(base) {
        Ok(relative) => base.join(relative),
        Err(_) if to.is_absolute() => {
            return Err(refuse(format!("`{}` is outside the payload", to.display())));
        }
        Err(_) => base.join(to),
    };
    crate::fs_util::copy_into(&source, &target)
        .map_err(|failure| io(failure.action, &failure.path, failure.source).into())
}
