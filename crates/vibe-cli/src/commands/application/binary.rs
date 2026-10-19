//! Built-in publication and removal for verified application distributions.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#distribution");

mod generation;
mod launchers;
mod ownership;
mod publication;
use generation::{GenerationTransaction, prepare_generation};
use launchers::{
    direct_launcher_body, make_launchers_executable, stable_launcher_body, stable_launcher_name_for,
};
use ownership::{ensure_owned_host, hash_file, read_management, slot_root_for_host, write_json};
use publication::{PublishedFiles, publish_files};

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use vibe_wire::generated::application::e1::management::{BinaryManagement, OwnedLauncher};

use super::distribution::{BundleManifest, VerifiedDistribution};
use super::model::{ApplicationLauncherOwnership, ManagementEntry, ManagementRuntime};

const MANAGEMENT_PROTOCOL: &str = "vibe-application-binary-management/1";

/// Resolve the selected payload from the existing validated management receipt.
pub fn payload_root(entry: &ManagementEntry) -> Result<PathBuf> {
    if entry.runtime != ManagementRuntime::Builtin {
        bail!("application is not managed by the built-in provider");
    }
    Ok(read_management(entry)?.payload_root)
}

pub struct SuspendedBinary {
    host_root: PathBuf,
    host_backup: PathBuf,
    launchers: Vec<SuspendedFile>,
}

struct SuspendedFile {
    path: PathBuf,
    bytes: Vec<u8>,
    permissions: fs::Permissions,
}

pub struct BinaryPublication {
    management: ManagementEntry,
    launchers: Vec<ApplicationLauncherOwnership>,
    provider_runners: Vec<ProviderRunner>,
    transaction: PublishedFiles,
    generation: GenerationTransaction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRunner {
    pub name: String,
    pub logical_name: String,
    pub relative_path: PathBuf,
}

/// Validated removal of exactly the launchers and selected payload named by a
/// built-in receipt. Validation is deliberately side-effect free: the durable
/// application index and alternative pointers must commit before cleanup.
#[derive(Debug, Clone)]
pub struct BinaryUninstall {
    pub(super) launchers: Vec<ApplicationLauncherOwnership>,
    pub(super) payload_root: PathBuf,
    pub(super) force: bool,
}

impl BinaryUninstall {
    /// Delete validated owned files after the index and all alternative
    /// pointers have committed. Every step is safe to repeat after a crash.
    pub(super) fn cleanup(&self) -> Result<()> {
        for launcher in &self.launchers {
            remove_owned_launcher(launcher, self.force)?;
        }
        remove_owned_payload(&self.payload_root)
    }
}

impl BinaryPublication {
    pub fn management(&self) -> ManagementEntry {
        self.management.clone()
    }
    pub fn launchers(&self) -> Vec<ApplicationLauncherOwnership> {
        self.launchers.clone()
    }
    pub fn provider_runners(&self) -> Vec<ProviderRunner> {
        self.provider_runners.clone()
    }
    pub fn commit(self) {
        self.transaction.commit();
        self.generation.commit();
    }
    pub fn rollback(self) -> Result<()> {
        let files = self.transaction.rollback();
        let generation = self.generation.rollback();
        files.and(generation)
    }
}

pub fn suspend(host_root: &Path, entry: &ManagementEntry) -> Result<SuspendedBinary> {
    let management = read_management(entry)?;
    if !entry.entry.starts_with(host_root) {
        bail!(
            "binary application management entry `{}` is outside host `{}`",
            entry.entry.display(),
            host_root.display()
        );
    }
    for launcher in &management.launchers {
        if hash_file(&launcher.destination)? != launcher.sha256 {
            bail!(
                "binary application launcher `{}` has drifted; refusing suspension",
                launcher.destination.display()
            );
        }
    }
    let mut launchers = Vec::new();
    for path in management
        .launchers
        .iter()
        .map(|value| value.destination.clone())
    {
        let metadata = fs::metadata(&path)
            .with_context(|| format!("reading binary-owned file `{}`", path.display()))?;
        let bytes = fs::read(&path)
            .with_context(|| format!("reading binary-owned file `{}`", path.display()))?;
        launchers.push(SuspendedFile {
            path,
            bytes,
            permissions: metadata.permissions(),
        });
    }
    let name = host_root
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| anyhow::anyhow!("binary application host has no portable name"))?;
    let host_backup =
        host_root.with_file_name(format!(".{name}-binary-suspended-{}", std::process::id()));
    if host_backup.exists() {
        bail!(
            "binary application transition backup already exists at `{}`",
            host_backup.display()
        );
    }
    fs::rename(host_root, &host_backup).with_context(|| {
        format!(
            "suspending binary application host `{}`",
            host_root.display()
        )
    })?;
    for (removed, file) in launchers.iter().enumerate() {
        if let Err(error) = fs::remove_file(&file.path) {
            for restore in launchers[..removed].iter().rev() {
                let _ = write_suspended_file(restore);
            }
            let _ = fs::rename(&host_backup, host_root);
            return Err(error).with_context(|| {
                format!("suspending binary-owned file `{}`", file.path.display())
            });
        }
    }
    Ok(SuspendedBinary {
        host_root: host_root.to_path_buf(),
        host_backup,
        launchers,
    })
}

