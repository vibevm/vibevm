//! Retained publication identity for bounded cooperative undo, not namespace CAS.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#APPLY-FILE-UNDO");

use std::fs::File;
use std::path::Path;

use anyhow::{Context, Result};
use sha2::Digest;

use super::{StableFileSnapshot, cap_options, file_identity, stable, verify_regular_single_link};
use crate::{DurableWrite, FileIdentity, Pinned, Project, PublishError, PublishStage};

/// Evidence minted from the actual create-new staging handle before rename.
/// The handle stays private and alive; a later destination read cannot mint it.
///
/// ```
/// # fn main() -> anyhow::Result<()> {
/// let dir = tempfile::tempdir()?;
/// let project = vibe_safefs::Project::open(dir.path())?;
/// let publication = project.write_atomic_owned_in_with_mode(
///     &project.root_dir()?, "state", b"ours", None)?;
/// let receipt: vibe_safefs::OwnedFileReceipt = publication.receipt;
/// let current = project.read_file_snapshot_bounded("state", 4)?.unwrap();
/// assert!(receipt.matches_snapshot(&current));
/// # Ok(())
/// # }
/// ```
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#APPLY-FILE-UNDO"
)]
pub struct OwnedFileReceipt {
    _held: File,
    identity: FileIdentity,
    sha256: String,
    size: u64,
    mode: Option<u32>,
}

impl std::fmt::Debug for OwnedFileReceipt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OwnedFileReceipt").finish_non_exhaustive()
    }
}

impl OwnedFileReceipt {
    pub(super) fn from_stage(file: &File, path: &Path, bytes: &[u8]) -> Result<Self> {
        verify_regular_single_link(file, path)?;
        Ok(Self {
            _held: file
                .try_clone()
                .context("retaining the publication stage")?,
            identity: file_identity(file, path)?,
            sha256: format!("{:x}", sha2::Sha256::digest(bytes)),
            size: bytes.len() as u64,
            mode: stable::unix_mode(&file.metadata()?),
        })
    }

    /// Compare an independently observed current name with the known after-image.
    /// This observation is not a kernel compare-and-swap: a final mutation
    /// window against uncooperative writers remains.
    pub fn matches_snapshot(&self, current: &StableFileSnapshot) -> bool {
        self.matches_state(
            current.identity,
            &current.sha256,
            current.size,
            current.unix_mode,
        )
    }

    pub(super) fn matches_state(
        &self,
        identity: FileIdentity,
        sha256: &str,
        size: u64,
        mode: Option<u32>,
    ) -> bool {
        self.identity == identity && self.sha256 == sha256 && self.size == size && self.mode == mode
    }
}

/// Logical publication facts together with privately retained object evidence.
///
/// ```
/// # fn main() -> anyhow::Result<()> {
/// let dir = tempfile::tempdir()?;
/// let project = vibe_safefs::Project::open(dir.path())?;
/// let publication: vibe_safefs::OwnedPublication = project.write_atomic_owned_in_with_mode(
///     &project.root_dir()?, "state", b"complete", None)?;
/// assert!(publication.publication.file_synced);
/// assert!(publication.publication.published.created_directories.is_empty());
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#APPLY-FILE-UNDO"
)]
pub struct OwnedPublication {
    pub publication: DurableWrite,
    pub receipt: OwnedFileReceipt,
}

/// Publication failure retaining the original stage identity when it may have
/// landed. Neither this type nor its receipt promises hostile-writer CAS.
///
/// ```
/// # fn main() -> anyhow::Result<()> {
/// let dir = tempfile::tempdir()?;
/// std::fs::create_dir(dir.path().join("occupied"))?;
/// let project = vibe_safefs::Project::open(dir.path())?;
/// let error: vibe_safefs::OwnedPublishError = project.write_atomic_owned_in_with_mode(
///     &project.root_dir()?, "occupied", b"refused", None).unwrap_err();
/// assert_eq!(error.publication.stage, vibe_safefs::PublishStage::BeforePublication);
/// assert!(error.receipt.is_none());
/// assert!(dir.path().join("occupied").is_dir());
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#APPLY-FILE-UNDO"
)]
pub struct OwnedPublishError {
    pub publication: PublishError,
    pub receipt: Option<OwnedFileReceipt>,
}

