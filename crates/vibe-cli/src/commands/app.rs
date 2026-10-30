//! CLI adapter for installed applications: `vibe app installers`,
//! `launchers`, `run`, `start` and `list` over the `vibe-app` library. This
//! adapter parses, chooses an ID in an attended terminal and renders.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-operations");

pub(crate) mod global;

use std::ffi::OsString;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use dialoguer::Select;
use serde_json::Value;
use vibe_app::{Index, LaunchRequest, Storage};
use vibe_core::manifest::Manifest;

use crate::cli::{AppArgs, AppCommand};
use crate::output;

pub fn run(
    ctx: &output::Context,
    args: AppArgs,
    embedded_root: Option<&Path>,
    offline: bool,
) -> Result<Option<i32>> {
    let app_path = args.path;
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
        AppCommand::Launchers { package } => {
            let project_root = selected_project_root(app_path.as_deref())?;
            let listing = launcher_listing(&package, project_root.as_deref())?;
            if ctx.is_json() {
                ctx.emit_json(&listing)?;
            } else {
                print_listing(&listing, ctx.is_quiet());
            }
            Ok(None)
        }
        AppCommand::List => {
            let listing = serde_json::to_value(vibe_app::list(&global::settings_root()?)?)?;
            if ctx.is_json() {
                ctx.emit_json(&listing)?;
            } else {
                print_list(&listing, ctx.is_quiet());
            }
            Ok(None)
        }
        AppCommand::Run {
            package,
            launcher_id,
            args,
        } => {
            let project_root = selected_project_root(app_path.as_deref())?;
            let code = ctx.suspend_progress(|| {
                run_launcher(&package, &launcher_id, &args, project_root.as_deref())
            })?;
            Ok(Some(code))
        }
        AppCommand::Start {
            package,
            launcher,
            args,
        } => {
            let project_root = selected_project_root(app_path.as_deref())?;
            let id = match launcher {
                Some(id) => id,
                None => {
                    let listing = launcher_listing(&package, project_root.as_deref())?;
                    let Some(id) = choose_launcher(&listing, ctx)? else {
                        return Ok(None);
                    };
                    id
                }
            };
            let code = ctx
                .suspend_progress(|| run_launcher(&package, &id, &args, project_root.as_deref()))?;
            Ok(Some(code))
        }
    }
}

fn launcher_listing(package: &str, project_root: Option<&Path>) -> Result<Value> {
    let settings = global::settings_root()?;
    let index = Index::load(&settings)?;
    let storage = Storage::load(&settings)?;
    let (_, installation) = vibe_app::installed(&index, package)?;
    let listing = vibe_app::launcher_listing(installation, &storage, settings.path(), project_root);
    Ok(serde_json::to_value(listing)?)
}

fn run_launcher(
    package: &str,
    launcher_id: &str,
    args: &[String],
    project_root: Option<&Path>,
) -> Result<i32> {
    let settings = global::settings_root()?;
    let index = Index::load(&settings)?;
    let storage = Storage::load(&settings)?;
    let (_, installation) = vibe_app::installed(&index, package)?;
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

fn print_list(listing: &Value, quiet: bool) {
    let installations = listing
        .get("installations")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    println!("{} installed application(s)", installations.len());
    if quiet {
        return;
    }
    for entry in installations {
        let key = launcher_field(entry, "key");
        let slot = launcher_field(entry, "slot");
        let method = launcher_field(entry, "method");
        let status = launcher_field(entry, "status");
        let default = if entry.get("default").and_then(Value::as_bool) == Some(true) {
            " [default]"
        } else {
            ""
        };
        match entry.get("reason").and_then(Value::as_str) {
            Some(reason) => println!("  {slot}: {key} ({method}, {status}: {reason}){default}"),
            None => println!("  {slot}: {key} ({method}, {status}){default}"),
        }
    }
}

fn selected_project_root(explicit: Option<&Path>) -> Result<Option<PathBuf>> {
    if let Some(path) = explicit {
        return super::resolve_project_root(path).map(Some);
    }
    let cwd = std::env::current_dir()?;
    if cwd.join(Manifest::FILENAME).is_file() {
        return super::resolve_project_root(&cwd).map(Some);
    }
    Ok(None)
}

fn launchers(listing: &Value) -> Result<&[Value]> {
    listing
        .get("launchers")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| anyhow::anyhow!("application launcher listing has no launchers array"))
}

