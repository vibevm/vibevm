//! CLI adapter for installed applications: `vibe app installers`,
//! `launchers`, `icon`, `import-desktop`, `run`, `start`, `list`, `use` and
//! `alternatives` over the
//! `vibe-app` library. This adapter parses, chooses an ID in an attended
//! terminal and renders.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-operations");

pub(crate) mod agents;
mod alternatives;
mod describe;
mod environment;
mod facts;
pub(crate) mod global;
pub(crate) mod home;
mod interrupt;
mod lifecycle;
mod menu;
pub(crate) mod project;
mod records;
mod report;
mod views;

use views::{launcher_field, print_installers, print_list, print_listing};

use std::ffi::OsString;
use std::io::IsTerminal;
use std::path::Path;

use anyhow::{Result, bail};
use dialoguer::Select;
use serde_json::Value;
use vibe_app::{Index, LaunchRequest, Storage};

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
            let observe = global::Observation::new(ctx, false)?;
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
        AppCommand::Launchers { package, all, menu } => {
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
        AppCommand::Start {
            package,
            launcher,
            args,
        } => {
            let id = match launcher {
                Some(id) => id,
                None => {
                    let listing = launcher_listing(&app_home, &package, project_root.as_deref())?;
                    let Some(id) = choose_launcher(&listing, ctx)? else {
                        return Ok(None);
                    };
                    id
                }
            };
            let code = ctx.suspend_progress(|| {
                run_launcher(&app_home, &package, &id, &args, project_root.as_deref())
            })?;
            Ok(Some(code))
        }
    }
}

fn launcher_listing(
    app_home: &home::AppHome,
    package: &str,
    project_root: Option<&Path>,
) -> Result<Value> {
    let settings = app_home.settings();
    let index = Index::load(settings)?;
    let storage = Storage::load(settings)?;
    let (_, installation) = vibe_app::installed(settings, &index, package)?;
    let listing = vibe_app::launcher_listing(installation, &storage, settings.path(), project_root);
    Ok(serde_json::to_value(listing)?)
}

fn run_launcher(
    app_home: &home::AppHome,
    package: &str,
    launcher_id: &str,
    args: &[String],
    project_root: Option<&Path>,
) -> Result<i32> {
    let settings = app_home.settings();
    let index = Index::load(settings)?;
    let storage = Storage::load(settings)?;
    let (_, installation) = vibe_app::installed(settings, &index, package)?;
    let extra_args: Vec<OsString> = args.iter().map(OsString::from).collect();
    let request = LaunchRequest {
        launcher_id,
        extra_args: &extra_args,
        vibe_home: settings.path(),
        project_root,
        attended_terminal: std::io::stdin().is_terminal() && std::io::stdout().is_terminal(),
    };
    Ok(vibe_app::run_launcher(installation, &storage, &request)?)
}

fn launchers(listing: &Value) -> Result<&[Value]> {
    listing
        .get("launchers")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| anyhow::anyhow!("application launcher listing has no launchers array"))
}

fn choose_launcher(listing: &Value, ctx: &output::Context) -> Result<Option<String>> {
    let launchers = launchers(listing)?;
    if launchers.is_empty() {
        bail!("no_launchers: installed application declares no launchers");
    }
    if let Some(default) = listing.get("default_launcher_id").and_then(Value::as_str) {
        return Ok(Some(default.to_owned()));
    }

    let options = launchers
        .iter()
        .filter_map(|launcher| {
            let id = launcher.get("id")?.as_str()?;
            let name = launcher_field(launcher, "name");
            let status = launcher_field(launcher, "status");
            let reason = launcher_field(launcher, "reason");
            Some((
                id.to_owned(),
                name.to_owned(),
                status.to_owned(),
                reason.to_owned(),
            ))
        })
        .collect::<Vec<_>>();
    let available = options
        .iter()
        .filter(|(_, _, status, _)| status == "available")
        .count();
    if available == 0 {
        let reasons = options
            .iter()
            .map(|(id, _, status, reason)| format!("{id}: {status} {reason}"))
            .collect::<Vec<_>>();
        bail!("launcher_unavailable: {}", reasons.join("; "));
    }

    let interactive = !ctx.is_json()
        && !ctx.is_unattended()
        && std::io::stdin().is_terminal()
        && std::io::stderr().is_terminal();
    if !interactive {
        let ids = options
            .iter()
            .map(|(id, _, _, _)| id.as_str())
            .collect::<Vec<_>>();
        bail!(
            "selection_required: specify --launcher or use `vibe app run PACKAGE ID`; available IDs: {}",
            ids.join(", ")
        );
    }

    let labels = options
        .iter()
        .map(|(id, name, status, reason)| {
            if status == "available" {
                format!("{name} ({id})")
            } else {
                format!("{name} ({id}) — {status}: {reason}")
            }
        })
        .collect::<Vec<_>>();
    let first_available = options
        .iter()
        .position(|(_, _, status, _)| status == "available")
        .ok_or_else(|| anyhow::anyhow!("launcher_unavailable: no available launchers"))?;
    let selected = Select::new()
        .with_prompt("Select application launcher")
        .items(&labels)
        .default(first_available)
        .interact_opt()?;
    let Some(index) = selected else {
        return Ok(None);
    };
    let (id, _, status, reason) = &options[index];
    if status != "available" {
        bail!("launcher_unavailable: `{id}`: {status}: {reason}");
    }
    Ok(Some(id.clone()))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{choose_launcher, launchers};
    use crate::cli::AgentModeArg;
    use crate::output::Context;

    fn unattended_context() -> Context {
        Context::from_flags(false, false, None, true, AgentModeArg::Auto)
    }

    #[test]
    fn listing_reader_preserves_explicit_empty_launchers() {
        assert!(launchers(&json!({"launchers": []})).unwrap().is_empty());
        assert!(launchers(&json!({})).is_err());
    }

    #[test]
    fn explicit_default_wins_even_when_unavailable() {
        let listing = json!({
            "default_launcher_id": "editor",
            "launchers": [
                {"id": "editor", "name": "Editor", "status": "requires_project"},
                {"id": "console", "name": "Console", "status": "available"},
            ],
        });
        assert_eq!(
            choose_launcher(&listing, &unattended_context()).unwrap(),
            Some("editor".into())
        );
    }

    #[test]
    fn one_non_default_launcher_still_requires_selection_unattended() {
        let listing = json!({
            "default_launcher_id": null,
            "launchers": [{"id": "only", "name": "Only", "status": "available"}],
        });
        let error = choose_launcher(&listing, &unattended_context())
            .unwrap_err()
            .to_string();
        assert!(error.contains("selection_required"), "{error}");
        assert!(error.contains("only"), "{error}");
    }
}
