//! Identity-bound removal: delete the object you inspected, not the name.
//!
//! A caller that decides "this file may go" by opening it, judging it, and
//! then calling a remove-by-name has decided about one object and acted on a
//! different one. Between the two calls the name can be rebound — a swap, a
//! rename, a fresh file with the same spelling — and the removal takes
//! whatever is there now. For an archive sweep that is the difference between
//! collecting a spent diagnostic and deleting somebody's file.
//!
//! So judgement and removal are joined by an [`EntryProof`]: an **opaque**,
//! capability-derived identity taken from the very handle the judgement read.
//! Removal reopens through the same pinned directory capability, re-derives
//! the proof, and refuses unless it is the same object. The proof carries no
//! public accessors and no `Display`: it is evidence to hand back, never a
//! number to compute with or log.
//!
//! ## What this does and does not close
//!
//! It closes the window the caller opens — everything between "I decided" and
//! "I acted", which is where a sweep spends almost all of its time and where
//! every plantable swap lands. It does not close the instruction between the
//! final re-derivation and the `unlink` itself: removing *by handle* is not
//! portable (Windows can do it through `SetFileInformationByHandle`, POSIX
//! has no `unlinkat` by inode), and this crate forbids `unsafe`. The residual
//! window is one syscall wide and unplantable from a test; the seconds-wide
//! one is gone. Both are stated here rather than in a comment nobody reads.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use specmark::spec;

use crate::component::ensure_safe_component;
use crate::file::identity::{FileIdentity, file_identity};
use crate::file::{cap_options, verify_regular_single_link};
use crate::project::{Pinned, Project};

/// Opaque proof of WHICH object a name held when it was inspected.
///
/// Deliberately not `Display`, not destructurable and carrying no accessor:
/// the only thing a caller may do with one is hand it back to a proved
/// removal. Two proofs comparing equal means the OS calls them one object.
#[derive(Clone, Copy, PartialEq, Eq)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
pub struct EntryProof(FileIdentity);

impl std::fmt::Debug for EntryProof {
    /// Reports that a proof exists, never what it is. A volume serial and a
    /// file index in a log line are exactly the two numbers this type exists
    /// to keep out of one.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EntryProof(..)")
    }
}

/// Why a proved removal removed nothing.
///
/// `Changed` is the load-bearing arm: it means the name no longer holds the
/// object the caller judged, so the caller's decision does not apply to what
/// is there — and what is there was left exactly as it was.
#[derive(Debug)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
pub enum ProofRefusal {
    /// The entry is gone, is a different object, or is no longer an ordinary
    /// owned file/directory. Nothing was removed.
    Changed { path: PathBuf, detail: String },
    /// The proved directory is no longer empty. Nothing was removed.
    NotEmpty { path: PathBuf },
    /// The identity held, and the removal itself failed.
    Failed {
        path: PathBuf,
        source: anyhow::Error,
    },
}

impl ProofRefusal {
    /// Whether the name stopped naming the proved object. `false` means the
    /// object was still there and something else went wrong.
    #[must_use]
    pub const fn changed(&self) -> bool {
        matches!(self, Self::Changed { .. })
    }

    fn changed_at(path: &Path, detail: impl Into<String>) -> Self {
        Self::Changed {
            path: path.to_path_buf(),
            detail: detail.into(),
        }
    }
}

impl std::fmt::Display for ProofRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Changed { path, detail } => write!(
                f,
                "`{}` no longer names the inspected object ({detail}); it was left in place",
                path.display()
            ),
            Self::NotEmpty { path } => write!(
                f,
                "`{}` is no longer empty; it was left in place",
                path.display()
            ),
            Self::Failed { path, source } => {
                write!(f, "removing `{}` failed: {source:#}", path.display())
            }
        }
    }
}

impl std::error::Error for ProofRefusal {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Failed { source, .. } => source.source(),
            _ => None,
        }
    }
}

