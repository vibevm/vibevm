//! Built-in publication and removal for verified application distributions.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#distribution");

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

pub struct SuspendedBinary {
    files: Vec<(PathBuf, Vec<u8>)>,
}

pub struct BinaryPublication {
    management: ManagementEntry,
    launchers: Vec<ApplicationLauncherOwnership>,
    transaction: PublishedFiles,
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
    }
    pub fn rollback(self) -> Result<()> {
        self.transaction.rollback()
    }
}

pub fn suspend(entry: &ManagementEntry) -> Result<SuspendedBinary> {
    let management = read_management(entry)?;
    for launcher in &management.launchers {
        if hash_file(&launcher.destination)? != launcher.sha256 {
            bail!(
                "binary application launcher `{}` has drifted; refusing suspension",
                launcher.destination.display()
            );
        }
    }
    let mut paths: Vec<_> = management
        .launchers
        .iter()
        .map(|value| value.destination.clone())
        .collect();
    paths.push(entry.entry.clone());
    paths.push(entry.entry.with_file_name("launch.cmd"));
    let mut files = Vec::new();
    for path in paths {
        let bytes = fs::read(&path)
            .with_context(|| format!("reading binary-owned file `{}`", path.display()))?;
        files.push((path, bytes));
    }
    for (removed, (path, _)) in files.iter().enumerate() {
        if let Err(error) = fs::remove_file(path) {
            for (restore, bytes) in files[..removed].iter().rev() {
                let _ = fs::write(restore, bytes);
            }
            return Err(error)
                .with_context(|| format!("suspending binary-owned file `{}`", path.display()));
        }
    }
    Ok(SuspendedBinary { files })
}

impl SuspendedBinary {
    pub fn restore(self) -> Result<()> {
        for (path, bytes) in self.files {
            if path.exists() {
                if fs::read(&path)? == bytes {
                    continue;
                }
                bail!(
                    "cannot restore binary application because `{}` was replaced",
                    path.display()
                );
            }
            fs::write(&path, bytes)
                .with_context(|| format!("restoring binary-owned file `{}`", path.display()))?;
        }
        Ok(())
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
        for (path, _) in &self.files {
            if path.exists() {
                fs::remove_file(path).with_context(|| {
                    format!("removing replacement application file `{}`", path.display())
                })?;
            }
        }
        self.restore()
    }
}

