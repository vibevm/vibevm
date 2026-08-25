//! The `vibe` CLI entry point. Keeps this file thin: parse args, dispatch.
//!
//! Spec: `VIBEVM-SPEC.md` §9.

#![deny(unsafe_code)]

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#cli-surface");

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::OnceLock;

use clap::Parser;
use vibe_core::user_config::UserConfig;

mod cli;
mod commands;
mod exit_code;
mod output;
mod registry;

use cli::{Cli, Command};
use exit_code::as_exit_code;

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

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            // clap handles its own formatting and picks the right category.
            e.exit();
        }
    };

    promote_user_config_env();
    init_tracing();

    // VVM: derive the running version from the binary's own path, and warn
    // when the inherited $VIBEVM_HOME is stale (PROP-019 §2.5).
    let self_loc = commands::vvm::derive_self(std::env::current_exe().ok().as_deref());
    if let Some(loc) = &self_loc
        && let Some(env_home) = read_env_opt(commands::vvm::VIBEVM_HOME_ENV)
        && !commands::vvm::same_location(&env_home, &loc.home)
    {
        eprintln!(
            "vibe: note: $VIBEVM_HOME is stale (env={env_home}); the running version is {} \
             — open a new shell or `eval \"$(vibe self env)\"`",
            loc.home.display()
        );
    }

    let ctx = output::Context::from_flags(
        cli.quiet,
        cli.json,
        cli.invoked_by.as_deref(),
        cli.unattended,
    );

    // Ensure `~/.vibe/registry.toml` exists with the default pair (vibespecs
    // GitHub + GitVerse) on any registry-needing command. A fresh checkout on
    // a new machine resolves packages out of the box without a manual
    // `vibe registry add`. Never overwrites an existing file — user edits are
    // preserved. Soft-warns on failure (the global layer is optional).
    // Skipped when VIBE_NO_DEFAULT_REGISTRY is set (tests, CI, or explicit
    // opt-out).
    if needs_global_registry(&cli.command)
        && read_env_opt("VIBE_NO_DEFAULT_REGISTRY").is_none()
        && let Err(e) = vibe_core::ensure_default_global_registry()
    {
        eprintln!("vibe: warning: could not seed ~/.vibe/registry.toml: {e}");
    }

    // PROP-030: the embedded registry belongs to a source-*installed* `vibe` —
    // one whose `current_exe` sits under a VVM install slot, so `derive_self`
    // resolves it (`self_loc`). A `cargo run` binary or a test harness is not
    // installed: it has no embedded registry and must NOT pick up the
    // developer's `~/.vibe/opt` one. Gate discovery on the running install and read
    // its active record's source path through that install's own root. Shared
    // by the install-family commands (install / update / reinstall).
    let discover_embedded_root = || -> Option<PathBuf> {
        // PROP-030 §5 (CI-off): CI resolves from declared registries only, so a
        // machine-local embedded lock cannot silently pass there.
        if read_env_opt("CI").is_some() || read_env_opt("VIBE_NO_DEFAULT_REGISTRY").is_some() {
            return None;
        }
        commands::vvm::embedded_root_at(self_loc.as_ref()?.root.clone())
    };
    let prepare_lifecycle_install = || {
        if read_env_opt("VIBE_NO_DEFAULT_REGISTRY").is_none()
            && let Err(e) = vibe_core::ensure_default_global_registry()
        {
            eprintln!("vibe: warning: could not seed ~/.vibe/registry.toml: {e}");
        }
        discover_embedded_root()
    };
    let run_lifecycle = |phase, args| {
        commands::lifecycle::run(&ctx, phase, args, prepare_lifecycle_install, cli.offline)
    };

    let result = match cli.command {
        Command::Init(args) => commands::init::run(&ctx, args),
        Command::List(args) => commands::list::run(&ctx, args),
        Command::Validate(args) => run_lifecycle(vibe_lifecycle::Phase::Validate, args),
        Command::Install(args) => commands::install::run_with_world_callback(
            &ctx,
            args,
            discover_embedded_root(),
            cli.offline,
            |project_root, disposition, run| {
                commands::lifecycle::after_direct_install(&ctx, project_root, disposition, run)
            },
        )
        .map(|_| ()),
        Command::Generate(args) => run_lifecycle(vibe_lifecycle::Phase::Generate, args),
        Command::Build(args) => run_lifecycle(vibe_lifecycle::Phase::Build, args),
        Command::Test(args) => run_lifecycle(vibe_lifecycle::Phase::Test, args),
        Command::Create(args) => run_lifecycle(vibe_lifecycle::Phase::Create, args),
        Command::Verify(args) => run_lifecycle(vibe_lifecycle::Phase::Verify, args),
        Command::Package(args) => run_lifecycle(vibe_lifecycle::Phase::Package, args),
        Command::Deploy(args) => run_lifecycle(vibe_lifecycle::Phase::Deploy, args),
        Command::Clean(args) => {
            commands::clean::run(&ctx, args, prepare_lifecycle_install, cli.offline)
        }
        Command::Outdated(args) => commands::outdated::run(&ctx, args),
        Command::Search(args) => {
            // The composition root reads the search command's
            // environment overrides; the domain never touches the
            // ambient env itself (CONVERT-PLAN v0.1 §1 item 0.4).
            let search_env = commands::search::SearchEnv {
                github_api_base: read_env_opt(commands::search::GITHUB_API_BASE_ENV),
                cache_dir: read_env_opt(vibe_registry::search::cache::CACHE_ROOT_ENV),
            };
            commands::search::run(&ctx, args, search_env)
        }
        Command::Mcp(args) => commands::mcp::run(&ctx, args),
        Command::Aiui(args) => commands::aiui::run(&ctx, args),
        Command::Term(args) => commands::term::run(&ctx, args),
        Command::Frame(args) => commands::term::run_frame(&ctx, args),
        Command::Skill(args) => commands::skill::run(&ctx, args),
        Command::Agentic(args) => commands::agentic::run(&ctx, args),
        Command::Drain(args) => commands::agentic::run_command(&ctx, args),
        Command::Uninstall(args) => commands::uninstall::run(&ctx, args),
        Command::Update(args) => {
            commands::update::run(&ctx, args, discover_embedded_root(), cli.offline)
        }
        Command::Reinstall(args) => {
            commands::reinstall::run(&ctx, args, discover_embedded_root(), cli.offline)
        }
        Command::Check(args) => commands::check::run(&ctx, args),
        Command::Facts(args) => commands::facts::run(&ctx, args),
        Command::Why(args) => commands::why::run(&ctx, args),
        Command::Refactor(args) => commands::refactor::run(&ctx, args),
        Command::Friends(args) => commands::friends::run(&ctx, args),
        Command::Explain(args) => commands::explain::run(&ctx, args),
        Command::Query(args) => commands::query::run(&ctx, args),
        Command::Select(args) => commands::select::run(&ctx, args),
        Command::Specmap(args) => commands::specmap::run(&ctx, args),
        Command::Show(args) => commands::show::run(&ctx, args),
        Command::Prefs(args) => commands::prefs::run(&ctx, args),
        Command::Tree(args) => commands::tree::run(&ctx, args),
        Command::Registry(args) => commands::registry::run(&ctx, args),
        Command::Cache(args) => commands::cache::run(&ctx, args, cli.offline),
        Command::Workspace(args) => commands::workspace::run(&ctx, args),
        Command::Vvm(args) => {
            // The root is the running version's own (current_exe-derived)
            // when managed, else $VIBEVM_INSTALL_ROOT/opt, else ~/.vibe/opt
            // (PROP-019 §2.5). Ambient reads live at the composition root.
            let vvm_env = commands::vvm::VvmEnv {
                root: commands::vvm::resolve_root(
                    self_loc.as_ref().map(|l| l.root.clone()),
                    read_env_opt(commands::vvm::VIBEVM_INSTALL_ROOT_ENV).map(PathBuf::from),
                    dirs::home_dir(),
                ),
                cwd: std::env::current_dir().ok(),
                home: dirs::home_dir(),
                shell: read_env_opt("SHELL"),
                path_var: read_env_opt("PATH"),
            };
            commands::vvm::run(&ctx, args, vvm_env)
        }
        Command::Vars(args) => {
            let install_base = commands::vvm::resolve_root(
                self_loc.as_ref().map(|l| l.root.clone()),
                read_env_opt(commands::vvm::VIBEVM_INSTALL_ROOT_ENV).map(PathBuf::from),
                dirs::home_dir(),
            )
            .and_then(|root| root.parent().map(|p| p.display().to_string()))
            .unwrap_or_default();
            let home_actual = self_loc
                .as_ref()
                .map(|l| l.home.display().to_string())
                .or_else(|| read_env_opt(commands::vvm::VIBEVM_HOME_ENV))
                .unwrap_or_else(|| "(none)".to_string());
            let (invoked, _) = output::resolve_invoked_by(cli.invoked_by.as_deref());
            let rows = vec![
                commands::vars::VarRow {
                    name: "VIBEVM_INSTALL_ROOT",
                    actual: install_base,
                    env: read_env_opt(commands::vvm::VIBEVM_INSTALL_ROOT_ENV),
                },
                commands::vars::VarRow {
                    name: "VIBEVM_HOME",
                    actual: home_actual,
                    env: read_env_opt(commands::vvm::VIBEVM_HOME_ENV),
                },
                commands::vars::VarRow {
                    name: "VIBE_INVOKED_BY",
                    actual: invoked.unwrap_or_default(),
                    env: read_env_opt("VIBE_INVOKED_BY"),
                },
                commands::vars::VarRow {
                    name: "VIBE_UNATTENDED",
                    actual: output::resolve_unattended(cli.unattended).to_string(),
                    env: read_env_opt("VIBE_UNATTENDED"),
                },
                commands::vars::VarRow {
                    name: "VIBE_LOG",
                    actual: read_env_opt("VIBE_LOG").unwrap_or_else(|| "warn".to_string()),
                    env: read_env_opt("VIBE_LOG"),
                },
            ];
            commands::vars::run(args, rows)
        }
        Command::Progress(args) => commands::progress::run(&ctx, args),
        Command::Tools { json } => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
            commands::tools::run(&cwd, json)
        }
        Command::Bin { cmd } => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
            match cmd {
                cli::BinCmd::List => commands::bin::run_list(&cwd),
                cli::BinCmd::Build { names, assume_yes } => {
                    commands::bin::run_build(&cwd, &names, assume_yes)
                }
                cli::BinCmd::Path { name } => commands::bin::run_path(&cwd, &name),
                cli::BinCmd::Exec {
                    name,
                    assume_yes,
                    args,
                } => match commands::bin::run_exec(&cwd, &name, &args, assume_yes) {
                    Ok(code) => {
                        return ExitCode::from(u8::try_from(code.clamp(0, 255)).unwrap_or(1));
                    }
                    Err(err) => Err(err),
                },
            }
        }
        Command::Trace { args } => match commands::trace::run(&args) {
            Ok(code) => {
                return ExitCode::from(u8::try_from(code.clamp(0, 255)).unwrap_or(1));
            }
            Err(err) => Err(err),
        },
        Command::Version => {
            println!("vibe {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            ctx.error(&err);
            as_exit_code(&err)
        }
    }
}

