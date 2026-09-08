//! Filesystem primitives for recoverable scrape export and in-place work.
//!
//! These operations deliberately expose expected state and third-state
//! outcomes.  A transaction must never turn "the name changed" into either
//! success or absence, because that would authorize mutation of an object it
//! did not inspect.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#SEC-NO-FOLLOW");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use sha2::{Digest as _, Sha256};

use crate::file::identity::{FileIdentity, file_identity};
use crate::file::{cap_options, verify_regular_single_link};
use crate::project::absolute::{absolute_parts, open_anchor};
use crate::{Pinned, Project};

#[cfg(windows)]
use cap_std::fs::OpenOptionsExt as _;

mod platform;
mod tree;

pub use tree::{
    CleanupCompletion, CleanupIntent, CleanupPreparation, EntryIdentity, EntryState,
    EntryStateKind, ExistingTreeEntryLease, OwnedDirectory, OwnedDirectoryCreateError,
    OwnedDirectoryIdentity, OwnedTreeCleanupError, OwnedTreeCleanupProgress, OwnedTreeObservation,
    OwnedTreePublishError, PublishedPendingVerification, ReopenOwnedDirectoryError,
    ReopenedOwnedDirectory, TreeEntry, TreeManifest,
};
#[cfg(any(test, feature = "inject-failures"))]
pub use tree::{
    arm_after_owned_tree_publish_move, arm_before_owned_tree_check, arm_before_owned_tree_publish,
    arm_between_manifest_passes, arm_during_manifest_lease,
};

/// Whether the containing directory accepted an explicit durability flush.
///
/// File data is always `sync_all`'d before a [`DurableWrite`] is returned.
/// Directory flush support varies by OS/filesystem, so its narrower guarantee
/// is data rather than an ignored error.
///
/// ```
/// use vibe_safefs::DirectoryDurability;
///
/// let durability = DirectoryDurability::JournalRecoverable;
/// assert!(matches!(durability, DirectoryDurability::JournalRecoverable));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectoryDurability {
    Synced,
    /// The namespace change is atomic and expected-handle guarded but Windows
    /// exposes no directory-fsync/write-through guarantee for this operation.
    /// A caller may accept it only under a durable intent with exact
    /// before/after recovery.
    JournalRecoverable,
    Unsupported(std::io::ErrorKind),
    Failed(std::io::ErrorKind),
}

/// One attempted parent-directory durability checkpoint.
///
/// ```
/// use vibe_safefs::Project;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let scope = tempfile::tempdir()?;
/// let project = Project::open(scope.path())?;
/// let write = project.write_atomic_durable("checkpoint.bin", b"durable")?;
/// let sync = write.directory_syncs.first().ok_or_else(|| {
///     std::io::Error::other("durable publication reported no directory checkpoint")
/// })?;
/// assert!(sync.directory.is_absolute());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectorySync {
    pub directory: PathBuf,
    pub durability: DirectoryDurability,
}

/// One atomically published file plus the durability level the host supplied.
///
/// ```
/// use vibe_safefs::Project;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let scope = tempfile::tempdir()?;
/// let project = Project::open(scope.path())?;
/// let write = project.write_atomic_durable("artifact.bin", b"published")?;
/// assert!(write.file_synced);
/// assert!(!write.directory_syncs.is_empty());
/// assert_eq!(std::fs::read(scope.path().join("artifact.bin"))?, b"published");
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableWrite {
    pub published: crate::Published,
    pub file_synced: bool,
    pub parent: DirectoryDurability,
    /// Every newly-created directory entry's parent, followed by the final
    /// file parent. This is the complete metadata-durability attempt chain.
    pub directory_syncs: Vec<DirectorySync>,
}

include!("transaction/external.rs");

/// A capability-relative rename refusal.  `SourceChanged` and `Occupied` are
/// expected transaction outcomes, while `Unsupported` is an honest platform
/// limit rather than a check-then-rename fallback.
///
/// ```
/// use vibe_safefs::RenameError;
///
/// let error = RenameError::Unsupported;
/// assert_eq!(error.to_string(), "atomic no-replace directory rename is unsupported");
/// ```
#[derive(Debug)]
pub enum RenameError {
    SourceChanged {
        path: PathBuf,
        detail: String,
    },
    Occupied {
        path: PathBuf,
    },
    PossiblyMoved {
        source: PathBuf,
        destination: PathBuf,
        detail: String,
    },
    CrossFilesystem,
    Unsupported,
    Failed(anyhow::Error),
}

impl std::fmt::Display for RenameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SourceChanged { path, detail } => {
                write!(f, "`{}` changed before rename: {detail}", path.display())
            }
            Self::Occupied { path } => write!(f, "`{}` is occupied", path.display()),
            Self::PossiblyMoved {
                source,
                destination,
                detail,
            } => write!(
                f,
                "rename from `{}` to `{}` may have moved a third state: {detail}",
                source.display(),
                destination.display()
            ),
            Self::CrossFilesystem => {
                f.write_str("source and destination are on different filesystems")
            }
            Self::Unsupported => f.write_str("atomic no-replace directory rename is unsupported"),
            Self::Failed(error) => write!(f, "{error:#}"),
        }
    }
}

