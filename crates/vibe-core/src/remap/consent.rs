//! The acceptance ledger of one home (`PROP-066#consent`).
//!
//! `<home>/remap/consent.json` is the whole of what a user agreed to:
//! per package key the content hash they answered about, what they
//! answered, when, and whether the acceptance is currently switched on.
//! Acceptance is **bound to the content hash**, so a new version of an
//! accepted package is a new offer and not a silent upgrade of what the
//! machine fetches from.
//!
//! The ledger is never the sole home of a fact: the packages themselves
//! are on disk and the provider map of [`super::providers`] is derived
//! from the two. What lives only here is the user's answer, which nothing
//! else can recompute.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#consent");

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specmark::spec;

/// The document protocol of `consent.json`.
pub const CONSENT_PROTOCOL: &str = "vibe-remap-consent/1";

/// The `<home>/remap` directory every file of this module lives under.
pub fn remap_dir(home: &Path) -> PathBuf {
    home.join("remap")
}

/// `<home>/remap/consent.json`.
pub fn consent_path(home: &Path) -> PathBuf {
    remap_dir(home).join("consent.json")
}

/// What the user answered about one package, or that they have not.
///
/// ```
/// use vibe_core::remap::OfferState;
///
/// // An offer nobody answered is the default, and the only state that
/// // makes a run ask.
/// assert_eq!(OfferState::default(), OfferState::Offered);
/// assert!(OfferState::Offered.asks());
/// assert!(!OfferState::Rejected.asks());
/// assert!(OfferState::Accepted.applies());
/// assert!(!OfferState::Disabled.applies());
/// assert_eq!(OfferState::Disabled.label(), "disabled");
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OfferState {
    /// Neither accepted nor rejected — the run asks.
    #[default]
    Offered,
    /// Accepted and switched on: the rows apply.
    Accepted,
    /// Refused, so the question is not asked again.
    Rejected,
    /// Accepted once and switched off; the acceptance is remembered.
    Disabled,
}

impl OfferState {
    /// Whether a run that meets this offer has to ask the user.
    pub fn asks(self) -> bool {
        matches!(self, OfferState::Offered)
    }

    /// Whether this package's rows take part in the Locator.
    pub fn applies(self) -> bool {
        matches!(self, OfferState::Accepted)
    }

    /// The word `remap list` prints.
    pub fn label(self) -> &'static str {
        match self {
            OfferState::Offered => "offered",
            OfferState::Accepted => "accepted",
            OfferState::Rejected => "rejected",
            OfferState::Disabled => "disabled",
        }
    }
}

/// One package's record in the ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#consent")]
pub struct ConsentRecord {
    /// The content hash the answer was given about. A different hash on
    /// disk is a new offer.
    pub content_hash: String,
    /// Whether the answer was yes or no.
    pub accepted: bool,
    /// Whether an accepted package is currently switched on. Meaningless
    /// for a rejection, and kept `true` there so a later `accept` of the
    /// same hash needs no second flag.
    #[serde(default = "yes")]
    pub enabled: bool,
    /// When the answer was recorded, RFC 3339.
    pub answered: String,
}

fn yes() -> bool {
    true
}

/// The ledger of one home.
///
/// ```
/// use vibe_core::remap::{ConsentLedger, OfferState};
///
/// let mut ledger = ConsentLedger::default();
/// // Nothing answered yet: every offer asks.
/// assert_eq!(ledger.state("org.corp/intranet", "sha256:a"), OfferState::Offered);
/// ledger.accept("org.corp/intranet", "sha256:a", "2026-11-01T00:00:00Z");
/// assert_eq!(ledger.state("org.corp/intranet", "sha256:a"), OfferState::Accepted);
/// // A new content hash is a new offer, not an inherited acceptance.
/// assert_eq!(ledger.state("org.corp/intranet", "sha256:b"), OfferState::Offered);
/// // Switching it off keeps the acceptance and stops the rows applying.
/// assert!(ledger.set_enabled("org.corp/intranet", false));
/// assert_eq!(ledger.state("org.corp/intranet", "sha256:a"), OfferState::Disabled);
/// // Removing the package removes its answer with it.
/// assert!(ledger.forget("org.corp/intranet"));
/// assert_eq!(ledger.state("org.corp/intranet", "sha256:a"), OfferState::Offered);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-066#consent")]
pub struct ConsentLedger {
    /// `vibe-remap-consent/1`.
    pub protocol: String,
    /// Package key → the answer about it.
    #[serde(default)]
    pub packages: BTreeMap<String, ConsentRecord>,
}

impl Default for ConsentLedger {
    fn default() -> Self {
        ConsentLedger {
            protocol: CONSENT_PROTOCOL.to_owned(),
            packages: BTreeMap::new(),
        }
    }
}

