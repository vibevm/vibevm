//! Complete no-follow manifests and identity-bound owned-tree cleanup.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#SEC-NO-FOLLOW");

use anyhow::{Result, bail};
use sha2::{Digest as _, Sha256};

use crate::file::identity::FileIdentity;
use crate::{Pinned, Project};

use super::{DirectoryDurability, identity_token, project_view, sync_directory};

/// Opaque identity of one manifest entry.  The token is a domain-separated
/// digest of the OS identity; raw device, volume and inode/index values never
/// leave the crate.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntryIdentity(String);

impl EntryIdentity {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn from_token(token: &str) -> Result<Self> {
        validate_identity_token(token)?;
        Ok(Self(token.to_owned()))
    }
}

impl std::fmt::Debug for EntryIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EntryIdentity(..)")
    }
}

/// Opaque identity seal for an exclusively created owned directory.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct OwnedDirectoryIdentity(String);

impl OwnedDirectoryIdentity {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn from_token(token: &str) -> Result<Self> {
        validate_identity_token(token)?;
        Ok(Self(token.to_owned()))
    }
}

impl std::fmt::Debug for OwnedDirectoryIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OwnedDirectoryIdentity(..)")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryStateKind {
    File,
    Directory,
}

/// Complete expected state for one direct rename operand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryState {
    pub kind: EntryStateKind,
    pub sha256: Option<String>,
    pub bytes: Option<u64>,
    pub unix_mode: Option<u32>,
    pub identity: EntryIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeEntry {
    /// Forward-slashed path relative to the owned root.
    pub path: String,
    pub state: EntryState,
}

/// Complete canonical descendant set.  `digest` commits to paths, kinds,
/// contents, sizes, modes and opaque object identities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeManifest {
    pub digest: String,
    pub entries: Vec<TreeEntry>,
}

impl TreeManifest {
    /// Reconstruct journaled manifest evidence only when its complete shape,
    /// opaque identity tokens, canonical order and aggregate digest agree.
    pub fn from_persisted(digest: String, entries: Vec<TreeEntry>) -> Result<Self> {
        let manifest = Self { digest, entries };
        manifest.validate_persisted_mode(None)?;
        Ok(manifest)
    }

    /// Reconstruct a journaled manifest containing exactly one deterministic
    /// transaction stage selected by its full relative path. Generic persisted
    /// manifests continue to reject the reserved stage namespace.
    pub fn from_persisted_with_transaction_stage(
        digest: String,
        entries: Vec<TreeEntry>,
        authorized_stage_path: &str,
    ) -> Result<Self> {
        validate_authorized_transaction_stage_path(authorized_stage_path)?;
        let manifest = Self { digest, entries };
        manifest.validate_persisted_mode(Some(authorized_stage_path))?;
        Ok(manifest)
    }

    pub fn validate_persisted(&self) -> Result<()> {
        self.validate_persisted_mode(None)
    }

    fn validate_persisted_mode(&self, authorized_stage_path: Option<&str>) -> Result<()> {
        validate_identity_token(&self.digest)?;
        let mut previous: Option<&str> = None;
        let mut authorized_stage_found = false;
        for entry in &self.entries {
            if authorized_stage_path == Some(entry.path.as_str()) {
                if authorized_stage_found {
                    bail!("persisted manifest repeats its journal-authorized transaction stage");
                }
                validate_authorized_transaction_stage_path(&entry.path)?;
                authorized_stage_found = true;
            } else {
                crate::split_relative(&entry.path)?;
            }
            if previous.is_some_and(|prior| prior.as_bytes() >= entry.path.as_bytes()) {
                bail!("persisted manifest paths are not unique and byte-sorted");
            }
            previous = Some(&entry.path);
            validate_identity_token(entry.state.identity.as_str())?;
            if entry.state.unix_mode.is_some_and(|mode| mode > 0o7777) {
                bail!("persisted manifest contains an invalid Unix mode");
            }
            match entry.state.kind {
                EntryStateKind::File => {
                    let Some(digest) = entry.state.sha256.as_deref() else {
                        bail!("persisted manifest file lacks SHA-256");
                    };
                    if entry.state.bytes.is_none() || !is_raw_sha256(digest) {
                        bail!("persisted manifest file has invalid size or SHA-256");
                    }
                }
                EntryStateKind::Directory => {
                    if entry.state.sha256.is_some() || entry.state.bytes.is_some() {
                        bail!("persisted manifest directory carries file evidence");
                    }
                }
            }
        }
        if self.digest != manifest_digest(&self.entries) {
            bail!("persisted manifest aggregate digest does not match its entries");
        }
        if authorized_stage_path.is_some() && !authorized_stage_found {
            bail!("persisted manifest lacks its journal-authorized transaction stage");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnedTreeObservation {
    Absent,
    MatchesAtObservation(TreeManifest),
    Third { detail: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedTreeCleanupProgress {
    completed: Vec<String>,
}

impl OwnedTreeCleanupProgress {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            completed: Vec::new(),
        }
    }

    pub fn from_completed(completed: Vec<String>) -> Result<Self> {
        if completed.iter().any(|key| key.is_empty()) {
            bail!("cleanup progress contains an empty key");
        }
        Ok(Self { completed })
    }

    #[must_use]
    pub fn completed(&self) -> &[String] {
        &self.completed
    }

    pub fn record(&mut self, completion: &CleanupCompletion) -> Result<()> {
        self.completed.push(completion.progress_key.clone());
        Ok(())
    }
}

impl Default for OwnedTreeCleanupProgress {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CleanupPreparation {
    Intent(CleanupIntent),
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupIntent {
    pub intent_token: String,
    pub progress_key: String,
    pub path: String,
    pub expected: EntryState,
    pub root: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupCompletion {
    pub progress_key: String,
    pub path: String,
    pub parent: DirectoryDurability,
    pub recovered_after_syscall: bool,
}

impl CleanupCompletion {
    #[must_use]
    pub fn progress_key(&self) -> &str {
        &self.progress_key
    }

    #[must_use]
    pub const fn durability(&self) -> DirectoryDurability {
        self.parent
    }
}

#[derive(Debug)]
pub enum OwnedTreeCleanupError {
    Third { detail: String },
    Io(anyhow::Error),
    Unsupported,
}

impl std::fmt::Display for OwnedTreeCleanupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Third { detail } => write!(f, "owned tree is a third state: {detail}"),
            Self::Io(error) => write!(f, "owned-tree cleanup I/O failed: {error:#}"),
            Self::Unsupported => {
                f.write_str("identity-bound by-handle tree removal is unsupported")
            }
        }
    }
}

impl std::error::Error for OwnedTreeCleanupError {}

#[derive(Debug)]
pub enum OwnedDirectoryCreateError {
    NotCreated(anyhow::Error),
    CreatedButUnsealed {
        path: std::path::PathBuf,
        source: anyhow::Error,
    },
    Unsupported,
}

impl std::fmt::Display for OwnedDirectoryCreateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotCreated(error) => write!(f, "{error:#}"),
            Self::CreatedButUnsealed { path, source } => write!(
                f,
                "created `{}` but could not seal its identity: {source:#}",
                path.display()
            ),
            Self::Unsupported => f.write_str(
                "strong create-and-hold directory ownership is unsupported on this platform",
            ),
        }
    }
}

