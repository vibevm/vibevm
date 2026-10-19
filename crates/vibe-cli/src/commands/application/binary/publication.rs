//! Transactional publication of owned application files.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#distribution");

use anyhow::{Context, Result, bail};
use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub(super) struct PublishedFiles {
    backups: Vec<(PathBuf, PathBuf)>,
    published: Vec<PathBuf>,
}

impl PublishedFiles {
    pub(super) fn commit(self) {
        let _ = self.commit_checked();
    }

    pub(super) fn commit_checked(self) -> Result<()> {
        for (backup, _) in self.backups {
            fs::remove_file(backup).context("removing retired application launcher backup")?;
        }
        Ok(())
    }

    pub(super) fn rollback(self) -> Result<()> {
        for destination in self.published.iter().rev() {
            match fs::remove_file(destination) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error).context("removing rolled-back application file"),
            }
        }
        for (backup, destination) in self.backups.into_iter().rev() {
            if existing_regular_file(&destination)? {
                bail!("refusing to overwrite an unowned file during application rollback");
            }
            fs::rename(&backup, &destination).context("restoring prior application file")?;
        }
        Ok(())
    }
}

pub(super) fn publish_files(
    desired: &[(PathBuf, PathBuf)],
    retire: &[PathBuf],
) -> Result<PublishedFiles> {
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
    let preparation = (|| {
        for (index, (source, destination)) in desired.iter().enumerate() {
            let temporary = sibling_temporary(destination, "pending", index)?;
            regular_file(source)?;
            existing_regular_file(destination)?;
            let mut input = fs::File::open(source)?;
            let mut output = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
                .with_context(|| format!("staging launcher `{}`", destination.display()))?;
            io::copy(&mut input, &mut output)?;
            output.sync_all()?;
            fs::set_permissions(&temporary, fs::metadata(source)?.permissions())?;
            staged.push((temporary, destination.clone()));
        }
        for destination in retire {
            regular_file(destination)?;
        }
        Ok::<(), anyhow::Error>(())
    })();
    if let Err(error) = preparation {
        for (temporary, _) in &staged {
            let _ = fs::remove_file(temporary);
        }
        return Err(error);
    }
    let mut backups = Vec::new();
    let mut published = Vec::new();
    let result = (|| {
        for (index, destination) in retire.iter().enumerate() {
            let backup = sibling_temporary(destination, "prior", desired.len() + index)?;
            if existing_regular_file(&backup)? {
                bail!("application publication backup already exists");
            }
            fs::rename(destination, &backup)?;
            backups.push((backup, destination.clone()));
        }
        for (index, (temporary, destination)) in staged.iter().enumerate() {
            let backup = sibling_temporary(destination, "prior", index)?;
            if existing_regular_file(destination)? {
                if existing_regular_file(&backup)? {
                    bail!("application publication backup already exists");
                }
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

fn regular_file(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        bail!("application-owned publication path is not a regular file");
    }
    Ok(())
}

fn existing_regular_file(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => Ok(true),
        Ok(_) => bail!("application-owned publication path is not a regular file"),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).context("checking application publication destination"),
    }
}

fn sibling_temporary(destination: &Path, role: &str, index: usize) -> Result<PathBuf> {
    let name = destination
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| anyhow::anyhow!("application destination has no portable file name"))?;
    Ok(destination.with_file_name(format!("{name}.vibe-{role}-{}-{index}", std::process::id())))
}
