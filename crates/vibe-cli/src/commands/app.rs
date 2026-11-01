//! CLI adapter for installed applications: `vibe app installers`,
//! `launchers`, `icon`, `import-desktop`, `run`, `start`, `running`,
//! `stop`, `watch`, `list`, `use` and `alternatives` over the
//! `vibe-app` library. This adapter parses, chooses an ID in an attended
//! terminal and renders.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-operations");

pub(crate) mod agents;
mod alternatives;
mod catalog;
mod describe;
pub(crate) mod display;
mod environment;
mod export;
mod facts;
pub(crate) mod global;
pub(crate) mod home;
mod interrupt;
mod launch;
mod lifecycle;
mod menu;
mod processes;
pub(crate) mod project;
mod records;
mod report;
mod runtime;
mod views;
mod wizard;

use launch::{launcher_listing, run_launcher};
use views::{print_installers, print_list, print_listing};

use std::path::Path;

use anyhow::Result;

use crate::cli::{AppArgs, AppCommand};
use crate::output;

pub fn run(
    ctx: &output::Context,
    args: AppArgs,
    embedded_root: Option<&Path>,
    offline: bool,
) -> Result<Option<i32>> {
    let project_root = home::selected_project(args.path.as_deref())?;
    let app_home = home::select(args.global, project_root.as_deref())?;
    match args.command {
        AppCommand::Installers { package, registry } => {
            let settings = global::settings_root()?;
            let resolved = global::resolve_package(
                &settings,
                registry.as_deref(),
                embedded_root,
                &package,
                offline,
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
            registry,
            platform,
            arch,
            installer,
            choice,
            agent,
        } => {
            let settings = global::settings_root()?;
            let resolved = global::resolve_package(
                &settings,
                registry.as_deref(),
                embedded_root,
                &package,
                offline,
            )?;
            let proxy_image = global::proxy_image()?;
            let flat = display::StepDisplay::flat();
            let observe = global::Observation::new(ctx, false, &flat)?;
            let agents = agents::AgentSetup::new(ctx, agent.as_deref(), false)?;
            let packages = global::Packages {
                settings: settings.clone(),
                registry: registry.clone(),
                embedded: embedded_root.map(Path::to_path_buf),
                offline,
            };
            let resolver = |spelling: &str| packages.resolve(spelling);
            let operation = vibe_app::OperationContext {
                agent: agents.hooks(),
                resolver: Some(&resolver),
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
            );
            let plan = ctx.suspend_progress(|| vibe_app::plan(&operation, &resolved, &request))?;
            global::render_plan(ctx, &plan)?;
            Ok(None)
        }
        AppCommand::Runs { package, prune } => {
            records::runs(ctx, app_home.settings(), &package, prune)?;
            Ok(None)
        }
        AppCommand::Work {
            package,
            clean,
            all: _,
        } => {
            records::work(ctx, app_home.settings(), package.as_deref(), clean)?;
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
            )?;
            Ok(None)
        }
        AppCommand::Expose { package } => {
            lifecycle::change(
                ctx,
                app_home.settings(),
                &package,
                lifecycle::StateVerb::Expose,
            )?;
            Ok(None)
        }
        AppCommand::Hide { package } => {
            lifecycle::change(
                ctx,
                app_home.settings(),
                &package,
                lifecycle::StateVerb::Hide,
            )?;
            Ok(None)
        }
        AppCommand::Mark { mark, package } => {
            let mark = match mark {
                crate::cli::MarkArg::Manual => vibe_app::Mark::Manual,
                crate::cli::MarkArg::Auto => vibe_app::Mark::Auto,
            };
            lifecycle::change(
                ctx,
                app_home.settings(),
                &package,
                lifecycle::StateVerb::Mark(mark),
            )?;
            Ok(None)
        }
        AppCommand::Hold { package } => {
            lifecycle::change(
                ctx,
                app_home.settings(),
                &package,
                lifecycle::StateVerb::Hold(true),
            )?;
            Ok(None)
        }
        AppCommand::Unhold { package } => {
            lifecycle::change(
                ctx,
                app_home.settings(),
                &package,
                lifecycle::StateVerb::Hold(false),
            )?;
            Ok(None)
        }
        AppCommand::Rollback { package } => {
            lifecycle::change(
                ctx,
                app_home.settings(),
                &package,
                lifecycle::StateVerb::Rollback,
            )?;
            Ok(None)
        }
        AppCommand::Autoremove { plan_only } => {
            lifecycle::autoremove(ctx, app_home.settings(), plan_only, offline)?;
            Ok(None)
        }
        AppCommand::Steps { package, registry } => {
            describe::show(
                ctx,
                registry.as_deref(),
                embedded_root,
                &package,
                offline,
                true,
            )?;
            Ok(None)
        }
        AppCommand::Describe { package, registry } => {
            describe::show(
                ctx,
                registry.as_deref(),
                embedded_root,
                &package,
                offline,
                false,
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
            let package = package.unwrap_or_default();
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
            }
            Ok(None)
        }
        AppCommand::Use { slot, key } => {
            alternatives::use_default(ctx, app_home.settings(), &slot, &key)?;
            Ok(None)
        }
        AppCommand::Alternatives { slot } => {
            alternatives::alternatives(ctx, app_home.settings(), &slot)?;
            Ok(None)
        }
        AppCommand::Run {
            package,
            launcher_id,
            args,
        } => {
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
            Ok(Some(code))
        }
    }
}