impl std::error::Error for OwnedDirectoryCreateError {}

#[derive(Debug)]
pub enum ReopenOwnedDirectoryError {
    InvalidPersisted(anyhow::Error),
    Third { detail: String },
    Io(anyhow::Error),
    Unsupported,
}

impl std::fmt::Display for ReopenOwnedDirectoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPersisted(error) => {
                write!(f, "invalid persisted ownership evidence: {error:#}")
            }
            Self::Third { detail } => write!(f, "owned directory is a third state: {detail}"),
            Self::Io(error) => write!(f, "reopening owned directory failed: {error:#}"),
            Self::Unsupported => f.write_str("owned-directory recovery rebind is Windows-only"),
        }
    }
}

impl std::error::Error for ReopenOwnedDirectoryError {}

/// A just-created sibling directory whose handle and namespace identity have
/// both been pinned.
#[derive(Debug)]
pub struct OwnedDirectory {
    parent: Pinned,
    name: String,
    directory: Pinned,
    identity: OwnedDirectoryIdentity,
    parent_durability: DirectoryDurability,
}

#[derive(Debug)]
pub struct ReopenedOwnedDirectory {
    owned: OwnedDirectory,
    pub entry_lease: ExistingTreeEntryLease,
}

impl ReopenedOwnedDirectory {
    #[must_use]
    pub fn identity(&self) -> &OwnedDirectoryIdentity {
        self.owned.identity()
    }

    #[must_use]
    pub fn manifest(&self) -> &TreeManifest {
        self.entry_lease.manifest()
    }

    pub fn directory(&self) -> Result<Pinned> {
        self.owned.directory()
    }

    #[must_use]
    pub fn into_parts(self) -> (OwnedDirectory, ExistingTreeEntryLease) {
        (self.owned, self.entry_lease)
    }
}

#[derive(Debug)]
pub struct ExistingTreeEntryLease {
    manifest: TreeManifest,
    identity: OwnedDirectoryIdentity,
    root_state: EntryState,
    _handles: Vec<std::fs::File>,
}

impl ExistingTreeEntryLease {
    #[must_use]
    pub fn manifest(&self) -> &TreeManifest {
        &self.manifest
    }

    #[must_use]
    pub fn identity(&self) -> &OwnedDirectoryIdentity {
        &self.identity
    }
}

#[derive(Debug)]
pub struct PublishedPendingVerification {
    /// Holds existing entry identities/bytes stable. It does not seal
    /// directory membership; callers must still perform final re-observation.
    pub entry_lease: ExistingTreeEntryLease,
    destination_parent_capability: Pinned,
    destination_name: String,
    pub source_parent: DirectoryDurability,
    pub destination_parent: DirectoryDurability,
}

impl PublishedPendingVerification {
    /// Re-enumerate the complete destination after health and immediately
    /// before the transaction's durable `Verified` transition. This
    /// point-in-time observation—not the entry lease—is the membership
    /// evidence. A transaction that cannot place its transition immediately
    /// after this call must re-observe again.
    pub fn reobserve_published(
        &self,
        expected_identity: &OwnedDirectoryIdentity,
        expected_manifest: &TreeManifest,
    ) -> Result<OwnedTreeObservation> {
        if self.entry_lease.identity() != expected_identity {
            return Ok(OwnedTreeObservation::Third {
                detail: "pending publication is bound to a different root identity".to_owned(),
            });
        }
        if self.entry_lease.manifest() != expected_manifest {
            return Ok(OwnedTreeObservation::Third {
                detail:
                    "caller manifest differs from the manifest sealed into the pending publication"
                        .to_owned(),
            });
        }
        let directory = match self
            .destination_parent_capability
            .open_child_checked(&self.destination_name)
        {
            Ok(Some(directory)) => directory,
            Ok(None) => return Ok(OwnedTreeObservation::Absent),
            Err(error) => {
                return Ok(OwnedTreeObservation::Third {
                    detail: format!("published root cannot be reopened no-follow: {error:#}"),
                });
            }
        };
        if directory_state(&directory)?.identity != self.entry_lease.root_state.identity {
            return Ok(OwnedTreeObservation::Third {
                detail: "published root identity changed".to_owned(),
            });
        }
        let actual = match manifest(&directory) {
            Ok(actual) => actual,
            Err(error) => {
                return Ok(OwnedTreeObservation::Third {
                    detail: format!("published tree cannot be fully re-observed: {error:#}"),
                });
            }
        };
        if actual == *expected_manifest {
            Ok(OwnedTreeObservation::MatchesAtObservation(actual))
        } else {
            Ok(OwnedTreeObservation::Third {
                detail: manifest_difference(expected_manifest, &actual),
            })
        }
    }
}

#[derive(Debug)]
pub enum OwnedTreePublishError {
    BeforeMove {
        detail: String,
    },
    Occupied {
        path: std::path::PathBuf,
    },
    Unsupported,
    PossiblyMoved {
        source_identity: OwnedDirectoryIdentity,
        destination_identity: Option<OwnedDirectoryIdentity>,
        detail: String,
        destination_entry_lease: Option<Box<ExistingTreeEntryLease>>,
    },
}

impl std::fmt::Display for OwnedTreePublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BeforeMove { detail } => write!(f, "owned tree was not published: {detail}"),
            Self::Occupied { path } => {
                write!(f, "publication target `{}` is occupied", path.display())
            }
            Self::Unsupported => f.write_str("owned-tree publication is unsupported"),
            Self::PossiblyMoved { detail, .. } => {
                write!(
                    f,
                    "owned tree may have moved and requires compensation: {detail}"
                )
            }
        }
    }
}

impl std::error::Error for OwnedTreePublishError {}

impl OwnedDirectory {
    #[must_use]
    pub fn path(&self) -> &std::path::Path {
        self.directory.path()
    }

    #[must_use]
    pub fn identity(&self) -> &OwnedDirectoryIdentity {
        &self.identity
    }

    #[must_use]
    pub const fn parent_durability(&self) -> DirectoryDurability {
        self.parent_durability
    }

    pub fn directory(&self) -> Result<Pinned> {
        self.directory.shallow_clone()
    }

    pub fn manifest(&self) -> Result<TreeManifest> {
        manifest(&self.directory)
    }

    /// Hold identities/bytes of every currently existing entry stable on
    /// Windows, then repeat the complete manifest observation. Directory
    /// membership is deliberately not claimed immutable: Windows directory
    /// handles cannot prevent creation of a new child.
    pub fn lease_existing_entries(&self) -> Result<ExistingTreeEntryLease> {
        self.lease_existing_entries_mode(false)
    }

