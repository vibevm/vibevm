//! Apply a selected package-owned installer and its hooks.

use super::*;
use vibe_core::manifest::{InstallerMethod, LauncherDecl};

pub(super) struct AppliedApplication {
    pub(super) application: model::ApplicationIdentity,
    pub(super) host_root: PathBuf,
    pub(super) app_root: PathBuf,
    pub(super) management: ManagementEntry,
    pub(super) provenance: ApplicationProvenance,
    pub(super) message: String,
    pub(super) publication: Option<binary::BinaryPublication>,
    pub(super) command_publication: Option<command::CommandPublication>,
    pub(super) launchers: Vec<model::ApplicationLauncherOwnership>,
    pub(super) user_launchers: Vec<UserLauncher>,
    pub(super) install_kind: InstallKind,
    pub(super) selected_option_id: String,
    pub(super) selected_platform: String,
    pub(super) suspended: Option<binary::SuspendedBinary>,
}

fn user_launchers(declarations: &[LauncherDecl]) -> Vec<UserLauncher> {
    declarations
        .iter()
        .map(|entry| UserLauncher {
            id: entry.id.clone(),
            name: entry.name.clone(),
            description: entry.description.clone(),
            executable: entry.executable.clone(),
            args: entry.args.clone(),
            working_directory: entry.working_directory.clone(),
            icon: entry.icon.clone(),
            terminal: entry.terminal,
            default: entry.default,
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply(
    ctx: &output::Context,
    _store: &ApplicationStore,
    settings: &Path,
    host: PathBuf,
    resolved: ResolvedApplication,
    operation: ApplicationOperation,
    offline: bool,
    from_source: bool,
    binary_only: bool,
    force: bool,
    prior: Option<&ApplicationRecord>,
    progress: &Progress,
) -> Result<AppliedApplication> {
    if from_source || binary_only {
        bail!("the selected installer option cannot be overridden by --from-source or --binary");
    }
    let resolved = resolved.materialize_source(settings, offline, progress)?;
    let active_prior = prior.filter(|record| record.status == ApplicationStatus::Ready);
    let selected_platform = format!("{}/{}", resolved.selected_platform, resolved.selected_arch);
    let selected_option_id = resolved.selected_option_id.clone();
    let authored_launchers = user_launchers(&resolved.launchers);
    let provenance = ApplicationProvenance {
        available_source: resolved.source.clone(),
        selected: ApplicationSelection::Source {
            commit: resolved
                .source
                .as_ref()
                .map(|value| value.resolved_commit.clone()),
            source_tree: resolved
                .source
                .as_ref()
                .map(|value| value.source_tree.clone()),
        },
    };

    let package_root = resolved.artifact_root()?.to_path_buf();
    let intended_app_root = match &resolved.installer {
        InstallerMethod::Shell { app_root, .. }
        | InstallerMethod::Exe { app_root, .. }
        | InstallerMethod::Msi { app_root, .. } => command::preview_app_root(app_root, settings)?,
        InstallerMethod::Zip { sha256, .. } | InstallerMethod::TarGz { sha256, .. } => {
            host.join("generations").join(sha256.to_ascii_lowercase())
        }
    };
    if let Some(script) = &resolved.pre_install {
        command::run_shell_hook(script, &package_root, &intended_app_root, ctx.is_json())
            .map_err(|error| error.context("pre_install failed before application installation"))?;
    }

    let applied: Result<AppliedApplication> = match &resolved.installer {
        InstallerMethod::Shell {
            app_root,
            install,
            force_install,
            update,
            uninstall,
            force_uninstall,
        } => {
            let script = if force {
                force_install.as_str()
            } else if operation == ApplicationOperation::Update
                && active_prior
                    .is_some_and(|record| record.management.runtime == ManagementRuntime::Command)
            {
                update.as_deref().ok_or_else(|| {
                    anyhow::anyhow!("selected shell installer has no update command")
                })?
            } else {
                install.as_str()
            };
            let suspended = active_prior
                .filter(|record| record.management.runtime == ManagementRuntime::Builtin)
                .map(|record| binary::suspend(&record.host_root, &record.management))
                .transpose()?;
            let applied = command::apply(command::ApplyRequest {
                invocation: command::Invocation::Shell(script),
                app_root_template: app_root,
                uninstall: Some(uninstall.as_str()),
                force_uninstall: Some(force_uninstall.as_str()),
                settings_root: settings,
                host_root: &host,
                application_id: &resolved.application.id,
                json_output: ctx.is_json(),
            });
            let (management, app_root, command_publication) = match applied {
                Ok(value) => value,
                Err(error) => {
                    if let Some(suspended) = suspended {
                        suspended.restore()?;
                    }
                    return Err(error);
                }
            };
            Ok(AppliedApplication {
                application: resolved.application.clone(),
                host_root: host,
                app_root,
                management,
                provenance,
                message: "shell installer completed".into(),
                publication: None,
                command_publication: Some(command_publication),
                launchers: Vec::new(),
                user_launchers: authored_launchers,
                install_kind: InstallKind::Shell,
                selected_option_id,
                selected_platform,
                suspended,
            })
        }
        InstallerMethod::Zip {
            source,
            sha256,
            payload_root,
            runners,
        }
        | InstallerMethod::TarGz {
            source,
            sha256,
            payload_root,
            runners,
        } => {
            let format = match &resolved.installer {
                InstallerMethod::Zip { .. } => distribution::DirectArchiveFormat::Zip,
                _ => distribution::DirectArchiveFormat::TarGz,
            };
            let source = artifact_source(resolved.artifact_root()?, source)?;
            std::fs::create_dir_all(&host)?;
            let staging = host.join(format!(".archive-pending-{}", std::process::id()));
            let verified = distribution::verify_direct_archive(
                &source,
                format,
                sha256,
                payload_root.as_deref(),
                runners,
                &resolved.application,
                &staging,
                offline,
                progress,
            )?;
            let publication = binary::publish(
                settings,
                &host,
                verified,
                active_prior.map(|record| &record.management),
                active_prior.map_or(&[], |record| record.launchers.as_slice()),
                force,
            )?;
            let management = publication.management();
            let app_root = binary::payload_root(&management)?;
            let launchers = publication.launchers();
            Ok(AppliedApplication {
                application: resolved.application.clone(),
                host_root: host,
                app_root,
                management,
                provenance,
                message: "verified archive installed".into(),
                publication: Some(publication),
                command_publication: None,
                launchers,
                user_launchers: authored_launchers,
                install_kind: if matches!(format, distribution::DirectArchiveFormat::Zip) {
                    InstallKind::Zip
                } else {
                    InstallKind::TarGz
                },
                selected_option_id,
                selected_platform,
                suspended: None,
            })
        }
        InstallerMethod::Exe {
            source,
            sha256,
            app_root,
            arguments,
            uninstall,
            force_arguments,
            force_uninstall,
        }
        | InstallerMethod::Msi {
            source,
            sha256,
            app_root,
            arguments,
            uninstall,
            force_arguments,
            force_uninstall,
        } => {
            if !cfg!(windows) {
                bail!("EXE and MSI installer files can run only on Windows");
            }
            let source = artifact_source(resolved.artifact_root()?, source)?;
            let extension = if matches!(&resolved.installer, InstallerMethod::Exe { .. }) {
                "exe"
            } else {
                "msi"
            };
            let staging = settings.join("applications").join(format!(
                ".installer-pending-{}.{}",
                std::process::id(),
                extension
            ));
            let verified =
                distribution::verify_installer_file(&source, sha256, &staging, offline, progress)?;
            let suspended = active_prior
                .filter(|record| record.management.runtime == ManagementRuntime::Builtin)
                .map(|record| binary::suspend(&record.host_root, &record.management))
                .transpose()?;
            let selected_arguments = if force {
                force_arguments.as_deref().ok_or_else(|| {
                    anyhow::anyhow!("selected {extension} installer has no force_arguments")
                })?
            } else {
                arguments.as_slice()
            };
            let invocation = if extension == "exe" {
                command::Invocation::Exe(&verified, selected_arguments)
            } else {
                command::Invocation::Msi(&verified, selected_arguments)
            };
            let applied = command::apply(command::ApplyRequest {
                invocation,
                app_root_template: app_root,
                uninstall: uninstall.as_deref(),
                force_uninstall: force_uninstall.as_deref(),
                settings_root: settings,
                host_root: &host,
                application_id: &resolved.application.id,
                json_output: ctx.is_json(),
            });
            let _ = std::fs::remove_file(&verified);
            let (management, app_root, command_publication) = match applied {
                Ok(value) => value,
                Err(error) => {
                    if let Some(suspended) = suspended {
                        suspended.restore()?;
                    }
                    return Err(error);
                }
            };
            Ok(AppliedApplication {
                application: resolved.application.clone(),
                host_root: host,
                app_root,
                management,
                provenance,
                message: format!("verified {extension} installer completed"),
                publication: None,
                command_publication: Some(command_publication),
                launchers: Vec::new(),
                user_launchers: authored_launchers,
                install_kind: if extension == "exe" {
                    InstallKind::Exe
                } else {
                    InstallKind::Msi
                },
                selected_option_id,
                selected_platform,
                suspended,
            })
        }
    };
    let mut applied = applied?;
    if let Some(script) = &resolved.post_install
        && let Err(error) =
            command::run_shell_hook(script, &package_root, &applied.app_root, ctx.is_json())
    {
        if let Some(publication) = applied.publication.take() {
            publication.rollback()?;
        }
        if let Some(publication) = applied.command_publication.take() {
            publication.rollback()?;
        }
        if let Some(suspended) = applied.suspended.take() {
            suspended.rollback_after_source(&applied.launchers)?;
        }
        return Err(error.context(
                "post_install failed; the prior index remains, but an external installer may have changed its payload",
            ));
    }
    Ok(applied)
}

fn artifact_source(root: &Path, source: &str) -> Result<String> {
    if source.starts_with("https://") {
        return Ok(source.to_owned());
    }
    if source.contains("://") || Path::new(source).is_absolute() {
        bail!("installer artifact source must be package-relative or HTTPS");
    }
    let candidate = root.join(source);
    let canonical = std::fs::canonicalize(&candidate)?;
    if !canonical.starts_with(root) {
        bail!("installer artifact source escapes its package");
    }
    Ok(canonical.to_string_lossy().into_owned())
}
