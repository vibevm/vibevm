//! The `vibe` CLI entry point. Keeps this file thin: parse args, dispatch.
//!
//! Spec: `VIBEVM-SPEC.md` §9.

#![deny(unsafe_code)]

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#cli-surface");

use std::collections::BTreeSet;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::OnceLock;

use clap::Parser;
use vibe_core::user_config::UserConfig;

mod cli;
mod command_progress;
mod commands;
mod composition;
mod env_allowlist;
mod exit_code;
mod output;
mod registry;

use env_allowlist::partition_env_promotions;

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
    // clap's derived builders keep every subcommand's temporaries in one
    // frame, and a debug build of a CLI this wide needs more than the 1 MiB
    // a Windows main thread owns: `vibe --version` overflowed there the day
    // the launcher catalog added its verbs (2026-10-31), while Linux's 8 MiB
    // hid it. The work runs on a thread sized for it, as rustc does; the
    // main thread only waits, so nothing observes the environment while
    // `promote_user_config_env` sets it.
    const STACK: usize = 32 << 20;
    let worker = std::thread::Builder::new()
        .name("vibe".into())
        .stack_size(STACK)
        .spawn(run);
    match worker {
        Ok(handle) => handle.join().unwrap_or_else(|_| ExitCode::from(101)),
        Err(_) => run(),
    }
}

