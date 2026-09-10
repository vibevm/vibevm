//! `vibe self remove` and `vibe self gc` (PROP-019 §2.9, §2.10) over the
//! instance layout: removing a version drops all its instances and its
//! managed source clone; gc prunes non-active instances. A committer's
//! external tree is never touched.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#remove");

use std::fmt;
use std::fs;

use anyhow::{Context, Result};
use dialoguer::{MultiSelect, Select};

use super::error::VvmError;
use super::model::{self, InstallRecord, InstanceId, Selector, VersionId};
use super::store::VersionStore;
use super::{VvmEnv, confirm, forced_kind, require_tty, resolve_installed};
use crate::cli::{VvmGcArgs, VvmRemoveArgs};
use crate::output;

/// What `self remove` deletes for a version (PROP-019 §2.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RemoveScope {
    Bin,
    Src,
    Both,
}

fn removal_scope(bin: bool, src: bool) -> RemoveScope {
    match (bin, src) {
        (true, false) => RemoveScope::Bin,
        (false, true) => RemoveScope::Src,
        _ => RemoveScope::Both,
    }
}

/// Distinct version ids in the inventory, in first-seen order.
fn distinct_ids(state: &model::State) -> Vec<VersionId> {
    let mut ids: Vec<VersionId> = Vec::new();
    for r in &state.installs {
        let id = r.version_id();
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RemoveTarget {
    Version(VersionId),
    Instance(InstanceId),
}

impl RemoveTarget {
    fn version_id(&self) -> &VersionId {
        match self {
            RemoveTarget::Version(id) => id,
            RemoveTarget::Instance(id) => &id.version,
        }
    }

    fn matches(&self, record: &InstallRecord) -> bool {
        match self {
            RemoveTarget::Version(id) => &record.version_id() == id,
            RemoveTarget::Instance(id) => {
                record.version_id() == id.version && record.instance == id.instance
            }
        }
    }
}

impl fmt::Display for RemoveTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RemoveTarget::Version(id) => id.fmt(f),
            RemoveTarget::Instance(id) => id.fmt(f),
        }
    }
}

/// Remove a version id: all its instances (Bin/Both) and its managed source
/// clone (Src/Both). Never removes an external committer tree (it lives at
/// `source_path`, not under `src/`). Best-effort: a locked instance dir is
/// skipped (PROP-019 §2.9).
fn remove_target(
    ctx: &output::Context,
    store: &VersionStore,
    target: &RemoveTarget,
    scope: RemoveScope,
) -> Result<()> {
    let id = target.version_id();
    let mut removed = false;
    if matches!(scope, RemoveScope::Bin | RemoveScope::Both) {
        for rec in store
            .instances_of(id)?
            .into_iter()
            .filter(|record| target.matches(record))
        {
            let dir = store.instance_dir(id, rec.instance);
            let forget = if dir.exists() {
                match fs::remove_dir_all(&dir) {
                    Ok(()) => {
                        ctx.removed(&dir.display().to_string());
                        removed = true;
                        true
                    }
                    Err(e) => {
                        ctx.summary(&format!("skipped {} (in use?): {e}", dir.display()));
                        false
                    }
                }
            } else {
                removed = true;
                true
            };
            if forget {
                store.forget_instance(id, rec.instance)?;
            }
        }
    }
    if matches!(scope, RemoveScope::Src | RemoveScope::Both) {
        let src = store.src_dir(id);
        if src.exists() {
            fs::remove_dir_all(&src).with_context(|| format!("removing `{}`", src.display()))?;
            ctx.removed(&src.display().to_string());
            removed = true;
        }
    }
    if !removed {
        ctx.summary(&format!("{target}: nothing on disk to remove"));
    }
    Ok(())
}

