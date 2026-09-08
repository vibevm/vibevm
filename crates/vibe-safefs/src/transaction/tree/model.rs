specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#SEC-NO-FOLLOW");

/// Opaque identity of one manifest entry.  The token is a domain-separated
/// digest of the OS identity; raw device, volume and inode/index values never
/// leave the crate.
///
/// ```
/// use vibe_safefs::EntryIdentity;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let token = format!("sha256:{}", "a".repeat(64));
/// let identity = EntryIdentity::from_token(&token)?;
/// assert_eq!(identity.as_str(), token);
/// # Ok(())
/// # }
/// ```
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
///
/// ```
/// use vibe_safefs::OwnedDirectoryIdentity;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let token = format!("sha256:{}", "b".repeat(64));
/// let identity = OwnedDirectoryIdentity::from_token(&token)?;
/// assert_eq!(identity.as_str(), token);
/// # Ok(())
/// # }
/// ```
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

/// The physical shape recorded for one manifest entry.
///
/// ```
/// use vibe_safefs::EntryStateKind;
///
/// assert_ne!(EntryStateKind::File, EntryStateKind::Directory);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryStateKind {
    File,
    Directory,
}

/// Complete expected state for one direct rename operand.
///
/// ```
/// use vibe_safefs::{EntryIdentity, EntryState, EntryStateKind};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let identity = EntryIdentity::from_token(&format!("sha256:{}", "a".repeat(64)))?;
/// let state = EntryState {
///     kind: EntryStateKind::Directory,
///     sha256: None,
///     bytes: None,
///     unix_mode: None,
///     identity,
/// };
/// assert_eq!(state.kind, EntryStateKind::Directory);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryState {
    pub kind: EntryStateKind,
    pub sha256: Option<String>,
    pub bytes: Option<u64>,
    pub unix_mode: Option<u32>,
    pub identity: EntryIdentity,
}

/// One path and its exact state in a [`TreeManifest`](vibe_safefs::TreeManifest).
///
/// ```
/// use vibe_safefs::{EntryIdentity, EntryState, EntryStateKind, TreeEntry};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let entry = TreeEntry {
///     path: "docs".into(),
///     state: EntryState {
///         kind: EntryStateKind::Directory,
///         sha256: None,
///         bytes: None,
///         unix_mode: None,
///         identity: EntryIdentity::from_token(&format!("sha256:{}", "a".repeat(64)))?,
///     },
/// };
/// assert_eq!(entry.path, "docs");
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeEntry {
    /// Forward-slashed path relative to the owned root.
    pub path: String,
    pub state: EntryState,
}

/// Complete canonical descendant set.  `digest` commits to paths, kinds,
/// contents, sizes, modes and opaque object identities.
///
/// ```
/// use vibe_safefs::TreeManifest;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let manifest = TreeManifest::from_persisted(
///     "sha256:62e0b53b3a1ad3271d913b9f2c96f45b970408793990fd27367017a871acc1a6".into(),
///     Vec::new(),
/// )?;
/// assert!(manifest.entries.is_empty());
/// # Ok(())
/// # }
/// ```
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

/// Point-in-time observation of a journal-owned tree.
///
/// ```
/// use vibe_safefs::OwnedTreeObservation;
///
/// let observation = OwnedTreeObservation::Absent;
/// assert!(matches!(observation, OwnedTreeObservation::Absent));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnedTreeObservation {
    Absent,
    MatchesAtObservation(TreeManifest),
    Third { detail: String },
}

/// Durable prefix of an exact owned-tree cleanup schedule.
///
/// ```
/// use vibe_safefs::OwnedTreeCleanupProgress;
///
/// let progress = OwnedTreeCleanupProgress::new();
/// assert!(progress.completed().is_empty());
/// ```
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

/// Either the next journalable cleanup intent or a completed schedule.
///
/// ```
/// use vibe_safefs::CleanupPreparation;
///
/// let preparation = CleanupPreparation::Complete;
/// assert!(matches!(preparation, CleanupPreparation::Complete));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CleanupPreparation {
    Intent(CleanupIntent),
    Complete,
}

/// One manifest-bound removal intent that must be durable before execution.
///
/// ```
/// use vibe_safefs::{CleanupIntent, EntryIdentity, EntryState, EntryStateKind};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let intent = CleanupIntent {
///     intent_token: "journal-step".into(),
///     progress_key: "root".into(),
///     path: "candidate".into(),
///     expected: EntryState {
///         kind: EntryStateKind::Directory,
///         sha256: None,
///         bytes: None,
///         unix_mode: None,
///         identity: EntryIdentity::from_token(&format!("sha256:{}", "a".repeat(64)))?,
///     },
///     root: true,
/// };
/// assert!(intent.root);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupIntent {
    pub intent_token: String,
    pub progress_key: String,
    pub path: String,
    pub expected: EntryState,
    pub root: bool,
}

/// Durable result of exactly one owned-tree cleanup intent.
///
/// ```
/// use vibe_safefs::{CleanupCompletion, DirectoryDurability};
///
/// let completion = CleanupCompletion {
///     progress_key: "root".into(),
///     path: "candidate".into(),
///     parent: DirectoryDurability::JournalRecoverable,
///     recovered_after_syscall: false,
/// };
/// assert_eq!(completion.progress_key(), "root");
/// assert_eq!(completion.durability(), DirectoryDurability::JournalRecoverable);
/// ```
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

/// Typed refusal from identity-bound tree cleanup.
///
/// ```
/// use vibe_safefs::OwnedTreeCleanupError;
///
/// let error = OwnedTreeCleanupError::Unsupported;
/// assert!(error.to_string().contains("unsupported"));
/// ```
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

/// Typed result when exclusive creation cannot yield a sealed owned root.
///
/// ```
/// use vibe_safefs::OwnedDirectoryCreateError;
///
/// let error = OwnedDirectoryCreateError::Unsupported;
/// assert!(error.to_string().contains("unsupported"));
/// ```
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

/// Typed refusal while rebinding a journal-owned directory after restart.
///
/// ```
/// use vibe_safefs::ReopenOwnedDirectoryError;
///
/// let error = ReopenOwnedDirectoryError::Unsupported;
/// assert!(error.to_string().contains("Windows-only"));
/// ```
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