impl std::error::Error for RenameError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Failed(error) => error.source(),
            _ => None,
        }
    }
}

include!("transaction/operations.rs");

#[cfg(any(test, feature = "inject-failures"))]
mod rename_hook {
    use std::cell::RefCell;

    type Hook = Box<dyn Fn(&crate::Pinned, &crate::Pinned, &str, &str)>;
    thread_local! {
        static BEFORE: RefCell<Option<Hook>> = const { RefCell::new(None) };
    }
    pub fn arm(hook: Option<Hook>) {
        BEFORE.with(|slot| *slot.borrow_mut() = hook);
    }
    pub fn before(source: &crate::Pinned, destination: &crate::Pinned, old: &str, new: &str) {
        let hook = BEFORE.with(|slot| slot.borrow_mut().take());
        if let Some(hook) = hook {
            hook(source, destination, old, new);
        }
    }
}

#[cfg(not(any(test, feature = "inject-failures")))]
mod rename_hook {
    pub fn before(_: &crate::Pinned, _: &crate::Pinned, _: &str, _: &str) {}
}

#[cfg(any(test, feature = "inject-failures"))]
pub type RenameNoReplaceHook = Box<dyn Fn(&Pinned, &Pinned, &str, &str)>;

#[cfg(any(test, feature = "inject-failures"))]
pub fn arm_before_rename_noreplace(hook: Option<RenameNoReplaceHook>) {
    rename_hook::arm(hook);
}

#[cfg(any(test, feature = "inject-failures"))]
mod final_rename_hook {
    use std::cell::RefCell;

    type Hook = Box<dyn Fn(&crate::Pinned, &crate::Pinned, &str, &str)>;
    thread_local! {
        static AFTER: RefCell<Option<Hook>> = const { RefCell::new(None) };
    }
    pub fn arm(hook: Option<Hook>) {
        AFTER.with(|slot| *slot.borrow_mut() = hook);
    }
    pub fn after(source: &crate::Pinned, destination: &crate::Pinned, old: &str, new: &str) {
        let hook = AFTER.with(|slot| slot.borrow_mut().take());
        if let Some(hook) = hook {
            hook(source, destination, old, new);
        }
    }
}

#[cfg(not(any(test, feature = "inject-failures")))]
mod final_rename_hook {
    pub fn after(_: &crate::Pinned, _: &crate::Pinned, _: &str, _: &str) {}
}

#[cfg(any(test, feature = "inject-failures"))]
pub type FinalRenameCheckHook = Box<dyn Fn(&Pinned, &Pinned, &str, &str)>;

#[cfg(any(test, feature = "inject-failures"))]
pub fn arm_after_rename_source_check(hook: Option<FinalRenameCheckHook>) {
    final_rename_hook::arm(hook);
}

#[cfg(any(test, feature = "inject-failures"))]
pub(crate) mod native_mutation_hook {
    use std::cell::RefCell;
    type Hook = Box<dyn Fn(&crate::Pinned, &str)>;
    thread_local! {
        static DURING: RefCell<Option<Hook>> = const { RefCell::new(None) };
    }
    pub fn arm(hook: Option<Hook>) {
        DURING.with(|slot| *slot.borrow_mut() = hook);
    }
    pub fn during(parent: &crate::Pinned, name: &str) {
        let hook = DURING.with(|slot| slot.borrow_mut().take());
        if let Some(hook) = hook {
            hook(parent, name);
        }
    }
}

#[cfg(not(any(test, feature = "inject-failures")))]
pub(crate) mod native_mutation_hook {
    pub fn during(_: &crate::Pinned, _: &str) {}
}

#[cfg(any(test, feature = "inject-failures"))]
pub type NativeMutationHook = Box<dyn Fn(&Pinned, &str)>;

#[cfg(any(test, feature = "inject-failures"))]
pub fn arm_during_native_mutation(hook: Option<NativeMutationHook>) {
    native_mutation_hook::arm(hook);
}

#[cfg(any(test, feature = "inject-failures"))]
mod volume_hook {
    use std::cell::RefCell;

    type Hook = Box<dyn Fn(bool) -> bool>;
    thread_local! {
        static CHECK: RefCell<Option<Hook>> = const { RefCell::new(None) };
    }
    pub fn arm(hook: Option<Hook>) {
        CHECK.with(|slot| *slot.borrow_mut() = hook);
    }
    pub fn check(actual: bool) -> bool {
        CHECK.with(|slot| slot.borrow().as_ref().map_or(actual, |hook| hook(actual)))
    }
}

#[cfg(not(any(test, feature = "inject-failures")))]
mod volume_hook {
    pub const fn check(actual: bool) -> bool {
        actual
    }
}

#[cfg(any(test, feature = "inject-failures"))]
pub fn arm_same_filesystem_check(hook: Option<Box<dyn Fn(bool) -> bool>>) {
    volume_hook::arm(hook);
}

#[cfg(test)]
#[path = "transaction/tests.rs"]
mod tests;