pub(super) fn run_remove_cmd(
    ctx: &output::Context,
    env: &VvmEnv,
    args: VvmRemoveArgs,
) -> Result<()> {
    let store = env.store()?;
    let state = store.load_state()?;
    let active = store.active()?;

    let targets: Vec<RemoveTarget> = if args.all {
        let ids = distinct_ids(&state);
        if ids.is_empty() {
            ctx.summary("no versions installed.");
            return Ok(());
        }
        if !confirm(
            ctx,
            args.yes,
            &format!("Remove ALL {} version(s)?", ids.len()),
        )? {
            ctx.summary("aborted.");
            return Ok(());
        }
        ids.into_iter().map(RemoveTarget::Version).collect()
    } else if let Some(raw) = args.selector.as_deref() {
        let selector = model::Selector::parse(raw, forced_kind(&args.kind))?;
        let record = resolve_installed(&state, &selector, raw)?;
        vec![match selector {
            Selector::Exact(instance) => RemoveTarget::Instance(instance),
            _ => RemoveTarget::Version(record.version_id()),
        }]
    } else {
        pick_ids(ctx, &state)?
            .into_iter()
            .map(RemoveTarget::Version)
            .collect()
    };

    if targets.is_empty() {
        ctx.summary("nothing to remove.");
        return Ok(());
    }
    let scope = removal_scope(args.bin, args.src);
    let mut effective = Vec::new();
    for target in targets {
        if active
            .as_ref()
            .map(|record| target.matches(record))
            .unwrap_or(false)
            && !args.force
        {
            ctx.summary(&format!(
                "skipped active {} — use --force to remove it",
                active.as_ref().unwrap().selector()
            ));
            continue;
        }
        effective.push(target);
    }

    let previous = store.previous()?;
    let pointer_targeted = active
        .as_ref()
        .into_iter()
        .chain(previous.as_ref())
        .any(|record| effective.iter().any(|target| target.matches(record)));
    if matches!(scope, RemoveScope::Bin | RemoveScope::Both) && pointer_targeted {
        repair_activation_before_removal(&store, &state, &effective)?;
    }
    for target in &effective {
        remove_target(ctx, &store, target, scope)?;
    }
    ctx.summary("done.");
    Ok(())
}

fn repair_activation_before_removal(
    store: &VersionStore,
    state: &model::State,
    targets: &[RemoveTarget],
) -> Result<()> {
    let survivors: Vec<&InstallRecord> = state
        .installs
        .iter()
        .filter(|record| !targets.iter().any(|target| target.matches(record)))
        .filter(|record| {
            store
                .binary_path(&record.version_id(), record.instance)
                .is_file()
        })
        .collect();
    let active = store.active()?;
    let previous = store.previous()?;
    let current = active
        .as_ref()
        .and_then(|candidate| {
            survivors
                .iter()
                .copied()
                .find(|record| same_record(record, candidate))
        })
        .or_else(|| {
            previous.as_ref().and_then(|candidate| {
                survivors
                    .iter()
                    .copied()
                    .find(|record| same_record(record, candidate))
            })
        })
        .or_else(|| {
            survivors
                .iter()
                .copied()
                .max_by_key(|record| record.instance)
        });
    let rollback = survivors
        .iter()
        .copied()
        .find(|record| {
            previous
                .as_ref()
                .is_some_and(|candidate| same_record(record, candidate))
                && current.is_none_or(|active| !same_record(record, active))
        })
        .or_else(|| {
            survivors
                .iter()
                .copied()
                .filter(|record| current.is_none_or(|active| !same_record(record, active)))
                .max_by_key(|record| record.instance)
        });
    let current_path =
        current.map(|record| store.instance_dir(&record.version_id(), record.instance));
    let previous_path =
        rollback.map(|record| store.instance_dir(&record.version_id(), record.instance));
    store.reset_activation(current_path.as_deref(), previous_path.as_deref())?;
    Ok(())
}

fn same_record(left: &InstallRecord, right: &InstallRecord) -> bool {
    left.version_id() == right.version_id() && left.instance == right.instance
}

/// Interactively pick version ids to remove. A non-TTY / unattended run with
/// no selector is an error, not a wipe (PROP-019 §2.9).
fn pick_ids(ctx: &output::Context, state: &model::State) -> Result<Vec<VersionId>> {
    let ids = distinct_ids(state);
    if ids.is_empty() {
        ctx.summary("no versions installed.");
        return Ok(Vec::new());
    }
    require_tty(
        ctx,
        "no version selected: pass a selector (e.g. `vibe self remove tag:1.2.3`) or `--all`",
    )?;
    let labels: Vec<String> = ids.iter().map(|i| i.to_string()).collect();
    let chosen = MultiSelect::new()
        .with_prompt("Select versions to remove (space toggles, enter confirms)")
        .items(&labels)
        .interact()
        .unwrap_or_default();
    Ok(chosen.into_iter().map(|i| ids[i].clone()).collect())
}

enum GcAction {
    Build,
    Prune,
    Cancel,
}

