//! The one rule for where bytes may be read from (`PROP-066#locations`).
//!
//! Every fetch path that turns a declared or remapped source into bytes —
//! `vibe self bootstrap`, `self update`, `self install`, `install -g` —
//! classifies the location here, so the three verbs agree about which
//! spellings are readable: `https://` and `http://` are remote (the
//! transport and source policy decide whether they may be contacted),
//! `file://` and a `path` target are local, and anything else is refused
//! before any I/O.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#locations");

use std::path::{Path, PathBuf};

use specmark::spec;

use super::row::RemapTarget;

/// A readable location: a remote URL, or a local file or directory.
///
/// ```
/// use std::path::PathBuf;
/// use vibe_core::remap::{ByteLocation, RemapTarget};
///
/// let remote = ByteLocation::parse("http://mirror.corp.example/v1/a.zip").unwrap();
/// assert!(matches!(remote, ByteLocation::Remote(_)));
/// // A query on a local URL — the release lane's cache-buster — is not
/// // part of the file name.
/// let local = ByteLocation::parse("file:///srv/drop/v1.0.0/a.zip?vvm_nonce=1").unwrap();
/// assert_eq!(local, ByteLocation::Local(PathBuf::from("/srv/drop/v1.0.0/a.zip")));
/// assert!(ByteLocation::parse("ftp://host/a").is_err());
/// let target = RemapTarget::Path(PathBuf::from("/srv/drop"));
/// assert_eq!(ByteLocation::of_target(&target).unwrap().to_url(), "file:///srv/drop");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#locations")]
pub enum ByteLocation {
    /// An `http://` or `https://` URL, verbatim.
    Remote(String),
    /// A local file or directory.
    Local(PathBuf),
}

/// Why a location cannot be read.
///
/// ```
/// use vibe_core::remap::{ByteLocation, LocationError};
///
/// let error = ByteLocation::parse("ftp://mirror.example/a.zip").unwrap_err();
/// assert!(matches!(error, LocationError::Unsupported { .. }));
/// assert!(error.to_string().contains("PROP-066#locations"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-066#locations")]
pub enum LocationError {
    #[error(
        "`{location}` is not a readable location: only `https://`, `http://`, `file://` URLs \
         and local paths are read \
         (violates spec://org.vibevm.core/vibevm/common/PROP-066#locations; \
          fix: point the remap row, release source or input at one of those)"
    )]
    Unsupported { location: String },
    #[error(
        "`{location}` is a `file://` URL that names no local path \
         (violates spec://org.vibevm.core/vibevm/common/PROP-066#locations; \
          fix: use `file:///absolute/path` — a host part is not supported)"
    )]
    NotLocal { location: String },
    #[error(
        "`{location}` is a relative path \
         (violates spec://org.vibevm.core/vibevm/common/PROP-066#locations; \
          fix: give the directory or file as an absolute path)"
    )]
    Relative { location: String },
}

impl ByteLocation {
    /// Classify a spelled location: an absolute local path, a `file://`
    /// URL, or an `http(s)://` URL.
    pub fn parse(raw: &str) -> Result<Self, LocationError> {
        let trimmed = raw.trim();
        // A local path first: `C:\drop` parses as a URL with scheme `c`.
        let as_path = Path::new(trimmed);
        if as_path.is_absolute() && !trimmed.contains("://") {
            return Ok(ByteLocation::Local(as_path.to_path_buf()));
        }
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("https://") || lower.starts_with("http://") {
            return Ok(ByteLocation::Remote(trimmed.to_owned()));
        }
        if lower.starts_with("file://") {
            let url = url::Url::parse(trimmed).map_err(|_| LocationError::NotLocal {
                location: trimmed.to_owned(),
            })?;
            return url.to_file_path().map(ByteLocation::Local).map_err(|()| {
                LocationError::NotLocal {
                    location: trimmed.to_owned(),
                }
            });
        }
        if !trimmed.contains("://") {
            return Err(LocationError::Relative {
                location: trimmed.to_owned(),
            });
        }
        Err(LocationError::Unsupported {
            location: trimmed.to_owned(),
        })
    }

    /// Classify a remap target. A `path` target must be absolute; a `url`
    /// target follows [`ByteLocation::parse`]; `deny` is never a location.
    pub fn of_target(target: &RemapTarget) -> Result<Self, LocationError> {
        match target {
            RemapTarget::Url(url) => Self::parse(url),
            RemapTarget::Path(path) if path.is_absolute() => Ok(ByteLocation::Local(path.clone())),
            RemapTarget::Path(path) => Err(LocationError::Relative {
                location: path.display().to_string(),
            }),
            RemapTarget::Deny => Err(LocationError::Unsupported {
                location: "deny".to_owned(),
            }),
        }
    }

    /// The location as one URL string: remote URLs verbatim, local paths
    /// as `file://` URLs (so a caller with one URL slot can carry either).
    pub fn to_url(&self) -> String {
        match self {
            ByteLocation::Remote(url) => url.clone(),
            ByteLocation::Local(path) => url::Url::from_file_path(path)
                .map(String::from)
                .unwrap_or_else(|()| format!("file://{}", path.display())),
        }
    }

    /// Append one `/`-separated child (a file name or `v<version>`).
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use vibe_core::remap::ByteLocation;
    ///
    /// let local = ByteLocation::Local(PathBuf::from("/srv/drop"));
    /// assert_eq!(local.child("v1.0.0"), ByteLocation::Local(PathBuf::from("/srv/drop/v1.0.0")));
    /// let remote = ByteLocation::Remote("https://n.example/r/".into());
    /// assert_eq!(remote.child("v1.0.0").to_url(), "https://n.example/r/v1.0.0");
    /// ```
    pub fn child(&self, name: &str) -> Self {
        match self {
            ByteLocation::Remote(url) => {
                ByteLocation::Remote(format!("{}/{name}", url.trim_end_matches('/')))
            }
            ByteLocation::Local(path) => ByteLocation::Local(path.join(name)),
        }
    }

    /// Whether the bytes are on this machine.
    pub fn is_local(&self) -> bool {
        matches!(self, ByteLocation::Local(_))
    }
}