    /// Hold a complete owned-tree manifest that contains exactly the one
    /// deterministic transaction stage authorized by the caller's durable
    /// journal.  No other reserved stage spelling is admitted.
    pub fn lease_existing_entries_with_transaction_stage(
        &self,
        authorized_stage_path: &str,
    ) -> Result<ExistingTreeEntryLease> {
        validate_authorized_transaction_stage_path(authorized_stage_path)?;
        let lease = self.lease_existing_entries_mode(true)?;
        let mut found = false;
        for entry in &lease.manifest.entries {
            let name = entry
                .path
                .rsplit_once('/')
                .map_or(entry.path.as_str(), |(_, name)| name);
            if is_transaction_stage_name(name) {
                if found || entry.path != authorized_stage_path {
                    bail!(
                        "owned tree contains a transaction stage other than the journal-authorized `{authorized_stage_path}`"
                    );
                }
                found = true;
            }
        }
        if !found {
            bail!(
                "journal-authorized transaction stage `{authorized_stage_path}` is absent from the owned tree"
            );
        }
        Ok(lease)
    }

    fn lease_existing_entries_mode(
        &self,
        allow_transaction_stage: bool,
    ) -> Result<ExistingTreeEntryLease> {
        let first = manifest_mode(&self.directory, allow_transaction_stage)?;
        let root_state = directory_state(&self.directory)?;
        let mut handles = vec![super::platform::lease_entry(
            &self.parent,
            &self.name,
            &root_state,
        )?];
        let view = project_view(&self.directory)?;
        for entry in &first.entries {
            let (parent, name) = holder(&view, &entry.path)?;
            handles.push(super::platform::lease_entry(&parent, &name, &entry.state)?);
        }
        lease_hook::during(&self.directory);
        let second = manifest_mode(&self.directory, allow_transaction_stage)?;
        if first != second {
            bail!("tree changed while acquiring its manifest lease");
        }
        Ok(ExistingTreeEntryLease {
            manifest: second,
            identity: self.identity.clone(),
            root_state,
            _handles: handles,
        })
    }

    /// Move this owned directory into its publication slot, returning only a
    /// pending-verification state. The returned entry lease stabilizes entries
    /// that existed at the post-move observation point, not membership.
    pub fn publish_noreplace_to(
        self,
        destination: &Pinned,
        destination_name: &str,
        ownership_token: &str,
        expected_manifest: &TreeManifest,
        source_lease: ExistingTreeEntryLease,
    ) -> std::result::Result<PublishedPendingVerification, OwnedTreePublishError> {
        if source_lease.identity != self.identity || source_lease.manifest != *expected_manifest {
            return Err(OwnedTreePublishError::BeforeMove {
                detail: "source lease is not bound to the expected identity/manifest".to_owned(),
            });
        }
        let expected_owned = owned_identity(
            ownership_token,
            self.directory
                .identity()
                .map_err(|error| OwnedTreePublishError::BeforeMove {
                    detail: format!("source identity cannot be rechecked: {error:#}"),
                })?,
        );
        if expected_owned != self.identity {
            return Err(OwnedTreePublishError::BeforeMove {
                detail: "ownership token does not bind the source directory".to_owned(),
            });
        }
        let source_identity = self.identity.clone();
        let source_parent = self.parent;
        let source_name = self.name;
        let root_state = source_lease.root_state.clone();
        drop(source_lease);
        drop(self.directory);
        publish_hook::before_move(&source_parent, &source_name);
        let rename_durability = match source_parent.rename_child_noreplace_to_durable(
            destination,
            &source_name,
            destination_name,
            &root_state,
        ) {
            Ok(durability) => durability,
            Err(crate::RenameError::Occupied { path }) => {
                return Err(OwnedTreePublishError::Occupied { path });
            }
            Err(crate::RenameError::Unsupported) => {
                return Err(OwnedTreePublishError::Unsupported);
            }
            Err(crate::RenameError::PossiblyMoved { detail, .. }) => {
                return Err(OwnedTreePublishError::PossiblyMoved {
                    source_identity,
                    destination_identity: None,
                    detail,
                    destination_entry_lease: None,
                });
            }
            Err(error) => {
                return Err(OwnedTreePublishError::BeforeMove {
                    detail: error.to_string(),
                });
            }
        };
        publish_hook::after_move(destination, destination_name);
        let published_dir = match destination.open_child(destination_name) {
            Ok(directory) => directory,
            Err(error) => {
                return Err(OwnedTreePublishError::PossiblyMoved {
                    source_identity,
                    destination_identity: None,
                    detail: format!("destination cannot be pinned after rename: {error:#}"),
                    destination_entry_lease: None,
                });
            }
        };
        let destination_identity = owned_identity(
            ownership_token,
            published_dir
                .identity()
                .map_err(|error| OwnedTreePublishError::PossiblyMoved {
                    source_identity: source_identity.clone(),
                    destination_identity: None,
                    detail: format!("destination identity cannot be read: {error:#}"),
                    destination_entry_lease: None,
                })?,
        );
        if destination_identity != source_identity {
            return Err(OwnedTreePublishError::PossiblyMoved {
                source_identity,
                destination_identity: Some(destination_identity),
                detail: "destination name does not hold the moved owned root".to_owned(),
                destination_entry_lease: None,
            });
        }
        let published = OwnedDirectory {
            parent: destination.shallow_clone().map_err(|error| {
                OwnedTreePublishError::PossiblyMoved {
                    source_identity: source_identity.clone(),
                    destination_identity: Some(destination_identity.clone()),
                    detail: format!("destination parent cannot be retained: {error:#}"),
                    destination_entry_lease: None,
                }
            })?,
            name: destination_name.to_owned(),
            directory: published_dir,
            identity: destination_identity.clone(),
            parent_durability: rename_durability,
        };
        let lease = published.lease_existing_entries().map_err(|error| {
            OwnedTreePublishError::PossiblyMoved {
                source_identity: source_identity.clone(),
                destination_identity: Some(destination_identity.clone()),
                detail: format!("destination could not be sealed: {error:#}"),
                destination_entry_lease: None,
            }
        })?;
        if lease.manifest != *expected_manifest {
            return Err(OwnedTreePublishError::PossiblyMoved {
                source_identity,
                destination_identity: Some(destination_identity),
                detail: manifest_difference(expected_manifest, &lease.manifest),
                destination_entry_lease: Some(Box::new(lease)),
            });
        }
        Ok(PublishedPendingVerification {
            entry_lease: lease,
            destination_parent_capability: destination.shallow_clone().map_err(|error| {
                OwnedTreePublishError::PossiblyMoved {
                    source_identity: source_identity.clone(),
                    destination_identity: Some(destination_identity.clone()),
                    detail: format!("destination parent cannot be retained: {error:#}"),
                    destination_entry_lease: None,
                }
            })?,
            destination_name: destination_name.to_owned(),
            source_parent: rename_durability,
            destination_parent: rename_durability,
        })
    }
}