impl SuspendedBinary {
    pub fn restore(self) -> Result<()> {
        self.restore_host_and_launchers()
    }

    pub fn commit(self) -> Result<()> {
        fs::remove_dir_all(&self.host_backup).with_context(|| {
            format!(
                "removing retired binary application host `{}`",
                self.host_backup.display()
            )
        })
    }

    pub fn rollback_after_source(
        self,
        source_launchers: &[ApplicationLauncherOwnership],
    ) -> Result<()> {
        for launcher in source_launchers {
            if hash_file(&launcher.destination)? != launcher.sha256 {
                bail!(
                    "cannot roll back source transition because `{}` drifted",
                    launcher.destination.display()
                );
            }
            fs::remove_file(&launcher.destination)?;
        }
        self.restore_host_and_launchers()
    }

    fn restore_host_and_launchers(self) -> Result<()> {
        if !self.host_backup.is_dir() {
            bail!(
                "cannot restore binary application because transition backup `{}` is unavailable",
                self.host_backup.display()
            );
        }
        if self.host_root.exists() {
            fs::remove_dir_all(&self.host_root).with_context(|| {
                format!(
                    "removing replacement application host `{}`",
                    self.host_root.display()
                )
            })?;
        }
        fs::rename(&self.host_backup, &self.host_root).with_context(|| {
            format!(
                "restoring binary application host `{}`",
                self.host_root.display()
            )
        })?;
        for file in &self.launchers {
            if file.path.exists() {
                fs::remove_file(&file.path).with_context(|| {
                    format!(
                        "removing replacement application file `{}`",
                        file.path.display()
                    )
                })?;
            }
            write_suspended_file(file)?;
        }
        Ok(())
    }
}

fn write_suspended_file(file: &SuspendedFile) -> Result<()> {
    fs::write(&file.path, &file.bytes)
        .with_context(|| format!("restoring binary-owned file `{}`", file.path.display()))?;
    fs::set_permissions(&file.path, file.permissions.clone()).with_context(|| {
        format!(
            "restoring permissions for binary-owned file `{}`",
            file.path.display()
        )
    })
}