pub fn publish(
    settings_root: &Path,
    host_root: &Path,
    verified: VerifiedDistribution,
    prior: Option<&ManagementEntry>,
    prior_launchers: &[ApplicationLauncherOwnership],
) -> Result<BinaryPublication> {
    let host_root = ensure_owned_host(settings_root, host_root)?;
    let generations = host_root.join("generations");
    fs::create_dir_all(&generations)?;
    let generation = generations.join(&verified.asset_sha256);
    if generation.exists() {
        verify_generation(&generation, &verified.manifest)?;
        fs::remove_dir_all(&verified.payload_root)
            .context("removing duplicate staged distribution")?;
    } else {
        fs::rename(&verified.payload_root, &generation)
            .context("publishing immutable application payload")?;
    }

    let previous = match prior {
        Some(entry) if entry.runtime == ManagementRuntime::Builtin => Some(read_management(entry)?),
        _ => None,
    };
    let opt_bin = settings_root.join("opt").join("bin");
    fs::create_dir_all(&opt_bin)?;
    let mut prior_by_path: BTreeMap<_, _> = previous
        .as_ref()
        .into_iter()
        .flat_map(|value| value.launchers.iter())
        .map(|launcher| (launcher.destination.clone(), launcher.sha256.clone()))
        .collect();
    prior_by_path.extend(
        prior_launchers
            .iter()
            .map(|value| (value.destination.clone(), value.sha256.clone())),
    );

    let mut desired = Vec::new();
    for launcher in &verified.manifest.launchers {
        let source = generation.join(&launcher.path);
        let destination = opt_bin.join(&launcher.destination);
        if destination.exists() {
            let current = hash_file(&destination)?;
            let replacement = hash_file(&source)?;
            if prior_by_path.get(&destination) != Some(&current) && current != replacement {
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
            if hash_file(destination)? != *expected_hash {
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
    let stable_launcher = management_dir.join("launch.cmd");
    let management_path = management_dir.join("binary.json");
    let launcher_count = verified.manifest.launchers.len();
    let stage_dir = host_root.join(format!(".management-pending-{}", std::process::id()));
    fs::create_dir(&stage_dir)?;
    let staged_launcher = stage_dir.join("launch.cmd");
    fs::write(
        &staged_launcher,
        format!(
            "@echo off\r\ncall \"{}\" %*\r\n",
            bundle_entry.display().to_string().replace('%', "%%")
        ),
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
    })
}

pub fn uninstall(entry: &ManagementEntry) -> Result<String> {
    if entry.runtime != ManagementRuntime::Builtin {
        bail!("application is not managed by the built-in binary provider");
    }
    let management = read_management(entry)?;
    for launcher in &management.launchers {
        let current = hash_file(&launcher.destination).with_context(|| {
            format!(
                "verifying application launcher `{}`",
                launcher.destination.display()
            )
        })?;
        if current != launcher.sha256 {
            bail!(
                "application launcher `{}` has drifted; refusing removal",
                launcher.destination.display()
            );
        }
    }
    for launcher in &management.launchers {
        fs::remove_file(&launcher.destination).with_context(|| {
            format!(
                "removing owned application launcher `{}`",
                launcher.destination.display()
            )
        })?;
    }
    Ok(format!(
        "removed {} receipt-owned launcher(s)",
        management.launchers.len()
    ))
}

struct PublishedFiles {
    backups: Vec<(PathBuf, PathBuf)>,
    published: Vec<PathBuf>,
}

impl PublishedFiles {
    fn commit(self) {
        for (backup, _) in self.backups {
            let _ = fs::remove_file(backup);
        }
    }

    fn rollback(self) -> Result<()> {
        for destination in self.published.iter().rev() {
            match fs::remove_file(destination) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error).context("removing rolled-back application file"),
            }
        }
        for (backup, destination) in self.backups.into_iter().rev() {
            fs::rename(&backup, &destination).context("restoring prior application file")?;
        }
        Ok(())
    }
}

fn publish_files(desired: &[(PathBuf, PathBuf)], retire: &[PathBuf]) -> Result<PublishedFiles> {
    let mut destinations = std::collections::BTreeSet::new();
    if desired
        .iter()
        .any(|(_, destination)| !destinations.insert(destination.to_path_buf()))
    {
        bail!("application publication repeats a destination");
    }
    for destination in retire {
        if !destinations.insert(destination.clone()) {
            bail!("application publication both replaces and retires a destination");
        }
    }
    let mut staged = Vec::new();
    for (index, (source, destination)) in desired.iter().enumerate() {
        let temporary = sibling_temporary(destination, "pending", index)?;
        fs::copy(source, &temporary)
            .with_context(|| format!("staging launcher `{}`", destination.display()))?;
        staged.push((temporary, destination.clone()));
    }
    let mut backups = Vec::new();
    let mut published = Vec::new();
    let result = (|| {
        for (index, destination) in retire.iter().enumerate() {
            let backup = sibling_temporary(destination, "prior", desired.len() + index)?;
            fs::rename(destination, &backup)?;
            backups.push((backup, destination.clone()));
        }
        for (index, (temporary, destination)) in staged.iter().enumerate() {
            let backup = sibling_temporary(destination, "prior", index)?;
            if destination.exists() {
                fs::rename(destination, &backup)?;
                backups.push((backup, destination.clone()));
            }
            fs::rename(temporary, destination)?;
            published.push(destination.clone());
        }
        Ok::<(), anyhow::Error>(())
    })();
    if let Err(error) = result {
        for destination in published.iter().rev() {
            let _ = fs::remove_file(destination);
        }
        for (backup, destination) in backups.iter().rev() {
            let _ = fs::rename(backup, destination);
        }
        for (temporary, _) in &staged {
            let _ = fs::remove_file(temporary);
        }
        return Err(error).context("publishing application files");
    }
    Ok(PublishedFiles { backups, published })
}

fn sibling_temporary(destination: &Path, role: &str, index: usize) -> Result<PathBuf> {
    let name = destination
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| anyhow::anyhow!("application destination has no portable file name"))?;
    Ok(destination.with_file_name(format!("{name}.vibe-{role}-{}-{index}", std::process::id())))
}

