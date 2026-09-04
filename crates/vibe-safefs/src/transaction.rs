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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectorySync {
    pub directory: PathBuf,
    pub durability: DirectoryDurability,
}

/// One atomically published file plus the durability level the host supplied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableWrite {
    pub published: crate::Published,
    pub file_synced: bool,
    pub parent: DirectoryDurability,
    /// Every newly-created directory entry's parent, followed by the final
    /// file parent. This is the complete metadata-durability attempt chain.
    pub directory_syncs: Vec<DirectorySync>,
}

/// A capability root selected explicitly by the caller, never inferred from
/// the project and never hardcoded below project `.vibe`.
#[derive(Debug)]
pub struct ExternalStore {
    root: Pinned,
    ancestor_identities: Vec<FileIdentity>,
    bootstrap_syncs: Vec<DirectorySync>,
}

/// One retained directory below an [`ExternalStore`]. All operations remain
/// component-relative to this capability; `path()` is display-only.
#[derive(Debug)]
pub struct ExternalDirectory {
    pinned: Pinned,
}

impl ExternalStore {
    /// Open or create an explicit absolute store path one no-follow component
    /// at a time.  A raced creator is reopened no-follow; a link, junction,
    /// file or special entry refuses.
    pub fn open_or_create(path: &Path) -> Result<Self> {
        let (anchor, components) = absolute_parts(path)?;
        if components.is_empty() {
            bail!("external store must not be a filesystem anchor");
        }
        let mut current = open_anchor(&anchor)?;
        let mut ancestors = vec![current.identity()?];
        for component in components {
            current = current.ensure_child(&component)?;
            ancestors.push(current.identity()?);
        }
        Ok(Self {
            root: current,
            ancestor_identities: ancestors,
            bootstrap_syncs: Vec::new(),
        })
    }

    /// Open/create an external store with the disjointness proof ordered
    /// before the first creation.  This is the transaction entry point:
    /// failure cannot leave even an empty store directory inside the project.
    pub fn open_or_create_disjoint(path: &Path, project: &Project) -> Result<Self> {
        let (anchor, components) = absolute_parts(path)?;
        if components.is_empty() {
            bail!("external store must not be a filesystem anchor");
        }
        let project_identity = project.root_identity()?;
        let mut current = open_anchor(&anchor)?;
        let mut ancestors = vec![current.identity()?];
        let mut missing_at = None;
        for (index, component) in components.iter().enumerate() {
            match current.open_child_checked(component) {
                Ok(Some(child)) => {
                    current = child;
                    ancestors.push(current.identity()?);
                }
                Ok(None) => {
                    missing_at = Some(index);
                    break;
                }
                Err(error) => {
                    return Err(error.context(format!(
                        "opening external-store component `{component}` no-follow"
                    )));
                }
            }
        }
        if let Some(index) = missing_at {
            if ancestors.contains(&project_identity) {
                bail!(
                    "external store `{}` would be created inside project `{}`; nothing was created",
                    path.display(),
                    project.root_path().display()
                );
            }
            let mut bootstrap_syncs = Vec::new();
            for component in &components[index..] {
                let parent_path = current.path().to_path_buf();
                let (child, created, durability) =
                    ensure_external_child_durable(&current, component)?;
                if created {
                    bootstrap_syncs.push(DirectorySync {
                        directory: parent_path,
                        durability,
                    });
                }
                current = child;
                ancestors.push(current.identity()?);
            }
            let store = Self {
                root: current,
                ancestor_identities: ancestors,
                bootstrap_syncs,
            };
            store.prove_disjoint_from(project)?;
            return Ok(store);
        }
        let store = Self {
            root: current,
            ancestor_identities: ancestors,
            bootstrap_syncs: Vec::new(),
        };
        store.prove_disjoint_from(project)?;
        Ok(store)
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        self.root.path()
    }

    #[must_use]
    pub fn bootstrap_durability(&self) -> &[DirectorySync] {
        &self.bootstrap_syncs
    }

    pub fn require_durable_bootstrap(&self) -> Result<()> {
        if let Some(sync) = self.bootstrap_syncs.iter().find(|sync| {
            let strong = matches!(sync.durability, DirectoryDurability::Synced);
            // Epoch-1 Windows has no persistent-media directory-fsync claim.
            // A capability-relative create is nevertheless safe to retry before
            // the initial journal: losing it can only strand liveness-only
            // external state, never a project mutation.
            let windows_liveness_only =
                cfg!(windows) && sync.durability == DirectoryDurability::JournalRecoverable;
            !strong && !windows_liveness_only
        }) {
            bail!(
                "external-store parent `{}` did not provide durable metadata sync: {:?}",
                sync.directory.display(),
                sync.durability
            );
        }
        Ok(())
    }

