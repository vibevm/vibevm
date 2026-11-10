//! `uninstall`, tree removal and `autoremove` (`PROP-059#commands`,
//! `PROP-065#uninstall-tree`): the authored removal of each installation
//! (recorded uninstall commands and env rollbacks, `finish` before
//! `prepare`, last step first), its proxies, its owned payload and its
//! record, with a run record per removed installation.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#commands");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#ownership");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#uninstall-tree");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#project-scope");

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use specmark::spec;
use vibe_core::manifest::RequirementScope;
use vibe_wire::generated::application_removal as wire;

use crate::alternatives::reserved_names;
use crate::error::{AppError, InstallerError};
use crate::error_lifecycle::LifecycleError;
use crate::index::{
    AuthoredCommands, EnvEntry, Index, Installation, Location, PackageCoordinate, StepOutcome,
};
use crate::installer::{self, ChildIo, shell};
use crate::launch;
use crate::proxies;
use crate::steps::{RunOutcome, RunRecord, StepState, StepsFile};
use crate::storage::{Availability, Storage};

use super::listing::installed;
use super::tree::{self, Reason, Undo, script_of};
use super::{Command, OperationContext, Outcome, normalize_slots};

/// The flags of `uninstall` (`PROP-065#uninstall-tree`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#uninstall-tree")]
pub struct RemoveRequest {
    /// Run the forced variants of the recorded uninstall commands.
    pub force: bool,
    /// Remove the runtime dependents too instead of refusing.
    pub cascade: bool,
    /// Remove the orphans this removal leaves.
    pub autoremove: bool,
    /// Show the removal as a plan and change nothing.
    pub plan_only: bool,
}

/// What a removal did or would do: the outcome of the named installation
/// (absent for a plan or `autoremove`) and the whole removal document.
#[derive(Debug, Clone, PartialEq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#uninstall-tree")]
pub struct Removal {
    pub outcome: Option<Outcome>,
    pub report: wire::ApplicationRemoval,
}

/// Remove exactly the installation `spelling` names — with `cascade` its
/// runtime dependents first — then list (or with `autoremove` remove) the
/// auto installations nothing requires any more. `plan_only` changes
/// nothing and reports the plan.
pub fn uninstall(
    ctx: &OperationContext<'_>,
    spelling: &str,
    request: &RemoveRequest,
) -> Result<Removal, AppError> {
    let held = ctx.settings.lock()?;
    let lock = &held;
    let index = Index::load(ctx.settings)?;
    let storage = Storage::load(ctx.settings)?;
    let (key, _) = installed(ctx.settings, &index, spelling)?;
    let key = key.to_owned();
    let order = tree::removal_order(&index, &key, request.cascade)?;
    let mut after = index.clone();
    for (doomed, _) in &order {
        after.installations.remove(doomed);
    }
    let orphans = tree::orphans(&after);
    let mut planned = order.clone();
    if request.autoremove {
        planned.extend(tree::autoremove_order(&after));
    }
    if request.plan_only {
        return Ok(Removal {
            outcome: None,
            report: report(&index, &storage, &planned, &orphans, request),
        });
    }
    let mut outcome = None;
    for (doomed, _) in &planned {
        let removed = remove_locked(lock, ctx, doomed, request.force)?;
        if *doomed == key {
            outcome = Some(removed);
        }
    }
    let left = if request.autoremove {
        Vec::new()
    } else {
        orphans
    };
    let mut outcome = outcome.ok_or_else(|| AppError::not_installed(ctx.settings, &key))?;
    let others: Vec<&str> = planned
        .iter()
        .filter(|(doomed, _)| *doomed != key)
        .map(|(doomed, _)| doomed.as_str())
        .collect();
    if !others.is_empty() {
        outcome
            .message
            .push_str(&format!("; also removed {}", others.join(", ")));
    }
    if !left.is_empty() {
        outcome.message.push_str(&format!(
            "; orphans nothing requires any more: {} (remove them with --autoremove or `vibe app autoremove`)",
            left.join(", ")
        ));
    }
    Ok(Removal {
        outcome: Some(outcome),
        report: report(&index, &storage, &planned, &left, request),
    })
}

/// Remove every auto installation nothing requires any more and every kept
/// previous installation (`PROP-065#uninstall-tree`,
/// `#update-and-rollback`); a manual installation is never removed.
pub fn autoremove(ctx: &OperationContext<'_>, plan_only: bool) -> Result<Removal, AppError> {
    let held = ctx.settings.lock()?;
    let lock = &held;
    let index = Index::load(ctx.settings)?;
    let storage = Storage::load(ctx.settings)?;
    let planned = tree::autoremove_order(&index);
    let request = RemoveRequest {
        plan_only,
        autoremove: true,
        ..RemoveRequest::default()
    };
    if !plan_only {
        for (doomed, _) in &planned {
            remove_locked(lock, ctx, doomed, false)?;
        }
    }
    Ok(Removal {
        outcome: None,
        report: report(&index, &storage, &planned, &[], &request),
    })
}

