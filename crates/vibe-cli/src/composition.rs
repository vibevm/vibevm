//! Small composition-root helpers that keep ambient input assembly out of dispatch.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#vars");

use std::collections::BTreeSet;
use std::sync::OnceLock;

use anyhow::Result;

use crate::cli::VarsArgs;
use crate::commands;
use crate::env_allowlist::partition_env_promotions;
use crate::output;
use vibe_core::user_config::UserConfig;

pub(crate) mod cancellation;
pub(crate) mod native_helper;
pub(crate) mod network;
pub(crate) mod remap;
pub(crate) mod templates;
pub(crate) mod transfers;

/// Names of environment variables that were promoted from the
/// user-level config at startup (i.e. the live env was unset and
/// the user-config carried a default that we wrote into the
/// process env). `vibe show config` reads this set so it can
/// distinguish "operator-set live env" from "promoted from user-
/// config" without re-loading the file mid-run. Empty when no
/// promotions happened.
static PROMOTED_FROM_USER_CONFIG: OnceLock<BTreeSet<String>> = OnceLock::new();

/// Public read-only accessor consumed by `vibe show config`. Returns
/// an empty set if `promote_user_config_env` has not yet run (e.g.
/// embedded test harnesses).
pub(crate) fn promoted_env_names() -> &'static BTreeSet<String> {
    PROMOTED_FROM_USER_CONFIG.get_or_init(BTreeSet::new)
}

/// Read `<settings-dir>/config.toml` (per `vibe-core::user_config`)
/// and promote its `[env]` entries into the process environment. This
/// makes the user-config layer actually load-bearing per
/// `VIBEVM-SPEC.md` §9.5: subsequent consumers
/// (`vibe-registry::default_cache_root`, the tracing init, the
/// publish-token loader) read whatever is in the process env without
/// caring who put it there.
///
/// # What may be promoted
///
/// Only names beginning `VIBE_` or `VIBEVM_`
/// ([`crate::env_allowlist::PROMOTABLE_ENV_PREFIXES`]). Every other name in the table is
/// ignored, and the rejected set is reported once, by name.
///
/// The table used to accept any name at all. That made one per-user
/// file — which no invocation opts into, and which *every* subcommand
/// reads before dispatch — able to set `DATABASE_URL`, `AWS_*`,
/// `KUBECONFIG` or `PATH` for vibe and for everything vibe spawns. The
/// concrete failure mode is a test run that forgot to isolate
/// `$VIBE_SETTINGS`, inheriting the operator's real config and reaching
/// a production database. An allowlist bounds the blast radius to
/// vibevm's own namespace: the feature keeps doing the job it was built
/// for — defaulting `VIBE_REGISTRY_CACHE`, `VIBE_LOG` — and can no
/// longer reach anything outside vibevm. Widening the list later is one
/// entry; resurrecting a capability that was deleted is an argument, so
/// this is the reversible half.
///
/// Matching is case-sensitive against those exact uppercase prefixes.
/// vibevm's variables are uppercase by convention, so the only thing
/// case-folding would buy is admitting more names — and on Windows,
/// where the environment compares names case-insensitively, refusing
/// `vibe_thing` costs an operator a rename while refusing
/// `database_url` is the whole point.
///
/// Two of vibevm's own variables fall outside the list, and should:
/// `VIBETERM` / `VIBEFRAME` are markers a vibe desktop terminal sets in
/// the PTY it spawns (`commands::tree::host`), so a config file
/// claiming one would be lying to `vibe tree` about where it is
/// running. The one shape that could legitimately want a name outside
/// the namespace is a manifest that overrides `token_env` on a
/// `[[registry]]` / `[redirect]` block: the defaults
/// (`VIBEVM_REGISTRY_TOKEN_<HOST>`, `VIBEVM_TARGET_TOKEN_<HOST>`) are
/// admitted, an arbitrary override is not. No such configuration exists
/// in this repository, and a token's home is
/// `~/.vibe/<host>.publish.token` (PROP-000 §20) rather than a config
/// file either way — but that is the edge to widen the list for, if it
/// ever shows up.
///
/// # What wins
///
/// 1. **The live environment.** A name already set when vibe starts is
///    left exactly as the operator set it — it was in the process env
///    by the time we observe it via `std::env::var_os`, so the
///    `is_some` guard is the whole mechanism. An `[env]` entry is a
///    default, never an override.
/// 2. **The `[env]` value**, for an allowlisted name that is unset.
/// 3. **The built-in default**, for everything else.
///
/// The allowlist is consulted *before* the live environment, so a
/// refused name is reported whether or not the operator also exports
/// it: the verdict is a property of the name, and a diagnostic that
/// came and went with the ambient environment would be worse than none.
///
/// # What is never printed
///
/// The value. A refused name is named; its value is not, at any
/// verbosity. `[env]` is a plausible place for someone to have parked a
/// credential, and the discipline `vibe-publish`'s `Token` applies to
/// publish tokens applies here for the same reason. Reporting is not
/// optional either — a silently dropped variable is a debugging
/// nightmare — so the rule is exactly: once, by name, never the value.
///
/// A malformed user-config file is reported via `eprintln!` and the
/// promotion silently continues with whatever fields parsed —
/// failing the entire CLI invocation because of an inert config
/// layer would be the wrong UX. `vibe show config` is the
/// authoritative path for surfacing that the layer is broken;
/// every other command just runs.
#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "unsafe-gate: startup env promotion runs at the top of main, \
              before the dispatcher, on the one working thread while the \
              main thread only waits in `join` — set_var's race is with \
              concurrent readers, and none can be observing yet; \
              the env-audit crate is test infrastructure and a mutate-anytime \
              safe production API would advertise soundness it cannot prove"
)]
pub(crate) fn promote_user_config_env() -> vibe_core::user_config::TransferConfig {
    let cfg = match UserConfig::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "{}",
                output::language::warning(&format!("user-level config could not be read: {e}"))
            );
            let _ = PROMOTED_FROM_USER_CONFIG.set(BTreeSet::new());
            return Default::default();
        }
    };
    let (admitted, rejected) = partition_env_promotions(&cfg.env);
    if !rejected.is_empty() {
        // Once, by name, never the value.
        eprintln!(
            "{}",
            output::language::warning(&format!(
                "user config [env] may only set VIBE_* / VIBEVM_* names; ignored: {}",
                rejected.join(", ")
            ))
        );
    }
    // PROP-068 `#proxy`: `[proxy]` is the route; its names promote as below.
    let proxy = network::install_route(&cfg.proxy, &cfg.tls, super::read_env_opt);
    let mut promoted: BTreeSet<String> = BTreeSet::new();
    for (name, value) in admitted
        .into_iter()
        .chain(proxy.iter().map(|(name, value)| (*name, &value[..])))
    {
        if std::env::var_os(name).is_some() {
            // Live env wins — leave it alone.
            continue;
        }
        // SAFETY: vibe runs on one working thread (`main` only waits
        // for it). Promotion happens at the very top of `run`, before
        // the dispatcher selects a subcommand and well before any
        // further thread is spawned (rayon, reqwest's tokio internals
        // when present, etc.). The Rust 1.85+ `unsafe` marker on `set_var` exists
        // to flag mid-execution multi-threaded mutation, which we
        // are not doing here. No other thread can be observing the
        // environment variables at this point.
        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var(name, value);
        }
        promoted.insert(name.to_string());
    }
    let _ = PROMOTED_FROM_USER_CONFIG.set(promoted);
    cfg.transfer
}

