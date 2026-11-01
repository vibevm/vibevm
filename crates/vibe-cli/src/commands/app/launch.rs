//! Running a launch point from the CLI (PROP-059 `#launcher-operations`,
//! `#launcher-catalog`): the one-installation listing a selection reads, the
//! attended `app run` / `app start`, and the attended choice of a launcher
//! when the package declares no default.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launcher-operations");

use std::ffi::OsString;
use std::io::IsTerminal;
use std::path::Path;

use anyhow::{Result, bail};
use serde_json::Value;
use vibe_app::{Index, LaunchRequest, LauncherCatalog, Storage};

use super::home::AppHome;
use super::views::launcher_field;
use crate::output;

pub(super) fn launcher_listing(
    app_home: &AppHome,
    package: &str,
    project_root: Option<&Path>,
) -> Result<Value> {
    let settings = app_home.settings();
    let index = Index::load(settings)?;
    let storage = Storage::load(settings)?;
    let (_, installation) = vibe_app::installed(settings, &index, package)?;
    let catalog = LauncherCatalog::load(settings)?;
    let listing = vibe_app::launcher_listing(installation, &storage, &catalog, project_root);
    Ok(serde_json::to_value(listing)?)
}

pub(super) fn run_launcher(
    app_home: &AppHome,
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

pub(super) fn choose_launcher(listing: &Value, ctx: &output::Context) -> Result<Option<String>> {
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

    // An unavailable launcher stays in the list with its reason as the
    // row's sentence, so the reader sees why it cannot be chosen.
    let choices = options
        .iter()
        .map(|(id, name, status, reason)| vibe_app::Choice {
            description: (status != "available").then(|| format!("{status}: {reason}")),
            ..vibe_app::Choice::named(name, id)
        })
        .collect::<Vec<_>>();
    let first_available = options
        .iter()
        .position(|(_, _, status, _)| status == "available")
        .ok_or_else(|| anyhow::anyhow!("launcher_unavailable: no available launchers"))?;
    let selected = crate::output::prompts::choose_opt(
        "Select application launcher",
        &choices,
        first_available,
    )?;
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