pub fn publish(
    settings_root: &Path,
    host_root: &Path,
    verified: VerifiedDistribution,
    prior: Option<&ManagementEntry>,
    prior_launchers: &[ApplicationLauncherOwnership],
    force: bool,
) -> Result<BinaryPublication> {
    let host_root = ensure_owned_host(settings_root, host_root)?;
    let slot_root = slot_root_for_host(&host_root)?;
    let publish_public = host_root == slot_root;
    let previous = match prior {
        Some(entry) if entry.runtime == ManagementRuntime::Builtin => Some(read_management(entry)?),
        _ => None,
    };
    if previous.as_ref().is_some_and(|old| {
        old.application_id != verified.manifest.application.id
            || !old.payload_root.starts_with(&slot_root)
    }) {
        bail!("prior binary receipt belongs to another application host");
    }
    let (generation, generation_transaction) =
        prepare_generation(&host_root, &verified, previous.as_ref(), force)?;
    let result = (|| {
        let provider_runners: Vec<_> = verified
            .manifest
            .launchers
            .iter()
            .map(|runner| ProviderRunner {
                name: runner.destination.clone(),
                logical_name: runner.command.clone(),
                relative_path: PathBuf::from(&runner.path),
            })
            .collect();
        let opt_bin = settings_root.join("opt").join("bin");
        fs::create_dir_all(&opt_bin)?;
        let mut prior_by_path: BTreeMap<_, _> = previous
            .as_ref()
            .into_iter()
            .flat_map(|value| value.launchers.iter())
            .map(|launcher| (launcher.destination.clone(), launcher.sha256.clone()))
            .collect();
        let receipt_owned: std::collections::BTreeSet<_> = previous
            .as_ref()
            .into_iter()
            .flat_map(|value| value.launchers.iter())
            .map(|launcher| launcher.destination.clone())
            .collect();
        if publish_public {
            prior_by_path.extend(
                prior_launchers
                    .iter()
                    .map(|value| (value.destination.clone(), value.sha256.clone())),
            );
        }

        let mut desired = Vec::new();
        let direct_wrappers = tempfile::tempdir_in(&host_root)?;
        for launcher in verified
            .manifest
            .launchers
            .iter()
            .filter(|_| publish_public)
        {
            let payload_runner = generation.join(&launcher.path);
            let source = if verified.direct_archive {
                let staged = direct_wrappers.path().join(&launcher.destination);
                fs::write(
                    &staged,
                    direct_launcher_body(&payload_runner, &verified.manifest.os)?,
                )?;
                staged
            } else {
                payload_runner
            };
            let destination = opt_bin.join(&launcher.destination);
            if destination.exists() {
                let current = hash_file(&destination)?;
                let replacement = hash_file(&source)?;
                if prior_by_path.get(&destination) != Some(&current)
                    && current != replacement
                    && !(force && receipt_owned.contains(&destination))
                {
                    bail!(
                        "application launcher destination `{}` is not owned by this application or has drifted",
                        destination.display()
                    );
                }
            }
            desired.push((source, destination));
        }
        let desired_paths: std::collections::BTreeSet<_> =
            desired.iter().map(|(_, path)| path.clone()).collect();
        let mut retire = Vec::new();
        for (destination, expected_hash) in &prior_by_path {
            if !desired_paths.contains(destination) {
                if hash_file(destination)? != *expected_hash
                    && !(force && receipt_owned.contains(destination))
                {
                    bail!(
                        "source-owned launcher `{}` drifted before binary transition",
                        destination.display()
                    );
                }
                retire.push(destination.clone());
            }
        }
        let launchers = desired
            .iter()
            .map(|(source, destination)| {
                Ok(OwnedLauncher {
                    destination: destination.clone(),
                    sha256: hash_file(source)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let public_launchers: Vec<_> = launchers
            .iter()
            .map(|value| ApplicationLauncherOwnership {
                destination: value.destination.clone(),
                sha256: value.sha256.clone(),
            })
            .collect();
        let management_dir = host_root.join("management");
        fs::create_dir_all(&management_dir)?;
        let bundle_entry = generation.join(&verified.manifest.management.entry);
        let stable_launcher = management_dir.join(stable_launcher_name_for(
            &bundle_entry,
            &verified.manifest.os,
        ));
        let management_path = management_dir.join("binary.json");
        let launcher_count = desired.len();
        let stage_dir = host_root.join(format!(".management-pending-{}", std::process::id()));
        fs::create_dir(&stage_dir)?;
        let staged_launcher = stage_dir.join(stable_launcher_name_for(
            &bundle_entry,
            &verified.manifest.os,
        ));
        fs::write(
            &staged_launcher,
            stable_launcher_body(&bundle_entry, &verified.manifest.os),
        )?;
        let staged_management = stage_dir.join("binary.json");
        write_json(
            &staged_management,
            &BinaryManagement {
                protocol: MANAGEMENT_PROTOCOL.into(),
                application_id: verified.manifest.application.id,
                payload_root: generation,
                bundle_entry,
                launchers,
            },
        )?;
        desired.push((staged_launcher, stable_launcher));
        desired.push((staged_management, management_path.clone()));
        let transaction = publish_files(&desired, &retire)?;
        if let Err(error) = make_launchers_executable(
            desired
                .iter()
                .take(launcher_count + 1)
                .map(|(_, destination)| destination.as_path()),
        ) {
            transaction.rollback()?;
            return Err(error).context("marking application launchers executable");
        }
        for (source, _) in desired.iter().skip(launcher_count) {
            let _ = fs::remove_file(source);
        }
        let _ = fs::remove_dir(&stage_dir);
        Ok(BinaryPublication {
            management: ManagementEntry {
                runtime: ManagementRuntime::Builtin,
                entry: management_path,
            },
            launchers: public_launchers,
            provider_runners,
            transaction,
            generation: GenerationTransaction::Existing,
        })
    })();
    match result {
        Ok(mut publication) => {
            publication.generation = generation_transaction;
            Ok(publication)
        }
        Err(error) => {
            generation_transaction.rollback()?;
            Err(error)
        }
    }
}

pub fn prepare_uninstall(
    entry: &ManagementEntry,
    force: bool,
) -> Result<(String, BinaryUninstall)> {
    if entry.runtime != ManagementRuntime::Builtin {
        bail!("application is not managed by the built-in binary provider");
    }
    let management = read_management(entry)?;
    validate_owned_payload_tree(&management.payload_root)?;
    for launcher in &management.launchers {
        let current = hash_file(&launcher.destination).with_context(|| {
            format!(
                "verifying application launcher `{}`",
                launcher.destination.display()
            )
        })?;
        if current != launcher.sha256 && !force {
            bail!(
                "application launcher `{}` has drifted; refusing removal",
                launcher.destination.display()
            );
        }
    }
    let launchers: Vec<_> = management
        .launchers
        .iter()
        .map(|launcher| ApplicationLauncherOwnership {
            destination: launcher.destination.clone(),
            sha256: launcher.sha256.clone(),
        })
        .collect();
    Ok((
        format!(
            "removed {} receipt-owned launcher(s) and selected payload",
            management.launchers.len()
        ),
        BinaryUninstall {
            launchers,
            payload_root: management.payload_root,
            force,
        },
    ))
}

fn remove_owned_launcher(launcher: &ApplicationLauncherOwnership, force: bool) -> Result<()> {
    let metadata = match fs::symlink_metadata(&launcher.destination) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "reading application launcher `{}` for cleanup",
                    launcher.destination.display()
                )
            });
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        bail!(
            "application launcher `{}` is not a regular file during cleanup",
            launcher.destination.display()
        );
    }
    let current = hash_file(&launcher.destination)?;
    if current != launcher.sha256 && !force {
        bail!(
            "application launcher `{}` drifted after uninstall validation; refusing cleanup",
            launcher.destination.display()
        );
    }
    match fs::remove_file(&launcher.destination) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| {
            format!(
                "removing application launcher `{}`",
                launcher.destination.display()
            )
        }),
    }
}

fn remove_owned_payload(root: &Path) -> Result<()> {
    let metadata = match fs::symlink_metadata(root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("reading selected payload `{}`", root.display()));
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        bail!("selected application payload is no longer an owned directory");
    }
    validate_owned_payload_tree(root)?;
    match fs::remove_dir_all(root) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).context("removing selected application payload"),
    }
}

fn validate_owned_payload_tree(root: &Path) -> Result<()> {
    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let entry = entry?;
        let kind = entry.file_type();
        if kind.is_symlink() || (!kind.is_dir() && !kind.is_file()) {
            bail!("selected application payload contains a link or special entry");
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "binary/tests.rs"]
mod tests;
