//! Project-recorded acquisition facts beside the shared lock.
//!
//! These historical report rows are not current-machine acquisition evidence
//! and never select, verify or materialise a source. Runtime/user/invocation
//! routes belong to operation evidence and user-local journals. An unchanged
//! canonical selection may retain these rows without claiming a new contact.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#provenance");

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specmark::spec;

use crate::error::{Error, Result};

use crate::manifest::{read_toml, write_toml};

/// The sidecar's file name, beside `vibe.lock`.
pub const PROVENANCE_FILENAME: &str = "vibe.lock.provenance";

/// The sidecar beside the lock at `lockfile_path`.
pub fn provenance_path(lockfile_path: &Path) -> PathBuf {
    lockfile_path.with_file_name(PROVENANCE_FILENAME)
}

/// One locked package's fetch provenance.
///
/// ```
/// use vibe_core::lock_provenance::FetchedFrom;
///
/// let row: FetchedFrom = toml::from_str(
///     "package = \"org.vibevm/wal\"\nversion = \"0.3.0\"\n\
///      fetched_from = \"file:///srv/mirror/wal\"\n",
/// )
/// .unwrap();
/// assert_eq!(row.package, "org.vibevm/wal");
/// assert_eq!(row.fetched_from, "file:///srv/mirror/wal");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FetchedFrom {
    /// `<group>/<name>` of the locked package.
    pub package: String,
    /// Its locked version.
    pub version: String,
    /// The URL or path the bytes were actually read from. Present only
    /// when it differs from the row's canonical `source_url`.
    pub fetched_from: String,
    /// The remap row or `[[mirror]]` that chose it, when one did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row: Option<String>,
}

/// The whole sidecar.
///
/// ```
/// use vibe_core::lock_provenance::{FetchedFrom, LockProvenance};
///
/// let mut record = LockProvenance::default();
/// assert!(record.is_empty());
/// record.packages.push(FetchedFrom {
///     package: "org.vibevm/wal".into(),
///     version: "0.3.0".into(),
///     fetched_from: "file:///srv/mirror/wal".into(),
///     row: Some("row 1 of `~/.vibe/settings.toml` (local)".into()),
/// });
/// assert_eq!(
///     record.of("org.vibevm/wal").map(|r| r.fetched_from.as_str()),
///     Some("file:///srv/mirror/wal"),
/// );
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-066#provenance")]
pub struct LockProvenance {
    /// `[[package]]` rows, sorted by package key.
    #[serde(default, rename = "package", skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<FetchedFrom>,
}

impl LockProvenance {
    /// Whether the optional project-recorded report contains any rows.
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }

    /// The record for one `<group>/<name>`.
    pub fn of(&self, package: &str) -> Option<&FetchedFrom> {
        self.packages.iter().find(|row| row.package == package)
    }

    /// Read the sidecar beside `lockfile_path`; a missing file is an empty
    /// record, because the sidecar is provenance and never a prerequisite.
    pub fn read_beside(lockfile_path: &Path) -> Result<Self> {
        let path = provenance_path(lockfile_path);
        if !path.exists() {
            return Ok(Self::default());
        }
        read_toml(path)
    }

    /// Decode a captured project-recorded report, never an acquisition grant.
    /// ```
    /// let record=vibe_core::lock_provenance::LockProvenance::parse_str("# no recorded rows").unwrap();
    /// assert!(record.is_empty());
    /// ```
    pub fn parse_str(text: &str) -> Result<Self> {
        toml::from_str(text).map_err(|_| Error::InvalidLockfile {
            reason:
                "vibe.lock.provenance is malformed; retain it rather than guessing recorded rows"
                    .into(),
        })
    }

    /// Prepare the exact sorted sidecar text from captured decoration.
    /// `None` means removal: an empty provenance record has no sidecar.
    /// No filesystem path is read or modified.
    ///
    /// ```
    /// let provenance = vibe_core::lock_provenance::LockProvenance::default();
    /// assert_eq!(provenance.render_with_comments(Some("# prior record")).unwrap(), None);
    /// ```
    pub fn render_with_comments(&self, previous: Option<&str>) -> Result<Option<String>> {
        if self.is_empty() {
            return Ok(None);
        }
        let sorted = self.sorted();
        crate::manifest::render_toml(&sorted, previous).map(Some)
    }

    /// Write the sidecar beside `lockfile_path`, or remove it when there
    /// is nothing to record.
    #[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#provenance")]
    pub fn write_beside(&self, lockfile_path: &Path) -> Result<()> {
        let path = provenance_path(lockfile_path);
        if self.is_empty() {
            if path.exists() {
                std::fs::remove_file(&path).map_err(|source| Error::Write {
                    path: path.clone(),
                    source,
                })?;
            }
            return Ok(());
        }
        let sorted = self.sorted();
        write_toml(path, &sorted)
    }

    fn sorted(&self) -> Self {
        let mut sorted = self.clone();
        sorted
            .packages
            .sort_by(|left, right| left.package.cmp(&right.package));
        sorted
    }
}