pub(super) fn run_gc_cmd(ctx: &output::Context, env: &VvmEnv, args: VvmGcArgs) -> Result<()> {
    let store = env.store()?;
    let action = if args.build {
        GcAction::Build
    } else if args.prune_others {
        GcAction::Prune
    } else {
        gc_menu(ctx)?
    };

    match action {
        GcAction::Cancel => ctx.summary("nothing to do."),
        GcAction::Build => {
            let dir = store.build_dir();
            if dir.exists() {
                fs::remove_dir_all(&dir)
                    .with_context(|| format!("removing `{}`", dir.display()))?;
                ctx.summary("cleaned the Rust build cache.");
            } else {
                ctx.summary("build cache already empty.");
            }
        }
        GcAction::Prune => {
            let active = store.active()?.ok_or(VvmError::NoActiveVersion)?;
            let state = store.load_state()?;
            let saved_previous = store.previous()?;
            let rollback = saved_previous
                .as_ref()
                .filter(|record| {
                    !same_record(record, &active)
                        && store
                            .binary_path(&record.version_id(), record.instance)
                            .is_file()
                })
                .cloned()
                .or_else(|| {
                    state
                        .installs
                        .iter()
                        .filter(|record| !same_record(record, &active))
                        .filter(|record| {
                            store
                                .binary_path(&record.version_id(), record.instance)
                                .is_file()
                        })
                        .max_by_key(|record| record.instance)
                        .cloned()
                });
            let others: Vec<_> = state
                .installs
                .iter()
                .filter(|r| {
                    !same_record(r, &active)
                        && rollback.as_ref().is_none_or(|saved| !same_record(r, saved))
                })
                .cloned()
                .collect();
            let active_path = store.instance_dir(&active.version_id(), active.instance);
            let rollback_path = rollback
                .as_ref()
                .map(|record| store.instance_dir(&record.version_id(), record.instance));
            if others.is_empty() {
                store.reset_activation(Some(&active_path), rollback_path.as_deref())?;
                ctx.summary("no instances outside the active + rollback pair to prune.");
                return Ok(());
            }
            if !confirm(
                ctx,
                args.yes,
                &format!(
                    "Remove {} instance(s) except the active and immediate rollback? This cannot be undone.",
                    others.len()
                ),
            )? {
                ctx.summary("aborted.");
                return Ok(());
            }
            store.reset_activation(Some(&active_path), rollback_path.as_deref())?;
            let mut pruned = 0usize;
            for r in &others {
                let dir = store.instance_dir(&r.version_id(), r.instance);
                let forget = if dir.exists() {
                    match fs::remove_dir_all(&dir) {
                        Ok(()) => true,
                        Err(error) => {
                            ctx.summary(&format!("skipped {} (in use?): {error}", dir.display()));
                            false
                        }
                    }
                } else {
                    true
                };
                if forget {
                    store.forget_instance(&r.version_id(), r.instance)?;
                    pruned += 1;
                }
            }
            let dir = store.build_dir();
            if dir.exists() {
                let _ = fs::remove_dir_all(&dir);
            }
            ctx.summary(&format!(
                "pruned {} instance(s); kept the active and immediate rollback.",
                pruned
            ));
        }
    }
    Ok(())
}

