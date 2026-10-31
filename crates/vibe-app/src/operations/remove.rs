//! `uninstall -g`, the installed-record lookup and the `app list` inventory.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#commands");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#ownership");

use std::fs;
use std::path::Path;

use vibe_wire::generated::application_list as wire;

use crate::alternatives::reserved_names;
use crate::error::{AppError, InstallerError};
use crate::index::{AuthoredCommands, Index, Installation, Location, StepOutcome};
use crate::installer::{self, ChildIo, shell};
use crate::launch;
use crate::proxies;
use crate::resolve::qualified_ref;
use crate::settings::SettingsRoot;
use crate::steps::{RunOutcome, RunRecord, StepState, StepsFile};
use crate::storage::{Availability, Storage};

use super::{Command, OperationContext, Outcome, normalize_slots};

/// Find the one installed record a coordinate names: an exact match, or the
/// slot default among several matching versions.
pub fn installed<'a>(
    index: &'a Index,
    spelling: &str,
) -> Result<(&'a str, &'a Installation), AppError> {
    let requested = qualified_ref(spelling)?;
    let group = requested
        .group
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_default();
    let candidates: Vec<(&String, &Installation)> = index
        .installations
        .iter()
        .filter(|(_, record)| {
            record.package.group == group
                && record.package.name == requested.name.as_str()
                && semver::Version::parse(&record.package.version)
                    .is_ok_and(|version| requested.version.matches(&version))
        })
        .collect();
    // `pkg@1.0.0` is a caret requirement to the package grammar, but an
    // installed version spelled out literally is what the user means.
    let literal = spelling
        .rsplit_once('@')
        .map(|(_, version)| version.trim_start_matches('='));
    if let Some((key, record)) = candidates
        .iter()
        .find(|(_, record)| Some(record.package.version.as_str()) == literal)
    {
        return Ok((key.as_str(), record));
    }
    let defaults: Vec<&(&String, &Installation)> = candidates
        .iter()
        .filter(|(key, record)| index.default_of(&record.slot) == Some(key.as_str()))
        .collect();
    match (candidates.as_slice(), defaults.as_slice()) {
        ([], _) => Err(AppError::NotInstalled {
            package: spelling.into(),
        }),
        ([(key, record)], _) | (_, [(key, record)]) => Ok((key.as_str(), record)),
        _ => Err(AppError::AmbiguousInstallation {
            package: spelling.into(),
        }),
    }
}

/// Remove exactly one installation: run the recorded uninstall commands in
/// reverse step order (with `--force`, their forced variants), then delete
/// an owned payload directory unconditionally; never touch an external
/// root.
pub fn uninstall(
    ctx: &OperationContext<'_>,
    spelling: &str,
    force: bool,
) -> Result<Outcome, AppError> {
    let _lock = ctx.settings.lock()?;
    let mut index = Index::load(ctx.settings)?;
    let storage = Storage::load(ctx.settings)?;
    let (key, installation) = installed(&index, spelling)?;
    let key = key.to_owned();
    let installation = installation.clone();
    let root = match &installation.location {
        Location::Owned { library, .. } => match storage.availability(library) {
            Availability::Available => launch::app_root(&installation, &storage).ok(),
            Availability::Unavailable(reason) => {
                return Err(AppError::Unavailable { key, reason });
            }
        },
        Location::External { app_root } => Some(app_root.clone()),
    };
    // The method form keeps its commands in `commands`; step forms record
    // them per step. Both run last step first.
    let recorded: Vec<(&str, &AuthoredCommands)> =
        std::iter::once(("uninstall", &installation.commands))
            .chain(
                installation
                    .steps
                    .iter()
                    .rev()
                    .map(|step| (step.id.as_str(), &step.uninstall)),
            )
            .filter(|(_, commands)| commands.has_uninstall())
            .collect();
    let external = matches!(installation.location, Location::External { .. });
    if force
        && external
        && (recorded.is_empty()
            || recorded
                .iter()
                .any(|(_, commands)| commands.force_uninstall.is_none()))
    {
        return Err(AppError::ForceUnsupported {
            key,
            reason: "the installation declares no force_uninstall",
        });
    }
    let scripts: Vec<(&str, &str, &AuthoredCommands)> = recorded
        .iter()
        .filter_map(|(id, commands)| {
            let script = match (force, &commands.force_uninstall) {
                (true, Some(forced)) => Some(forced),
                _ => commands.uninstall.as_ref(),
            };
            script.map(|script| (*id, script.as_str(), *commands))
        })
        .collect();
    let mut record = RunRecord::open(
        ctx.settings,
        &installation.package,
        Command::Uninstall,
        &installation.option,
        None,
    )?;
    if let Some(root) = root.as_deref().filter(|_| !scripts.is_empty()) {
        run_uninstalls(ctx, &installation, &scripts, root, &mut record)?;
    }
    record.close(RunOutcome::Done);
    let mut doomed = None;
    let mut message = match (&installation.location, root) {
        (Location::Owned { .. }, Some(directory)) if directory.exists() => {
            let text = format!("removed {key} and its payload `{}`", directory.display());
            doomed = Some(directory);
            text
        }
        (Location::Owned { .. }, _) => {
            format!("removed {key}; its payload directory was already gone")
        }
        (Location::External { app_root }, _) if scripts.is_empty() => format!(
            "removed {key}; no uninstall is authored, so `{}` stays in place",
            app_root.display()
        ),
        (Location::External { .. }, _) => format!("ran the authored uninstall of {key}"),
    };
    let slot = installation.slot.clone();
    let reserved = reserved_names(&index);
    index.installations.remove(&key);
    if index.default_of(&slot) == Some(key.as_str()) {
        let ready: Vec<&String> = index
            .installations
            .iter()
            .filter(|(_, record)| record.slot == slot && is_ready(record, &storage))
            .map(|(key, _)| key)
            .collect();
        let next = match ready.as_slice() {
            [only] => Some((*only).clone()),
            _ => None,
        };
        index.slots.insert(slot.clone(), next);
        message.push_str(&match index.default_of(&slot) {
            Some(next) => format!("; slot `{slot}` default is now {next}"),
            None => format!("; slot `{slot}` has no default"),
        });
    }
    normalize_slots(&mut index);
    index.save(ctx.settings)?;
    if let Some(directory) = doomed {
        fs::remove_dir_all(&directory)
            .map_err(|source| installer::io("removing", &directory, source))?;
    }
    proxies::sync(
        ctx.settings,
        &reserved,
        &reserved_names(&index),
        ctx.proxy_image,
    )?;
    Ok(Outcome {
        command: Command::Uninstall,
        slot_default: index.default_of(&slot).map(str::to_owned),
        slot,
        key,
        method: installation.method,
        app_root: launch::app_root(&installation, &storage).ok(),
        message,
    })
}