fn run() -> ExitCode {
    // Clap owns argument diagnostics and their exit classification.
    let cli = Cli::parse();

    if let Command::Refactor(args) = &cli.command
        && let Some(exit) = commands::refactor::machine_entry(args)
    {
        return exit;
    }
    let transfer_config = promote_user_config_env();
    init_tracing();
    // VVM: derive the running version from the binary's own path, and warn
    // when the inherited $VIBEVM_HOME is stale (PROP-019 §2.5).
    let self_loc = commands::vvm::derive_self(std::env::current_exe().ok().as_deref());
    if let Some(loc) = &self_loc
        && let Some(env_home) = read_env_opt(commands::vvm::VIBEVM_HOME_ENV)
        && !commands::vvm::same_location(&env_home, &loc.home)
    {
        eprintln!(
            "{}",
            output::language::warning(&format!(
                "$VIBEVM_HOME is stale (env={env_home}); the running version is {} \
                 — open a new shell or run eval \"$(vibe self env)\"",
                loc.home.display()
            ))
        );
    }

    let terminal_vocabulary = read_env_opt("TERM")
        .map(|term| !term.eq_ignore_ascii_case("dumb"))
        .unwrap_or(true);
    let progress_interactive = std::io::stderr().is_terminal() && terminal_vocabulary && !cli.plain;
    output::configure_language(
        std::io::stdout().is_terminal() && std::io::stderr().is_terminal(),
        terminal_vocabulary && !cli.plain && !cli.json && !cli.quiet,
    );
    let (ctx, command_activity) = command_progress::configure_context(
        output::Context::from_flags(
            cli.quiet,
            cli.json,
            cli.invoked_by.as_deref(),
            cli.unattended,
            cli.agent_mode,
        )
        .with_plain(cli.plain),
        &cli.command,
        cli.verbose,
        command_progress::Streams {
            stdout: std::io::stdout().is_terminal(),
            stderr: progress_interactive,
            stdin: std::io::stdin().is_terminal(),
        },
        command_progress::progress_disabled(
            cli.no_progress,
            read_env_opt("VIBE_NO_PROGRESS").as_deref(),
        ),
        command_progress::machine_format(
            cli.progress_format,
            read_env_opt("VIBE_PROGRESS_FORMAT").as_deref(),
        ),
    );
    let ctx = composition::transfers::attach(ctx, &transfer_config);

    // PROP-066 `#locator`: this invocation's one Locator, built and
    // installed before any command — it is write-once — and after the
    // context, because `#consent`'s question needs to know whether anyone
    // is there to answer it. A refusal stops the run here.
    if let Err(error) = composition::remap::install(
        &ctx,
        composition::remap::Inputs {
            flags: &cli.remap,
            env_list: read_env_opt(vibe_core::remap::REMAP_ENV),
            release_origin: read_env_opt(composition::remap::RELEASE_ORIGIN_ENV),
            consent: commands::remap::ConsentFlags::from_args(&cli.accept_remap, cli.reject_remap),
            command: &cli.command,
            offline: cli.offline,
        },
    ) {
        ctx.error_with_suffix(&error, None);
        return as_exit_code(&error);
    }

    // Seed `~/.vibe/registry.toml` with the default pair on registry-needing
    // commands; never overwrites, soft-warns, skipped by VIBE_NO_DEFAULT_REGISTRY.
    if composition::needs_global_registry(&cli.command)
        && read_env_opt("VIBE_NO_DEFAULT_REGISTRY").is_none()
        && let Err(e) = vibe_core::ensure_default_global_registry()
    {
        ctx.diagnostic(&format!("could not seed ~/.vibe/registry.toml: {e}"));
    }

    // PROP-030: the embedded registry belongs to a source-*installed* `vibe`
    // (`derive_self` resolves its VVM slot); a `cargo run` binary or a test
    // harness has none and must NOT pick up the developer's `~/.vibe/opt` one.
    // Shared by the install-family commands (install / update / reinstall).
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
            ctx.diagnostic(&format!("could not seed ~/.vibe/registry.toml: {e}"));
        }
        discover_embedded_root()
    };
    // Only `vibe deploy` carries a profile request (`None` here): every
    // other phase verb stops before the deploy fence would arm.
    let run_lifecycle = |phase, args| {
        commands::lifecycle::run(
            &ctx,
            phase,
            args,
            prepare_lifecycle_install,
            cli.offline,
            None,
        )
    };

    let result = match cli.command {
        Command::Init(args) => commands::init::run(&ctx, args),
        Command::List(args) => commands::list::run(&ctx, args),
        Command::Extensions(args) => match args.command {
            // Bare `vibe extensions` keeps its exhaustive-registry report;
            // `vibe extensions analyze` routes to the R4.3 lane analyzer.
            None => commands::extensions::run(
                &ctx,
                cli::ExtensionsArgs {
                    path: args.path,
                    command: None,
                },
            ),
            Some(cli::ExtensionsCommand::Analyze(analyze)) => {
                commands::extensions_analyze::run(&ctx, analyze)
            }
            Some(cli::ExtensionsCommand::Compile(compile)) => {
                commands::extensions::run_compile(&ctx, compile, cli.offline)
            }
        },
        Command::Validate(args) => run_lifecycle(vibe_lifecycle::Phase::Validate, args),
        Command::Install(args) => commands::global_mcp_dispatch::run_install(
            &ctx,
            args,
            discover_embedded_root(),
            cli.offline,
        ),
        Command::Generate(args) => run_lifecycle(vibe_lifecycle::Phase::Generate, args),
        Command::Build(args) => run_lifecycle(vibe_lifecycle::Phase::Build, args),
        Command::Test(args) => run_lifecycle(vibe_lifecycle::Phase::Test, args),
        Command::Run(args) => match commands::run::run(&ctx, args) {
            Ok(code) => {
                return ExitCode::from(u8::try_from(code.clamp(0, 255)).unwrap_or(1));
            }
            Err(error) => Err(error),
        },
        Command::App(args) => {
            match commands::app::run(&ctx, args, discover_embedded_root().as_deref(), cli.offline) {
                Ok(Some(code)) => {
                    return ExitCode::from(u8::try_from(code.clamp(0, 255)).unwrap_or(1));
                }
                Ok(None) => Ok(()),
                Err(error) => Err(error),
            }
        }
        Command::Create(args) => run_lifecycle(vibe_lifecycle::Phase::Create, args),
        Command::Verify(args) => run_lifecycle(vibe_lifecycle::Phase::Verify, args),
        Command::Package(args) => run_lifecycle(vibe_lifecycle::Phase::Package, args),
        Command::Deploy(args) => {
            commands::deploy::run(&ctx, args, prepare_lifecycle_install, cli.offline)
        }
        Command::Undeploy(args) => commands::deploy::run_undeploy(&ctx, args),
        Command::Deployments => commands::deploy::run_deployments(&ctx),
        Command::Clean(args) => {
            commands::clean::run(&ctx, args, prepare_lifecycle_install, cli.offline)
        }
        Command::Scrape(args) => commands::scrape::run(
            &ctx,
            args,
            commands::scrape::ScrapeEnvironment::new(
                std::env::var_os("VIBE_SETTINGS"),
                std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }),
            ),
        ),
        Command::Outdated(args) => commands::outdated::run(&ctx, args),
        Command::Search(args) => {
            // The composition root reads the search command's environment overrides; the
            // domain never touches the ambient env itself (CONVERT-PLAN v0.1 §1 item 0.4).
            let search_env = commands::search::SearchEnv {
                github_api_base: read_env_opt(commands::search::GITHUB_API_BASE_ENV),
                cache_dir: read_env_opt(vibe_registry::search::cache::CACHE_ROOT_ENV),
            };
            commands::search::run(&ctx, args, search_env)
        }
        Command::Mcp(args) => commands::mcp::run(
            &ctx,
            args,
            commands::mcp::McpRuntime {
                root_offline: cli.offline,
                embedded_registry_root: discover_embedded_root(),
                seed_default_registry: read_env_opt("VIBE_NO_DEFAULT_REGISTRY").is_none(),
            },
        ),
        Command::Aiui(args) => commands::aiui::run(&ctx, args),
        Command::Shell(args) => commands::shell::run(&ctx, args),
        Command::Storage(args) => commands::storage::run(&ctx, args),
        Command::Agent(args) => commands::agent::run(&ctx, args),
        Command::Term(args) => commands::term::run(&ctx, args),
        Command::Frame(args) => commands::term::run_frame(&ctx, args),
        Command::Skill(args) => commands::skill::run(&ctx, args, cli.offline),
        Command::Agentic(args) => commands::agentic::run(&ctx, args),
        Command::Drain(args) => commands::agentic::run_command(&ctx, args),
        Command::Uninstall(args) => {
            commands::global_mcp_dispatch::run_uninstall(&ctx, args, cli.offline)
        }
        Command::Update(args) => commands::global_mcp_dispatch::run_update(
            &ctx,
            args,
            discover_embedded_root(),
            cli.offline,
        ),
        Command::Reinstall(args) => {
            commands::reinstall::run(&ctx, args, discover_embedded_root(), cli.offline)
        }
        Command::Check(args) => commands::check::run(&ctx, args),
        Command::Doc(args) => commands::doc::run(
            &ctx,
            args,
            // The documentation library reads no ambient environment of
            // its own: the settings dir, the operator's home, the temp
            // root, the tree and the running binary are resolved HERE and
            // travel down as data (PROP-057 ##PIPE-EXAMPLE-RUNNER).
            commands::doc::DocEnv {
                progress: ctx.progress(),
                settings: std::env::var_os("VIBE_SETTINGS"),
                home: std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }),
                temp: std::env::temp_dir(),
                cwd: std::env::current_dir().ok(),
                current_exe: std::env::current_exe().ok(),
                pid: std::process::id(),
                // The reader's shell is kept in the VVM store, so the
                // root is the one `vibe self` resolves — the running
                // version's own when managed, else the install base.
                install_root: commands::vvm::resolve_root(
                    self_loc.as_ref().map(|l| l.root.clone()),
                    read_env_opt(commands::vvm::VIBEVM_INSTALL_ROOT_ENV).map(PathBuf::from),
                    dirs::home_dir(),
                ),
                unattended: ctx.is_unattended(),
                json: ctx.is_json(),
            },
        ),
        Command::Facts(args) => commands::facts::run(&ctx, args),
        Command::Requirements(args) => commands::requirements::run(&ctx, args),
        Command::Why(args) => commands::why::run(&ctx, args),
        Command::Refactor(args) => commands::refactor::run(&ctx, args),
        Command::Friends(args) => commands::friends::run(&ctx, args),
        Command::Explain(args) => commands::explain::run(&ctx, args),
        Command::Query(args) => commands::query::run(&ctx, args),
        Command::Select(args) => commands::select::run(&ctx, args),
        Command::Specmap(args) => commands::specmap::run(&ctx, args),
        Command::Show(args) => commands::show::run(&ctx, args, cli.offline),
        Command::Prefs(args) => commands::prefs::run(&ctx, args),
        Command::Tree(args) => commands::tree::run(&ctx, args),
        Command::Registry(args) => commands::registry::run(&ctx, args, cli.offline),
        // `remap test` answers with a status (PROP-066 `#policy`).
        Command::Remap(args) => match commands::remap::dispatch(
            &ctx,
            args,
            composition::vvm_offline(cli.offline),
            !cli.accept_remap.is_empty(),
        ) {
            Ok(Some(code)) => {
                return ExitCode::from(u8::try_from(code.clamp(0, 255)).unwrap_or(1));
            }
            Ok(None) => Ok(()),
            Err(error) => Err(error),
        },
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
                offline: composition::vvm_offline(cli.offline),
                source_build_environment: commands::vvm::SOURCE_BUILD_ENVIRONMENT
                    .into_iter()
                    .map(|name| (name.to_owned(), std::env::var_os(name)))
                    .collect(),
            };
            commands::vvm::run(&ctx, args, vvm_env)
        }
        Command::Vars(args) => composition::run_vars(
            args,
            self_loc.as_ref(),
            cli.unattended,
            cli.invoked_by.as_deref(),
        ),
        Command::Progress(args) => commands::progress::run(&ctx, args),
        Command::Transfers(args) => commands::transfers::run(&ctx, args),
        Command::Tools { json } => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
            commands::tools::run(&ctx, &cwd, json)
        }
        Command::Bin { cmd } => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
            match cmd {
                cli::BinCmd::List => commands::bin::run_list(&ctx, &cwd),
                cli::BinCmd::Build { names, assume_yes } => {
                    commands::bin::run_build(&ctx, &cwd, &names, assume_yes)
                }
                cli::BinCmd::Path { name } => commands::bin::run_path(&ctx, &cwd, &name),
                cli::BinCmd::Exec {
                    name,
                    assume_yes,
                    args,
                } => match commands::bin::run_exec(&ctx, &cwd, &name, &args, assume_yes) {
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

    command_activity.complete(result.is_ok());
    match result {
        Ok(()) => {
            // A command that returned without emitting its own document (a
            // pure-narration verb) still owes any plan it deferred.
            let _ = ctx.flush_json_plans();
            ExitCode::SUCCESS
        }
        Err(err) => {
            // A failure is an outcome too: release the held-back plan
            // documents rather than swallowing them with the error.
            let _ = ctx.flush_json_plans();
            // A quiet command's only line is the one below, so a traced
            // failure has nowhere else to report its trace. The wrapper is
            // consumed HERE and immediately: what comes back is the same error
            // object — same context chain for stderr, same downcast identity
            // for the exit code — and a suffix that only `HumanQuiet` prints.
            let (err, suffix) = commands::compile_trace::detach_quiet_suffix(err);
            ctx.error_with_suffix(&err, suffix.as_deref());
            as_exit_code(&err)
        }
    }
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
fn promote_user_config_env() -> vibe_core::user_config::TransferConfig {
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
    // PROP-066 `#locator`: the one `[proxy]` table reaches every reqwest
    // client and every child `git` by the mechanism both already read —
    // `HTTPS_PROXY` / `HTTP_PROXY` / `NO_PROXY`. Three fixed names from a
    // typed table, not the free-form `[env]` map, so the allowlist above
    // neither admits nor refuses them; the live-env rule below is shared.
    let proxy = cfg.proxy.child_env();
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

fn init_tracing() {
    use tracing_subscriber::{EnvFilter, fmt};

    let filter = EnvFilter::try_from_env("VIBE_LOG").unwrap_or_else(|_| EnvFilter::new("warn"));
    let _ = fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr)
        .try_init();
}

/// Read a nonempty environment override at the composition root, as required
/// by the Phase-5 `ambient-env` rule.
fn read_env_opt(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|s| !s.trim().is_empty())
}