    /// Prove by capability ancestry, not textual prefix, that neither root is
    /// inside the other.  This also catches aliased spellings of the project.
    pub fn prove_disjoint_from(&self, project: &Project) -> Result<()> {
        let project_identity = project.root_identity()?;
        if self.ancestor_identities.contains(&project_identity) {
            bail!(
                "external store `{}` is inside project `{}`",
                self.path().display(),
                project.root_path().display()
            );
        }
        let store_identity = self.root.identity()?;
        if project.ancestor_identities.contains(&store_identity) {
            bail!(
                "project `{}` is inside external store `{}`; the roots are not disjoint",
                project.root_path().display(),
                self.path().display()
            );
        }
        Ok(())
    }

    /// Acquire an exclusive identity-rechecked OS lock below this explicit
    /// store.  The arbitrary project key is hashed into a portable filename;
    /// raw project paths and raw OS identity numbers never enter that name.
    pub fn open_and_lock_project(&self, project_key: &str) -> Result<ExternalProjectLock> {
        if project_key.is_empty() {
            bail!("project key must not be empty");
        }
        let (locks, _, _) = ensure_external_child_durable(&self.root, "locks")?;
        let mut digest = Sha256::new();
        digest.update(b"vibe-safefs-external-lock-e1\0");
        digest.update(project_key.as_bytes());
        let name = format!("{:x}.lock", digest.finalize());
        acquire_external_lock(&locks, &name)
    }

    /// Atomic durable file publication below the explicit store capability.
    pub fn write_durable(
        &self,
        relative: &str,
        bytes: &[u8],
    ) -> Result<DurableWrite, crate::PublishError> {
        let view = project_view(&self.root)
            .map_err(|error| crate::PublishError::before(Vec::new(), error))?;
        view.write_atomic_durable(relative, bytes)
    }

    pub fn root(&self) -> Result<Pinned> {
        self.root.shallow_clone()
    }

    pub fn root_directory(&self) -> Result<ExternalDirectory> {
        Ok(ExternalDirectory {
            pinned: self.root.shallow_clone()?,
        })
    }

    /// Stable bounded read rooted at the retained store capability.
    pub fn read_stable_bounded(
        &self,
        relative: &str,
        cap: usize,
    ) -> Result<Option<crate::StableFileSnapshot>> {
        project_view(&self.root)?.read_file_snapshot_bounded(relative, cap)
    }

    /// Open a relative directory chain no-follow; absence is distinct from
    /// link/non-directory/I/O refusal.
    pub fn open_directory(&self, relative: &str) -> Result<Option<ExternalDirectory>> {
        let components = directory_components(relative)?;
        let mut current = self.root.shallow_clone()?;
        for component in components {
            let Some(child) = current.open_child_checked(&component)? else {
                return Ok(None);
            };
            current = child;
        }
        Ok(Some(ExternalDirectory { pinned: current }))
    }
}

impl ExternalDirectory {
    /// Display-only absolute spelling. Callers receive no ambient mutation
    /// authority from this value.
    #[must_use]
    pub fn path(&self) -> &Path {
        self.pinned.path()
    }

    /// Canonical byte-sorted direct child names under a width fence.
    pub fn child_names_bounded(&self, max: usize) -> Result<Vec<String>> {
        let view = project_view(&self.pinned)?;
        let mut names = view.child_names_bounded(&self.pinned, max)?;
        names.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
        Ok(names)
    }

    /// Open one direct child no-follow. `None` is only absence.
    pub fn open_child(&self, name: &str) -> Result<Option<Self>> {
        Ok(self
            .pinned
            .open_child_checked(name)?
            .map(|pinned| Self { pinned }))
    }

    /// Ensure one direct child and report whether this call created it plus
    /// the exact parent-directory durability result for that creation.
    pub fn ensure_child(&self, name: &str) -> Result<(Self, bool, Option<DirectoryDurability>)> {
        let (pinned, created, checkpoint) = ensure_external_child_durable(&self.pinned, name)?;
        let durability = created.then_some(checkpoint);
        Ok((Self { pinned }, created, durability))
    }

