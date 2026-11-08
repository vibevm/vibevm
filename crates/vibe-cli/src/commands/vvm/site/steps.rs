//! The two machine steps of `vibe self apply`: the consented PATH fix and
//! the import of an unmanaged running binary (`PROP-019#site-profile`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile");

use std::io::IsTerminal;

use anyhow::{Context, Result};

use super::super::env::{
    self, PathFixConsent, PathFixDecision, PathFixPolicy, Shell, ensure_shim_dir_on_durable_path,
};
use super::super::store::VersionStore;
use super::super::{VvmEnv, import, install::InstallLock, make_persister, model, provenance};
use super::report::{Imported, PathPreview, PathReport};
use crate::output;

/// The PATH step (`PROP-019#path`): with consent, put the shim directory on
/// the durable PATH through the platform persister — on Windows the
/// registry API, never a shell. A dry run only reports the decision.
pub(super) fn path_step(
    ctx: &output::Context,
    env: &VvmEnv,
    store: &VersionStore,
    flag: Option<bool>,
    policy: PathFixPolicy,
    dry_run: bool,
) -> Result<PathReport> {
    let shim_dir = store.shim_dir();
    let persister = make_persister(env, Shell::detect(env.shell.as_deref()))?;
    let interactive = !ctx.is_unattended() && std::io::stdin().is_terminal();
    let consent = PathFixConsent::new(flag, policy, interactive);
    if dry_run {
        let before = persister.inspect_path(&shim_dir);
        let on_path = before.path_entry == env::PathEntryStatus::Correct;
        let (planned, consent_source, skip_reason) = match (on_path, env::decide(&consent)) {
            (true, _) => ("already_on_path", None, None),
            (false, PathFixDecision::Apply(source)) => ("would_apply", Some(source), None),
            (false, PathFixDecision::Ask { .. }) => ("would_ask", None, None),
            (false, PathFixDecision::Skip(reason)) => ("would_skip", None, Some(reason)),
        };
        return Ok(PathReport::Planned(PathPreview {
            shim_dir: shim_dir.display().to_string(),
            planned,
            consent: consent_source,
            skip_reason,
            target: before.target,
        }));
    }
    let observation = ensure_shim_dir_on_durable_path(
        &*persister,
        &shim_dir,
        &consent,
        &mut |question, default| {
            ctx.suspend_progress(|| {
                crate::output::prompts::confirm(question, default).unwrap_or(false)
            })
        },
    )?;
    Ok(PathReport::Done(observation))
}

/// From an unmanaged binary — a `vibe` downloaded by hand into a closed
/// network — with no active instance, import it as the active instance
/// first so the store and the stable shims exist (`self import --use`).
pub(super) fn import_running_if_unmanaged(
    ctx: &output::Context,
    env: &VvmEnv,
    dry_run: bool,
) -> Result<Option<Imported>> {
    let store = env.store()?;
    if provenance::running_record(&store)?.is_some() || store.active()?.is_some() {
        return Ok(None);
    }
    let executable = std::env::current_exe().context("locating the running vibe")?;
    let version = env!("CARGO_PKG_VERSION");
    if dry_run {
        return Ok(Some(Imported {
            executable: executable.display().to_string(),
            selector: version.to_owned(),
            reused: false,
            planned: true,
        }));
    }
    let _lock = InstallLock::acquire(&store)?;
    let now = chrono::Utc::now().to_rfc3339();
    let outcome = import::perform_import(
        ctx,
        &store,
        &import::ImportRequest {
            executable: &executable,
            tag: version,
            commit: None,
            profile: if cfg!(debug_assertions) {
                model::Profile::Debug
            } else {
                model::Profile::Release
            },
            replace_candidate: false,
            now: &now,
        },
    )?;
    env::activate_without_environment(&store, &outcome.home)?;
    Ok(Some(Imported {
        executable: executable.display().to_string(),
        selector: outcome.record.selector().to_string(),
        reused: outcome.reused,
        planned: false,
    }))
}
