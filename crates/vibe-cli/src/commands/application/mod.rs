//! Generic user-local application command routing.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#commands");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#ownership");

pub(super) mod alternatives;
mod apply;
mod arguments;
mod binary;
mod command;
mod distribution;
mod launch;
mod lease;
mod local_source;
mod model;
mod mutations;
mod options;
mod process;
mod remote;
mod removal;
mod report;
mod source;
mod store;

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use vibe_core::progress::Progress;

use crate::cli::{InstallArgs, UninstallArgs, UpdateArgs};
use crate::output;

use apply::apply;
use arguments::{validate_install_args, validate_uninstall_args, validate_update_args};
use report::render;

use model::{
    ApplicationContext, ApplicationOperation, ApplicationProvenance, ApplicationRecord,
    ApplicationSelection, ApplicationStatus, CONTEXT_PROTOCOL, InstallKind, ManagementEntry,
    ManagementRuntime, UserLauncher,
};
use mutations::{
    ensure_external_isolation, ensure_external_uninstall_isolated, fresh_revision,
    record_for_package, register_alternative, reject_identity_collision, retire_alternative,
    settings_root,
};
use options::InstallChoice;
use process::{dispatch, validate_management};
use source::{ResolvedApplication, qualified_ref, resolve_global_application};
use store::{ApplicationStore, installation_host_root, installation_key};

/// Inspect authored launch points using the currently installed application record.
pub fn launcher_listing(package: &str, project_root: Option<&Path>) -> Result<serde_json::Value> {
    launch::listing(package, project_root)
}

/// Run a named point from the current installed record and return its exit code.
pub fn run_launcher(
    package: &str,
    launcher_id: &str,
    extra_args: &[String],
    project_root: Option<&Path>,
) -> Result<i32> {
    launch::run(package, launcher_id, extra_args, project_root)
}

