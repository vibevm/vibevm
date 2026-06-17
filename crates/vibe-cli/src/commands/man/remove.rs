//! `vibe man remove` and `vibe man gc` (PROP-019 §2.9, §2.10) over the
//! instance layout: removing a version drops all its instances and its
//! managed source clone; gc prunes non-active instances. A committer's
//! external tree is never touched.

specmark::scope!("spec://vibevm/common/PROP-019#remove");

use std::fs;
use std::io::IsTerminal;

use anyhow::{Context, Result, bail};
use dialoguer::{MultiSelect, Select};

use super::model::{self, VersionId};
use super::store::VersionStore;
use super::{ManEnv, confirm, forced_kind, resolve_installed};
use crate::cli::{ManGcArgs, ManRemoveArgs};
use crate::output;

/// What `man remove` deletes for a version (PROP-019 §2.9).
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

/// Remove a version id: all its instances (Bin/Both) and its managed source
/// clone (Src/Both). Never removes an external committer tree (it lives at
/// `source_path`, not under `src/`). Best-effort: a locked instance dir is
/// skipped (PROP-019 §2.9).
fn remove_id(
    ctx: &output::Context,
    store: &VersionStore,
    id: &VersionId,
    scope: RemoveScope,
) -> Result<()> {
    let mut removed = false;
    if matches!(scope, RemoveScope::Bin | RemoveScope::Both) {
        for rec in store.instances_of(id)? {
            let dir = store.instance_dir(id, rec.instance);
            if dir.exists() {
                match fs::remove_dir_all(&dir) {
                    Ok(()) => {
                        ctx.removed(&dir.display().to_string());
                        removed = true;
                    }
                    Err(e) => ctx.summary(&format!("skipped {} (in use?): {e}", dir.display())),
                }
            }
        }
        store.forget_id(id)?;
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
        ctx.summary(&format!("{id}: nothing on disk to remove"));
    }
    Ok(())
}

pub(super) fn run_remove_cmd(
    ctx: &output::Context,
    env: &ManEnv,
    args: ManRemoveArgs,
) -> Result<()> {
    let store = env.store()?;
    let state = store.load_state()?;
    let active = store.active()?;

    let targets: Vec<VersionId> = if args.all {
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
        ids
    } else if let Some(raw) = args.selector.as_deref() {
        let selector =
            model::Selector::parse(raw, forced_kind(args.tag, args.branch, args.commit))?;
        vec![resolve_installed(&state, &selector, raw)?.version_id()]
    } else {
        pick_ids(ctx, &state)?
    };

    if targets.is_empty() {
        ctx.summary("nothing to remove.");
        return Ok(());
    }
    let scope = removal_scope(args.bin, args.src);
    for id in &targets {
        if active
            .as_ref()
            .map(|a| &a.version_id() == id)
            .unwrap_or(false)
            && !args.force
        {
            ctx.summary(&format!(
                "skipped active {id} — use --force to remove the active version"
            ));
            continue;
        }
        remove_id(ctx, &store, id, scope)?;
    }
    ctx.summary("done.");
    Ok(())
}

/// Interactively pick version ids to remove. A non-TTY / unattended run with
/// no selector is an error, not a wipe (PROP-019 §2.9).
fn pick_ids(ctx: &output::Context, state: &model::State) -> Result<Vec<VersionId>> {
    let ids = distinct_ids(state);
    if ids.is_empty() {
        ctx.summary("no versions installed.");
        return Ok(Vec::new());
    }
    if ctx.is_unattended() || !std::io::stdin().is_terminal() {
        bail!("no version selected: pass a selector (e.g. `vibe man remove tag:1.2.3`) or `--all`");
    }
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

pub(super) fn run_gc_cmd(ctx: &output::Context, env: &ManEnv, args: ManGcArgs) -> Result<()> {
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
            let active = store.active()?;
            let state = store.load_state()?;
            let others: Vec<_> = state
                .installs
                .iter()
                .filter(|r| {
                    active
                        .as_ref()
                        .map(|a| !(a.version_id() == r.version_id() && a.instance == r.instance))
                        .unwrap_or(true)
                })
                .cloned()
                .collect();
            if others.is_empty() {
                ctx.summary("no other instances to prune.");
                return Ok(());
            }
            if !confirm(
                ctx,
                args.yes,
                &format!(
                    "Remove {} instance(s) except the active? This cannot be undone.",
                    others.len()
                ),
            )? {
                ctx.summary("aborted.");
                return Ok(());
            }
            for r in &others {
                let dir = store.instance_dir(&r.version_id(), r.instance);
                if dir.exists() {
                    let _ = fs::remove_dir_all(&dir);
                }
                store.forget_instance(&r.version_id(), r.instance)?;
            }
            let dir = store.build_dir();
            if dir.exists() {
                let _ = fs::remove_dir_all(&dir);
            }
            ctx.summary(&format!(
                "pruned {} instance(s); kept the active.",
                others.len()
            ));
        }
    }
    Ok(())
}

fn gc_menu(ctx: &output::Context) -> Result<GcAction> {
    if ctx.is_unattended() || !std::io::stdin().is_terminal() {
        bail!("pass `--build` (clean the Rust build cache) or `--prune-others`");
    }
    let items = [
        "Clean the Rust build cache (the shared --target-dir)",
        "Prune all instances except the active",
    ];
    let sel = Select::new()
        .with_prompt("vibe man gc")
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
    use crate::commands::man::model::{InstallRecord, Kind, Origin};
    use specmark::verifies;

    fn rec(kind: Kind, id: &str, instance: u64) -> InstallRecord {
        InstallRecord {
            kind,
            id: id.into(),
            instance,
            commit: "c".into(),
            toolchain: "t".into(),
            profile: "debug".into(),
            installed_at: "now".into(),
            origin: Origin::Managed,
            source_path: None,
        }
    }

    fn quiet() -> output::Context {
        output::Context::from_flags(true, false, None, true)
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#remove", r = 1)]
    fn removal_scope_defaults_to_both() {
        assert_eq!(removal_scope(false, false), RemoveScope::Both);
        assert_eq!(removal_scope(true, false), RemoveScope::Bin);
        assert_eq!(removal_scope(false, true), RemoveScope::Src);
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#remove", r = 1)]
    fn remove_id_drops_all_instances_and_forgets() {
        let tmp = tempfile::tempdir().unwrap();
        let store = VersionStore::new(tmp.path());
        let id = VersionId::new(Kind::Tag, "1.0.0");
        for n in [1u64, 2] {
            store.record_install(rec(Kind::Tag, "1.0.0", n)).unwrap();
            fs::create_dir_all(store.instance_dir(&id, n)).unwrap();
        }
        remove_id(&quiet(), &store, &id, RemoveScope::Both).unwrap();
        assert!(!store.instance_dir(&id, 1).exists());
        assert!(!store.instance_dir(&id, 2).exists());
        assert!(store.instances_of(&id).unwrap().is_empty());
    }
}
