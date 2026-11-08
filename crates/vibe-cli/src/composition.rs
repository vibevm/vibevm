//! Small composition-root helpers that keep ambient input assembly out of dispatch.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#vars");

use anyhow::Result;

use crate::cli::VarsArgs;
use crate::commands;
use crate::output;
use vibe_core::user_config::UserConfig;

pub(crate) mod cancellation;
pub(crate) mod remap;
pub(crate) mod transfers;

pub(crate) fn run_vars(
    args: VarsArgs,
    self_location: Option<&commands::vvm::SelfLocation>,
    unattended: bool,
    invoked_by: Option<&str>,
) -> Result<()> {
    let install_base = commands::vvm::resolve_root(
        self_location.map(|location| location.root.clone()),
        super::read_env_opt(commands::vvm::VIBEVM_INSTALL_ROOT_ENV).map(Into::into),
        dirs::home_dir(),
    )
    .and_then(|root| root.parent().map(|parent| parent.display().to_string()))
    .unwrap_or_default();
    let home = self_location
        .map(|location| location.home.display().to_string())
        .or_else(|| super::read_env_opt(commands::vvm::VIBEVM_HOME_ENV))
        .unwrap_or_else(|| "(none)".to_string());
    let (invoked_by, _) = crate::output::resolve_invoked_by(invoked_by);
    commands::vars::run(
        args,
        vec![
            commands::vars::VarRow::new(
                "VIBEVM_INSTALL_ROOT",
                install_base,
                super::read_env_opt(commands::vvm::VIBEVM_INSTALL_ROOT_ENV),
            ),
            commands::vars::VarRow::new(
                "VIBEVM_HOME",
                home,
                super::read_env_opt(commands::vvm::VIBEVM_HOME_ENV),
            ),
            commands::vars::VarRow::new(
                "VIBE_INVOKED_BY",
                invoked_by.unwrap_or_default(),
                super::read_env_opt("VIBE_INVOKED_BY"),
            ),
            commands::vars::VarRow::new(
                "VIBE_UNATTENDED",
                crate::output::resolve_unattended(unattended).to_string(),
                super::read_env_opt("VIBE_UNATTENDED"),
            ),
            commands::vars::VarRow::new(
                "VIBE_LOG",
                super::read_env_opt("VIBE_LOG").unwrap_or_else(|| "warn".to_string()),
                super::read_env_opt("VIBE_LOG"),
            ),
        ],
    )
}

/// The offline posture `vibe self` runs under (PROP-019 `##CMD-OFFLINE`).
///
/// The same ladder every registry-touching command resolves — the root
/// `--offline` flag, then `VIBE_OFFLINE`, then the user-config
/// `[net] offline` key (PROP-010 `##OFFLINE-LAYERING`) — through the same
/// [`output::resolve_offline`], so the two cannot drift. It is resolved
/// HERE because both of the lower rungs are ambient reads and the version
/// manager's domain is forbidden them: it is handed the verdict as data.
///
/// A user config that cannot be parsed has already been reported once by
/// [`promote_user_config_env`] at startup. Failing the command a second
/// time over an optional layer would be the wrong trade, so its rung
/// falls back to its own default — online — and the two rungs above it
/// still decide.
pub(crate) fn vvm_offline(cli_offline: bool) -> bool {
    output::resolve_offline(
        cli_offline,
        UserConfig::load()
            .map(|cfg| cfg.net.offline)
            .unwrap_or(false),
    )
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;

/// Whether a command touches the registry layer (and thus benefits from the
/// default `~/.vibe/registry.toml` being seeded). Commands like `version` /
/// `vars` / `tree` / `term` don't resolve packages, so they skip the seed
/// check. This is advisory — `ensure_default_global_registry` is a cheap
/// file-existence check regardless.
/// The no-publication project route, selected before startup side effects.
pub(crate) fn project_preview(cmd: &crate::cli::Command) -> bool {
    matches!(cmd, crate::cli::Command::Install(args) if args.plan_only && !args.global)
}

pub(crate) fn needs_global_registry(cmd: &crate::cli::Command) -> bool {
    if project_preview(cmd) {
        return false;
    }
    use crate::cli::Command;
    matches!(
        cmd,
        Command::Init(_)
            | Command::Outdated(_)
            | Command::Search(_)
            | Command::Registry(_)
            | Command::Cache(_)
    )
}

mod outcome;
pub(crate) use outcome::finish;