/// List every package-authored installer without selecting or executing one.
pub fn installer_listing(
    package: &str,
    registry: Option<&Path>,
    embedded_root: Option<&Path>,
    offline: bool,
) -> Result<serde_json::Value> {
    source::list_global_installer_options(
        &settings_root()?,
        registry,
        embedded_root,
        package,
        offline,
        &Progress::default(),
    )
}

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
    alternatives::recover_pending(&index, &settings)?;
    let spelling = &args.packages[0];
    let offline = root_offline || args.offline;
    let choice = InstallChoice {
        platform: args.platform.clone(),
        arch: args.arch.clone(),
        installer_id: args.installer.clone(),
        interactive: !ctx.is_json() && !ctx.is_unattended() && console::user_attended(),
    };
    let resolved = resolve_global_application(
        &settings,
        args.registry.as_deref(),
        embedded_root.as_deref(),
        spelling,
        &choice,
        offline,
        &progress,
    )?;
    reject_identity_collision(&index, &resolved.application)?;
    let key = installation_key(&resolved.application.package);
    ensure_external_isolation(&index, &resolved, &key, &settings)?;
    let host = installation_host_root(
        &settings,
        &resolved.application.id,
        &resolved.application.package,
    );
    let prior = index.applications.get(&key);
    let _launch_lease = if prior.is_some_and(|record| record.status == ApplicationStatus::Ready) {
        Some(
            lease::try_acquire_exclusive(&settings, &key)?.ok_or_else(|| {
                anyhow::anyhow!("application_in_use: `{key}` has an active launcher")
            })?,
        )
    } else {
        None
    };
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
        args.force,
        prior,
        &progress,
    )?;
    let slot = applied.application.id.clone();
    index.applications.insert(
        key.clone(),
        ApplicationRecord {
            application: applied.application.clone(),
            host_root: applied.host_root.clone(),
            app_root: Some(applied.app_root.clone()),
            management: applied.management.clone(),
            status: ApplicationStatus::Ready,
            provenance: Some(applied.provenance.clone()),
            launchers: applied.launchers.clone(),
            user_launchers: applied.user_launchers.clone(),
            public_commands: applied.public_commands.clone(),
            install_kind: Some(applied.install_kind),
            selected_option_id: Some(applied.selected_option_id.clone()),
            selected_platform: Some(applied.selected_platform.clone()),
        },
    );
    if let Err(error) = register_alternative(&mut index, &slot, &key, &applied.public_commands) {
        applied.rollback()?;
        return Err(error);
    }
    index.revision = fresh_revision();
    let mut alternative_publication =
        match alternatives::projection::prepare_slot(&mut index, &settings, &slot) {
            Ok(publication) => publication,
            Err(error) => {
                applied.rollback()?;
                return Err(error);
            }
        };
    let recording = progress.task("Recording global application state");
    if let Err(error) = store.save(&index) {
        recording.fail("application state recording failed");
        if error
            .downcast_ref::<store::UncertainIndexCommit>()
            .is_some()
        {
            return Err(error.context("application state may already reference the prepared installation; preserved staged files for recovery"));
        }
        alternative_publication.rollback()?;
        applied.rollback()?;
        return Err(error);
    }
    recording.finish();
    alternative_publication.commit_current()?;
    alternative_publication.commit()?;
    applied.commit()?;
    render(
        ctx,
        "install",
        &applied.application.id,
        &applied.message,
        &applied.host_root,
        Some(&applied.provenance),
        Some(applied.install_kind),
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
    alternatives::recover_pending(&index, &settings)?;
    let prior = record_for_package(&index, &requested)?.clone();
    let reuse_prior = args.platform.is_none() && args.arch.is_none() && args.installer.is_none();
    let (prior_platform, prior_arch) = prior
        .selected_platform
        .as_deref()
        .and_then(|value| value.split_once('/'))
        .map_or((None, None), |(platform, arch)| {
            (Some(platform.to_owned()), Some(arch.to_owned()))
        });
    let choice = InstallChoice {
        platform: args
            .platform
            .clone()
            .or_else(|| reuse_prior.then_some(prior_platform).flatten()),
        arch: args
            .arch
            .clone()
            .or_else(|| reuse_prior.then_some(prior_arch).flatten()),
        installer_id: args.installer.clone().or_else(|| {
            reuse_prior
                .then(|| prior.selected_option_id.clone())
                .flatten()
        }),
        interactive: !ctx.is_json() && !ctx.is_unattended() && console::user_attended(),
    };
    let resolved = resolve_global_application(
        &settings,
        args.registry.as_deref(),
        embedded_root.as_deref(),
        spelling,
        &choice,
        offline,
        &progress,
    )?;
    if resolved.application.id != prior.application.id {
        bail!("updated application declaration changes the installed application id");
    }
    reject_identity_collision(&index, &resolved.application)?;
    let key = installation_key(&resolved.application.package);
    let same_installation = key == installation_key(&prior.application.package);
    let target_prior = index
        .applications
        .get(&key)
        .filter(|record| record.status == ApplicationStatus::Ready)
        .cloned();
    let _launch_lease = if target_prior.is_some() {
        Some(
            lease::try_acquire_exclusive(&settings, &key)?.ok_or_else(|| {
                anyhow::anyhow!("application_in_use: `{key}` has an active launcher")
            })?,
        )
    } else {
        None
    };
    ensure_external_isolation(&index, &resolved, &key, &settings)?;
    let host = installation_host_root(
        &settings,
        &resolved.application.id,
        &resolved.application.package,
    );
    let mut applied = apply(
        ctx,
        &store,
        &settings,
        host,
        resolved,
        ApplicationOperation::Update,
        offline,
        args.from_source,
        args.binary,
        args.force,
        target_prior
            .as_ref()
            .or_else(|| same_installation.then_some(&prior)),
        &progress,
    )?;
    let slot = applied.application.id.clone();
    let prior_key = installation_key(&prior.application.package);
    index.applications.insert(
        key.clone(),
        ApplicationRecord {
            application: applied.application.clone(),
            host_root: applied.host_root.clone(),
            app_root: Some(applied.app_root.clone()),
            management: applied.management.clone(),
            status: ApplicationStatus::Ready,
            provenance: Some(applied.provenance.clone()),
            launchers: applied.launchers.clone(),
            user_launchers: applied.user_launchers.clone(),
            public_commands: applied.public_commands.clone(),
            install_kind: Some(applied.install_kind),
            selected_option_id: Some(applied.selected_option_id.clone()),
            selected_platform: Some(applied.selected_platform.clone()),
        },
    );
    if let Err(error) = register_alternative(&mut index, &slot, &key, &applied.public_commands) {
        applied.rollback()?;
        return Err(error);
    }
    if let Some(alternative) = index.alternatives.get_mut(&slot)
        && alternative.global_default.as_deref() == Some(prior_key.as_str())
    {
        alternative.global_default = Some(key.clone());
    }
    index.revision = fresh_revision();
    let mut alternative_publication =
        match alternatives::projection::prepare_slot(&mut index, &settings, &slot) {
            Ok(publication) => publication,
            Err(error) => {
                applied.rollback()?;
                return Err(error);
            }
        };
    let recording = progress.task("Recording global application state");
    if let Err(error) = store.save(&index) {
        recording.fail("application state recording failed");
        if error
            .downcast_ref::<store::UncertainIndexCommit>()
            .is_some()
        {
            return Err(error.context("application state may already reference the prepared update; preserved staged files for recovery"));
        }
        alternative_publication.rollback()?;
        applied.rollback()?;
        return Err(error);
    }
    recording.finish();
    alternative_publication.commit_current()?;
    alternative_publication.commit()?;
    applied.commit()?;
    render(
        ctx,
        "update",
        &applied.application.id,
        &applied.message,
        &applied.host_root,
        Some(&applied.provenance),
        Some(applied.install_kind),
    )
}

