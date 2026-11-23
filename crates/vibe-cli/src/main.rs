//! The `vibe` CLI entry point. Keeps this file thin: parse args, dispatch.
//!
//! Spec: `VIBEVM-SPEC.md` §9.

#![deny(unsafe_code)]

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#cli-surface");

use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;
mod cli;
mod command_progress;
mod commands;
mod composition;
mod env_allowlist;
mod exit_code;
mod output;
mod process_supervision;
mod registry;
mod shell_syntax;

use cli::{Cli, Command};
use exit_code::as_exit_code;

// `vibe show config` reads the promoted names through the crate root.
pub(crate) use composition::promoted_env_names;

fn main() -> ExitCode {
    // The hidden native helper mode precedes every ordinary boot step.
    if let Some(exit) = composition::native_helper::entry() {
        return exit;
    }
    composition::native_helper::install();
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
    let (mut cli, explicit_path) = Cli::parse_invocation();
    let terminal_vocabulary = read_env_opt("TERM")
        .map(|term| !term.eq_ignore_ascii_case("dumb"))
        .unwrap_or(true);
    output::language::draw_ascii(!terminal_vocabulary);
    // Kit production is an explicit local-file operation. Do not discover an
    // installed destination, promote user configuration, or enter network/remap
    // composition merely to create or validate an authored recipe.
    if let Command::Kit(args) = cli.command {
        output::configure_language(false, !cli.plain && !cli.json && !cli.quiet);
        let ctx = output::Context::from_flags(
            cli.quiet,
            cli.json,
            cli.invoked_by.as_deref(),
            cli.unattended,
            cli.agent_mode,
        )
        .with_plain(cli.plain);
        return match commands::kit::run(&ctx, args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                ctx.error_with_suffix(&error, None);
                as_exit_code(&error)
            }
        };
    }
    if let Err(exit) = composition::select_installed_destination(&mut cli, explicit_path) {
        return exit;
    }

    if let Command::Refactor(args) = &cli.command
        && let Some(exit) = commands::refactor::machine_entry(args)
    {
        return exit;
    }
    let transfer_config = composition::promote_user_config_env();
    composition::init_tracing();
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
        .with_plain(cli.plain)
        .with_no_recommends(cli.no_recommends),
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
    let preview_only = composition::project_preview(&cli.command);
    let ctx = composition::transfers::attach_for_command(ctx, &transfer_config, &cli.command);
    let (ctx, cancellation, mut stop_input) =
        match composition::cancellation::for_command(&ctx, &cli.command, cli.stop_file.clone()) {
            Ok(value) => value,
            Err(error) => {
                ctx.error_with_suffix(&error, None);
                return ExitCode::FAILURE;
            }
        };
    if let Some(exit) =
        composition::cancellation::preflight(&ctx, &cancellation, &mut stop_input, preview_only)
    {
        return exit;
    }
    let mut network = vibe_net::enter(&cancellation, composition::network::route_for(&cli.command));

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

    if let Err(error) = composition::admit_no_recommends(cli.no_recommends, &cli.command) {
        ctx.error_with_suffix(&error, None);
        return ExitCode::FAILURE;
    }
    // PROP-075 `#TEMPLATE-CACHE`, after the Locator it fetches through.
    composition::templates::install_for(cli.offline, cli.refresh);

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
    let prepare_lifecycle_install = discover_embedded_root;
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

    let vvm_env = || composition::vvm_env(self_loc.as_ref(), cli.offline);

    let result = if let Err(error) = cancellation.checkpoint() {
        Err(error.into())
    } else {
        match cli.command {
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
                *args,
                discover_embedded_root(),
                cli.offline,
                read_env_opt("VIBE_NO_DEFAULT_REGISTRY").is_none(),
                &mut stop_input,
                &mut network,
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
                match commands::app::run(
                    &ctx,
                    args,
                    discover_embedded_root().as_deref(),
                    cli.offline,
                ) {
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
            Command::Hook(args) => commands::hook::run(&ctx, args),
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
            Command::Prompt(args) => commands::prompt::run(&ctx, args),
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
            Command::Ai(args) => commands::ai::run(&ctx, args, &cancellation),
            Command::Secrets(args) => commands::secrets::run(&ctx, args, &cancellation),
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
            Command::Kit(args) => commands::kit::run(&ctx, args),
            Command::Vvm(args) => {
                let vvm_env = vvm_env();
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
            Command::Artifacts(args) => commands::artifacts::run(&ctx, args, cli.offline),
            Command::Dl(args) => commands::dl::run(&ctx, args, cli.offline, cli.verbose),
            // PROP-068 `#proxy-tool`: `exec` answers with the child's own
            // exit code, so a wrapper fails exactly as its tool does.
            Command::Tool(args) => {
                let env = commands::tool::env(read_env_opt, cli.verbose);
                match commands::tool::run(&ctx, &args, &env, vvm_env, &cancellation) {
                    Ok(Some(code)) => {
                        return ExitCode::from(u8::try_from(code.clamp(0, 255)).unwrap_or(1));
                    }
                    Ok(None) => Ok(()),
                    Err(err) => Err(err),
                }
            }

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
            Command::User(args) => commands::user::run(&ctx, args.command, &cancellation),
            Command::Version => {
                println!("vibe {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
        }
    };

    stop_input.settle(&ctx, &cancellation);
    composition::network::finish(&ctx, network, result.is_err());
    command_activity.complete(result.is_ok());
    composition::finish(&ctx, &cancellation, result, preview_only)
}

/// Read a nonempty environment override at the composition root, as required
/// by the Phase-5 `ambient-env` rule.
fn read_env_opt(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|s| !s.trim().is_empty())
}