/// Route installed global operations before startup, preserving ordinary errors.
pub(crate) fn select_installed_destination(
    cli: &mut crate::cli::Cli,
    explicit_path: bool,
) -> std::result::Result<(), std::process::ExitCode> {
    if let Err(error) =
        commands::personal::select_installed_destination(&mut cli.command, explicit_path)
    {
        let ctx = output::Context::from_flags(
            cli.quiet,
            cli.json,
            cli.invoked_by.as_deref(),
            cli.unattended,
            cli.agent_mode,
        );
        ctx.error_with_suffix(&error, None);
        return Err(crate::exit_code::as_exit_code(&error));
    }
    Ok(())
}

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
    matches!(cmd, crate::cli::Command::Install(args) if args.plan_only)
}

/// `--no-recommends` answers one question — may vibe install things I did
/// not name? — and only the commands that solve or install, on either
/// plane, ever ask it. Everywhere else the flag is a usage refusal naming
/// it, because a flag silently ignored is worse than a flag refused
/// (`PROP-077#NO-RECOMMENDS-FLAG`).
pub(crate) fn admit_no_recommends(passed: bool, cmd: &crate::cli::Command) -> anyhow::Result<()> {
    use crate::cli::{AppCommand, Command};
    if !passed
        || matches!(
            cmd,
            Command::Install(_) | Command::Update(_) | Command::Reinstall(_)
        )
        // `vibe app extras` asks the same question of an author's
        // recommended **ecosystem** items: one idea with two spellings, so
        // one flag declines both (`PROP-075#SLOT-EXTRAS`). `apply` installs
        // them and `list` previews exactly what `apply` would do, and the
        // family's other verbs act on a list the user typed, where the flag
        // would mean nothing — so it is refused there as everywhere else.
        || matches!(
            cmd,
            Command::App(args)
                if matches!(
                    &args.command,
                    AppCommand::Extras(extras)
                        if matches!(
                            extras.command,
                            crate::cli::ExtrasCommand::Apply { .. }
                                | crate::cli::ExtrasCommand::List { .. }
                        )
                )
        )
    {
        return Ok(());
    }
    Err(anyhow::anyhow!(
        "`--no-recommends` is not available here: it decides whether vibe installs what you did not name, which only `vibe install`, `vibe update` and `vibe reinstall` — of a project or with `-g` — and `vibe app extras list|apply` ever ask"
    ))
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

/// Install the tracing subscriber this invocation uses, filtered by
/// `$VIBE_LOG`.
///
/// It lives beside the rest of the composition root's ambient reads rather
/// than in `main.rs` for the one reason already recorded for `VvmEnv`:
/// `main.rs` has a file budget, and the read is the same read at the same
/// root.
pub(crate) fn init_tracing() {
    use tracing_subscriber::{EnvFilter, fmt};

    // `rustls-platform-verifier` logs every refused chain at ERROR through
    // the `log` bridge, including the trial verifications `vibe dl` makes
    // to work out *which* part of the trust set accepted a server
    // (PROP-068 `#dl`). vibe reports that outcome itself, with the chain
    // and the fix, so the crate's own line is noise on a successful
    // download. It is silenced by default and not at all when `$VIBE_LOG`
    // is set, which stays the one switch that decides what is logged.
    let filter = EnvFilter::try_from_env("VIBE_LOG")
        .unwrap_or_else(|_| EnvFilter::new("warn,rustls_platform_verifier=off"));
    let _ = fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr)
        .try_init();
}
