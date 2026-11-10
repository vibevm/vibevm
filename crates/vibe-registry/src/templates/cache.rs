//! The offline cache of a template's external inputs, and its
//! deliberately **asymmetric** staleness rule (PROP-075
//! `#TEMPLATE-CACHE`).
//!
//! One directory per template package:
//!
//! ```text
//! <settings>/cache/templates/<group>/<name>/versions.json
//! <settings>/cache/templates/<group>/<name>/digests/<version>
//! <settings>/cache/templates/<group>/<name>/rendered/v<version>/
//! ```
//!
//! **A digest document of a released version is immutable**: it is cached
//! forever, never revalidated, and a changed one upstream is precisely the
//! alarm of `#TEMPLATE-LOCK-RECORD` — so caching it forever is what makes
//! that alarm meaningful rather than a race. **A version list is
//! mutable**: it is refetched once the cached copy is older than
//! `[template] version_list_ttl` (default fifteen minutes), and otherwise
//! read from the cache, so a shell prompt hook and a loop of
//! `vibe app exec` cost no network at all.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-CACHE");

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vibe_core::Group;

use super::error::TemplateError;

/// The cached version list, with everything needed to judge its age and
/// its provenance without a second file.
///
/// ```
/// use vibe_registry::templates::CachedVersionList;
///
/// let envelope: CachedVersionList = serde_json::from_str(
///     r#"{"source_url":"https://nodejs.org/dist/index.json",
///         "sha256":"ab",
///         "fetched_at":"2026-11-10T09:00:00Z",
///         "document":"[]"}"#,
/// )?;
/// assert_eq!(envelope.document, "[]");
/// assert_eq!(envelope.source_url, "https://nodejs.org/dist/index.json");
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedVersionList {
    /// The URL the bytes were read from, **after** remap — so a cache
    /// filled through a mirror says so.
    pub source_url: String,
    /// SHA-256 of the fetched bytes.
    pub sha256: String,
    /// RFC 3339 fetch time; the TTL is measured against it.
    pub fetched_at: String,
    /// The fetched document, verbatim.
    pub document: String,
}

impl CachedVersionList {
    /// Build an envelope around freshly fetched bytes, stamped with
    /// `now` as a UNIX epoch in seconds.
    pub fn of(source_url: &str, document: String, now: u64) -> Self {
        CachedVersionList {
            source_url: source_url.to_string(),
            sha256: digest_of(document.as_bytes()),
            fetched_at: vibe_core::timestamp::format_unix_utc(now),
            document,
        }
    }

    /// Whether this copy is older than `ttl` as of `now`. An unparseable
    /// or future `fetched_at` counts as **stale**, because the honest
    /// answer to «how old is this» is then «unknown», and a refetch costs
    /// one request while a wrong answer costs a wrong install.
    ///
    /// ```
    /// use std::time::Duration;
    /// use vibe_registry::templates::CachedVersionList;
    ///
    /// let now = 1_800_000_000;
    /// let fresh = CachedVersionList::of("https://e.invalid/i.json", "[]".into(), now);
    /// assert!(!fresh.is_stale(Duration::from_secs(900), now));
    /// assert!(fresh.is_stale(Duration::from_secs(900), now + 1_000));
    /// // Zero TTL means «always refetch».
    /// assert!(fresh.is_stale(Duration::ZERO, now));
    /// ```
    pub fn is_stale(&self, ttl: Duration, now: u64) -> bool {
        if ttl.is_zero() {
            return true;
        }
        match vibe_core::timestamp::parse_unix_utc(&self.fetched_at) {
            Some(fetched) if fetched <= now => now - fetched >= ttl.as_secs(),
            // A stamp in the future, or one that does not parse: the
            // honest answer to «how old is this» is «unknown», and a
            // refetch costs one request while a wrong answer costs a
            // wrong install.
            _ => true,
        }
    }
}

/// The cache directory of one template package.
///
/// ```
/// use std::path::Path;
/// use vibe_core::Group;
/// use vibe_registry::templates::TemplateCache;
///
/// let group = Group::parse("org.vibevm.node")?;
/// let cache = TemplateCache::at(Path::new("/s/cache/templates"), &group, "node");
/// assert!(cache.version_list_path().ends_with("org.vibevm.node/node/versions.json"));
/// assert!(
///     cache
///         .digest_path(&"24.22.2".parse()?)
///         .ends_with("org.vibevm.node/node/digests/24.22.2")
/// );
/// assert!(
///     cache
///         .rendered_dir(&"24.22.2".parse()?)
///         .ends_with("org.vibevm.node/node/rendered/v24.22.2")
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone)]
pub struct TemplateCache {
    root: PathBuf,
    package: String,
}

impl TemplateCache {
    /// The cache of `<group>/<name>` under a `…/cache/templates` root.
    pub fn at(templates_root: &Path, group: &Group, name: &str) -> Self {
        TemplateCache {
            root: templates_root.join(group.as_str()).join(name),
            package: format!("{group}/{name}"),
        }
    }