impl Pinned {
    /// Rebind an owned root after restart using only the identity that was
    /// durable before the interrupted mutation. The returned lease contains a
    /// fresh complete manifest; callers must accept it only as one of the
    /// finite before/after states authorized by their durable intent.
    pub fn reopen_owned_child_by_identity(
        &self,
        name: &str,
        ownership_token: &str,
        persisted_identity: &OwnedDirectoryIdentity,
    ) -> std::result::Result<ReopenedOwnedDirectory, ReopenOwnedDirectoryError> {
        #[cfg(not(windows))]
        {
            let _ = (name, ownership_token, persisted_identity);
            return Err(ReopenOwnedDirectoryError::Unsupported);
        }
        #[cfg(windows)]
        {
            crate::ensure_safe_component(name)
                .map_err(ReopenOwnedDirectoryError::InvalidPersisted)?;
            if ownership_token.is_empty() {
                return Err(ReopenOwnedDirectoryError::InvalidPersisted(
                    anyhow::anyhow!("ownership token is empty"),
                ));
            }
            validate_identity_token(persisted_identity.as_str())
                .map_err(ReopenOwnedDirectoryError::InvalidPersisted)?;
            let metadata = match self.dir.symlink_metadata(name) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    return Err(ReopenOwnedDirectoryError::Third {
                        detail: "journaled owned directory is absent".to_owned(),
                    });
                }
                Err(error) => {
                    return Err(ReopenOwnedDirectoryError::Io(anyhow::Error::new(error)));
                }
            };
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(ReopenOwnedDirectoryError::Third {
                    detail: "journaled owned name is now a link, file, or special entry".to_owned(),
                });
            }
            let parent = self
                .shallow_clone()
                .map_err(ReopenOwnedDirectoryError::Io)?;
            let directory = self
                .open_child(name)
                .map_err(ReopenOwnedDirectoryError::Io)?;
            let actual_identity = owned_identity(
                ownership_token,
                directory
                    .identity()
                    .map_err(ReopenOwnedDirectoryError::Io)?,
            );
            if actual_identity != *persisted_identity {
                return Err(ReopenOwnedDirectoryError::Third {
                    detail: "current root identity differs from the journaled owned root"
                        .to_owned(),
                });
            }
            let owned = OwnedDirectory {
                parent,
                name: name.to_owned(),
                directory,
                identity: actual_identity,
                parent_durability: sync_directory(self),
            };
            let entry_lease = owned.lease_existing_entries_mode(true).map_err(|error| {
                if error
                    .chain()
                    .any(|cause| cause.downcast_ref::<std::io::Error>().is_some())
                {
                    ReopenOwnedDirectoryError::Io(error)
                } else {
                    ReopenOwnedDirectoryError::Third {
                        detail: format!(
                            "current owned tree cannot be completely sealed: {error:#}"
                        ),
                    }
                }
            })?;
            Ok(ReopenedOwnedDirectory { owned, entry_lease })
        }
    }

    /// Rebind a journaled candidate/quarantine after process restart without
    /// adopting whatever merely occupies its old spelling. The current root
    /// identity and complete descendant observation must equal the persisted
    /// opaque evidence before handles are returned.
    pub fn reopen_owned_child(
        &self,
        name: &str,
        ownership_token: &str,
        persisted_identity: &OwnedDirectoryIdentity,
        persisted_manifest: &TreeManifest,
    ) -> std::result::Result<ReopenedOwnedDirectory, ReopenOwnedDirectoryError> {
        persisted_manifest
            .validate_persisted()
            .map_err(ReopenOwnedDirectoryError::InvalidPersisted)?;
        let reopened =
            self.reopen_owned_child_by_identity(name, ownership_token, persisted_identity)?;
        if reopened.manifest() != persisted_manifest {
            return Err(ReopenOwnedDirectoryError::Third {
                detail: manifest_difference(persisted_manifest, reopened.manifest()),
            });
        }
        Ok(reopened)
    }

    /// Exclusively create one direct child and seal its identity with the
    /// caller's already-durable ownership token.
    pub fn create_owned_child_exclusive(
        &self,
        name: &str,
        ownership_token: &str,
    ) -> std::result::Result<OwnedDirectory, OwnedDirectoryCreateError> {
        if ownership_token.is_empty() {
            return Err(OwnedDirectoryCreateError::NotCreated(anyhow::anyhow!(
                "ownership token must be durable and non-empty before directory creation"
            )));
        }
        crate::ensure_safe_component(name).map_err(OwnedDirectoryCreateError::NotCreated)?;
        let parent = self
            .shallow_clone()
            .map_err(OwnedDirectoryCreateError::NotCreated)?;
        let path = self.join(name);
        let (dir, parent_durability) =
            super::platform::create_directory(self, name).map_err(|error| match error {
                super::platform::NativeCreateError::NotCreated(error) => {
                    OwnedDirectoryCreateError::NotCreated(anyhow::Error::new(error))
                }
                super::platform::NativeCreateError::CreatedButUnsealed(error) => {
                    OwnedDirectoryCreateError::CreatedButUnsealed {
                        path: path.clone(),
                        source: anyhow::Error::new(error),
                    }
                }
                #[cfg(not(windows))]
                super::platform::NativeCreateError::Unsupported => {
                    OwnedDirectoryCreateError::Unsupported
                }
            })?;
        let directory = Pinned { dir, path };
        let raw = directory.identity().map_err(|source| {
            OwnedDirectoryCreateError::CreatedButUnsealed {
                path: directory.path().to_path_buf(),
                source,
            }
        })?;
        let identity = owned_identity(ownership_token, raw);
        Ok(OwnedDirectory {
            parent,
            name: name.to_owned(),
            directory,
            identity,
            parent_durability,
        })
    }

    /// Re-observe an owned child by its current namespace name. A matching
    /// result is explicitly point-in-time membership evidence; the supplied
    /// lease stabilizes only entries already present. Link/special/hardlink/
    /// walk errors are never collapsed to absence.
    pub fn observe_owned_tree(
        &self,
        name: &str,
        ownership_token: &str,
        expected_identity: &OwnedDirectoryIdentity,
        expected_manifest: &TreeManifest,
        lease: &ExistingTreeEntryLease,
    ) -> Result<OwnedTreeObservation> {
        crate::ensure_safe_component(name)?;
        if lease.identity() != expected_identity || lease.manifest() != expected_manifest {
            return Ok(OwnedTreeObservation::Third {
                detail: "manifest lease is not bound to the expected owned tree".to_owned(),
            });
        }
        tree_hook::before(self, name);
        let directory = match self.open_child_checked(name) {
            Ok(Some(directory)) => directory,
            Ok(None) => return Ok(OwnedTreeObservation::Absent),
            Err(error) => {
                return Ok(OwnedTreeObservation::Third {
                    detail: format!("owned root no longer opens no-follow: {error:#}"),
                });
            }
        };
        let actual_identity = match directory.identity() {
            Ok(raw) => owned_identity(ownership_token, raw),
            Err(error) => {
                return Ok(OwnedTreeObservation::Third {
                    detail: format!("owned root identity cannot be rechecked: {error:#}"),
                });
            }
        };
        if &actual_identity != expected_identity {
            return Ok(OwnedTreeObservation::Third {
                detail: "owned root name now denotes a different directory".to_owned(),
            });
        }
        let actual = match manifest_mode(
            &directory,
            manifest_has_transaction_stage(expected_manifest),
        ) {
            Ok(actual) => actual,
            Err(error) => {
                return Ok(OwnedTreeObservation::Third {
                    detail: format!(
                        "owned descendant set is not a complete ordinary tree: {error:#}"
                    ),
                });
            }
        };
        if &actual == expected_manifest {
            Ok(OwnedTreeObservation::MatchesAtObservation(actual))
        } else {
            Ok(OwnedTreeObservation::Third {
                detail: manifest_difference(expected_manifest, &actual),
            })
        }
    }

    /// Advance exact owned-tree cleanup by at most one by-handle removal.
    ///
    /// The caller durably records the returned `progress_key` before calling
    /// again. Absence is accepted only for that canonical completed prefix;
    /// a crash after deletion but before the record is therefore an explicit
    /// third state rather than guessed success. Every successful mutation
    /// carries its parent-directory durability result.
    pub fn prepare_owned_tree_cleanup_next(
        &self,
        name: &str,
        ownership_token: &str,
        expected_identity: &OwnedDirectoryIdentity,
        expected_manifest: &TreeManifest,
        progress: &OwnedTreeCleanupProgress,
    ) -> std::result::Result<CleanupPreparation, OwnedTreeCleanupError> {
        crate::ensure_safe_component(name).map_err(OwnedTreeCleanupError::Io)?;
        tree_hook::before(self, name);
        let order = cleanup_order(expected_manifest);
        if progress.completed.len() > order.len()
            || progress
                .completed
                .iter()
                .zip(&order)
                .any(|(completed, expected)| completed != expected)
        {
            return Err(third_error(
                "durable cleanup progress is not the canonical prefix".to_owned(),
            ));
        }
        if progress.completed.len() == order.len() {
            return match self.dir.symlink_metadata(name) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    Ok(CleanupPreparation::Complete)
                }
                Ok(_) => Err(third_error("completed owned root reappeared".to_owned())),
                Err(error) => Err(OwnedTreeCleanupError::Io(anyhow::Error::new(error))),
            };
        }

        let root_metadata = match self.dir.symlink_metadata(name) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(third_error(
                    "owned root is absent before its durable step".into(),
                ));
            }
            Err(error) => return Err(OwnedTreeCleanupError::Io(anyhow::Error::new(error))),
        };
        if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
            return Err(third_error(
                "owned root name is now a link, file, or special entry".to_owned(),
            ));
        }
        let root = self.open_child(name).map_err(OwnedTreeCleanupError::Io)?;
        let root_raw = root.identity().map_err(OwnedTreeCleanupError::Io)?;
        if owned_identity(ownership_token, root_raw) != *expected_identity {
            return Err(third_error("owned root identity changed".to_owned()));
        }
        let actual = manifest_mode(&root, manifest_has_transaction_stage(expected_manifest))
            .map_err(classify_manifest_cleanup_error)?;
        let completed = progress
            .completed
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        let remaining = expected_manifest
            .entries
            .iter()
            .filter(|entry| !completed.contains(&entry_key(entry)))
            .cloned()
            .collect::<Vec<_>>();
        if actual.entries != remaining {
            return Err(third_error(manifest_entries_difference(
                &remaining,
                &actual.entries,
            )));
        }

        let next_key = &order[progress.completed.len()];
        if next_key == "root" {
            let root_state = EntryState {
                kind: EntryStateKind::Directory,
                sha256: None,
                bytes: None,
                unix_mode: root.unix_mode().map_err(OwnedTreeCleanupError::Io)?,
                identity: entry_identity(root_raw),
            };
            return Ok(CleanupPreparation::Intent(CleanupIntent {
                intent_token: cleanup_intent_token(expected_identity, expected_manifest, next_key),
                progress_key: next_key.clone(),
                path: name.to_owned(),
                expected: root_state,
                root: true,
            }));
        }

        let entry = expected_manifest
            .entries
            .iter()
            .find(|entry| entry_key(entry) == *next_key)
            .ok_or_else(|| {
                third_error("cleanup order names an absent manifest entry".to_owned())
            })?;
        Ok(CleanupPreparation::Intent(CleanupIntent {
            intent_token: cleanup_intent_token(expected_identity, expected_manifest, next_key),
            progress_key: next_key.clone(),
            path: entry.path.clone(),
            expected: entry.state.clone(),
            root: false,
        }))
    }

    /// Execute exactly one intent that the caller durably journaled after
    /// `prepare_owned_tree_cleanup_next`. A missing target is recoverable only
    /// when this exact manifest-bound in-flight intent is supplied.
    pub fn execute_owned_tree_cleanup_intent(
        &self,
        name: &str,
        ownership_token: &str,
        expected_identity: &OwnedDirectoryIdentity,
        expected_manifest: &TreeManifest,
        progress: &OwnedTreeCleanupProgress,
        intent: &CleanupIntent,
    ) -> std::result::Result<CleanupCompletion, OwnedTreeCleanupError> {
        let order = cleanup_order(expected_manifest);
        if progress.completed.len() >= order.len()
            || progress
                .completed
                .iter()
                .zip(&order)
                .any(|(completed, expected)| completed != expected)
            || order[progress.completed.len()] != intent.progress_key
            || intent.intent_token
                != cleanup_intent_token(expected_identity, expected_manifest, &intent.progress_key)
        {
            return Err(third_error(
                "cleanup intent is not the exact canonical in-flight step".to_owned(),
            ));
        }
        let expected_entry = if intent.root {
            if intent.progress_key != "root" || intent.path != name {
                return Err(third_error("root cleanup intent shape changed".to_owned()));
            }
            None
        } else {
            let entry = expected_manifest
                .entries
                .iter()
                .find(|entry| entry_key(entry) == intent.progress_key)
                .ok_or_else(|| {
                    third_error("intent target is absent from the manifest".to_owned())
                })?;
            if entry.path != intent.path || entry.state != intent.expected {
                return Err(third_error("intent expected state changed".to_owned()));
            }
            Some(entry)
        };

        let root = match self.open_child_checked(name) {
            Ok(Some(root)) => root,
            Ok(None) if intent.root => {
                return Ok(CleanupCompletion {
                    progress_key: intent.progress_key.clone(),
                    path: intent.path.clone(),
                    parent: sync_directory(self),
                    recovered_after_syscall: true,
                });
            }
            Ok(None) => {
                return Err(third_error(
                    "owned root disappeared during cleanup".to_owned(),
                ));
            }
            Err(error) => return Err(OwnedTreeCleanupError::Io(error)),
        };
        if owned_identity(
            ownership_token,
            root.identity().map_err(OwnedTreeCleanupError::Io)?,
        ) != *expected_identity
        {
            return Err(third_error("owned root identity changed".to_owned()));
        }
        let actual = manifest_mode(&root, manifest_has_transaction_stage(expected_manifest))
            .map_err(classify_manifest_cleanup_error)?;
        let completed = progress
            .completed
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        let remaining = expected_manifest
            .entries
            .iter()
            .filter(|entry| !completed.contains(&entry_key(entry)))
            .cloned()
            .collect::<Vec<_>>();
        let after = remaining
            .iter()
            .filter(|entry| {
                Some(entry.path.as_str()) != expected_entry.map(|entry| entry.path.as_str())
            })
            .cloned()
            .collect::<Vec<_>>();
        let recovered = if actual.entries == remaining {
            false
        } else if !intent.root && actual.entries == after {
            true
        } else {
            return Err(third_error(manifest_entries_difference(
                &remaining,
                &actual.entries,
            )));
        };

        let parent = if intent.root {
            drop(root);
            if recovered {
                super::DirectoryDurability::JournalRecoverable
            } else {
                remove_native(self, name, &intent.expected)?
            }
        } else {
            let view = project_view(&root).map_err(OwnedTreeCleanupError::Io)?;
            let (parent, child) = holder(&view, &intent.path).map_err(OwnedTreeCleanupError::Io)?;
            let durability = if recovered {
                super::DirectoryDurability::JournalRecoverable
            } else {
                remove_native(&parent, &child, &intent.expected)?
            };
            return Ok(CleanupCompletion {
                progress_key: intent.progress_key.clone(),
                path: intent.path.clone(),
                parent: durability,
                recovered_after_syscall: recovered,
            });
        };
        Ok(CleanupCompletion {
            progress_key: intent.progress_key.clone(),
            path: intent.path.clone(),
            parent,
            recovered_after_syscall: recovered,
        })
    }
}