/// The environment-variable name prefixes a user-config `[env]` table
/// is allowed to set. Closed on purpose; [`promote_user_config_env`]
/// carries the reasoning, and widening it is an edit here.
const PROMOTABLE_ENV_PREFIXES: [&str; 2] = ["VIBE_", "VIBEVM_"];

/// Whether `name` is inside the promotion allowlist.
///
/// `VIBEVM_` needs its own entry rather than falling out of `VIBE_`:
/// `VIBEVM_HOME` does not start with `VIBE_` — the fifth character is
/// `V`, not the separator.
fn is_promotable_env_name(name: &str) -> bool {
    PROMOTABLE_ENV_PREFIXES
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

/// Split a `[env]` table into the entries the allowlist admits (as
/// name/value pairs) and the names it turns away.
///
/// Pure, and separate from the mutation, so the rule that decides what
/// reaches the process can be tested without a test that mutates the
/// process environment to check it.
fn partition_env_promotions(env: &BTreeMap<String, String>) -> (Vec<(&str, &str)>, Vec<&str>) {
    let mut admitted = Vec::new();
    let mut rejected = Vec::new();
    for (name, value) in env {
        if is_promotable_env_name(name) {
            admitted.push((name.as_str(), value.as_str()));
        } else {
            rejected.push(name.as_str());
        }
    }
    (admitted, rejected)
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
/// ([`PROMOTABLE_ENV_PREFIXES`]). Every other name in the table is
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
              before the dispatcher and before any thread exists — set_var's \
              race is with concurrent readers, and none can be observing yet; \
              the env-audit crate is test infrastructure and a mutate-anytime \
              safe production API would advertise soundness it cannot prove"
)]
fn promote_user_config_env() {
    let cfg = match UserConfig::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("warning: user-level config could not be read: {e}");
            let _ = PROMOTED_FROM_USER_CONFIG.set(BTreeSet::new());
            return;
        }
    };
    let (admitted, rejected) = partition_env_promotions(&cfg.env);
    if !rejected.is_empty() {
        // Once, by name, never the value.
        eprintln!(
            "vibe: warning: user config `[env]` may only set VIBE_* / VIBEVM_* names; \
             ignored: {}",
            rejected.join(", ")
        );
    }
    let mut promoted: BTreeSet<String> = BTreeSet::new();
    for (name, value) in admitted {
        if std::env::var_os(name).is_some() {
            // Live env wins — leave it alone.
            continue;
        }
        // SAFETY: vibe is a single-threaded CLI binary. Promotion
        // happens at the very top of `main`, before the dispatcher
        // selects a subcommand and well before any thread is
        // spawned (rayon, reqwest's tokio internals when present,
        // etc.). The Rust 1.85+ `unsafe` marker on `set_var` exists
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
}

