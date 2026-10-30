//! `vibe install|update|uninstall -g` for user applications: argument
//! checks, package resolution and rendering around the `vibe-app` library.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#commands");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use dialoguer::Select;
use vibe_app::{
    Command as AppCommand, InstallChoice, InstallRequest, OperationContext, Outcome,
    ResolvedPackage, SettingsRoot,
};
use vibe_wire::generated::application_report::{
    ApplicationReport, ApplicationReportCommand, ApplicationReportSelectedMode,
};

use crate::cli::{InstallArgs, UninstallArgs, UpdateArgs};
use crate::output;

/// The current user's settings root, resolved once at this boundary.
pub(crate) fn settings_root() -> Result<SettingsRoot> {
    let root = vibe_core::settings::settings_dir().ok_or_else(|| {
        anyhow::anyhow!(
            "the Vibe settings root is unavailable; set VIBE_SETTINGS or make the user home resolvable"
        )
    })?;
    Ok(SettingsRoot::new(&root))
}

/// Resolve one app package: an explicit (or embedded) local registry first,
/// otherwise the configured global registries.
pub(crate) fn resolve_package(
    settings: &SettingsRoot,
    registry: Option<&Path>,
    embedded_root: Option<&Path>,
    spelling: &str,
    offline: bool,
) -> Result<ResolvedPackage> {
    if let Some(root) = registry.or(embedded_root) {
        let local = crate::registry::application_local_registry(root.to_path_buf())
            .with_context(|| format!("opening the application registry `{}`", root.display()))?;
        return Ok(vibe_app::from_local_registry(&local, root, spelling)?);
    }
    Ok(vibe_app::from_global(settings.path(), spelling, offline)?)
}

fn chooser(prompt: &str, items: &[String]) -> std::result::Result<Option<usize>, String> {
    Select::new()
        .with_prompt(prompt)
        .items(items)
        .default(0)
        .interact_opt()
        .map_err(|error| error.to_string())
}

fn attended(ctx: &output::Context) -> bool {
    !ctx.is_json() && !ctx.is_unattended() && console::user_attended()
}

pub fn install(
    ctx: &output::Context,
    args: InstallArgs,
    embedded_root: Option<PathBuf>,
    root_offline: bool,
) -> Result<()> {
    if args.packages.len() != 1 {
        bail!("global install requires exactly one fully qualified application package");
    }
    if args.path != Path::new(".")
        || args.agent.is_some()
        || args.language.is_some()
        || !args.features.is_empty()
        || args.no_default_features
        || args.all_features
        || args.exact
        || args.auth_required
        || args.solver.is_some()
        || args.prefer_embedded
        || args.no_prefer_embedded
        || args.no_default_registry
        || args.embedded_short_circuit
        || args.prefer_local
        || args.no_prefer_local
        || args.git.is_some()
        || args.tag.is_some()
        || args.branch.is_some()
        || args.rev.is_some()
        || args.git_auth.is_some()
        || args.git_token_env.is_some()
        || args.trace_compile
    {
        bail!("global install received project-only package flags");
    }
    let offline = root_offline || args.offline;
    let settings = settings_root()?;
    let resolved = resolve_package(
        &settings,
        args.registry.as_deref(),
        embedded_root.as_deref(),
        &args.packages[0],
        offline,
    )?;
    let request = InstallRequest {
        choice: InstallChoice {
            platform: args.platform,
            arch: args.arch,
            installer: args.installer,
        },
        library: args.library,
        force: args.force,
    };
    let outcome = ctx.suspend_progress(|| {
        let attended = attended(ctx);
        let operation = OperationContext {
            settings: &settings,
            json: ctx.is_json(),
            offline,
            chooser: attended.then_some(&chooser as vibe_app::Chooser<'_>),
        };
        vibe_app::install(&operation, &resolved, &request)
    })?;
    render(ctx, &outcome)
}

pub fn update(
    ctx: &output::Context,
    args: UpdateArgs,
    embedded_root: Option<PathBuf>,
    root_offline: bool,
) -> Result<()> {
    if args.packages.len() != 1 || args.all {
        bail!("global update requires exactly one fully qualified application package");
    }
    if args.path != Path::new(".") || args.exact || args.auth_required || args.trace_compile {
        bail!("global update received project-only package flags");
    }
    let settings = settings_root()?;
    let resolved = resolve_package(
        &settings,
        args.registry.as_deref(),
        embedded_root.as_deref(),
        &args.packages[0],
        root_offline,
    )?;
    let request = InstallRequest {
        choice: InstallChoice {
            platform: args.platform,
            arch: args.arch,
            installer: args.installer,
        },
        library: args.library,
        force: args.force,
    };
    let outcome = ctx.suspend_progress(|| {
        let attended = attended(ctx);
        let operation = OperationContext {
            settings: &settings,
            json: ctx.is_json(),
            offline: root_offline,
            chooser: attended.then_some(&chooser as vibe_app::Chooser<'_>),
        };
        vibe_app::update(&operation, &resolved, &request)
    })?;
    render(ctx, &outcome)
}

pub fn uninstall(ctx: &output::Context, args: UninstallArgs, root_offline: bool) -> Result<()> {
    vibe_app::qualified_ref(&args.package)?;
    if args.path != Path::new(".") {
        bail!("global uninstall received project-only --path");
    }
    let settings = settings_root()?;
    let outcome = ctx.suspend_progress(|| {
        let operation = OperationContext {
            settings: &settings,
            json: ctx.is_json(),
            offline: root_offline,
            chooser: None,
        };
        vibe_app::uninstall(&operation, &args.package, args.force)
    })?;
    render(ctx, &outcome)
}

fn render(ctx: &output::Context, outcome: &Outcome) -> Result<()> {
    let command = match outcome.command {
        AppCommand::Install => "install",
        AppCommand::Update => "update",
        AppCommand::Uninstall => "uninstall",
    };
    if ctx.is_json() {
        return ctx.emit_json(&ApplicationReport {
            protocol: "vibe-application-command-report/2".into(),
            ok: true,
            command: match outcome.command {
                AppCommand::Install => ApplicationReportCommand::Install,
                AppCommand::Update => ApplicationReportCommand::Update,
                AppCommand::Uninstall => ApplicationReportCommand::Uninstall,
            },
            application_id: outcome.slot.clone(),
            installation: outcome.key.clone(),
            message: outcome.message.clone(),
            slot_default: outcome.slot_default.clone(),
            app_root: outcome
                .app_root
                .as_deref()
                .map(vibe_core::machine_json_path),
            selected_mode: Some(match outcome.method {
                vibe_app::Method::Shell => ApplicationReportSelectedMode::Shell,
                vibe_app::Method::Zip => ApplicationReportSelectedMode::Zip,
                vibe_app::Method::TarGz => ApplicationReportSelectedMode::TarGz,
                vibe_app::Method::Exe => ApplicationReportSelectedMode::Exe,
                vibe_app::Method::Msi => ApplicationReportSelectedMode::Msi,
                vibe_app::Method::SourceBuild => ApplicationReportSelectedMode::SourceBuild,
            }),
        });
    }
    ctx.summary(&format!(
        "vibe {command} -g: application `{}` — {}",
        outcome.slot, outcome.message
    ));
    Ok(())
}