    /// Stable bounded read below this retained directory.
    pub fn read_stable_bounded(
        &self,
        relative: &str,
        cap: usize,
    ) -> Result<Option<crate::StableFileSnapshot>> {
        project_view(&self.pinned)?.read_file_snapshot_bounded(relative, cap)
    }

    /// Observe one direct expected-state operand without following links.
    pub fn inspect_child_state(&self, name: &str) -> Result<Option<EntryState>> {
        self.pinned.inspect_child_state(name)
    }

    /// Strong exclusive owned subdirectory creation for a transaction id.
    pub fn create_owned_child_exclusive(
        &self,
        name: &str,
        ownership_token: &str,
    ) -> std::result::Result<OwnedDirectory, OwnedDirectoryCreateError> {
        self.pinned
            .create_owned_child_exclusive(name, ownership_token)
    }

    /// Remove one regular file through its expected state and a native held
    /// handle. The returned directory durability is part of the completion;
    /// absence or changed identity/content is a third state.
    pub fn remove_file_expected(
        &self,
        name: &str,
        expected: &EntryState,
    ) -> std::result::Result<DirectoryDurability, OwnedTreeCleanupError> {
        if expected.kind != EntryStateKind::File {
            return Err(OwnedTreeCleanupError::Third {
                detail: "expected-state removal requires a regular file state".to_owned(),
            });
        }
        platform::remove_expected(&self.pinned, name, expected).map_err(|error| match error {
            platform::NativeRemoveError::Changed(detail) => OwnedTreeCleanupError::Third { detail },
            platform::NativeRemoveError::Io(error) => {
                OwnedTreeCleanupError::Io(anyhow::Error::new(error))
            }
            #[cfg(not(windows))]
            platform::NativeRemoveError::Unsupported => OwnedTreeCleanupError::Unsupported,
        })
    }

    /// Prepare the next canonical manifest-bound retirement intent for an
    /// owned transaction directory. No removal occurs in this call.
    pub fn prepare_owned_child_retirement(
        &self,
        name: &str,
        ownership_token: &str,
        expected_identity: &OwnedDirectoryIdentity,
        expected_manifest: &TreeManifest,
        progress: &OwnedTreeCleanupProgress,
    ) -> std::result::Result<CleanupPreparation, OwnedTreeCleanupError> {
        self.pinned.prepare_owned_tree_cleanup_next(
            name,
            ownership_token,
            expected_identity,
            expected_manifest,
            progress,
        )
    }

    /// Execute one already-durable retirement intent, including the
    /// syscall-before-completion recovery case.
    pub fn execute_owned_child_retirement(
        &self,
        name: &str,
        ownership_token: &str,
        expected_identity: &OwnedDirectoryIdentity,
        expected_manifest: &TreeManifest,
        progress: &OwnedTreeCleanupProgress,
        intent: &CleanupIntent,
    ) -> std::result::Result<CleanupCompletion, OwnedTreeCleanupError> {
        self.pinned.execute_owned_tree_cleanup_intent(
            name,
            ownership_token,
            expected_identity,
            expected_manifest,
            progress,
            intent,
        )
    }
}

fn directory_components(relative: &str) -> Result<Vec<String>> {
    let (mut parents, name) = crate::split_relative(relative)?;
    parents.push(name);
    Ok(parents)
}

fn ensure_external_child_durable(
    parent: &Pinned,
    name: &str,
) -> Result<(Pinned, bool, DirectoryDurability)> {
    crate::ensure_safe_component(name)?;
    #[cfg(windows)]
    {
        match platform::create_directory(parent, name) {
            Ok((dir, durability)) => Ok((
                Pinned {
                    dir,
                    path: parent.join(name),
                },
                true,
                durability,
            )),
            Err(platform::NativeCreateError::NotCreated(error)) => {
                if let Some(existing) = parent.open_child_checked(name)? {
                    Ok((existing, false, DirectoryDurability::JournalRecoverable))
                } else {
                    Err(anyhow::Error::new(error).context(format!(
                        "creating external directory `{}` with a retained native handle",
                        parent.join(name).display()
                    )))
                }
            }
            Err(platform::NativeCreateError::CreatedButUnsealed(error)) => {
                Err(anyhow::Error::new(error).context(format!(
                    "external directory `{}` was created but could not be retained",
                    parent.join(name).display()
                )))
            }
        }
    }
    #[cfg(not(windows))]
    {
        let (pinned, created) = parent.ensure_child_recording(name)?;
        let durability = if created {
            sync_directory(parent)
        } else {
            DirectoryDurability::Synced
        };
        Ok((pinned, created, durability))
    }
}

