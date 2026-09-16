//! Generic user-local application command routing.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#commands");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#ownership");

mod model;
mod process;
mod source;
mod store;

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use vibe_wire::generated::application_report::{ApplicationReport, ApplicationReportCommand};

use crate::cli::{InstallArgs, UninstallArgs, UpdateArgs};
use crate::output;

use model::{
    ApplicationContext, ApplicationOperation, ApplicationRecord, ApplicationStatus,
    CONTEXT_PROTOCOL, ManagementEntry,
};
use process::{dispatch, validate_management};
use source::{qualified_ref, resolve_application};
use store::ApplicationStore;

pub fn install(
    ctx: &output::Context,
    args: InstallArgs,
    embedded_root: Option<PathBuf>,
    root_offline: bool,
) -> Result<()> {
    validate_install_args(&args)?;
    let spelling = &args.packages[0];
    let registry = select_registry(args.registry.as_deref(), embedded_root.as_deref())?;
    let resolved = resolve_application(&registry, spelling)?;
    let settings = settings_root()?;
    let store = ApplicationStore::open(&settings)?;
    let mut index = store.load()?;
    reject_identity_collision(&index, &resolved.application)?;
    let host = settings
        .join("opt")
        .join("apps")
        .join(&resolved.application.id);
    let context = context(
        ApplicationOperation::Install,
        resolved.application.clone(),
        &settings,
        host,
        Some(registry),
        root_offline || args.offline,
    )?;
    let (context_path, reply_path) = store.request_paths()?;
    let reply = dispatch(
        &resolved.installer_entry,
        &resolved.installer_root,
        &context,
        &context_path,
        &reply_path,
    )?;
    let management = ready_management(&context.host_root, &reply)?;
    index.applications.insert(
        context.application.id.clone(),
        ApplicationRecord {
            application: context.application.clone(),
            host_root: context.host_root.clone(),
            management,
            status: ApplicationStatus::Ready,
        },
    );
    store.save(&index)?;
    render(
        ctx,
        "install",
        &context.application.id,
        &reply.message,
        &context.host_root,
    )
}

pub fn update(
    ctx: &output::Context,
    args: UpdateArgs,
    embedded_root: Option<PathBuf>,
    root_offline: bool,
) -> Result<()> {
    validate_update_args(&args)?;
    let spelling = &args.packages[0];
    let requested = qualified_ref(spelling)?;
    let settings = settings_root()?;
    let store = ApplicationStore::open(&settings)?;
    let mut index = store.load()?;
    let prior = record_for_package(&index, &requested)?.clone();
    let registry = select_registry(args.registry.as_deref(), embedded_root.as_deref())?;
    let resolved = resolve_application(&registry, spelling)?;
    if resolved.application.id != prior.application.id {
        bail!("updated application declaration changes the installed application id");
    }
    reject_identity_collision(&index, &resolved.application)?;
    let context = context(
        ApplicationOperation::Update,
        resolved.application.clone(),
        &settings,
        prior.host_root,
        Some(registry),
        root_offline,
    )?;
    let (context_path, reply_path) = store.request_paths()?;
    let reply = dispatch(
        &resolved.installer_entry,
        &resolved.installer_root,
        &context,
        &context_path,
        &reply_path,
    )?;
    let management = ready_management(&context.host_root, &reply)?;
    index.applications.insert(
        context.application.id.clone(),
        ApplicationRecord {
            application: context.application.clone(),
            host_root: context.host_root.clone(),
            management,
            status: ApplicationStatus::Ready,
        },
    );
    store.save(&index)?;
    render(
        ctx,
        "update",
        &context.application.id,
        &reply.message,
        &context.host_root,
    )
}

pub fn uninstall(ctx: &output::Context, args: UninstallArgs, root_offline: bool) -> Result<()> {
    validate_uninstall_args(&args)?;
    let requested = qualified_ref(&args.package)?;
    let settings = settings_root()?;
    let store = ApplicationStore::open(&settings)?;
    let mut index = store.load()?;
    let prior = record_for_package(&index, &requested)?.clone();
    let management = validate_management(&prior.host_root, &prior.management)?;
    let context = context(
        ApplicationOperation::Uninstall,
        prior.application.clone(),
        &settings,
        prior.host_root.clone(),
        None,
        root_offline,
    )?;
    let (context_path, reply_path) = store.request_paths()?;
    let current_dir = management
        .entry
        .parent()
        .ok_or_else(|| anyhow::anyhow!("retained management entry has no parent"))?;
    let reply = dispatch(
        &management.entry,
        current_dir,
        &context,
        &context_path,
        &reply_path,
    )?;
    if reply.status != ApplicationStatus::Undeployed {
        bail!("application uninstall did not report undeployed");
    }
    index.applications.insert(
        context.application.id.clone(),
        ApplicationRecord {
            status: ApplicationStatus::Undeployed,
            ..prior
        },
    );
    store.save(&index)?;
    render(
        ctx,
        "uninstall",
        &context.application.id,
        &reply.message,
        &context.host_root,
    )
}