    /// This package's cache directory.
    pub fn dir(&self) -> &Path {
        &self.root
    }

    /// Where the version-list envelope lives.
    pub fn version_list_path(&self) -> PathBuf {
        self.root.join("versions.json")
    }

    /// Where one version's digest document lives.
    pub fn digest_path(&self, version: &semver::Version) -> PathBuf {
        self.root.join("digests").join(version.to_string())
    }

    /// Where the template document itself is kept beside its inputs.
    ///
    /// Caching the declaration — not an output, which
    /// `#TEMPLATE-RENDER-LOCATION` forbids a cache to hold — is what lets
    /// a **channel** be lowered from the cache alone, with no registry
    /// root, no index read and no request. That is the budget PROP-075
    /// `#ACTIVATION-HOOK` sets for a prompt hook: two small file reads.
    pub fn declaration_path(&self) -> PathBuf {
        self.root.join("template.toml")
    }

    /// The cached template document, or `None` when none was cached.
    pub fn read_declaration(&self) -> Result<Option<String>, TemplateError> {
        let path = self.declaration_path();
        match std::fs::read_to_string(&path) {
            Ok(text) => Ok(Some(text)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(self.io(&path, error)),
        }
    }

    /// Keep the template document beside its inputs, rewriting it when
    /// the revision on disk changed. Unlike a digest document this is not
    /// write-once: a template can be fixed without touching a produced
    /// version, and the cache must then hold the fix.
    pub fn write_declaration(&self, document: &str) -> Result<(), TemplateError> {
        let path = self.declaration_path();
        if self.read_declaration()?.as_deref() == Some(document) {
            return Ok(());
        }
        self.ensure_dir(self.root.as_path())?;
        std::fs::write(&path, document).map_err(|error| self.io(&path, error))
    }

    /// Where one version's materialised package directory lives.
    pub fn rendered_dir(&self, version: &semver::Version) -> PathBuf {
        self.root.join("rendered").join(format!("v{version}"))
    }

    /// The cached version list, or `None` when nothing is cached.
    pub fn read_version_list(&self) -> Result<Option<CachedVersionList>, TemplateError> {
        let path = self.version_list_path();
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(self.io(&path, error)),
        };
        // A corrupt envelope is treated as absent rather than fatal: the
        // cache is derived state, and the only cost of re-deriving it is
        // one request — whereas refusing would strand a user behind a
        // file they never wrote by hand.
        Ok(serde_json::from_slice(&bytes).ok())
    }

    /// Write the version-list envelope, creating the directory.
    pub fn write_version_list(&self, envelope: &CachedVersionList) -> Result<(), TemplateError> {
        let path = self.version_list_path();
        self.ensure_dir(self.root.as_path())?;
        let body = serde_json::to_vec_pretty(envelope).map_err(|error| TemplateError::Io {
            path: path.clone(),
            reason: error.to_string(),
        })?;
        std::fs::write(&path, body).map_err(|error| self.io(&path, error))
    }

    /// One version's cached digest document, or `None`.
    pub fn read_digest_document(
        &self,
        version: &semver::Version,
    ) -> Result<Option<String>, TemplateError> {
        let path = self.digest_path(version);
        match std::fs::read_to_string(&path) {
            Ok(text) => Ok(Some(text)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(self.io(&path, error)),
        }
    }

    /// Store one version's digest document verbatim. **Write-once**: an
    /// already-cached document is never overwritten, because a released
    /// version's checksums are immutable and a difference is the alarm,
    /// not an update.
    pub fn write_digest_document(
        &self,
        version: &semver::Version,
        document: &str,
    ) -> Result<(), TemplateError> {
        let path = self.digest_path(version);
        if path.exists() {
            return Ok(());
        }
        self.ensure_dir(&self.root.join("digests"))?;
        std::fs::write(&path, document).map_err(|error| self.io(&path, error))
    }

    /// The refusal for an offline miss, naming the package, the exact
    /// path and the command that would fill it.
    pub fn missing(&self, what: &str, path: PathBuf) -> TemplateError {
        TemplateError::CacheMissing {
            package: self.package.clone(),
            what: what.to_string(),
            path,
            command: format!("vibe app versions {}", self.package),
        }
    }

    fn ensure_dir(&self, dir: &Path) -> Result<(), TemplateError> {
        std::fs::create_dir_all(dir).map_err(|error| self.io(dir, error))
    }

    fn io(&self, path: &Path, error: std::io::Error) -> TemplateError {
        TemplateError::Io {
            path: path.to_path_buf(),
            reason: error.to_string(),
        }
    }
}

/// The hex SHA-256 of some bytes — the `sha256` of an envelope and the
/// `digest_source_sha256` of a lock record.
///
/// ```
/// use vibe_registry::templates::digest_of;
///
/// assert_eq!(
///     digest_of(b""),
///     "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
/// );
/// ```
pub fn digest_of(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}
