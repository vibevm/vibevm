//! Generic user-local application command routing.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#commands");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#ownership");

mod binary;
mod distribution;
mod local_source;
mod model;
mod process;
mod remote;
mod report;
mod selection;
mod source;
mod store;

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use vibe_core::progress::Progress;

use crate::cli::{InstallArgs, UninstallArgs, UpdateArgs};
use crate::output;

use report::render;

use model::{
    ApplicationContext, ApplicationOperation, ApplicationProvenance, ApplicationRecord,
    ApplicationSelection, ApplicationStatus, CONTEXT_PROTOCOL, ManagementEntry,
};
use process::{dispatch, validate_management};
use source::{ResolvedApplication, qualified_ref, resolve_global_application};
use store::ApplicationStore;

pub fn install(
    ctx: &output::Context,
    mut args: InstallArgs,
    embedded_root: Option<PathBuf>,
    root_offline: bool,
) -> Result<()> {
    let progress = ctx.progress();
    local_source::expand_install(&mut args)?;
    validate_install_args(&args)?;
    let settings = settings_root()?;
    let store = ApplicationStore::open(&settings)?;
    let mut index = store.load()?;
    let spelling = &args.packages[0];
    let offline = root_offline || args.offline;
    let resolved = resolve_global_application(
        &settings,
        args.registry.as_deref(),
        embedded_root.as_deref(),
        spelling,
        offline,
        &progress,
    )?;
    reject_identity_collision(&index, &resolved.application)?;
    let host = settings
        .join("opt")
        .join("apps")
        .join(&resolved.application.id);
    let prior = index.applications.get(&resolved.application.id);
    let mut applied = apply(
        ctx,
        &store,
        &settings,
        host,
        resolved,
        ApplicationOperation::Install,
        offline,
        args.from_source,
        false,
        prior,
        &progress,
    )?;
    index.applications.insert(
        applied.application.id.clone(),
        ApplicationRecord {
            application: applied.application.clone(),
            host_root: applied.host_root.clone(),
            management: applied.management,
            status: ApplicationStatus::Ready,
            provenance: Some(applied.provenance.clone()),
            launchers: applied.launchers.clone(),
        },
    );
    let recording = progress.task("Recording global application state");
    if let Err(error) = store.save(&index) {
        recording.fail("application state recording failed");
        if let Some(publication) = applied.publication.take() {
            publication.rollback()?;
        }
        if let Some(suspended) = applied.suspended.take() {
            suspended.rollback_after_source(&applied.launchers)?;
        }
        return Err(error);
    }
    recording.finish();
    if let Some(publication) = applied.publication.take() {
        publication.commit();
    }
    if let Some(suspended) = applied.suspended.take() {
        suspended.commit()?;
    }
    render(
        ctx,
        "install",
        &applied.application.id,
        &applied.message,
        &applied.host_root,
        Some(&applied.provenance),
    )
}

pub fn update(
    ctx: &output::Context,
    mut args: UpdateArgs,
    embedded_root: Option<PathBuf>,
    root_offline: bool,
) -> Result<()> {
    let progress = ctx.progress();
    let offline = root_offline || local_source::expand_update(&mut args)?;
    validate_update_args(&args)?;
    let spelling = &args.packages[0];
    let requested = qualified_ref(spelling)?;
    let settings = settings_root()?;
    let store = ApplicationStore::open(&settings)?;
    let mut index = store.load()?;
    let prior = record_for_package(&index, &requested)?.clone();
    let resolved = resolve_global_application(
        &settings,
        args.registry.as_deref(),
        embedded_root.as_deref(),
        spelling,
        offline,
        &progress,
    )?;
    if resolved.application.id != prior.application.id {
        bail!("updated application declaration changes the installed application id");
    }
    reject_identity_collision(&index, &resolved.application)?;
    let mut applied = apply(
        ctx,
        &store,
        &settings,
        prior.host_root.clone(),
        resolved,
        ApplicationOperation::Update,
        offline,
        args.from_source,
        args.binary,
        Some(&prior),
        &progress,
    )?;
    index.applications.insert(
        applied.application.id.clone(),
        ApplicationRecord {
            application: applied.application.clone(),
            host_root: applied.host_root.clone(),
            management: applied.management,
            status: ApplicationStatus::Ready,
            provenance: Some(applied.provenance.clone()),
            launchers: applied.launchers.clone(),
        },
    );
    let recording = progress.task("Recording global application state");
    if let Err(error) = store.save(&index) {
        recording.fail("application state recording failed");
        if let Some(publication) = applied.publication.take() {
            publication.rollback()?;
        }
        if let Some(suspended) = applied.suspended.take() {
            suspended.rollback_after_source(&applied.launchers)?;
        }
        return Err(error);
    }
    recording.finish();
    if let Some(publication) = applied.publication.take() {
        publication.commit();
    }
    if let Some(suspended) = applied.suspended.take() {
        suspended.commit()?;
    }
    render(
        ctx,
        "update",
        &applied.application.id,
        &applied.message,
        &applied.host_root,
        Some(&applied.provenance),
    )
}