/// A live external project lock.  Drop (or process death) releases it.
#[derive(Debug)]
pub struct ExternalProjectLock {
    file: std::fs::File,
    directory: Pinned,
    name: String,
}

impl ExternalProjectLock {
    /// Prove that the still-held lock handle remains the object named by the
    /// external lock capability. Windows additionally holds the file without
    /// `FILE_SHARE_DELETE`, so no rename/unlink can open a gap after this check.
    pub fn require_still_named(&self) -> Result<()> {
        let display = self.directory.join(&self.name);
        if lock_still_named(&self.directory, &self.name, &self.file, &display)? {
            Ok(())
        } else {
            bail!(
                "held external lock `{}` no longer occupies its capability-relative name",
                display.display()
            )
        }
    }
}

const LOCK_ATTEMPTS: u32 = 8;

fn acquire_external_lock(directory: &Pinned, name: &str) -> Result<ExternalProjectLock> {
    let display = directory.join(name);
    for _ in 0..LOCK_ATTEMPTS {
        let mut options = cap_options();
        #[cfg(windows)]
        options.share_mode(0x0000_0001 | 0x0000_0002);
        let file = directory
            .dir
            .open_with(name, options.read(true).write(true).create(true))
            .with_context(|| format!("opening external lock `{}`", display.display()))?
            .into_std();
        verify_regular_single_link(&file, &display)?;
        crate::race_hook::before_lock(directory, name);
        file.lock()
            .with_context(|| format!("locking `{}`", display.display()))?;
        if lock_still_named(directory, name, &file, &display)? {
            return Ok(ExternalProjectLock {
                file,
                directory: directory.shallow_clone()?,
                name: name.to_owned(),
            });
        }
        drop(file);
    }
    bail!(
        "external lock `{}` was replaced during every one of {LOCK_ATTEMPTS} attempts",
        display.display()
    )
}

