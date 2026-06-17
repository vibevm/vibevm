//! `vibe man remove` and `vibe man gc` — removing versions and reclaiming
//! disk (PROP-019 §2.9, §2.10). Split from the dispatcher so each file stays
//! within the module-grain file budget.

specmark::scope!("spec://vibevm/common/PROP-019#remove");

use std::fs;
use std::io::IsTerminal;

use anyhow::{Context, Result, bail};
use dialoguer::{MultiSelect, Select};

use super::store::VersionStore;
use super::{ManEnv, confirm, forced_kind, model, resolve_installed};
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

/// Delete a version's binary and/or source tree and forget it from the
/// inventory (PROP-019 §2.9). Removing only the source keeps the record.
fn remove_one(
    ctx: &output::Context,
    store: &VersionStore,
    id: &model::VersionId,
    scope: RemoveScope,
) -> Result<()> {
    let mut removed = false;
    if matches!(scope, RemoveScope::Bin | RemoveScope::Both) {
        let prefix = store.version_prefix(id);
        if prefix.exists() {
            fs::remove_dir_all(&prefix)
                .with_context(|| format!("removing `{}`", prefix.display()))?;
            ctx.removed(&prefix.display().to_string());
            removed = true;
        }
        store.forget(id)?;
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
    let active_id = store
        .active(env.active_home.as_deref())?
        .map(|r| r.version_id());

    let targets: Vec<model::VersionId> = if args.all {
        if state.installs.is_empty() {
            ctx.summary("no versions installed.");
            return Ok(());
        }
        if !confirm(
            ctx,
            args.yes,
            &format!("Remove ALL {} installed version(s)?", state.installs.len()),
        )? {
            ctx.summary("aborted.");
            return Ok(());
        }
        state.installs.iter().map(|r| r.version_id()).collect()
    } else if let Some(raw) = args.selector.as_deref() {
        let selector =
            model::Selector::parse(raw, forced_kind(args.tag, args.branch, args.commit))?;
        vec![resolve_installed(&state, &selector, raw)?]
    } else {
        pick_versions(ctx, &state)?
    };

    if targets.is_empty() {
        ctx.summary("nothing to remove.");
        return Ok(());
    }
    let scope = removal_scope(args.bin, args.src);
    for id in &targets {
        if active_id.as_ref() == Some(id) && !args.force {
            ctx.summary(&format!(
                "skipped active {id} — use --force to remove the active version"
            ));
            continue;
        }
        remove_one(ctx, &store, id, scope)?;
    }
    ctx.summary("done.");
    Ok(())
}

/// Interactively pick installed versions to remove. A non-TTY / unattended
/// run with no selector is an error, not a wipe (PROP-019 §2.9).
fn pick_versions(ctx: &output::Context, state: &model::State) -> Result<Vec<model::VersionId>> {
    if state.installs.is_empty() {
        ctx.summary("no versions installed.");
        return Ok(Vec::new());
    }
    if ctx.is_unattended() || !std::io::stdin().is_terminal() {
        bail!("no version selected: pass a selector (e.g. `vibe man remove tag:1.2.3`) or `--all`");
    }
    let labels: Vec<String> = state
        .installs
        .iter()
        .map(|r| r.version_id().to_string())
        .collect();
    let chosen = MultiSelect::new()
        .with_prompt("Select versions to remove (space toggles, enter confirms)")
        .items(&labels)
        .interact()
        .unwrap_or_default();
    Ok(chosen
        .into_iter()
        .map(|i| state.installs[i].version_id())
        .collect())
}

enum GcAction {
    Build,
    Prune,
    Cancel,
}

pub(super) fn run_gc_cmd(ctx: &output::Context, env: &ManEnv, args: ManGcArgs) -> Result<()> {
    let store = env.store()?;
    let state = store.load_state()?;
    let active_id = store
        .active(env.active_home.as_deref())?
        .map(|r| r.version_id());

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
            // The shared build cache is the only Rust garbage today (one
            // managed --target-dir for every build), so "current" and "all"
            // collapse to it. Never touches the shared ~/.cargo caches.
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
            let others: Vec<model::VersionId> = state
                .installs
                .iter()
                .map(|r| r.version_id())
                .filter(|id| active_id.as_ref() != Some(id))
                .collect();
            if others.is_empty() {
                ctx.summary("no other versions to prune.");
                return Ok(());
            }
            if !confirm(
                ctx,
                args.yes,
                &format!(
                    "Remove {} version(s) except the current, including sources? \
                     This cannot be undone.",
                    others.len()
                ),
            )? {
                ctx.summary("aborted.");
                return Ok(());
            }
            for id in &others {
                remove_one(ctx, &store, id, RemoveScope::Both)?;
            }
            let dir = store.build_dir();
            if dir.exists() {
                let _ = fs::remove_dir_all(&dir);
            }
            ctx.summary(&format!(
                "pruned {} version(s); kept the current.",
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
        "Prune all versions except the current (incl. sources)",
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
    use super::{RemoveScope, removal_scope, remove_one};
    use crate::commands::man::model::{InstallRecord, Kind, VersionId};
    use crate::commands::man::store::VersionStore;
    use crate::output;
    use specmark::verifies;

    fn rec(kind: Kind, id: &str) -> InstallRecord {
        InstallRecord {
            kind,
            id: id.into(),
            commit: "c".into(),
            toolchain: "t".into(),
            profile: "debug".into(),
            installed_at: "now".into(),
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
    fn remove_one_deletes_dirs_and_forgets() {
        let tmp = tempfile::tempdir().unwrap();
        let store = VersionStore::new(tmp.path());
        let id = VersionId::new(Kind::Tag, "1.0.0");
        store.record_install(rec(Kind::Tag, "1.0.0")).unwrap();
        std::fs::create_dir_all(store.version_prefix(&id)).unwrap();
        std::fs::create_dir_all(store.src_dir(&id)).unwrap();

        remove_one(&quiet(), &store, &id, RemoveScope::Both).unwrap();
        assert!(!store.version_prefix(&id).exists());
        assert!(!store.src_dir(&id).exists());
        assert!(store.load_state().unwrap().installs.is_empty());
    }
}