impl Project {
    /// Inspect an ordinary single-link file directly inside `directory`,
    /// returning the proof of WHICH file it is and how many bytes it holds.
    ///
    /// `Ok(None)` is absence. A directory, a link or reparse point, a device,
    /// or a file sharing its inode with another name is an error, not a
    /// length: an archive entry that is any of those is not an entry this
    /// crate is entitled to judge.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
    pub fn inspect_file_in(
        &self,
        directory: &Pinned,
        name: &str,
    ) -> Result<Option<(EntryProof, u64)>> {
        ensure_safe_component(name)?;
        let display = directory.join(name);
        let mut options = cap_options();
        match directory.dir.open_with(name, options.read(true)) {
            Ok(file) => {
                let std_file = file.into_std();
                verify_regular_single_link(&std_file, &display)?;
                let metadata = std_file
                    .metadata()
                    .with_context(|| format!("inspecting `{}`", display.display()))?;
                let proof = EntryProof(file_identity(&std_file, &display)?);
                Ok(Some((proof, metadata.len())))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(anyhow::Error::new(error).context(format!(
                "opening `{}` without following a link",
                display.display()
            ))),
        }
    }

    /// Remove one direct child file only while it is still the object `proof`
    /// describes.
    ///
    /// There is no by-name fallback and no "close enough": a name that now
    /// holds anything else — including an ordinary file with the same bytes —
    /// is [`ProofRefusal::Changed`] and survives untouched.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
    pub fn remove_file_proved_in(
        &self,
        directory: &Pinned,
        name: &str,
        proof: &EntryProof,
    ) -> Result<(), ProofRefusal> {
        let display = directory.join(name);
        ensure_safe_component(name)
            .map_err(|error| ProofRefusal::changed_at(&display, format!("{error:#}")))?;
        crate::race_hook::before_proved_removal(directory, name);
        let current = self
            .inspect_file_in(directory, name)
            .map_err(|error| ProofRefusal::changed_at(&display, format!("{error:#}")))?;
        match current {
            Some((current, _)) if current == *proof => {}
            Some(_) => {
                return Err(ProofRefusal::changed_at(
                    &display,
                    "a different file now holds that name",
                ));
            }
            None => return Err(ProofRefusal::changed_at(&display, "it is no longer there")),
        }
        directory
            .dir
            .remove_file(name)
            .map_err(|error| ProofRefusal::Failed {
                path: display,
                source: anyhow::Error::new(error),
            })
    }

    /// Remove one direct child directory only while it is still the object
    /// `proof` describes AND is empty.
    ///
    /// The inspecting capability is opened and DROPPED inside this call:
    /// Windows refuses to remove a directory any open handle still names, so
    /// the check that licenses the removal must also release what it opened.
    /// A caller holding its own capability to the same directory must drop it
    /// before calling — that is why the proof, not the handle, is what travels.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
    pub fn remove_dir_proved_in(
        &self,
        parent: &Pinned,
        name: &str,
        proof: &EntryProof,
    ) -> Result<(), ProofRefusal> {
        let display = parent.join(name);
        ensure_safe_component(name)
            .map_err(|error| ProofRefusal::changed_at(&display, format!("{error:#}")))?;
        crate::race_hook::before_proved_removal(parent, name);
        let child = match parent.open_child_checked(name) {
            Ok(Some(child)) => child,
            Ok(None) => return Err(ProofRefusal::changed_at(&display, "it is no longer there")),
            Err(error) => {
                return Err(ProofRefusal::changed_at(
                    &display,
                    format!("it no longer opens as a link-free directory: {error:#}"),
                ));
            }
        };
        let current = child
            .proof()
            .map_err(|error| ProofRefusal::changed_at(&display, format!("{error:#}")))?;
        if current != *proof {
            return Err(ProofRefusal::changed_at(
                &display,
                "a different directory now holds that name",
            ));
        }
        let empty = child
            .dir
            .entries()
            .map_err(|error| ProofRefusal::Failed {
                path: display.clone(),
                source: anyhow::Error::new(error).context("listing the proved directory"),
            })?
            .next()
            .is_none();
        // Released before the removal, and before the emptiness answer is
        // acted on: an open handle is the one thing that makes this fail on
        // Windows for a reason that has nothing to do with identity.
        drop(child);
        if !empty {
            return Err(ProofRefusal::NotEmpty { path: display });
        }
        parent
            .dir
            .remove_dir(name)
            .map_err(|error| ProofRefusal::Failed {
                path: display,
                source: anyhow::Error::new(error),
            })
    }
}

impl Pinned {
    /// The opaque identity of THIS pinned directory — taken from the
    /// capability itself, so it names the object the walk actually reached
    /// rather than whatever the path spells now.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#REPLY-SHAPE")]
    pub fn proof(&self) -> Result<EntryProof> {
        let handle = self
            .dir
            .try_clone()
            .with_context(|| format!("retaining `{}` to identify it", self.path().display()))?
            .into_std_file();
        Ok(EntryProof(file_identity(&handle, self.path())?))
    }
}

#[cfg(test)]
#[path = "proof/tests.rs"]
mod tests;