pub(super) fn inspect_child_state(parent: &Pinned, name: &str) -> Result<Option<EntryState>> {
    inspect_child_state_mode(parent, name, false)
}

fn inspect_child_state_mode(
    parent: &Pinned,
    name: &str,
    allow_transaction_stage: bool,
) -> Result<Option<EntryState>> {
    ensure_tree_component(name, allow_transaction_stage)?;
    if allow_transaction_stage && is_transaction_stage_name(name) {
        return inspect_transaction_stage_state(parent, name);
    }
    let metadata = match parent.dir.symlink_metadata(name) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(anyhow::Error::new(error).context(format!(
                "inspecting `{}` no-follow",
                parent.join(name).display()
            )));
        }
    };
    if metadata.file_type().is_symlink() {
        bail!(
            "`{}` is a link or reparse point",
            parent.join(name).display()
        );
    }
    if metadata.is_dir() {
        let directory = parent.open_child(name)?;
        return Ok(Some(directory_state(&directory)?));
    }
    if !metadata.is_file() {
        bail!(
            "`{}` is a special filesystem entry",
            parent.join(name).display()
        );
    }
    let view = project_view(parent)?;
    match view.stable_file_state_with_identity(name) {
        Ok(Some((state, identity))) => Ok(Some(EntryState {
            kind: EntryStateKind::File,
            sha256: Some(state.sha256),
            bytes: Some(state.bytes),
            unix_mode: state.unix_mode,
            identity: entry_identity(identity),
        })),
        Ok(None) => Ok(None),
        Err(error) => Err(error.context(format!(
            "inspecting `{}` as a no-follow ordinary entry",
            parent.join(name).display()
        ))),
    }
}