fn launcher_field<'a>(launcher: &'a Value, field: &str) -> &'a str {
    launcher.get(field).and_then(Value::as_str).unwrap_or("")
}

fn print_installers(listing: &Value, package: &str, quiet: bool) {
    let platforms = listing
        .get("platforms")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let installer_count: usize = platforms
        .iter()
        .flat_map(|platform| {
            platform
                .get("architectures")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .map(|arch| {
            arch.get("installers")
                .and_then(Value::as_array)
                .map_or(0, Vec::len)
        })
        .sum();
    println!("{package}: {installer_count} installer(s)");
    if quiet {
        return;
    }
    if let (Some(platform), Some(arch)) = (
        listing
            .pointer("/detected/platform")
            .and_then(Value::as_str),
        listing.pointer("/detected/arch").and_then(Value::as_str),
    ) {
        println!("Detected: {platform}/{arch}");
    }
    for platform in platforms {
        let platform_id = launcher_field(platform, "id");
        let platform_name = launcher_field(platform, "name");
        let detected = if platform.get("detected").and_then(Value::as_bool) == Some(true) {
            " [detected]"
        } else {
            ""
        };
        println!("  {platform_name} ({platform_id}){detected}");
        let Some(architectures) = platform.get("architectures").and_then(Value::as_array) else {
            continue;
        };
        for arch in architectures {
            let arch_id = launcher_field(arch, "id");
            let arch_name = launcher_field(arch, "name");
            let detected = if arch.get("detected").and_then(Value::as_bool) == Some(true) {
                " [detected]"
            } else {
                ""
            };
            println!("    {arch_name} ({arch_id}){detected}");
            let Some(installers) = arch.get("installers").and_then(Value::as_array) else {
                continue;
            };
            for installer in installers {
                let id = launcher_field(installer, "id");
                let name = launcher_field(installer, "name");
                println!("      {id}: {name}");
                if let Some(installation) = installer.get("installation") {
                    for field in [
                        "type",
                        "source",
                        "sha256",
                        "payload_root",
                        "app_root",
                        "install",
                        "force_install",
                        "update",
                        "uninstall",
                        "force_uninstall",
                        "arguments",
                        "force_arguments",
                        "interpreter",
                        "sources",
                        "recipe",
                    ] {
                        if let Some(value) =
                            installation.get(field).filter(|value| !value.is_null())
                        {
                            println!("        {field}: {value}");
                        }
                    }
                }
                for field in ["portable", "pre_install", "post_install", "launchers"] {
                    if let Some(value) = installer.get(field).filter(|value| !value.is_null()) {
                        println!("        {field}: {value}");
                    }
                }
            }
        }
    }
}

fn print_listing(listing: &Value, quiet: bool) {
    let group = listing.pointer("/package/group").and_then(Value::as_str);
    let name = listing.pointer("/package/name").and_then(Value::as_str);
    let version = listing.pointer("/package/version").and_then(Value::as_str);
    let package = match (group, name, version) {
        (Some(group), Some(name), Some(version)) => format!("{group}/{name}@{version}"),
        _ => "application".to_owned(),
    };
    let default = listing.get("default_launcher_id").and_then(Value::as_str);
    let Some(launchers) = listing.get("launchers").and_then(Value::as_array) else {
        return;
    };
    if quiet {
        println!("{package}: {} launcher(s)", launchers.len());
        return;
    }
    println!("{package}: {} launcher(s)", launchers.len());
    for launcher in launchers {
        let id = launcher_field(launcher, "id");
        let name = launcher_field(launcher, "name");
        let status = launcher_field(launcher, "status");
        let marker = if default == Some(id) {
            " [default]"
        } else {
            ""
        };
        let reason = launcher_field(launcher, "reason");
        if reason.is_empty() {
            println!("  {id}{marker}: {name} ({status})");
        } else {
            println!("  {id}{marker}: {name} ({status}: {reason})");
        }
        if let Some(description) = launcher.get("description").and_then(Value::as_str) {
            println!("    {description}");
        }
        let terminal = launcher
            .get("terminal")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        println!("    terminal: {terminal}");
        for field in ["executable", "args", "working_directory", "icon"] {
            if let Some(value) = launcher.get(field) {
                println!("    {field}: {value}");
            }
        }
    }
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