impl ConsentLedger {
    /// Read the ledger of `home`. A missing file is an empty ledger —
    /// nobody has answered anything yet, which is exactly what an absent
    /// file means.
    pub fn load(home: &Path) -> Result<Self, ConsentError> {
        let path = consent_path(home);
        match std::fs::read_to_string(&path) {
            Ok(body) => serde_json::from_str(&body).map_err(|source| ConsentError::Malformed {
                path: path.display().to_string(),
                source,
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(ConsentError::Io {
                action: "reading",
                path: path.display().to_string(),
                source,
            }),
        }
    }

    /// Write the ledger of `home`, replacing it atomically.
    pub fn store(&self, home: &Path) -> Result<(), ConsentError> {
        let path = consent_path(home);
        let body =
            serde_json::to_string_pretty(self).map_err(|source| ConsentError::Malformed {
                path: path.display().to_string(),
                source,
            })?;
        write_atomic(&path, format!("{body}\n").as_bytes())
    }

    /// What the user answered about `key` at `content_hash`.
    ///
    /// A record about a *different* hash answers `Offered`: the user
    /// agreed to the rows they were shown, and different rows are a
    /// different question (`PROP-066#consent`).
    #[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#consent")]
    pub fn state(&self, key: &str, content_hash: &str) -> OfferState {
        match self.packages.get(key) {
            Some(record) if record.content_hash != content_hash => OfferState::Offered,
            Some(record) if !record.accepted => OfferState::Rejected,
            Some(record) if record.enabled => OfferState::Accepted,
            Some(_) => OfferState::Disabled,
            None => OfferState::Offered,
        }
    }

    /// Record an acceptance of `content_hash`.
    pub fn accept(&mut self, key: &str, content_hash: &str, answered: &str) {
        self.packages.insert(
            key.to_owned(),
            ConsentRecord {
                content_hash: content_hash.to_owned(),
                accepted: true,
                enabled: true,
                answered: answered.to_owned(),
            },
        );
    }

    /// Record a refusal of `content_hash`, so the question is not asked
    /// again.
    pub fn reject(&mut self, key: &str, content_hash: &str, answered: &str) {
        self.packages.insert(
            key.to_owned(),
            ConsentRecord {
                content_hash: content_hash.to_owned(),
                accepted: false,
                enabled: true,
                answered: answered.to_owned(),
            },
        );
    }

    /// Switch an accepted package off or on without forgetting the
    /// acceptance. `false` when nothing is recorded about `key`.
    pub fn set_enabled(&mut self, key: &str, enabled: bool) -> bool {
        match self.packages.get_mut(key) {
            Some(record) => {
                record.enabled = enabled;
                true
            }
            None => false,
        }
    }

    /// Drop the answer about `key` — what uninstalling the package does
    /// (`PROP-066#uninstall`). `false` when there was nothing to drop.
    pub fn forget(&mut self, key: &str) -> bool {
        self.packages.remove(key).is_some()
    }
}

/// Replace a derived map atomically — the provider map's writer, which
/// shares this module's one atomic replace rather than growing a second.
pub(super) fn write_map(path: &Path, bytes: &[u8]) -> Result<(), ConsentError> {
    write_atomic(path, bytes)
}

/// Replace `path` atomically: write a sibling, flush it, rename it over.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), ConsentError> {
    use std::io::Write;

    let parent = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent).map_err(|source| ConsentError::Io {
        action: "creating",
        path: parent.display().to_string(),
        source,
    })?;
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let sibling = parent.join(format!(".{name}.{}.tmp", std::process::id()));
    fn wrote(path: &Path) -> impl Fn(std::io::Error) -> ConsentError + '_ {
        move |source| ConsentError::Io {
            action: "writing",
            path: path.display().to_string(),
            source,
        }
    }
    let mut file = std::fs::File::create(&sibling).map_err(wrote(&sibling))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(wrote(&sibling))?;
    drop(file);
    std::fs::rename(&sibling, path).map_err(|source| {
        let _ = std::fs::remove_file(&sibling);
        ConsentError::Io {
            action: "replacing",
            path: path.display().to_string(),
            source,
        }
    })
}

/// Why a ledger or a provider map could not be read or written.
///
/// ```
/// use vibe_core::remap::ConsentError;
///
/// let error = ConsentError::Io {
///     action: "reading",
///     path: "/home/u/.vibe/remap/consent.json".into(),
///     source: std::io::Error::other("nope"),
/// };
/// assert!(error.to_string().contains("PROP-066#consent"));
/// ```
#[derive(Debug, thiserror::Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-066#consent")]
pub enum ConsentError {
    #[error(
        "{action} `{path}` failed: {source} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-066#consent; \
          fix: check the home's permissions — the acceptance ledger is the only \
          record of what this machine agreed to apply)"
    )]
    Io {
        action: &'static str,
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "`{path}` is not a readable `vibe-remap-consent/1` document: {source} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-066#consent; \
          fix: repair or delete the file — deleting it turns every accepted \
          package back into an offer, which is the safe direction)"
    )]
    Malformed {
        path: String,
        #[source]
        source: serde_json::Error,
    },
}

/// Take the home's remap lock, so a rebuild of the derived provider map
/// cannot race a second one (`PROP-066#providers`).
///
/// A held lock is refused rather than awaited, as every other mutation
/// lock in the workspace is: two sessions answering the same question at
/// once is an operator mistake with a one-word fix, not something to
/// serialise silently.
pub fn lock(home: &Path) -> Result<HomeLock, ConsentError> {
    let dir = remap_dir(home);
    std::fs::create_dir_all(&dir).map_err(|source| ConsentError::Io {
        action: "creating",
        path: dir.display().to_string(),
        source,
    })?;
    let path = dir.join(".mutation.lock");
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&path)
        .map_err(|source| ConsentError::Io {
            action: "opening",
            path: path.display().to_string(),
            source,
        })?;
    file.try_lock().map_err(|_| ConsentError::Io {
        action: "locking",
        path: path.display().to_string(),
        source: std::io::Error::other("another vibe process holds the home's remap lock"),
    })?;
    Ok(HomeLock { _file: file })
}

/// Held for one mutation of the home's remap state.
#[derive(Debug)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#providers")]
pub struct HomeLock {
    _file: std::fs::File,
}
