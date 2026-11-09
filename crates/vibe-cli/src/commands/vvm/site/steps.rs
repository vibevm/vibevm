//! The three machine steps of `vibe self apply`: the consented PATH fix,
//! the import of an unmanaged running binary, and the px companion a
//! profile that selects px cannot work without (`PROP-019#site-profile`,
//! `#companion-tools`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile");

use std::io::IsTerminal;
use std::path::Path;

use anyhow::{Context, Result};
use vibe_net::px_store::PxPin;

use super::super::env::{
    self, PathFixConsent, PathFixDecision, PathFixPolicy, Shell, ensure_shim_dir_on_durable_path,
};
use super::super::store::VersionStore;
use super::super::{VvmEnv, import, install::InstallLock, make_persister, model, provenance};
use super::report;
use crate::output;
use vibe_wire::generated::self_apply_report::{Imported, PathPlan, PathReport, PxOutcome, PxStep};

/// The PATH step (`PROP-019#path`): with consent, put the shim directory on
/// the durable PATH through the platform persister — on Windows the
/// registry API, never a shell. A dry run only reports the decision.
/// `install_root` is the base a configured install root names
/// (`PROP-019#install-root`); a performed step that was allowed to touch
/// the durable environment also makes `$VIBEVM_INSTALL_ROOT` durable, so
/// the next terminal resolves the same store. One consent governs both
/// writes: a step skipped for want of consent writes neither.
pub(super) fn path_step(
    ctx: &output::Context,
    env: &VvmEnv,
    store: &VersionStore,
    flag: Option<bool>,
    policy: PathFixPolicy,
    dry_run: bool,
    install_root: Option<&std::path::Path>,
) -> Result<PathReport> {
    let shim_dir = store.shim_dir();
    let persister = make_persister(env, Shell::detect(env.shell.as_deref()))?;
    let interactive = !ctx.is_unattended() && std::io::stdin().is_terminal();
    let consent = PathFixConsent::new(flag, policy, interactive);
    if dry_run {
        let before = persister.inspect_path(&shim_dir);
        let on_path = before.path_entry == env::PathEntryStatus::Correct;
        let (planned, consent_source, skip_reason) = match (on_path, env::decide(&consent)) {
            (true, _) => (PathPlan::AlreadyOnPath, None, None),
            (false, PathFixDecision::Apply(source)) => (PathPlan::WouldApply, Some(source), None),
            (false, PathFixDecision::Ask { .. }) => (PathPlan::WouldAsk, None, None),
            (false, PathFixDecision::Skip(reason)) => (PathPlan::WouldSkip, None, Some(reason)),
        };
        // A planned step reached no persister: it carries `planned` and the
        // target it would write, and none of a performed step's members.
        return Ok(PathReport {
            shim_dir: shim_dir.display().to_string(),
            outcome: None,
            was_on_durable_path: None,
            on_durable_path: None,
            planned: Some(planned),
            consent: consent_source.map(report::consent_source),
            skip_reason: skip_reason.map(report::skip_reason),
            changed: None,
            activation_hint: None,
            fix_hint: None,
            durable_environment: None,
            target: before.target,
        });
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
    if let Some(root) = install_root
        && observation.outcome != env::PathFixOutcome::Skipped
    {
        persister.set_install_root(root).with_context(|| {
            format!(
                "making ${} = `{}` durable",
                crate::commands::vvm::VIBEVM_INSTALL_ROOT_ENV,
                root.display()
            )
        })?;
    }
    Ok(report::path_done(observation))
}

/// From an unmanaged binary — a `vibe` downloaded by hand into a closed
/// network — with no active instance, import it as the active instance
/// first so the store and the stable shims exist (`self import --use`).
pub(super) fn import_running_if_unmanaged(
    ctx: &output::Context,
    dry_run: bool,
    store: &VersionStore,
) -> Result<Option<Imported>> {
    if provenance::running_record(store)?.is_some() || store.active()?.is_some() {
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
    let _lock = InstallLock::acquire(store)?;
    let now = chrono::Utc::now().to_rfc3339();
    let outcome = import::perform_import(
        ctx,
        store,
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
    env::activate_without_environment(store, &outcome.home)?;
    Ok(Some(Imported {
        executable: executable.display().to_string(),
        selector: outcome.record.selector().to_string(),
        reused: outcome.reused,
        planned: false,
    }))
}

/// The directory the running vibe sits in — on a closed network, the
/// unpacked release, and the only place the px companion archive can be.
fn running_dir() -> Option<std::path::PathBuf> {
    std::env::current_exe()
        .ok()?
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .map(Path::to_path_buf)
}

/// The px step (`PROP-019#companion-tools`): a profile that selects
/// `[proxy] mode = "px"` configures a route that cannot work without px,
/// and the release zip does not carry px — it is a companion asset the
/// operator puts beside `vibe` — so apply makes sure px is in the tool
/// store of the install root THIS profile chooses, and says so in the
/// report when it is not.
///
/// It runs whether or not the import step ran, so adding the companion
/// archive and re-running `vibe self apply` installs px; it is idempotent,
/// so the run after that changes nothing.
pub(super) fn px_step(
    store: &VersionStore,
    needed: bool,
    dry_run: bool,
    imported: bool,
    progress: &vibe_core::progress::Progress,
) -> Result<PxStep> {
    px_step_with(
        store,
        PxPin::embedded(),
        running_dir().as_deref(),
        needed,
        dry_run,
        imported,
        progress,
    )
}

/// The px step against an explicit pin and release directory, for the
/// tests that need a px archive this platform's pinned digest does not
/// forbid them to fabricate.
pub(super) fn px_step_with(
    store: &VersionStore,
    pin: &PxPin,
    dir: Option<&Path>,
    needed: bool,
    dry_run: bool,
    imported: bool,
    progress: &vibe_core::progress::Progress,
) -> Result<PxStep> {
    use super::super::bundle::px_lane::{self, PxPresence};
    let presence = px_lane::ensure_px_for_apply(
        store, pin, dir, needed, dry_run,
        // A performed import has just run this lane against the same
        // directory and reported on its own line; nothing is said twice.
        !imported, progress,
    )?;
    let store_dir = px_lane::px_store_dir(store, pin);
    let asset = px_lane::px_asset_name(pin);
    let (outcome, program) = match &presence {
        PxPresence::NotNeeded => (PxOutcome::NotNeeded, None),
        PxPresence::Present(program) => (PxOutcome::AlreadyInstalled, Some(program.clone())),
        PxPresence::Placed { program, .. } => (PxOutcome::Installed, Some(program.clone())),
        PxPresence::WouldPlace { .. } => (PxOutcome::Installed, None),
        PxPresence::Absent => (PxOutcome::Missing, None),
        PxPresence::NoTarget => (PxOutcome::NoTarget, None),
    };
    let fix = match (&outcome, &asset) {
        (PxOutcome::Missing, Some(asset)) => Some(format!(
            "px is not installed, and this profile's `[proxy] mode = \"px\"` cannot work without \
             it: the release zip does not carry px — put `{asset}` beside `vibe` (or in \
             `tools/px/` there) and run `vibe self px install --from <directory-or-zip>`; px is \
             installed into `{}`",
            store_dir.display()
        )),
        (PxOutcome::Missing, None) | (PxOutcome::NoTarget, _) => Some(
            "px has no default build for this platform, and this profile's `[proxy] mode = \"px\"` \
             needs one: install a px of your own with `vibe self px install --from \
             <archive|directory|executable>`, or name one with `[proxy] px_path`"
                .to_owned(),
        ),
        _ => None,
    };
    Ok(PxStep {
        needed,
        outcome,
        store: store_dir.display().to_string(),
        asset,
        program: program.map(|path| path.display().to_string()),
        fix,
        planned: dry_run,
    })
}

#[cfg(test)]
#[path = "px_step_tests.rs"]
mod px_step_tests;