pub fn uninstall(ctx: &output::Context, args: UninstallArgs, root_offline: bool) -> Result<()> {
    let progress = ctx.progress();
    validate_uninstall_args(&args)?;
    let requested = qualified_ref(&args.package)?;
    let settings = settings_root()?;
    let store = ApplicationStore::open(&settings)?;
    let mut index = store.load()?;
    alternatives::recover_pending(&index, &settings)?;
    let prior = record_for_package(&index, &requested)?.clone();
    let key = installation_key(&prior.application.package);
    let slot = prior.application.id.clone();
    if index
        .alternatives
        .get(&slot)
        .and_then(|alternative| alternative.global_default.as_deref())
        == Some(key.as_str())
        && index.applications.iter().any(|(other_key, record)| {
            other_key != &key
                && record.application.id == slot
                && record.status == ApplicationStatus::Ready
        })
    {
        bail!(
            "selected installation `{key}` has other ready alternatives; select a replacement before uninstalling it"
        );
    }
    let _launch_lease = lease::try_acquire_exclusive(&settings, &key)?
        .ok_or_else(|| anyhow::anyhow!("application_in_use: `{key}` has an active launcher"))?;
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
        let removal = progress.task("Preparing owned application removal");
        let (message, removal_tx) = match binary::prepare_uninstall(&management, args.force) {
            Ok(value) => {
                removal.finish();
                value
            }
            Err(error) => {
                removal.fail("binary application removal failed");
                return Err(error);
            }
        };
        let prior_revision = index.revision.clone();
        index.applications.insert(
            key.clone(),
            ApplicationRecord {
                status: ApplicationStatus::Undeployed,
                ..prior.clone()
            },
        );
        retire_alternative(&mut index, &slot, &key);
        index.revision = fresh_revision();
        let mut alternative_publication =
            alternatives::projection::prepare_slot(&mut index, &settings, &slot)?;
        let removal_intent = match removal::prepare(
            &settings,
            &key,
            &prior_revision,
            &index.revision,
            &prior,
            &removal_tx,
        ) {
            Ok(intent) => intent,
            Err(error) => {
                alternative_publication.rollback()?;
                return Err(error);
            }
        };
        let recording = progress.task("Recording global application state");
        if let Err(error) = store.save(&index) {
            recording.fail("application state recording failed");
            if error
                .downcast_ref::<store::UncertainIndexCommit>()
                .is_some()
            {
                return Err(error.context("application removal has an uncertain index commit; retained projection and cleanup intents for recovery"));
            }
            let cleanup_rollback = removal_intent.rollback();
            let projection_rollback = alternative_publication.rollback();
            cleanup_rollback?;
            projection_rollback?;
            return Err(error);
        }
        recording.finish();
        alternative_publication.commit_current()?;
        alternative_publication.commit()?;
        removal_intent.commit()?;
        return render(
            ctx,
            "uninstall",
            &prior.application.id,
            &message,
            &prior.host_root,
            prior.provenance.as_ref(),
            prior.install_kind,
        );
    }
    if management.runtime == ManagementRuntime::Command {
        ensure_external_uninstall_isolated(&index, &slot, &key)?;
        let message = command::uninstall(
            &management,
            &prior.application.id,
            ctx.is_json(),
            args.force,
        )?;
        index.applications.insert(
            key.clone(),
            ApplicationRecord {
                status: ApplicationStatus::Undeployed,
                ..prior.clone()
            },
        );
        retire_alternative(&mut index, &slot, &key);
        index.revision = fresh_revision();
        let mut alternative_publication =
            alternatives::projection::prepare_slot(&mut index, &settings, &slot)?;
        store.save(&index).map_err(|error| error.context(
            "recording command uninstall failed after the external uninstaller ran; external payload state may have changed"
        ))?;
        alternative_publication.commit_current()?;
        alternative_publication.commit()?;
        return render(
            ctx,
            "uninstall",
            &prior.application.id,
            &message,
            &prior.host_root,
            prior.provenance.as_ref(),
            prior.install_kind,
        );
    }
    if args.force {
        bail!("force uninstall is unsupported by the retained legacy manager");
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
        management.runtime,
        &context,
        &context_path,
        &reply_path,
        &progress,
    )?;
    if reply.status != ApplicationStatus::Undeployed {
        bail!("application uninstall did not report undeployed");
    }
    index.applications.insert(
        key.clone(),
        ApplicationRecord {
            status: ApplicationStatus::Undeployed,
            ..prior.clone()
        },
    );
    retire_alternative(&mut index, &slot, &key);
    index.revision = fresh_revision();
    let mut alternative_publication =
        alternatives::projection::prepare_slot(&mut index, &settings, &slot)?;
    let recording = progress.task("Recording global application state");
    if let Err(error) = store.save(&index) {
        recording.fail("application state recording failed");
        return Err(error);
    }
    recording.finish();
    alternative_publication.commit_current()?;
    alternative_publication.commit()?;
    render(
        ctx,
        "uninstall",
        &context.application.id,
        &reply.message,
        &context.host_root,
        prior.provenance.as_ref(),
        prior.install_kind,
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