fn lock_still_named(
    directory: &Pinned,
    name: &str,
    locked: &std::fs::File,
    display: &Path,
) -> Result<bool> {
    let mut options = cap_options();
    match directory.dir.open_with(name, options.read(true)) {
        Ok(current) => {
            let current = current.into_std();
            verify_regular_single_link(&current, display)?;
            Ok(crate::race_hook::lock_identity_matches(
                file_identity(locked, display)? == file_identity(&current, display)?,
            ))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(anyhow::Error::new(error)
            .context(format!("rechecking external lock `{}`", display.display()))),
    }
}

/// A capability-relative rename refusal.  `SourceChanged` and `Occupied` are
/// expected transaction outcomes, while `Unsupported` is an honest platform
/// limit rather than a check-then-rename fallback.
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

impl Project {
    /// Stable opaque project identity suitable as input to project-key
    /// hashing.  Only a domain-separated SHA-256 leaves this crate.
    pub fn identity_token(&self) -> Result<String> {
        Ok(identity_token(
            b"vibe-safefs-project-identity-e1\0",
            self.root_identity()?,
        ))
    }

    /// Atomically publish a file and report the host's parent-directory flush
    /// capability instead of silently discarding it.
    pub fn write_atomic_durable(
        &self,
        relative: &str,
        bytes: &[u8],
    ) -> Result<DurableWrite, crate::PublishError> {
        let root = self
            .root_dir()
            .map_err(|error| crate::PublishError::before(Vec::new(), error))?;
        self.write_atomic_durable_in(&root, relative, bytes)
    }
}

impl Pinned {
    /// Compare only filesystem/volume identity.  Object identity remains
    /// opaque and is not exposed as a platform number.
    pub fn same_filesystem(&self, other: &Pinned) -> Result<bool> {
        Ok(volume_hook::check(
            self.identity()?.same_filesystem(other.identity()?),
        ))
    }

    /// Ask this exact held directory capability for its strongest available
    /// metadata flush and report the result explicitly.
    #[must_use]
    pub fn sync_directory(&self) -> DirectoryDurability {
        sync_directory(self)
    }

    /// Observe a direct child as the complete expected state needed by a
    /// later guarded rename. Links, hard links and special files refuse.
    pub fn inspect_child_state(&self, name: &str) -> Result<Option<EntryState>> {
        tree::inspect_child_state(self, name)
    }

    /// Inspect only the exact reserved deterministic transaction-stage
    /// grammar. Ordinary component APIs continue to reject this namespace.
    pub fn inspect_transaction_stage_state(&self, name: &str) -> Result<Option<EntryState>> {
        tree::inspect_transaction_stage_state(self, name)
    }

    /// Remove one exact direct child through its held native handle and
    /// return the host's truthful namespace-persistence evidence.
    pub fn remove_child_expected(
        &self,
        name: &str,
        expected: &EntryState,
    ) -> std::result::Result<DirectoryDurability, OwnedTreeCleanupError> {
        platform::remove_expected(self, name, expected).map_err(|error| match error {
            platform::NativeRemoveError::Changed(detail) => OwnedTreeCleanupError::Third { detail },
            platform::NativeRemoveError::Io(error) => {
                OwnedTreeCleanupError::Io(anyhow::Error::new(error))
            }
            #[cfg(not(windows))]
            platform::NativeRemoveError::Unsupported => OwnedTreeCleanupError::Unsupported,
        })
    }

    /// Exclusively create one direct child for a journaled namespace intent.
    /// The returned capability is the object created by the syscall, never a
    /// name-based reopen; persistence evidence remains journal-recoverable.
    pub fn create_child_exclusive_journaled(
        &self,
        name: &str,
    ) -> std::result::Result<(Pinned, DirectoryDurability), OwnedDirectoryCreateError> {
        crate::ensure_safe_component(name).map_err(OwnedDirectoryCreateError::NotCreated)?;
        let path = self.join(name);
        let (dir, durability) =
            platform::create_directory(self, name).map_err(|error| match error {
                platform::NativeCreateError::NotCreated(error) => {
                    OwnedDirectoryCreateError::NotCreated(anyhow::Error::new(error))
                }
                platform::NativeCreateError::CreatedButUnsealed(error) => {
                    OwnedDirectoryCreateError::CreatedButUnsealed {
                        path: path.clone(),
                        source: anyhow::Error::new(error),
                    }
                }
                #[cfg(not(windows))]
                platform::NativeCreateError::Unsupported => OwnedDirectoryCreateError::Unsupported,
            })?;
        Ok((Pinned { dir, path }, durability))
    }

    /// Capability-relative, source-state-guarded, atomic no-replace move for
    /// either a file or a directory. For a directory this proves only the root
    /// entry; it never claims manifest-bound tree publication. Scrape export
    /// must use [`OwnedDirectory::publish_noreplace_to`].
    pub fn rename_child_to(
        &self,
        destination: &Pinned,
        source_name: &str,
        destination_name: &str,
        expected: &EntryState,
    ) -> Result<(), RenameError> {
        self.rename_child_noreplace_to(destination, source_name, destination_name, expected)
    }

    /// The explicit publication spelling of [`Self::rename_child_to`].  On
    /// Epoch-1 execution is Windows-only and uses a source handle plus native
    /// no-replace rename. Other platforms return `Unsupported`; Linux's
    /// `renameat2` helper remains an explicitly partial future primitive.
    pub fn rename_child_noreplace_to(
        &self,
        destination: &Pinned,
        source_name: &str,
        destination_name: &str,
        expected: &EntryState,
    ) -> Result<(), RenameError> {
        self.rename_child_noreplace_to_durable(destination, source_name, destination_name, expected)
            .map(|_| ())
    }

    /// The same handle-relative no-replace rename, retaining truthful
    /// namespace-persistence evidence for a surrounding WAL transaction.
    pub fn rename_child_noreplace_to_durable(
        &self,
        destination: &Pinned,
        source_name: &str,
        destination_name: &str,
        expected: &EntryState,
    ) -> Result<DirectoryDurability, RenameError> {
        crate::ensure_safe_component(source_name).map_err(RenameError::Failed)?;
        crate::ensure_safe_component(destination_name).map_err(RenameError::Failed)?;
        if !self
            .same_filesystem(destination)
            .map_err(RenameError::Failed)?
        {
            return Err(RenameError::CrossFilesystem);
        }
        require_expected_source(self, source_name, expected)?;
        rename_hook::before(self, destination, source_name, destination_name);
        // Recheck after the deterministic race seam.  The syscall itself is
        // still the no-replace authority for the destination name.
        require_expected_source(self, source_name, expected)?;
        final_rename_hook::after(self, destination, source_name, destination_name);
        let durability =
            platform::rename_noreplace(self, destination, source_name, destination_name, expected)
                .map_err(|error| match error {
                    platform::NoReplaceError::Occupied => RenameError::Occupied {
                        path: destination.join(destination_name),
                    },
                    platform::NoReplaceError::SourceChanged => RenameError::SourceChanged {
                        path: self.join(source_name),
                        detail: "native source handle did not match the expected state".to_owned(),
                    },
                    platform::NoReplaceError::SourceReappeared => RenameError::PossiblyMoved {
                        source: self.join(source_name),
                        destination: destination.join(destination_name),
                        detail: "source name was concurrently recreated after rename".to_owned(),
                    },
                    platform::NoReplaceError::CrossFilesystem => RenameError::CrossFilesystem,
                    platform::NoReplaceError::Unsupported => RenameError::Unsupported,
                    platform::NoReplaceError::Io(error) => {
                        RenameError::Failed(anyhow::Error::new(error).context(format!(
                            "renaming `{}` to `{}` without replacement",
                            self.join(source_name).display(),
                            destination.join(destination_name).display()
                        )))
                    }
                })?;
        match destination.inspect_child_state(destination_name) {
            Ok(Some(actual)) if &actual == expected => Ok(durability),
            Ok(Some(_)) => Err(RenameError::PossiblyMoved {
                source: self.join(source_name),
                destination: destination.join(destination_name),
                detail: "destination identity/content differs from the expected source".to_owned(),
            }),
            Ok(None) => Err(RenameError::PossiblyMoved {
                source: self.join(source_name),
                destination: destination.join(destination_name),
                detail: "destination is absent after the rename primitive reported success"
                    .to_owned(),
            }),
            Err(error) => Err(RenameError::PossiblyMoved {
                source: self.join(source_name),
                destination: destination.join(destination_name),
                detail: format!("destination cannot be re-observed: {error:#}"),
            }),
        }
    }
}

fn require_expected_source(
    source: &Pinned,
    name: &str,
    expected: &EntryState,
) -> Result<(), RenameError> {
    match source.inspect_child_state(name) {
        Ok(Some(actual)) if &actual == expected => Ok(()),
        Ok(Some(_)) => Err(RenameError::SourceChanged {
            path: source.join(name),
            detail: "the name holds a different object or state".to_owned(),
        }),
        Ok(None) => Err(RenameError::SourceChanged {
            path: source.join(name),
            detail: "the expected entry is absent".to_owned(),
        }),
        Err(error) => Err(RenameError::SourceChanged {
            path: source.join(name),
            detail: format!("{error:#}"),
        }),
    }
}

pub(crate) fn identity_token(domain: &[u8], identity: FileIdentity) -> String {
    let mut hash = Sha256::new();
    hash.update(domain);
    hash.update(identity.identity_bytes());
    format!("sha256:{:x}", hash.finalize())
}

pub(crate) fn project_view(root: &Pinned) -> Result<Project> {
    Ok(Project {
        root: root
            .dir
            .try_clone()
            .with_context(|| format!("retaining `{}`", root.path().display()))?,
        root_path: root.path().to_path_buf(),
        ancestor_identities: vec![root.identity()?],
    })
}

pub(crate) fn sync_directory(directory: &Pinned) -> DirectoryDurability {
    match directory
        .dir
        .try_clone()
        .and_then(|handle| handle.into_std_file().sync_all())
    {
        Ok(()) => DirectoryDurability::Synced,
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::Unsupported | std::io::ErrorKind::PermissionDenied
            ) =>
        {
            DirectoryDurability::Unsupported(error.kind())
        }
        Err(error) => DirectoryDurability::Failed(error.kind()),
    }
}

/// Classify an otherwise unsupported Windows directory flush after an atomic
/// capability-relative namespace operation. This is not a durability upgrade:
/// callers may accept it only when a prior WAL makes before/after replay exact,
/// or before the initial journal while failure can strand liveness-only
/// external state but cannot accompany a product mutation.
pub(crate) fn journal_recoverable_checkpoint(
    durability: DirectoryDurability,
) -> DirectoryDurability {
    #[cfg(windows)]
    if matches!(
        durability,
        DirectoryDurability::Unsupported(
            std::io::ErrorKind::Unsupported | std::io::ErrorKind::PermissionDenied
        )
    ) {
        return DirectoryDurability::JournalRecoverable;
    }
    durability
}

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