fn gc_menu(ctx: &output::Context) -> Result<GcAction> {
    require_tty(
        ctx,
        "pass `--build` (clean the Rust build cache) or `--prune-others`",
    )?;
    let items = [
        "Clean the Rust build cache (the shared --target-dir)",
        "Prune all instances except the active",
    ];
    let sel = Select::new()
        .with_prompt("vibe self gc")
        .items(items)
        .default(0)
        .interact()
        .ok();
    Ok(match sel {
        Some(0) => GcAction::Build,
        Some(1) => GcAction::Prune,
        _ => GcAction::Cancel,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{ForcedKind, VvmGcArgs, VvmRemoveArgs};
    use crate::commands::vvm::model::{InstallRecord, Kind, Origin, Profile};
    use crate::commands::vvm::store::BINARY_NAME;
    use specmark::verifies;

    fn rec(kind: Kind, id: &str, instance: u64) -> InstallRecord {
        InstallRecord {
            kind,
            id: id.into(),
            instance,
            commit: "c".into(),
            toolchain: "t".into(),
            profile: Profile::Debug,
            installed_at: "now".into(),
            origin: Origin::Managed,
            source_path: None,
            payload_sha256: None,
        }
    }

    fn quiet() -> output::Context {
        output::Context::from_flags(true, false, None, true, crate::cli::AgentModeArg::Auto)
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#remove", r = 1)]
    fn removal_scope_defaults_to_both() {
        assert_eq!(removal_scope(false, false), RemoveScope::Both);
        assert_eq!(removal_scope(true, false), RemoveScope::Bin);
        assert_eq!(removal_scope(false, true), RemoveScope::Src);
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#remove", r = 1)]
    fn remove_id_drops_all_instances_and_forgets() {
        let tmp = tempfile::tempdir().unwrap();
        let store = VersionStore::new(tmp.path());
        let id = VersionId::new(Kind::Tag, "1.0.0");
        for n in [1u64, 2] {
            store.record_install(rec(Kind::Tag, "1.0.0", n)).unwrap();
            fs::create_dir_all(store.instance_dir(&id, n)).unwrap();
        }
        remove_target(
            &quiet(),
            &store,
            &RemoveTarget::Version(id.clone()),
            RemoveScope::Both,
        )
        .unwrap();
        assert!(!store.instance_dir(&id, 1).exists());
        assert!(!store.instance_dir(&id, 2).exists());
        assert!(store.instances_of(&id).unwrap().is_empty());
    }

    fn no_forced_kind() -> ForcedKind {
        ForcedKind {
            tag: false,
            branch: false,
            commit: false,
        }
    }

    fn installed_store(instances: &[u64]) -> (tempfile::TempDir, VersionStore, VersionId) {
        let tmp = tempfile::tempdir().unwrap();
        let store = VersionStore::new(tmp.path());
        let id = VersionId::new(Kind::Tag, "1.0.0");
        for &instance in instances {
            store
                .record_install(rec(Kind::Tag, "1.0.0", instance))
                .unwrap();
            let dir = store.instance_dir(&id, instance);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join(BINARY_NAME), format!("payload-{instance}")).unwrap();
            store.write_current(&dir).unwrap();
        }
        (tmp, store, id)
    }

    #[test]
    fn forced_active_exact_removal_repoints_current_to_rollback_first() {
        let (tmp, store, id) = installed_store(&[1, 2]);
        let env = VvmEnv {
            root: Some(tmp.path().to_path_buf()),
            ..VvmEnv::default()
        };
        run_remove_cmd(
            &quiet(),
            &env,
            VvmRemoveArgs {
                selector: Some("tag:1.0.0#2".into()),
                kind: no_forced_kind(),
                all: false,
                bin: false,
                src: false,
                force: true,
                yes: true,
            },
        )
        .unwrap();

        assert_eq!(store.active().unwrap().unwrap().instance, 1);
        assert!(!store.instance_dir(&id, 2).exists());
        assert!(store.instance_dir(&id, 1).exists());
        assert!(store.previous().unwrap().is_none());
    }

    #[test]
    fn removing_saved_rollback_repoints_previous_without_moving_current() {
        let (tmp, store, id) = installed_store(&[1, 2, 3]);
        let env = VvmEnv {
            root: Some(tmp.path().to_path_buf()),
            ..VvmEnv::default()
        };
        run_remove_cmd(
            &quiet(),
            &env,
            VvmRemoveArgs {
                selector: Some("tag:1.0.0#2".into()),
                kind: no_forced_kind(),
                all: false,
                bin: false,
                src: false,
                force: false,
                yes: true,
            },
        )
        .unwrap();

        assert_eq!(store.active().unwrap().unwrap().instance, 3);
        assert_eq!(store.previous().unwrap().unwrap().instance, 1);
        assert!(!store.instance_dir(&id, 2).exists());
    }

    #[test]
    fn gc_keeps_active_and_immediate_rollback_instances() {
        let (tmp, store, id) = installed_store(&[1, 2, 3]);
        let env = VvmEnv {
            root: Some(tmp.path().to_path_buf()),
            ..VvmEnv::default()
        };
        run_gc_cmd(
            &quiet(),
            &env,
            VvmGcArgs {
                build: false,
                prune_others: true,
                yes: true,
            },
        )
        .unwrap();

        assert!(!store.instance_dir(&id, 1).exists());
        assert!(store.instance_dir(&id, 2).exists());
        assert!(store.instance_dir(&id, 3).exists());
        assert_eq!(store.active().unwrap().unwrap().instance, 3);
        assert_eq!(store.previous().unwrap().unwrap().instance, 2);
    }
}