/// Run the recorded uninstall commands, each teed into the run record and
/// observed as one task; the first failure stops them.
fn run_uninstalls(
    ctx: &OperationContext<'_>,
    installation: &Installation,
    scripts: &[(&str, &str, &AuthoredCommands)],
    root: &Path,
    record: &mut RunRecord,
) -> Result<(), AppError> {
    let mut state = StepsFile {
        package: installation.package.clone(),
        option: installation.option.clone(),
        features: installation.features.clone(),
        steps: Vec::new(),
    };
    for (id, script, commands) in scripts {
        let task = ctx.progress.task(format!("{id}: uninstall"));
        let log = record.log(id);
        let observe = ChildIo {
            relay: ctx.relay(),
            log: Some(&log),
            stdout_log: None,
            task: Some(&task),
            interrupt: ctx.interrupt,
        };
        let started = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let result = shell::run(
            commands.interpreter,
            script,
            ctx.settings.path(),
            root,
            "authored uninstall",
            &observe,
        );
        let outcome = match &result {
            Ok(()) => {
                task.finish();
                StepOutcome::Done
            }
            Err(InstallerError::Interrupted { .. }) => {
                task.fail("interrupted");
                StepOutcome::Interrupted
            }
            Err(error) => {
                task.fail(error.to_string());
                StepOutcome::Failed
            }
        };
        state.record(StepState {
            id: (*id).into(),
            kind: "shell".into(),
            outcome,
            reason: result.as_ref().err().map(ToString::to_string),
            started: Some(started),
            finished: Some(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
            uninstall: AuthoredCommands::default(),
            agent: None,
        });
        record.steps(&state);
        if let Err(error) = result {
            if outcome == StepOutcome::Interrupted {
                record.close(RunOutcome::Interrupted);
            }
            return Err(error.into());
        }
    }
    Ok(())
}

fn is_ready(installation: &Installation, storage: &Storage) -> bool {
    launch::app_root(installation, storage).is_ok_and(|root| root.is_dir())
}

/// Every installation with its status and whether it is its slot's default.
pub fn list(settings: &SettingsRoot) -> Result<wire::ApplicationList, AppError> {
    let index = Index::load(settings)?;
    let storage = Storage::load(settings)?;
    let installations = index
        .installations
        .iter()
        .map(|(key, record)| {
            let root = launch::app_root(record, &storage);
            let reason = launch::unavailable_reason(&root);
            wire::Installation {
                key: key.clone(),
                slot: record.slot.clone(),
                package: wire::Package {
                    group: record.package.group.clone(),
                    name: record.package.name.clone(),
                    version: record.package.version.clone(),
                },
                method: record.method.as_str().into(),
                status: if reason.is_none() {
                    wire::Status::Ready
                } else {
                    wire::Status::Unavailable
                },
                reason,
                library: match &record.location {
                    Location::Owned { library, .. } => Some(library.clone()),
                    Location::External { .. } => None,
                },
                icon: match (&record.location, &record.icon, &root) {
                    (Location::Owned { .. }, Some(icon), Ok(root)) => {
                        Some(root.join(icon)).filter(|path| path.is_file())
                    }
                    _ => None,
                }
                .map(|path| path_text(&path)),
                app_root: root.ok().map(|root| path_text(&root)),
                portable: record.portable,
                default: index.default_of(&record.slot) == Some(key.as_str()),
            }
        })
        .collect();
    Ok(wire::ApplicationList {
        protocol: "vibe-app-list/1".into(),
        installations,
    })
}

fn path_text(path: &Path) -> String {
    path.display().to_string()
}