/// Remove every installation of a project home that its lockfile no
/// longer records (`PROP-059#project-scope`: the lockfile is the record;
/// `PROP-065#uninstall-tree`: deleting the dependency row leads to the
/// same removal, orphans included). `recorded` answers whether a package
/// coordinate has a lock row. A kept previous installation stays, and so
/// does whatever a staying installation requires at runtime; dependents
/// are removed before what they require.
pub fn remove_unrecorded(
    ctx: &OperationContext<'_>,
    recorded: &dyn Fn(&PackageCoordinate) -> bool,
) -> Result<Vec<Outcome>, AppError> {
    let held = ctx.settings.lock()?;
    let lock = &held;
    let index = Index::load(ctx.settings)?;
    let mut doomed: BTreeSet<String> = index
        .installations
        .iter()
        .filter(|(_, record)| {
            record.lifecycle.superseded_by.is_none() && !recorded(&record.package)
        })
        .map(|(key, _)| key.clone())
        .collect();
    loop {
        let held: Vec<String> = doomed
            .iter()
            .filter(|key| {
                index.required_by(key).iter().any(|edge| {
                    !edge.weak
                        && edge.scope == RequirementScope::Runtime
                        && !doomed.contains(edge.owner)
                })
            })
            .cloned()
            .collect();
        if held.is_empty() {
            break;
        }
        for key in held {
            doomed.remove(&key);
        }
    }
    let mut order: Vec<String> = Vec::new();
    for key in &doomed {
        for (next, _) in tree::removal_order(&index, key, true)? {
            if doomed.contains(&next) && !order.contains(&next) {
                order.push(next);
            }
        }
    }
    order
        .iter()
        .map(|key| remove_locked(lock, ctx, key, false))
        .collect()
}

fn report(
    index: &Index,
    storage: &Storage,
    planned: &[(String, Reason)],
    orphans: &[String],
    request: &RemoveRequest,
) -> wire::ApplicationRemoval {
    wire::ApplicationRemoval {
        protocol: "vibe-app-removal/1".into(),
        plan_only: request.plan_only,
        installations: planned
            .iter()
            .filter_map(|(key, reason)| {
                let record = index.installations.get(key)?;
                Some(tree::plan_entry(index, storage, key, record, *reason))
            })
            .collect(),
        orphans: orphans.to_vec(),
    }
}

