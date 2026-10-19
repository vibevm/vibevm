//! Built-in publication and removal for verified application distributions.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#distribution");

mod launchers;
mod ownership;
mod publication;
use launchers::{
    direct_launcher_body, make_launchers_executable, stable_launcher_body, stable_launcher_name_for,
};
use ownership::{ensure_owned_host, hash_file, read_management, verify_generation, write_json};
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
    transaction: PublishedFiles,
    generation: GenerationTransaction,
}

/// Prepared removal of exactly the launchers and selected payload named by a
/// validated built-in receipt. The application index decides commit or rollback.
#[derive(Debug)]
pub struct BinaryUninstall {
    launchers: PublishedFiles,
    payload_root: PathBuf,
    payload_backup: PathBuf,
    backup_dir: PathBuf,
}

impl BinaryUninstall {
    pub fn rollback(self) -> Result<()> {
        let payload = (|| {
            if fs::symlink_metadata(&self.payload_root).is_ok() {
                bail!("refusing to overwrite an unowned payload during application rollback");
            }
            validate_owned_payload_tree(&self.payload_backup)?;
            fs::rename(&self.payload_backup, &self.payload_root)
                .context("restoring selected application payload after failed index save")
        })();
        let launchers = self.launchers.rollback();
        if payload.is_ok() {
            let _ = fs::remove_dir(&self.backup_dir);
        }
        payload.and(launchers)
    }

    pub fn commit(self) -> Result<()> {
        validate_owned_payload_tree(&self.payload_backup)?;
        let launchers = self.launchers.commit_checked();
        let payload =
            fs::remove_dir_all(&self.backup_dir).context("removing retired application payload");
        launchers.and(payload)
    }
}

enum GenerationTransaction {
    Existing,
    Added(PathBuf),
    Replaced { active: PathBuf, backup: PathBuf },
}

impl GenerationTransaction {
    fn commit(self) {
        if let Self::Replaced { backup, .. } = self {
            let _ = fs::remove_dir_all(backup);
        }
    }

    fn rollback(self) -> Result<()> {
        match self {
            Self::Existing => Ok(()),
            Self::Added(active) => fs::remove_dir_all(&active)
                .with_context(|| format!("removing rolled-back generation {}", active.display())),
            Self::Replaced { active, backup } => {
                fs::remove_dir_all(&active)?;
                fs::rename(&backup, &active).context("restoring prior application generation")
            }
        }
    }
}

impl BinaryPublication {
    pub fn management(&self) -> ManagementEntry {
        self.management.clone()
    }
    pub fn launchers(&self) -> Vec<ApplicationLauncherOwnership> {
        self.launchers.clone()
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
    let previous = match prior {
        Some(entry) if entry.runtime == ManagementRuntime::Builtin => Some(read_management(entry)?),
        _ => None,
    };
    if previous.as_ref().is_some_and(|old| {
        old.application_id != verified.manifest.application.id
            || old.payload_root.parent() != Some(&host_root.join("generations"))
    }) {
        bail!("prior binary receipt belongs to another application host");
    }
    let generations = host_root.join("generations");
    fs::create_dir_all(&generations)?;
    let generation = generations.join(&verified.asset_sha256);
    let generation_transaction = if generation.exists() {
        let metadata = fs::symlink_metadata(&generation)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            bail!("application generation is not an owned regular directory");
        }
        if verify_generation(&generation, &verified.manifest).is_ok() {
            fs::remove_dir_all(&verified.payload_root)
                .context("removing duplicate staged distribution")?;
            GenerationTransaction::Existing
        } else if force
            && previous
                .as_ref()
                .is_some_and(|old| old.payload_root == generation)
        {
            let backup = generation.with_file_name(format!(
                ".{}-force-prior-{}",
                verified.asset_sha256,
                std::process::id()
            ));
            if backup.exists() {
                bail!("application generation backup already exists");
            }
            fs::rename(&generation, &backup).context("backing up drifted owned generation")?;
            if let Err(error) = fs::rename(&verified.payload_root, &generation) {
                fs::rename(&backup, &generation)
                    .context("restoring prior generation after failed replacement")?;
                return Err(error).context("replacing drifted owned generation");
            }
            GenerationTransaction::Replaced {
                active: generation.clone(),
                backup,
            }
        } else {
            bail!("cached application generation differs from its verified manifest");
        }
    } else {
        fs::rename(&verified.payload_root, &generation)
            .context("publishing immutable application payload")?;
        GenerationTransaction::Added(generation.clone())
    };
    let result = (|| {
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
        prior_by_path.extend(
            prior_launchers
                .iter()
                .map(|value| (value.destination.clone(), value.sha256.clone())),
        );

        let mut desired = Vec::new();
        let direct_wrappers = tempfile::tempdir_in(&host_root)?;
        for launcher in &verified.manifest.launchers {
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
        let launcher_count = verified.manifest.launchers.len();
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
    let paths: Vec<_> = management
        .launchers
        .iter()
        .map(|launcher| launcher.destination.clone())
        .collect();
    let backup_dir = tempfile::Builder::new()
        .prefix(".application-uninstall-")
        .tempdir_in(
            management
                .payload_root
                .parent()
                .ok_or_else(|| anyhow::anyhow!("owned payload has no generations parent"))?,
        )?;
    let payload_backup = backup_dir.path().join("payload");
    let launchers = publish_files(&[], &paths)?;
    if let Err(error) = fs::rename(&management.payload_root, &payload_backup) {
        launchers.rollback()?;
        return Err(error).context("suspending selected application payload");
    }
    Ok((
        format!(
            "removed {} receipt-owned launcher(s) and selected payload",
            management.launchers.len()
        ),
        BinaryUninstall {
            launchers,
            payload_root: management.payload_root,
            payload_backup,
            backup_dir: backup_dir.keep(),
        },
    ))
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