impl std::fmt::Display for OwnedPublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.publication.fmt(f)
    }
}
impl std::error::Error for OwnedPublishError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.publication)
    }
}

impl Project {
    /// Publish through the ordinary atomic staging path while retaining its
    /// actual object for a later checked, cooperative undo. Existing logical
    /// publication APIs remain separate and do not grant this ownership.
    ///
    /// ```
    /// # fn main() -> anyhow::Result<()> {
    /// let dir = tempfile::tempdir()?;
    /// let project = vibe_safefs::Project::open(dir.path())?;
    /// let published = project.write_atomic_owned_in_with_mode(
    ///     &project.root_dir()?, "metadata", b"known after-image", None)?;
    /// let current = project.read_file_snapshot_bounded("metadata", 17)?.unwrap();
    /// assert!(published.receipt.matches_snapshot(&current));
    /// # Ok(())
    /// # }
    /// ```
    pub fn write_atomic_owned_in_with_mode(
        &self,
        directory: &Pinned,
        relative: &str,
        bytes: &[u8],
        unix_mode: Option<u32>,
    ) -> Result<OwnedPublication, OwnedPublishError> {
        let mut receipt = None;
        let result = self.write_atomic_durable_in_with_mode_stage(
            directory,
            relative,
            bytes,
            unix_mode,
            None,
            Some(&mut receipt),
        );
        match result {
            Ok(publication) => match receipt {
                Some(receipt) => Ok(OwnedPublication {
                    publication,
                    receipt,
                }),
                None => Err(OwnedPublishError {
                    publication: PublishError::possibly(
                        publication.published.created_directories,
                        anyhow::anyhow!(
                            "publication completed without retained stage evidence; inspect the destination"
                        ),
                    ),
                    receipt: None,
                }),
            },
            Err(publication) => {
                if publication.stage == PublishStage::BeforePublication {
                    receipt = None;
                }
                Err(OwnedPublishError {
                    publication,
                    receipt,
                })
            }
        }
    }
}

/// Do not clean a staging name that was replaced while its handle stayed held.
/// The check-to-unlink interval has the same cooperative boundary as undo.
pub(super) fn clean_held_stage(parent: &Pinned, name: &str, held: &File) -> Result<()> {
    let path = parent.join(name);
    let mut options = cap_options();
    let current = match parent.dir.open_with(name, options.read(true)) {
        Ok(file) => file.into_std(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(anyhow::Error::new(error).context(format!(
                "opening stage `{}` for checked cleanup; residue left untouched",
                path.display()
            )));
        }
    };
    verify_regular_single_link(&current, &path)?;
    if file_identity(&current, &path)? != file_identity(held, &path)? {
        anyhow::bail!(
            "staging name `{}` holds a replacement object; left untouched",
            path.display()
        );
    }
    parent
        .dir
        .remove_file(name)
        .with_context(|| format!("removing owned stage `{}`", path.display()))
}

/// Capture before rename; do not discard cleanup failure or remove a foreign stage.
pub(super) fn capture_stage(
    parent: &Pinned,
    name: &str,
    held: &File,
    bytes: &[u8],
) -> Result<OwnedFileReceipt> {
    OwnedFileReceipt::from_stage(held, &parent.join(name), bytes).map_err(|error| {
        match clean_held_stage(parent, name, held) {
            Ok(()) => error,
            Err(cleanup) => error.context(format!(
                "stage `{}` left untouched: {cleanup:#}",
                parent.join(name).display()
            )),
        }
    })
}
