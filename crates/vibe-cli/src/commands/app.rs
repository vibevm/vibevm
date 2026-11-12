//! CLI adapter for installed applications: `vibe app installers`,
//! `launchers`, `icon`, `import-desktop`, `run`, `start`, `running`,
//! `stop`, `watch`, `list`, `use`, `alternatives`, `export` and `import`
//! over the `vibe-app` library. This adapter parses, chooses an ID in an attended
//! terminal and renders.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-operations");

pub(crate) mod agents;
mod alternatives;
mod boundary;
mod catalog;
pub(crate) mod channels;
mod configuration;
mod describe;
pub(crate) mod display;
pub(crate) mod env;
pub(crate) mod environment;
mod exec;
mod export;
mod extras;
pub(crate) mod facts;
pub(crate) mod global;
pub(crate) mod home;
pub(crate) mod interrupt;
mod launch;
mod lifecycle;
mod menu;
pub(crate) mod migrate;
mod plan_text;
mod portable;
pub(crate) mod processes;
pub(crate) mod project;
mod records;
mod report;
mod resume;
mod runtime;
pub(crate) mod session;
pub(crate) mod short_name;
mod version_file;
pub(crate) mod version_select;
pub(crate) mod versions;
mod views;
mod wizard;

use launch::{launcher_listing, run_launcher};
use views::{print_installers, print_list, print_listing};

use std::path::Path;

use anyhow::Result;

use crate::cli::{AppArgs, AppCommand};
use crate::output;

use boundary::{from_home, resolved, state_verb};