/// Remove exactly one installation under the held lock: its authored
/// removal, then the record (the slot default passes on), then the owned
/// payload, its icon copies (`PROP-059#menu`), then the proxies. An external
/// root is never deleted.
pub(crate) fn remove_locked(
    lock: &crate::MutationLock,
    ctx: &OperationContext<'_>,
    key: &str,
    force: bool,
) -> Result<Outcome, AppError> {
    remove_with_boundary(lock, ctx, key, force, None)
}
pub(super) fn remove_with_boundary(
    lock: &crate::MutationLock,
    ctx: &OperationContext<'_>,
    key: &str,
    force: bool,
    before_effect: Option<&dyn Fn() -> Result<(), AppError>>,
) -> Result<Outcome, AppError> {
    let mut index = Index::load(ctx.settings)?;
    let storage = Storage::load(ctx.settings)?;
    let installation = index
        .installations
        .get(key)
        .cloned()
        .ok_or_else(|| AppError::not_installed(ctx.settings, key))?;
    // Full-import receipts reserve their exact successful execution while the
    // finite cohort may still consume it. Removal cannot erase that protection.
    let work = super::pending::work(ctx.settings, &installation.package);
    if let Some(state) = crate::steps::StepsFile::load(&work)?
        && let Some(owner) = super::pending::full_import_owner(ctx.settings, &state)?
    {
        return Err(super::pending::failure(
            ctx.settings,
            &installation.package,
            format!(
                "installation is reserved by unfinished full import {}; finish or explicitly clean that cohort before removal",
                owner.operation
            ),
        ));
    }
    let key = key.to_owned();
    let root = match &installation.location {
        Location::Owned { library, .. } => match storage.availability(library) {
            Availability::Available => launch::app_root(&installation, &storage).ok(),
            Availability::Unavailable(reason) => {
                return Err(AppError::Unavailable { key, reason });
            }
        },
        Location::External { app_root } => Some(app_root.clone()),
    };
    let undo = tree::authored_removal(&installation);
    let scripts: Vec<&AuthoredCommands> = undo
        .iter()
        .filter_map(|item| match item {
            Undo::Script { commands, .. } => Some(*commands),
            Undo::Env { .. } => None,
        })
        .collect();
    let external = matches!(installation.location, Location::External { .. });
    if force
        && external
        && (scripts.is_empty()
            || scripts
                .iter()
                .any(|commands| commands.force_uninstall.is_none()))
    {
        return Err(AppError::ForceUnsupported {
            key,
            reason: "the installation declares no force_uninstall",
        });
    }
    let ran_scripts = scripts
        .iter()
        .any(|commands| script_of(commands, force).is_some());
    let mut record = RunRecord::open(
        ctx.settings,
        &installation.package,
        Command::Uninstall,
        &installation.option,
        None,
    )?;
    if let Some(before) = before_effect {
        before()?;
    }
    if let Some(root) = root.as_deref() {
        undo_authored(ctx, &installation, &undo, force, root, &mut record)?;
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
        (Location::External { app_root }, _) if !ran_scripts => format!(
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
            .filter(|(_, record)| {
                record.slot == slot && !record.lifecycle.hidden && is_ready(record, &storage)
            })
            .map(|(key, _)| key)
            .collect();
        let next = match ready.as_slice() {
            [only] => Some((*only).clone()),
            _ => None,
        };
        index.set_default(&slot, next);
        message.push_str(&match index.default_of(&slot) {
            Some(next) => format!("; slot `{slot}` default is now {next}"),
            None => format!("; slot `{slot}` has no default"),
        });
    }
    normalize_slots(&mut index);
    index.save_locked(
        ctx.settings,
        &crate::catalog::HomeEvent::about(crate::catalog::EventKind::Removed, key.clone()),
        lock,
    )?;
    // The user layer follows the inventory: this package's keys leave every
    // collection, both pinned lists, the overrides and the usage facts, and
    // a seeded collection the removal emptied goes with them
    // (`PROP-059#launcher-catalog`).
    crate::catalog::drop_package(ctx.settings, &installation)?;
    // The system entries an export wrote for it go with it
    // (`PROP-059#launcher-runtime`).
    crate::export::forget_installation(ctx.settings, &installation)?;
    if let Some(directory) = doomed {
        fs::remove_dir_all(&directory)
            .map_err(|source| installer::io("removing", &directory, source))?;
    }
    let _ = fs::remove_dir_all(
        ctx.settings
            .app_proxies_dir()
            .join(installation.package.directory_name()),
    );
    crate::icons::remove_package_icons(ctx.settings, &installation.package);
    proxies::sync_locked(
        ctx.settings,
        &reserved,
        &reserved_names(&index),
        ctx.proxy_image,
        lock,
    )?;
    // The slot's links follow: the default moved to the next provider, or
    // the last provider went and the slot's link directories leave the
    // durable PATH (`PROP-059#path-directories`).
    crate::paths::apply_locked(ctx.settings, &mut index, &storage, ctx.environment, lock)?;
    Ok(Outcome {
        command: Command::Uninstall,
        slot_default: index.default_of(&slot).map(str::to_owned),
        slot,
        key,
        method: installation.method,
        app_root: launch::app_root(&installation, &storage).ok(),
        message,
        work: None,
        recommended: Vec::new(),
        suggested: Vec::new(),
    })
}

/// Roll back `entries` of `key` through the caller's user environment.
pub(crate) fn rollback_env(
    ctx: &OperationContext<'_>,
    key: &str,
    step: &str,
    entries: &[EnvEntry],
) -> Result<(), AppError> {
    if entries.is_empty() {
        return Ok(());
    }
    let environment = ctx
        .environment
        .ok_or_else(|| LifecycleError::NoEnvironment {
            key: key.into(),
            step: step.into(),
        })?;
    environment.remove(key, entries).map_err(|reason| {
        LifecycleError::Environment {
            key: key.into(),
            step: step.into(),
            reason,
        }
        .into()
    })
}

/// Run the authored removal in order, each script teed into the run record
/// and observed as one task; the first failure stops it.
pub(crate) fn undo_authored(
    ctx: &OperationContext<'_>,
    installation: &Installation,
    undo: &[Undo<'_>],
    force: bool,
    root: &Path,
    record: &mut RunRecord,
) -> Result<(), AppError> {
    let key = installation.package.key();
    let mut state = StepsFile {
        source_evidence: Default::default(),
        operation: None,
        package: installation.package.clone(),
        option: installation.option.clone(),
        features: installation.features.clone(),
        steps: Vec::new(),
    };
    for item in undo {
        let (id, commands) = match item {
            Undo::Env { step, entries } => {
                let owned: Vec<EnvEntry> = entries.iter().map(|entry| (*entry).clone()).collect();
                rollback_env(ctx, &key, step, &owned)?;
                continue;
            }
            Undo::Script { step, commands } => (*step, *commands),
        };
        let Some(script) = script_of(commands, force) else {
            continue;
        };
        let task = ctx.progress.task(format!("{id}: uninstall"));
        let log = record.log(id);
        let observe = ChildIo {
            relay: ctx.relay(),
            log: Some(&log),
            stdout_log: None,
            task: Some(&task),
            interrupt: ctx.interrupt,
            stdin_null: ctx.panel.is_some(),
            panel: ctx.panel,
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
            id: id.into(),
            kind: "shell".into(),
            outcome,
            reason: result.as_ref().err().map(ToString::to_string),
            started: Some(started),
            finished: Some(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
            uninstall: AuthoredCommands::default(),
            agent: None,
            env: Vec::new(),
            served: Vec::new(),
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