fn directory_state(directory: &Pinned) -> Result<EntryState> {
    Ok(EntryState {
        kind: EntryStateKind::Directory,
        sha256: None,
        bytes: None,
        unix_mode: directory.unix_mode()?,
        identity: entry_identity(directory.identity()?),
    })
}

fn ensure_tree_component(name: &str, allow_transaction_stage: bool) -> Result<()> {
    if allow_transaction_stage && is_transaction_stage_name(name) {
        Ok(())
    } else {
        crate::ensure_safe_component(name)
    }
}

fn is_transaction_stage_name(name: &str) -> bool {
    name.strip_prefix(".vibe-stage-tx-").is_some_and(|suffix| {
        suffix.len() == 32
            && suffix
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

fn validate_authorized_transaction_stage_path(path: &str) -> Result<()> {
    if path.is_empty() || path.starts_with('/') || path.ends_with('/') {
        bail!("unsafe journal-authorized transaction-stage path");
    }
    let mut components = path.split('/').collect::<Vec<_>>();
    let name = components
        .pop()
        .ok_or_else(|| anyhow::anyhow!("transaction-stage path has no final component"))?;
    for parent in components {
        crate::ensure_safe_component(parent)?;
    }
    if !is_transaction_stage_name(name) {
        bail!("journal-authorized transaction-stage path has invalid grammar");
    }
    Ok(())
}

fn manifest_has_transaction_stage(manifest: &TreeManifest) -> bool {
    manifest.entries.iter().any(|entry| {
        let name = entry
            .path
            .rsplit_once('/')
            .map_or(entry.path.as_str(), |(_, name)| name);
        is_transaction_stage_name(name)
    })
}

pub(super) fn inspect_transaction_stage_state(
    parent: &Pinned,
    name: &str,
) -> Result<Option<EntryState>> {
    use std::io::{Read, Seek, SeekFrom};

    let mut options = crate::file::cap_options();
    let file = match parent.dir.open_with(name, options.read(true)) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(anyhow::Error::new(error)),
    };
    let mut file = file.into_std();
    let display = parent.join(name);
    crate::file::verify_regular_single_link(&file, &display)?;
    let opening = file.metadata()?;
    let mut read_pass = || -> Result<(u64, String)> {
        file.seek(SeekFrom::Start(0))?;
        let mut hash = Sha256::new();
        let mut bytes = 0_u64;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let used = file.read(&mut buffer)?;
            if used == 0 {
                return Ok((bytes, format!("{:x}", hash.finalize())));
            }
            bytes = bytes
                .checked_add(used as u64)
                .ok_or_else(|| anyhow::anyhow!("transaction stage exceeds u64"))?;
            hash.update(&buffer[..used]);
        }
    };
    let first = read_pass()?;
    let second = read_pass()?;
    let closing = file.metadata()?;
    if first != second || first.0 != opening.len() || first.0 != closing.len() {
        bail!("transaction stage changed during stable inspection");
    }
    let identity = crate::file::identity::file_identity(&file, &display)?;
    Ok(Some(EntryState {
        kind: EntryStateKind::File,
        sha256: Some(first.1),
        bytes: Some(first.0),
        unix_mode: crate::file::unix_mode(&closing),
        identity: entry_identity(identity),
    }))
}

pub(super) fn entry_identity(identity: FileIdentity) -> EntryIdentity {
    EntryIdentity(identity_token(
        b"vibe-safefs-tree-entry-identity-e1\0",
        identity,
    ))
}

fn owned_identity(owner: &str, identity: FileIdentity) -> OwnedDirectoryIdentity {
    let mut hash = Sha256::new();
    hash.update(b"vibe-safefs-owned-directory-e1\0");
    hash.update(owner.as_bytes());
    hash.update(b"\0");
    hash.update(identity.identity_bytes());
    OwnedDirectoryIdentity(format!("sha256:{:x}", hash.finalize()))
}

fn manifest(root: &Pinned) -> Result<TreeManifest> {
    manifest_mode(root, false)
}

fn manifest_mode(root: &Pinned, allow_transaction_stage: bool) -> Result<TreeManifest> {
    let mut first = Vec::new();
    walk(root, "", &mut first, allow_transaction_stage)?;
    first.sort_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));
    manifest_hook::between(root);
    let mut entries = Vec::new();
    walk(root, "", &mut entries, allow_transaction_stage)?;
    entries.sort_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));
    if entries != first {
        bail!(
            "tree `{}` changed between its two complete manifest passes",
            root.path().display()
        );
    }
    let digest = manifest_digest(&entries);
    Ok(TreeManifest { digest, entries })
}