pub fn run(
    ctx: &output::Context,
    args: AppArgs,
    embedded_root: Option<&Path>,
    offline: bool,
) -> Result<Option<i32>> {
    let explicit_path = args.path.is_some();
    let project_root = home::selected_project(args.path.as_deref())?;
    let app_home = home::select(args.global, project_root.as_deref())?;
    let source_project = match &app_home {
        home::AppHome::Project { root, .. } => Some(root.as_path()),
        home::AppHome::User(_) => None,
    };
    // Deferred construction: source-free app views do not read policy or catalog files.
    let policy_operation =
        || global::source_operation_for_project(source_project, Default::default());
    match args.command {
        AppCommand::Installers { package, registry } => {
            let source_operation = policy_operation()?;
            let (package, resolved) = resolved(
                ctx,
                "vibe app installers",
                &app_home,
                &package,
                short_name::inputs(
                    registry.as_deref(),
                    embedded_root,
                    offline,
                    &source_operation,
                ),
            )?;
            let listing = vibe_app::options_listing(&package, &resolved.options);
            if ctx.is_json() {
                ctx.emit_json(&listing)?;
            } else {
                print_installers(&listing, &package, ctx.is_quiet());
            }
            Ok(None)
        }
        AppCommand::Plan {
            package,
            configuration,
            registry,
            platform,
            arch,
            installer,
            choice,
            agent,
        } => {
            let settings = global::settings_root()?;
            let source_operation = policy_operation()?;
            let (_, resolved) = resolved(
                ctx,
                "vibe app plan",
                &app_home,
                &package,
                short_name::inputs(
                    registry.as_deref(),
                    embedded_root,
                    offline,
                    &source_operation,
                ),
            )?;
            let proxy_image = global::proxy_image()?;
            let flat = display::StepDisplay::flat();
            let observe = global::Observation::new(ctx, false, &flat)?
                .with_source_operation(&source_operation);
            let agents = agents::AgentSetup::new(ctx, agent.as_deref(), false)?;
            let packages = global::Packages {
                operation: observe.http_client.source_operation().clone(),
                settings: settings.clone(),
                registry: registry.clone(),
                embedded: embedded_root.map(Path::to_path_buf),
                offline,
            };
            let resolver = |spelling: &str| packages.resolve(spelling);
            let operation = vibe_app::OperationContext {
                agent: agents.hooks(),
                resolver: Some(&resolver),
                resolver_scope: packages.scope(),
                ..global::operation(
                    ctx,
                    app_home.settings(),
                    offline,
                    &proxy_image,
                    None,
                    &observe,
                )
            };
            let request = global::plan_request(
                vibe_app::InstallChoice {
                    platform,
                    arch,
                    installer,
                },
                &choice,
            )?;
            if configuration {
                let document = ctx.suspend_progress(|| {
                    vibe_app::configuration_plan(&operation, &resolved, &request)
                })?;
                if ctx.is_json() {
                    ctx.emit_json(&document)?;
                } else {
                    println!("{}", serde_json::to_string_pretty(&document)?);
                }
                return Ok(None);
            }
            let plan = ctx.suspend_progress(|| vibe_app::plan(&operation, &resolved, &request))?;
            global::render_plan(ctx, &plan)?;
            Ok(None)
        }
        AppCommand::Resume {
            package,
            operation,
            parameters,
        } => {
            let package = from_home(ctx, "vibe app resume", &app_home, &package)?;
            resume::run(
                ctx,
                app_home.settings(),
                &package,
                &operation,
                parameters,
                offline,
                &policy_operation()?,
            )
        }
        AppCommand::Runs { package, prune } => {
            let package = from_home(ctx, "vibe app runs", &app_home, &package)?;
            records::runs(ctx, app_home.settings(), &package, prune)?;
            Ok(None)
        }
        AppCommand::Work {
            package,
            import,
            clean,
            all: _,
        } => {
            let package = package
                .map(|package| from_home(ctx, "vibe app work", &app_home, &package))
                .transpose()?;
            records::work(
                ctx,
                app_home.settings(),
                package.as_deref(),
                clean,
                import.as_deref(),
            )?;
            Ok(None)
        }
        AppCommand::Errors => {
            records::errors(ctx)?;
            Ok(None)
        }
        AppCommand::Modify {
            package,
            registry,
            installer,
            choice,
            agent,
            keep_work,
        } => {
            // `modify` changes an installation in place, so the home's
            // inventory is the authority on which package is meant.
            let package = from_home(ctx, "vibe app modify", &app_home, &package)?;
            lifecycle::modify(
                ctx,
                app_home.settings(),
                lifecycle::ModifyArgs {
                    package: &package,
                    registry: registry.as_deref(),
                    installer,
                    choice: &choice,
                    agent: agent.as_deref(),
                    keep_work,
                },
                embedded_root,
                offline,
                &policy_operation()?,
            )?;
            Ok(None)
        }
        AppCommand::Outdated { registry } => {
            lifecycle::outdated(
                ctx,
                app_home.settings(),
                registry.as_deref(),
                embedded_root,
                offline,
                &policy_operation()?,
            )?;
            Ok(None)
        }
        AppCommand::Expose { package } => state_verb(
            ctx,
            &app_home,
            "vibe app expose",
            &package,
            lifecycle::StateVerb::Expose,
        ),
        AppCommand::Hide { package } => state_verb(
            ctx,
            &app_home,
            "vibe app hide",
            &package,
            lifecycle::StateVerb::Hide,
        ),
        AppCommand::Mark { mark, package } => {
            let mark = match mark {
                crate::cli::MarkArg::Manual => vibe_app::Mark::Manual,
                crate::cli::MarkArg::Auto => vibe_app::Mark::Auto,
            };
            state_verb(
                ctx,
                &app_home,
                "vibe app mark",
                &package,
                lifecycle::StateVerb::Mark(mark),
            )
        }
        AppCommand::Hold { package } => state_verb(
            ctx,
            &app_home,
            "vibe app hold",
            &package,
            lifecycle::StateVerb::Hold(true),
        ),
        AppCommand::Unhold { package } => state_verb(
            ctx,
            &app_home,
            "vibe app unhold",
            &package,
            lifecycle::StateVerb::Hold(false),
        ),
        AppCommand::Rollback { package } => state_verb(
            ctx,
            &app_home,
            "vibe app rollback",
            &package,
            lifecycle::StateVerb::Rollback,
        ),
        AppCommand::Autoremove { plan_only } => {
            lifecycle::autoremove(
                ctx,
                app_home.settings(),
                plan_only,
                offline,
                &policy_operation()?,
            )?;
            Ok(None)
        }
        AppCommand::Steps { package, registry } => {
            describe::show(
                ctx,
                describe::DescribeSource {
                    home: app_home.settings(),
                    registry: registry.as_deref(),
                    embedded_root,
                    offline,
                    operation: &policy_operation()?,
                },
                &package,
                true,
                false,
            )?;
            Ok(None)
        }
        AppCommand::Describe {
            package,
            registry,
            configuration,
        } => {
            describe::show(
                ctx,
                describe::DescribeSource {
                    home: app_home.settings(),
                    registry: registry.as_deref(),
                    embedded_root,
                    offline,
                    operation: &policy_operation()?,
                },
                &package,
                false,
                configuration,
            )?;
            Ok(None)
        }
        AppCommand::Launchers {
            package,
            all,
            menu,
            revision,
            export_system,
            collection,
            remove_system,
            dry_run,
        } => {
            if export_system || remove_system {
                export::system(
                    ctx,
                    &app_home,
                    export::Selection {
                        remove: remove_system,
                        collection: collection.as_deref(),
                        project_root: project_root.as_deref(),
                        dry_run,
                    },
                )?;
                return Ok(None);
            }
            if revision {
                catalog::revision(ctx, &app_home)?;
                return Ok(None);
            }
            if all {
                menu::all(ctx, &app_home, project_root.as_deref(), menu)?;
                return Ok(None);
            }
            let package = match package {
                Some(package) => from_home(ctx, "vibe app launchers", &app_home, &package)?,
                None => String::new(),
            };
            let listing = launcher_listing(&app_home, &package, project_root.as_deref())?;
            if ctx.is_json() {
                ctx.emit_json(&listing)?;
            } else {
                print_listing(&listing, ctx.is_quiet());
            }
            Ok(None)
        }
        AppCommand::Icon {
            package,
            launcher,
            size,
            prefer,
        } => {
            let package = from_home(ctx, "vibe app icon", &app_home, &package)?;
            menu::icon(
                ctx,
                &app_home,
                &package,
                launcher.as_deref(),
                size,
                prefer.map(|prefer| prefer.slot()),
            )?;
            Ok(None)
        }
        AppCommand::ImportDesktop { files, app_root } => {
            menu::import_desktop(&files, app_root.as_deref())?;
            Ok(None)
        }
        AppCommand::List => {
            let listing = serde_json::to_value(vibe_app::list(app_home.settings())?)?;
            if ctx.is_json() {
                ctx.emit_json(&listing)?;
            } else {
                print_list(&listing, ctx.is_quiet());
                if let home::AppHome::User(settings) = &app_home
                    && !ctx.is_quiet()
                {
                    println!("\n{}", views::places(settings).join("\n"));
                }
            }
            Ok(None)
        }
        AppCommand::Export(export) => {
            // `-g` is implied: only an explicit `--path` exports a project home.
            let home = if explicit_path {
                app_home.clone()
            } else {
                home::AppHome::User(global::settings_root()?)
            };
            portable::export(ctx, home.settings(), &export)?;
            Ok(None)
        }
        AppCommand::Import(import) => portable::import(ctx, &import, explicit_path, offline),
        AppCommand::Use {
            slot,
            key,
            none,
            default,
            version_file,
        } => {
            let request = alternatives::UseRequest::of(key, none, default)?;
            let slot = short_name::slot_target(ctx, "vibe app use", app_home.settings(), &slot)?;
            alternatives::use_default(ctx, app_home.settings(), &slot, request, &version_file)?;
            Ok(None)
        }
        AppCommand::Alternatives { slot } => {
            let slot =
                short_name::slot_target(ctx, "vibe app alternatives", app_home.settings(), &slot)?;
            alternatives::alternatives(ctx, app_home.settings(), &slot)?;
            Ok(None)
        }
        AppCommand::Migrate(args) => {
            migrate::run(ctx, app_home.settings(), args, embedded_root, offline)
        }
        AppCommand::Extras(args) => {
            extras::run(ctx, app_home.settings(), args, embedded_root, offline)
        }
        AppCommand::Env(args) => {
            env::run(ctx, args)?;
            Ok(None)
        }
        AppCommand::Versions(args) => {
            versions::run(
                ctx,
                app_home.settings(),
                args,
                embedded_root,
                offline,
                &policy_operation()?,
            )?;
            Ok(None)
        }
        AppCommand::Exec(args) => exec::run(ctx, args),
        AppCommand::Run {
            package,
            launcher_id,
            args,
        } => {
            let package = from_home(ctx, "vibe app run", &app_home, &package)?;
            let code = ctx.suspend_progress(|| {
                run_launcher(
                    &app_home,
                    &package,
                    &launcher_id,
                    &args,
                    project_root.as_deref(),
                )
            })?;
            Ok(Some(code))
        }
        AppCommand::Running => {
            runtime::running(ctx, &app_home)?;
            Ok(None)
        }
        AppCommand::Stop { target } => {
            runtime::stop(ctx, &app_home, &target)?;
            Ok(None)
        }
        AppCommand::Watch => {
            runtime::watch(ctx, &app_home)?;
            Ok(None)
        }
        AppCommand::Collection { command } => {
            catalog::collection(ctx, &app_home, command)?;
            Ok(None)
        }
        AppCommand::Pin { key, to } => {
            catalog::pin(ctx, &app_home, &key, to, true)?;
            Ok(None)
        }
        AppCommand::Unpin { key, from } => {
            catalog::pin(ctx, &app_home, &key, from, false)?;
            Ok(None)
        }
        AppCommand::Launcher {
            key,
            hide,
            show,
            name,
            icon,
            reset,
        } => {
            let change = catalog::launcher_change(hide, show, name, icon, reset)?;
            catalog::launcher(ctx, &app_home, &key, change)?;
            Ok(None)
        }
        AppCommand::Start {
            package,
            launcher,
            args,
        } => {
            let (package, launcher) = catalog::start_target(&package, launcher)?;
            // After the `#<launcher id>` split, what is left is the package
            // the short name may stand for.
            let package = from_home(ctx, "vibe app start", &app_home, &package)?;
            let Some(id) = catalog::start_launcher_id(
                ctx,
                &app_home,
                &package,
                launcher,
                project_root.as_deref(),
            )?
            else {
                return Ok(None);
            };
            // `--json` answers a process id and a log instead of a status:
            // a client drawing a start menu is not blocked by the program
            // it launched (PROP-059 `#launcher-catalog`).
            if ctx.is_json() {
                catalog::start_detached(
                    ctx,
                    &app_home,
                    &package,
                    &id,
                    &args,
                    project_root.as_deref(),
                )?;
                return Ok(None);
            }
            let code = ctx.suspend_progress(|| {
                run_launcher(&app_home, &package, &id, &args, project_root.as_deref())
            })?;
            // The program's lines are not blocks of ours, but the trailer
            // still stands off them by the usual blank line.
            output::language::mark(output::language::Stream::Err, output::language::Kind::Block);
            ctx.trailer_on(
                output::language::Stream::Err,
                code == 0,
                &format!(
                    "exited {} · exit {code}",
                    vibe_app::launcher_key(&package, &id)
                ),
            );
            Ok(Some(code))
        }
    }
}
