//! Small composition-root helpers that keep ambient input assembly out of dispatch.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#vars");

use anyhow::Result;

use crate::cli::VarsArgs;
use crate::commands;
use crate::output;
use vibe_core::user_config::UserConfig;

pub(crate) mod cancellation;
pub(crate) mod native_helper;
pub(crate) mod network;
pub(crate) mod remap;
pub(crate) mod templates;
pub(crate) mod transfers;

pub(crate) fn run_vars(
    args: VarsArgs,
    self_location: Option<&commands::vvm::SelfLocation>,
    unattended: bool,
    invoked_by: Option<&str>,
) -> Result<()> {
    let install_base = commands::vvm::resolve_root(
        self_location.map(|location| location.root.clone()),
        install_base_override(),
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

/// The install base every command resolves its store under, below the
/// running version's own root (PROP-019 `#install-root`):
/// `$VIBEVM_INSTALL_ROOT`, then the configured `[self] install_root` — the
/// user's own `config.toml` over the applied site profile, exactly as
/// [`UserConfig::load`] layers them. The `~/.vibe` default belongs to
/// `resolve_root`, which owns the `/opt` suffix too.
///
/// A declared `~` / `~/<dir>` is expanded against the home directory
/// `~/.vibe` itself is read from (`PROP-019#install-root`), so a site
/// profile written for Macs and Linux boxes can say `install_root = "~"`
/// and vibe lives in each reader's own `~/opt`.
///
/// A config that cannot be parsed has already been reported once at
/// startup, so this rung falls back to its own absence rather than failing
/// a second command over an optional file — and so does a declared root
/// that names no directory, which `vibe self apply` and `vibe self apply
/// --show` refuse by name where the operator wrote it.
pub(crate) fn install_base_override() -> Option<std::path::PathBuf> {
    if let Some(base) = super::read_env_opt(commands::vvm::VIBEVM_INSTALL_ROOT_ENV) {
        return Some(base.into());
    }
    UserConfig::load()
        .ok()?
        .self_source
        .install_root_at(dirs::home_dir().as_deref())?
        .ok()
}

/// The installation `vibe self …` acts on, and the one `vibe tool px
/// status` reports: the running version's own root (current_exe-derived)
/// when managed, else `$VIBEVM_INSTALL_ROOT/opt`, else `~/.vibe/opt`
/// (PROP-019 §2.5). Every ambient read of it belongs to this root.
pub(crate) fn vvm_env(
    self_location: Option<&commands::vvm::SelfLocation>,
    cli_offline: bool,
) -> commands::vvm::VvmEnv {
    commands::vvm::VvmEnv {
        root: commands::vvm::resolve_root(
            self_location.map(|location| location.root.clone()),
            install_base_override(),
            dirs::home_dir(),
        ),
        cwd: std::env::current_dir().ok(),
        home: dirs::home_dir(),
        shell: super::read_env_opt("SHELL"),
        path_var: super::read_env_opt("PATH"),
        install_root_env: super::read_env_opt(commands::vvm::VIBEVM_INSTALL_ROOT_ENV)
            .map(std::path::PathBuf::from),
        offline: vvm_offline(cli_offline),
        source_build_environment: commands::vvm::SOURCE_BUILD_ENVIRONMENT
            .into_iter()
            .map(|name| (name.to_owned(), std::env::var_os(name)))
            .collect(),
    }
}

/// The install root store the user home's applications follow
/// (PROP-019 `#install-root`), with the precedence vibe itself resolves its
/// root by: the running managed vibe's own root, then
/// [`install_base_override`] (`$VIBEVM_INSTALL_ROOT`, then `[self]
/// install_root` of the user's config over the site profile) joined with
/// `opt`. `None` when nothing is declared: the applications then stay
/// under the settings root, which is `~/.vibe` unless `VIBE_SETTINGS`
/// moves it.
pub(crate) fn app_install_store() -> Option<std::path::PathBuf> {
    if let Some(location) = commands::vvm::derive_self(std::env::current_exe().ok().as_deref()) {
        return Some(location.root);
    }
    commands::vvm::resolve_root(None, Some(install_base_override()?), None)
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