fn manifest_digest(entries: &[TreeEntry]) -> String {
    let mut hash = Sha256::new();
    hash.update(b"vibe-safefs-tree-manifest-e1\0");
    for entry in entries {
        hash.update(entry.path.as_bytes());
        hash.update(b"\0");
        hash.update(match entry.state.kind {
            EntryStateKind::File => b"file\0".as_slice(),
            EntryStateKind::Directory => b"directory\0".as_slice(),
        });
        hash.update(entry.state.identity.as_str().as_bytes());
        hash.update(b"\0");
        if let Some(digest) = &entry.state.sha256 {
            hash.update(digest.as_bytes());
        }
        hash.update(b"\0");
        if let Some(bytes) = entry.state.bytes {
            hash.update(bytes.to_be_bytes());
        }
        hash.update(b"\0");
        if let Some(mode) = entry.state.unix_mode {
            hash.update(mode.to_be_bytes());
        }
        hash.update(b"\n");
    }
    format!("sha256:{:x}", hash.finalize())
}

fn walk(
    directory: &Pinned,
    prefix: &str,
    entries: &mut Vec<TreeEntry>,
    allow_transaction_stage: bool,
) -> Result<()> {
    let view = project_view(directory)?;
    let mut names = view.child_names(directory)?;
    names.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    for name in &names {
        ensure_tree_component(name, allow_transaction_stage)?;
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        let state = inspect_child_state_mode(directory, name, allow_transaction_stage)?
            .ok_or_else(|| anyhow::anyhow!("`{path}` vanished during manifest walk"))?;
        entries.push(TreeEntry {
            path: path.clone(),
            state: state.clone(),
        });
        if state.kind == EntryStateKind::Directory {
            let child = directory.open_child(name)?;
            if directory_state(&child)? != state {
                bail!("directory `{path}` was swapped during manifest walk");
            }
            walk(&child, &path, entries, allow_transaction_stage)?;
            if directory_state(&child)? != state {
                bail!("directory `{path}` changed identity or mode during manifest walk");
            }
        }
    }
    let mut after = view.child_names(directory)?;
    after.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    if after != names {
        bail!(
            "directory `{}` changed membership during manifest walk",
            directory.path().display()
        );
    }
    Ok(())
}

fn holder(project: &Project, relative: &str) -> Result<(Pinned, String)> {
    if relative.is_empty() || relative.starts_with('/') || relative.ends_with('/') {
        bail!("unsafe owned-tree relative path");
    }
    let mut components = relative.split('/').map(str::to_owned).collect::<Vec<_>>();
    let name = components
        .pop()
        .ok_or_else(|| anyhow::anyhow!("owned-tree path has no final component"))?;
    for parent in &components {
        crate::ensure_safe_component(parent)?;
    }
    ensure_tree_component(&name, true)?;
    let parents = components;
    if parents.is_empty() {
        return Ok((project.root_dir()?, name));
    }
    let chain = parents.iter().map(String::as_str).collect::<Vec<_>>();
    Ok((project.dir(&chain, false)?, name))
}

fn manifest_difference(expected: &TreeManifest, actual: &TreeManifest) -> String {
    manifest_entries_difference(&expected.entries, &actual.entries)
}