pub fn uninstall(ctx: &output::Context, args: UninstallArgs, root_offline: bool) -> Result<()> {
    let progress = ctx.progress();
    validate_uninstall_args(&args)?;
    let requested = qualified_ref(&args.package)?;
    let settings = settings_root()?;
    let store = ApplicationStore::open(&settings)?;
    let mut index = store.load()?;
    let prior = record_for_package(&index, &requested)?.clone();
    let validation = progress.task("Validating application management ownership");
    let management = match validate_management(&prior.host_root, &prior.management) {
        Ok(management) => {
            validation.finish();
            management
        }
        Err(error) => {
            validation.fail("application management validation failed");
            return Err(error);
        }
    };
    if management.runtime == model::ManagementRuntime::Builtin {
        let removal = progress.task("Removing binary application launchers");
        let message = match binary::uninstall(&management) {
            Ok(message) => {
                removal.finish();
                message
            }
            Err(error) => {
                removal.fail("binary application removal failed");
                return Err(error);
            }
        };
        index.applications.insert(
            prior.application.id.clone(),
            ApplicationRecord {
                status: ApplicationStatus::Undeployed,
                ..prior.clone()
            },
        );
        let recording = progress.task("Recording global application state");
        if let Err(error) = store.save(&index) {
            recording.fail("application state recording failed");
            return Err(error);
        }
        recording.finish();
        return render(
            ctx,
            "uninstall",
            &prior.application.id,
            &message,
            &prior.host_root,
            prior.provenance.as_ref(),
        );
    }
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
        &progress,
    )?;
    if reply.status != ApplicationStatus::Undeployed {
        bail!("application uninstall did not report undeployed");
    }
    index.applications.insert(
        context.application.id.clone(),
        ApplicationRecord {
            status: ApplicationStatus::Undeployed,
            ..prior.clone()
        },
    );
    let recording = progress.task("Recording global application state");
    if let Err(error) = store.save(&index) {
        recording.fail("application state recording failed");
        return Err(error);
    }
    recording.finish();
    render(
        ctx,
        "uninstall",
        &context.application.id,
        &reply.message,
        &context.host_root,
        prior.provenance.as_ref(),
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

struct AppliedApplication {
    application: model::ApplicationIdentity,
    host_root: PathBuf,
    management: ManagementEntry,
    provenance: ApplicationProvenance,
    message: String,
    publication: Option<binary::BinaryPublication>,
    launchers: Vec<model::ApplicationLauncherOwnership>,
    suspended: Option<binary::SuspendedBinary>,
}

#[allow(clippy::too_many_arguments)]
fn apply(
    ctx: &output::Context,
    store: &ApplicationStore,
    settings: &Path,
    host: PathBuf,
    resolved: ResolvedApplication,
    operation: ApplicationOperation,
    offline: bool,
    from_source: bool,
    binary_only: bool,
    prior: Option<&ApplicationRecord>,
    progress: &Progress,
) -> Result<AppliedApplication> {
    let active_prior = prior.filter(|record| record.status == ApplicationStatus::Ready);
    if let Some(selected) =
        selection::select_binary(ctx, &resolved, from_source, binary_only, offline, progress)?
    {
        let target = selected.target;
        std::fs::create_dir_all(&host)?;
        let staging = host.join(format!(".distribution-pending-{}", std::process::id()));
        if let Some(verified) =
            distribution::fetch_and_verify(&target, &selected.application, &staging, progress)?
        {
            let application = verified.manifest.application.clone();
            let provenance = ApplicationProvenance {
                available_source: resolved.source.clone(),
                selected: ApplicationSelection::Binary {
                    commit: target.source_commit,
                    source_tree: target.source_tree,
                    os: target.os,
                    arch: target.arch,
                    asset_sha256: target.sha256,
                },
            };
            let publishing = progress.task("Publishing verified application distribution");
            let publication = match binary::publish(
                settings,
                &host,
                verified,
                active_prior.map(|record| &record.management),
                active_prior.map_or(&[], |record| record.launchers.as_slice()),
            ) {
                Ok(publication) => {
                    publishing.finish();
                    publication
                }
                Err(error) => {
                    publishing.fail("application publication failed");
                    return Err(error);
                }
            };
            let management = publication.management();
            let launchers = publication.launchers();
            return Ok(AppliedApplication {
                application,
                host_root: host,
                management,
                provenance,
                message: "installed verified platform distribution".into(),
                publication: Some(publication),
                launchers,
                suspended: None,
            });
        }
    }
    if binary_only {
        bail!(
            "no verified published binary is available for this application and platform; remove `--binary` to permit source fallback"
        );
    }
    let resolved = resolved.materialize_source(settings, offline, progress)?;
    let (installer_entry, installer_root, registry_root) = resolved.source_runtime()?;
    let registry_root = Some(registry_root.to_path_buf());
    let provider_operation = if active_prior
        .is_some_and(|record| record.management.runtime == model::ManagementRuntime::Builtin)
    {
        ApplicationOperation::Install
    } else {
        operation
    };
    let context = context(
        provider_operation,
        resolved.application.clone(),
        settings,
        host,
        registry_root,
        offline,
    )?;
    let (context_path, reply_path) = store.request_paths()?;
    let suspension = progress.task("Preparing source application transition");
    let suspended = active_prior
        .filter(|record| record.management.runtime == model::ManagementRuntime::Builtin)
        .map(|record| binary::suspend(&record.host_root, &record.management))
        .transpose();
    let suspended = match suspended {
        Ok(Some(suspended)) => {
            suspension.finish();
            Some(suspended)
        }
        Ok(None) => {
            suspension.skip("no binary application transition required");
            None
        }
        Err(error) => {
            suspension.fail("application transition preparation failed");
            return Err(error);
        }
    };
    let reply = match dispatch(
        installer_entry,
        installer_root,
        &context,
        &context_path,
        &reply_path,
        progress,
    ) {
        Ok(reply) => reply,
        Err(error) => {
            if let Some(suspended) = suspended {
                suspended.restore()?;
            }
            return Err(error);
        }
    };
    let management = match ready_management(&context.host_root, &reply) {
        Ok(management) => management,
        Err(error) => {
            if let Some(suspended) = suspended {
                suspended.restore()?;
            }
            return Err(error);
        }
    };
    let selected = ApplicationSelection::Source {
        commit: resolved
            .source
            .as_ref()
            .map(|value| value.resolved_commit.clone()),
        source_tree: resolved
            .source
            .as_ref()
            .map(|value| value.source_tree.clone()),
    };
    Ok(AppliedApplication {
        application: context.application,
        host_root: context.host_root,
        management,
        provenance: ApplicationProvenance {
            available_source: resolved.source,
            selected,
        },
        message: reply.message,
        publication: None,
        launchers: reply.launchers,
        suspended,
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
    if (!args.local_source && args.path != Path::new("."))
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
    if (!args.local_source && args.path != Path::new("."))
        || args.exact
        || args.auth_required
        || args.trace_compile
    {
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
