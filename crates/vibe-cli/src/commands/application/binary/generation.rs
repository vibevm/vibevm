//! Immutable payload generation preparation and rollback.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_wire::generated::application::e1::management::BinaryManagement;

use super::VerifiedDistribution;
use super::ownership::verify_generation;

pub(super) enum GenerationTransaction {
    Existing,
    Added(PathBuf),
    Superseded { active: PathBuf, prior: PathBuf },
}

impl GenerationTransaction {
    pub(super) fn commit(self) {
        if let Self::Superseded { prior, .. } = self {
            let _ = fs::remove_dir_all(prior);
        }
    }

    pub(super) fn rollback(self) -> Result<()> {
        match self {
            Self::Existing => Ok(()),
            Self::Added(active) => fs::remove_dir_all(&active)
                .with_context(|| format!("removing rolled-back generation {}", active.display())),
            Self::Superseded { active, .. } => fs::remove_dir_all(&active)
                .with_context(|| format!("removing rolled-back replacement {}", active.display())),
        }
    }
}

pub(super) fn prepare_generation(
    host_root: &Path,
    verified: &VerifiedDistribution,
    previous: Option<&BinaryManagement>,
    force: bool,
) -> Result<(PathBuf, GenerationTransaction)> {
    let generations = host_root.join("generations");
    fs::create_dir_all(&generations)?;
    let mut generation = generations.join(&verified.asset_sha256);
    let transaction = if generation.exists() {
        let metadata = fs::symlink_metadata(&generation)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            bail!("application generation is not an owned regular directory");
        }
        if verify_generation(&generation, &verified.manifest).is_ok() {
            fs::remove_dir_all(&verified.payload_root)
                .context("removing duplicate staged distribution")?;
            GenerationTransaction::Existing
        } else if force && previous.is_some_and(|old| old.payload_root == generation) {
            let reservation = tempfile::Builder::new()
                .prefix(&format!("{}-force-", verified.asset_sha256))
                .tempdir_in(&generations)
                .context("reserving fresh application generation")?;
            let replacement = reservation.keep();
            fs::remove_dir(&replacement)
                .context("releasing fresh application generation reservation")?;
            fs::rename(&verified.payload_root, &replacement)
                .context("publishing repaired application generation")?;
            let prior = std::mem::replace(&mut generation, replacement.clone());
            GenerationTransaction::Superseded {
                active: replacement,
                prior,
            }
        } else {
            bail!("cached application generation differs from its verified manifest");
        }
    } else {
        fs::rename(&verified.payload_root, &generation)
            .context("publishing immutable application payload")?;
        GenerationTransaction::Added(generation.clone())
    };
    Ok((generation, transaction))
}