fn init_tracing() {
    use tracing_subscriber::{EnvFilter, fmt};

    let filter = EnvFilter::try_from_env("VIBE_LOG").unwrap_or_else(|_| EnvFilter::new("warn"));
    let _ = fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr)
        .try_init();
}

/// Read an environment override at the composition root: `Some(value)`
/// only when the variable is set and non-empty. vibe's domain commands
/// never read the ambient environment themselves — reads live here in
/// main and the value is threaded down (CONVERT-PLAN v0.1 §1 item 0.4;
/// the Phase-5 `ambient-env` rule names `main.rs` a recorded root).
fn read_env_opt(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|s| !s.trim().is_empty())
}

/// Whether a command touches the registry layer (and thus benefits from the
/// default `~/.vibe/registry.toml` being seeded). Commands like `version` /
/// `vars` / `tree` / `term` don't resolve packages, so they skip the seed
/// check. This is advisory — `ensure_default_global_registry` is a cheap
/// file-existence check regardless.
fn needs_global_registry(cmd: &cli::Command) -> bool {
    use cli::Command;
    matches!(
        cmd,
        Command::Init(_)
            | Command::Install(_)
            | Command::Update(_)
            | Command::Reinstall(_)
            | Command::Outdated(_)
            | Command::Search(_)
            | Command::Registry(_)
            | Command::Cache(_)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_table(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    /// The whole point, stated as the smallest case that shows it: a
    /// user config declaring a database URL next to a vibevm variable
    /// hands the process the second one and never the first. Asserted
    /// on the decision rather than on the live environment, because a
    /// test that promotes into its own process env would be mutating
    /// global state to observe a rule that is pure.
    #[test]
    fn allowlist_admits_vibe_names_and_refuses_the_rest() {
        let env = env_table(&[
            ("DATABASE_URL", "postgres://admin:hunter2@db.internal/prod"),
            ("VIBE_THING", "promoted-ok"),
        ]);

        let (admitted, rejected) = partition_env_promotions(&env);

        assert_eq!(admitted, vec![("VIBE_THING", "promoted-ok")]);
        assert_eq!(rejected, vec!["DATABASE_URL"]);
    }

    #[test]
    fn allowlist_covers_both_prefixes() {
        for name in [
            "VIBE_LOG",
            "VIBE_REGISTRY_CACHE",
            "VIBE_NO_DEFAULT_REGISTRY",
            // `VIBEVM_*` is a second prefix, not a special case of the
            // first: `VIBEVM_HOME` does not start with `VIBE_`.
            "VIBEVM_HOME",
            "VIBEVM_PUBLISH_TOKEN_GITHUB",
        ] {
            assert!(is_promotable_env_name(name), "{name} must be promotable");
        }
    }

    #[test]
    fn allowlist_refuses_everything_outside_the_namespace() {
        for name in [
            // The names this rule exists for.
            "DATABASE_URL",
            "AWS_SECRET_ACCESS_KEY",
            "KUBECONFIG",
            "PATH",
            "LD_PRELOAD",
            "HOME",
            // Near-misses: prefix means prefix, and it means the
            // separator too.
            "VIBE",
            "VIBEX_LOG",
            "MY_VIBE_LOG",
            // Case-sensitive on purpose — see `promote_user_config_env`.
            "vibe_thing",
        ] {
            assert!(!is_promotable_env_name(name), "{name} must be refused");
        }
    }

    /// An empty or absent `[env]` is unchanged and silent: nothing to
    /// promote, and nothing to warn about.
    #[test]
    fn an_empty_table_admits_and_refuses_nothing() {
        let empty = BTreeMap::new();
        let (admitted, rejected) = partition_env_promotions(&empty);
        assert!(admitted.is_empty());
        assert!(rejected.is_empty());
    }
}