fn context(
    operation: ApplicationOperation,
    application: model::ApplicationIdentity,
    settings_root: &Path,
    host_root: PathBuf,
    registry_root: Option<PathBuf>,
    offline: bool,
) -> Result<ApplicationContext> {
    let vibe_executable = crate::commands::init::strip_unc_public(
        std::fs::canonicalize(std::env::current_exe()?).map_err(anyhow::Error::from)?,
    );
    Ok(ApplicationContext {
        protocol: CONTEXT_PROTOCOL.into(),
        operation,
        application,
        settings_root: settings_root.to_path_buf(),
        host_root,
        registry_root,
        vibe_executable,
        offline,
    })
}

fn ready_management(host: &Path, reply: &model::ApplicationReply) -> Result<ManagementEntry> {
    if reply.status != ApplicationStatus::Ready {
        bail!(
            "application installer did not report ready: {}",
            reply.message
        );
    }
    let management = reply
        .management
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("ready application reply has no management entry"))?;
    validate_management(host, management)
}

fn select_registry(explicit: Option<&Path>, embedded: Option<&Path>) -> Result<PathBuf> {
    explicit
        .or(embedded)
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "global source application needs --registry <local-path> or a source-installed Vibe embedded registry"
            )
        })
}

fn settings_root() -> Result<PathBuf> {
    vibe_core::settings::settings_dir().ok_or_else(|| {
        anyhow::anyhow!(
            "the Vibe settings root is unavailable; set VIBE_SETTINGS or make the user home resolvable"
        )
    })
}

fn reject_identity_collision(
    index: &model::ApplicationIndex,
    application: &model::ApplicationIdentity,
) -> Result<()> {
    if let Some(existing) = index.applications.get(&application.id)
        && (existing.application.package.group != application.package.group
            || existing.application.package.name != application.package.name)
    {
        bail!(
            "application id `{}` is already bound to {}/{}",
            application.id,
            existing.application.package.group,
            existing.application.package.name
        );
    }
    if let Some((existing_id, _)) = index.applications.iter().find(|(id, record)| {
        id.as_str() != application.id
            && record.application.package.group == application.package.group
            && record.application.package.name == application.package.name
    }) {
        bail!(
            "application package {}/{} is already bound to application id `{existing_id}`",
            application.package.group,
            application.package.name
        );
    }
    Ok(())
}

fn record_for_package<'a>(
    index: &'a model::ApplicationIndex,
    requested: &vibe_core::PackageRef,
) -> Result<&'a ApplicationRecord> {
    let group = requested
        .group
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("global application package is unqualified"))?
        .to_string();
    index
        .applications
        .values()
        .find(|record| {
            record.application.package.group == group
                && record.application.package.name == requested.name.as_str()
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "global application {group}/{} is not installed",
                requested.name
            )
        })
}

fn validate_install_args(args: &InstallArgs) -> Result<()> {
    if args.packages.len() != 1 {
        bail!("global install requires exactly one fully qualified application package");
    }
    qualified_ref(&args.packages[0])?;
    if args.path != Path::new(".")
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
        || args.force
        || args.trace_compile
    {
        bail!("global install received project-only package flags");
    }
    Ok(())
}

fn validate_update_args(args: &UpdateArgs) -> Result<()> {
    if args.packages.len() != 1 || args.all {
        bail!("global update requires exactly one fully qualified application package");
    }
    qualified_ref(&args.packages[0])?;
    if args.path != Path::new(".") || args.exact || args.auth_required || args.trace_compile {
        bail!("global update received project-only package flags");
    }
    Ok(())
}

fn validate_uninstall_args(args: &UninstallArgs) -> Result<()> {
    qualified_ref(&args.package)?;
    if args.path != Path::new(".") {
        bail!("global uninstall received project-only --path");
    }
    Ok(())
}

fn render(
    ctx: &output::Context,
    command: &str,
    application_id: &str,
    message: &str,
    host_root: &Path,
) -> Result<()> {
    if ctx.is_json() {
        let command = match command {
            "install" => ApplicationReportCommand::Install,
            "update" => ApplicationReportCommand::Update,
            "uninstall" => ApplicationReportCommand::Uninstall,
            _ => unreachable!("closed application command vocabulary"),
        };
        return ctx.emit_json(&ApplicationReport {
            protocol: "vibe-application-command-report/1".into(),
            ok: true,
            command,
            application_id: application_id.into(),
            host_root: vibe_core::machine_json_path(host_root),
            message: message.into(),
        });
    }
    ctx.summary(&format!(
        "vibe {command} -g: application `{application_id}` — {message}"
    ));
    Ok(())
}