fn manifest_entries_difference(expected: &[TreeEntry], actual: &[TreeEntry]) -> String {
    let expected_paths = expected
        .iter()
        .map(|entry| entry.path.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let actual_paths = actual
        .iter()
        .map(|entry| entry.path.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    if let Some(extra) = actual_paths.difference(&expected_paths).next() {
        return format!("extra descendant `{extra}`");
    }
    if let Some(missing) = expected_paths.difference(&actual_paths).next() {
        return format!("missing descendant `{missing}`");
    }
    for (wanted, found) in expected.iter().zip(actual) {
        if wanted != found {
            return format!(
                "descendant `{}` changed content, size, mode, kind or identity",
                wanted.path
            );
        }
    }
    "manifest digest changed".to_owned()
}

fn depth(path: &str) -> usize {
    path.bytes().filter(|byte| *byte == b'/').count()
}

fn entry_key(entry: &TreeEntry) -> String {
    let prefix = match entry.state.kind {
        EntryStateKind::File => "file:",
        EntryStateKind::Directory => "directory:",
    };
    format!("{prefix}{}", entry.path)
}

fn cleanup_order(manifest: &TreeManifest) -> Vec<String> {
    let mut files = manifest
        .entries
        .iter()
        .filter(|entry| entry.state.kind == EntryStateKind::File)
        .map(entry_key)
        .collect::<Vec<_>>();
    files.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    let mut directories = manifest
        .entries
        .iter()
        .filter(|entry| entry.state.kind == EntryStateKind::Directory)
        .collect::<Vec<_>>();
    directories.sort_by(|left, right| {
        depth(&right.path)
            .cmp(&depth(&left.path))
            .then_with(|| right.path.as_bytes().cmp(left.path.as_bytes()))
    });
    files.extend(directories.into_iter().map(entry_key));
    files.push("root".to_owned());
    files
}

fn cleanup_intent_token(
    identity: &OwnedDirectoryIdentity,
    manifest: &TreeManifest,
    progress_key: &str,
) -> String {
    let mut hash = Sha256::new();
    hash.update(b"vibe-safefs-cleanup-intent-e1\0");
    hash.update(identity.as_str().as_bytes());
    hash.update(b"\0");
    hash.update(manifest.digest.as_bytes());
    hash.update(b"\0");
    hash.update(progress_key.as_bytes());
    format!("sha256:{:x}", hash.finalize())
}

fn remove_native(
    parent: &Pinned,
    name: &str,
    expected: &EntryState,
) -> std::result::Result<DirectoryDurability, OwnedTreeCleanupError> {
    super::platform::remove_expected(parent, name, expected).map_err(|error| match error {
        super::platform::NativeRemoveError::Changed(detail) => third_error(detail),
        super::platform::NativeRemoveError::Io(error) => {
            OwnedTreeCleanupError::Io(anyhow::Error::new(error))
        }
        #[cfg(not(windows))]
        super::platform::NativeRemoveError::Unsupported => OwnedTreeCleanupError::Unsupported,
    })
}

fn third_error(detail: String) -> OwnedTreeCleanupError {
    OwnedTreeCleanupError::Third { detail }
}

fn classify_manifest_cleanup_error(error: anyhow::Error) -> OwnedTreeCleanupError {
    if error
        .chain()
        .any(|cause| cause.downcast_ref::<std::io::Error>().is_some())
    {
        OwnedTreeCleanupError::Io(error)
    } else {
        third_error(format!(
            "owned descendant set changed or became unsafe: {error:#}"
        ))
    }
}

fn validate_identity_token(token: &str) -> Result<()> {
    let Some(hex) = token.strip_prefix("sha256:") else {
        bail!("identity token must use sha256:<64-lowercase-hex>");
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        bail!("identity token must use sha256:<64-lowercase-hex>");
    }
    Ok(())
}

fn is_raw_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(any(test, feature = "inject-failures"))]
mod tree_hook {
    use std::cell::RefCell;

    type Hook = Box<dyn Fn(&crate::Pinned, &str)>;
    thread_local! {
        static BEFORE: RefCell<Option<Hook>> = const { RefCell::new(None) };
    }
    pub fn arm(hook: Option<Hook>) {
        BEFORE.with(|slot| *slot.borrow_mut() = hook);
    }
    pub fn before(parent: &crate::Pinned, name: &str) {
        let hook = BEFORE.with(|slot| slot.borrow_mut().take());
        if let Some(hook) = hook {
            hook(parent, name);
        }
    }
}

#[cfg(not(any(test, feature = "inject-failures")))]
mod tree_hook {
    pub fn before(_: &crate::Pinned, _: &str) {}
}

#[cfg(any(test, feature = "inject-failures"))]
pub type OwnedTreeCheckHook = Box<dyn Fn(&Pinned, &str)>;

#[cfg(any(test, feature = "inject-failures"))]
pub fn arm_before_owned_tree_check(hook: Option<OwnedTreeCheckHook>) {
    tree_hook::arm(hook);
}

#[cfg(any(test, feature = "inject-failures"))]
mod manifest_hook {
    use std::cell::RefCell;

    type Hook = Box<dyn Fn(&crate::Pinned)>;
    thread_local! {
        static BETWEEN: RefCell<Option<Hook>> = const { RefCell::new(None) };
    }
    pub fn arm(hook: Option<Hook>) {
        BETWEEN.with(|slot| *slot.borrow_mut() = hook);
    }
    pub fn between(root: &crate::Pinned) {
        let hook = BETWEEN.with(|slot| slot.borrow_mut().take());
        if let Some(hook) = hook {
            hook(root);
        }
    }
}

#[cfg(not(any(test, feature = "inject-failures")))]
mod manifest_hook {
    pub fn between(_: &crate::Pinned) {}
}

#[cfg(any(test, feature = "inject-failures"))]
pub type ManifestPassHook = Box<dyn Fn(&Pinned)>;

#[cfg(any(test, feature = "inject-failures"))]
pub fn arm_between_manifest_passes(hook: Option<ManifestPassHook>) {
    manifest_hook::arm(hook);
}

#[cfg(any(test, feature = "inject-failures"))]
mod lease_hook {
    use std::cell::RefCell;
    type Hook = Box<dyn Fn(&crate::Pinned)>;
    thread_local! {
        static DURING: RefCell<Option<Hook>> = const { RefCell::new(None) };
    }
    pub fn arm(hook: Option<Hook>) {
        DURING.with(|slot| *slot.borrow_mut() = hook);
    }
    pub fn during(root: &crate::Pinned) {
        let hook = DURING.with(|slot| slot.borrow_mut().take());
        if let Some(hook) = hook {
            hook(root);
        }
    }
}

#[cfg(not(any(test, feature = "inject-failures")))]
mod lease_hook {
    pub fn during(_: &crate::Pinned) {}
}

#[cfg(any(test, feature = "inject-failures"))]
pub type LeaseAcquisitionHook = Box<dyn Fn(&Pinned)>;

#[cfg(any(test, feature = "inject-failures"))]
pub fn arm_during_manifest_lease(hook: Option<LeaseAcquisitionHook>) {
    lease_hook::arm(hook);
}

#[cfg(any(test, feature = "inject-failures"))]
mod publish_hook {
    use std::cell::RefCell;
    type Hook = Box<dyn Fn(&crate::Pinned, &str)>;
    thread_local! {
        static BEFORE_MOVE: RefCell<Option<Hook>> = const { RefCell::new(None) };
        static AFTER_MOVE: RefCell<Option<Hook>> = const { RefCell::new(None) };
    }
    pub fn arm_before(hook: Option<Hook>) {
        BEFORE_MOVE.with(|slot| *slot.borrow_mut() = hook);
    }
    pub fn arm_after(hook: Option<Hook>) {
        AFTER_MOVE.with(|slot| *slot.borrow_mut() = hook);
    }
    pub fn before_move(parent: &crate::Pinned, name: &str) {
        let hook = BEFORE_MOVE.with(|slot| slot.borrow_mut().take());
        if let Some(hook) = hook {
            hook(parent, name);
        }
    }
    pub fn after_move(parent: &crate::Pinned, name: &str) {
        let hook = AFTER_MOVE.with(|slot| slot.borrow_mut().take());
        if let Some(hook) = hook {
            hook(parent, name);
        }
    }
}

#[cfg(not(any(test, feature = "inject-failures")))]
mod publish_hook {
    pub fn before_move(_: &crate::Pinned, _: &str) {}
    pub fn after_move(_: &crate::Pinned, _: &str) {}
}

#[cfg(any(test, feature = "inject-failures"))]
pub type OwnedPublishHook = Box<dyn Fn(&Pinned, &str)>;

#[cfg(any(test, feature = "inject-failures"))]
pub fn arm_before_owned_tree_publish(hook: Option<OwnedPublishHook>) {
    publish_hook::arm_before(hook);
}

#[cfg(any(test, feature = "inject-failures"))]
pub fn arm_after_owned_tree_publish_move(hook: Option<OwnedPublishHook>) {
    publish_hook::arm_after(hook);
}