fn verify_generation(root: &Path, manifest: &BundleManifest) -> Result<()> {
    for file in &manifest.files {
        let path = root.join(&file.path);
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink()
            || !metadata.is_file()
            || metadata.len() != file.size
            || hash_file(&path)? != file.sha256
        {
            bail!("cached application generation differs from its verified manifest");
        }
    }
    Ok(())
}

fn ensure_owned_host(settings_root: &Path, host_root: &Path) -> Result<PathBuf> {
    fs::create_dir_all(settings_root)?;
    let settings = crate::commands::init::strip_unc_public(fs::canonicalize(settings_root)?);
    if host_root.exists() {
        let host = crate::commands::init::strip_unc_public(fs::canonicalize(host_root)?);
        if !host.starts_with(&settings) || host == settings {
            bail!("application host escapes settings root");
        }
        return Ok(host);
    }
    if !host_root.starts_with(&settings) {
        bail!("application host escapes settings root");
    }
    fs::create_dir_all(host_root)?;
    let host = crate::commands::init::strip_unc_public(fs::canonicalize(host_root)?);
    if !host.starts_with(&settings) || host == settings {
        bail!("application host escapes settings root");
    }
    Ok(host)
}

fn read_management(entry: &ManagementEntry) -> Result<BinaryManagement> {
    let raw = fs::read(&entry.entry).context("reading binary application management receipt")?;
    let value: BinaryManagement =
        serde_json::from_slice(&raw).context("parsing binary application management receipt")?;
    if value.protocol != MANAGEMENT_PROTOCOL {
        bail!("binary application management protocol is unsupported");
    }
    let management_dir = entry
        .entry
        .parent()
        .ok_or_else(|| anyhow::anyhow!("binary management entry has no parent"))?;
    let host = management_dir
        .parent()
        .ok_or_else(|| anyhow::anyhow!("binary management entry has no host"))?;
    let apps = host
        .parent()
        .filter(|path| path.file_name().is_some_and(|name| name == "apps"))
        .ok_or_else(|| {
            anyhow::anyhow!("binary management host is outside settings applications")
        })?;
    let opt = apps
        .parent()
        .filter(|path| path.file_name().is_some_and(|name| name == "opt"))
        .ok_or_else(|| anyhow::anyhow!("binary management host is outside settings opt"))?;
    let settings = opt
        .parent()
        .ok_or_else(|| anyhow::anyhow!("binary management host has no settings root"))?;
    if entry
        .entry
        .file_name()
        .is_none_or(|name| name != "binary.json")
        || management_dir
            .file_name()
            .is_none_or(|name| name != "management")
        || host.file_name().and_then(|name| name.to_str()) != Some(value.application_id.as_str())
        || value.payload_root.parent() != Some(&host.join("generations"))
        || value.bundle_entry.parent().is_none()
        || !value.bundle_entry.starts_with(&value.payload_root)
    {
        bail!("binary application management receipt escapes its owned host");
    }
    let bundle_metadata = fs::symlink_metadata(&value.bundle_entry)
        .context("reading binary bundle management entry")?;
    if bundle_metadata.file_type().is_symlink() || !bundle_metadata.is_file() {
        bail!("binary bundle management entry is not a regular owned file");
    }
    let launcher_root = settings.join("opt").join("bin");
    let mut destinations = std::collections::BTreeSet::new();
    for launcher in &value.launchers {
        if launcher.destination.parent() != Some(launcher_root.as_path())
            || launcher
                .destination
                .file_name()
                .and_then(|name| name.to_str())
                .is_none_or(|name| name.is_empty() || name.contains(['/', '\\']))
            || launcher.sha256.len() != 64
            || !launcher
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            || !destinations.insert(launcher.destination.clone())
        {
            bail!("binary application management receipt has an invalid launcher destination");
        }
    }
    Ok(value)
}

fn write_json(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(())
}

fn hash_file(path: &Path) -> Result<String> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        bail!("application-owned path is not a regular file");
    }
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

#[cfg(test)]
#[path = "binary/tests.rs"]
mod tests;
