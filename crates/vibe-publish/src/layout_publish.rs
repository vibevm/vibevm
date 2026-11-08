//! Publishing into a directory-layout registry (PROP-002 `#registry-type`):
//! a `monorepo` (one Git repository, pushed with plain `git`, no host API)
//! or a `localdir` (a directory written straight on the filesystem). Both
//! place the package at `<group>/<name>/v<version>/`.
//!
//! A version directory is immutable by default: re-publishing identical
//! content is a no-op, different content is refused unless the operator
//! passes `replace`. Identity is the package's content hash, so build
//! output and `.git` never make two copies differ.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-type");

use std::path::{Path, PathBuf};

use vibe_core::PackageKind;
use vibe_core::cancellation::CancellationToken;
use vibe_core::manifest::Manifest;

use crate::PublishError;

mod localdir;
mod monorepo;

/// Where a layout publish writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutTarget {
    /// One Git repository; packages on `branch`.
    Monorepo { url: String, branch: String },
    /// A directory (possibly a network share).
    Localdir { root: PathBuf },
}

/// Inputs to one layout publish.
#[derive(Debug, Clone)]
pub struct LayoutPublishConfig {
    pub source_dir: PathBuf,
    pub target: LayoutTarget,
    /// Overwrite an existing version directory whose content differs.
    pub replace: bool,
    /// Decide and report, write nothing.
    pub dry_run: bool,
}

/// What happened to the version directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutAction {
    /// The version directory did not exist and was written.
    Created,
    /// It existed with different content and `replace` overwrote it.
    Replaced,
    /// It already held identical content; nothing was written.
    Unchanged,
}

impl LayoutAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            LayoutAction::Created => "created",
            LayoutAction::Replaced => "replaced",
            LayoutAction::Unchanged => "unchanged",
        }
    }
}

/// The outcome the CLI renders.
#[derive(Debug, Clone)]
pub struct LayoutPublishOutcome {
    pub kind: PackageKind,
    pub group: String,
    pub name: String,
    pub version: semver::Version,
    /// `<group>/<name>/v<version>` inside the registry.
    pub package_path: String,
    pub action: LayoutAction,
    /// The pushed commit (monorepo, when something was pushed).
    pub commit: Option<String>,
    /// A non-fast-forward rejection was absorbed by one refetch.
    pub retried: bool,
    /// `sha256:` content hash of the published tree.
    pub content_hash: String,
    pub dry_run: bool,
}

/// Publish `config.source_dir` into its layout registry.
///
/// ```no_run
/// use std::path::PathBuf;
/// use vibe_publish::layout_publish::{LayoutPublishConfig, LayoutTarget, publish};
///
/// let outcome = publish(
///     &LayoutPublishConfig {
///         source_dir: PathBuf::from("my-package"),
///         target: LayoutTarget::Localdir { root: PathBuf::from("/srv/vibe") },
///         replace: false,
///         dry_run: true,
///     },
///     &Default::default(),
/// )
/// .unwrap();
/// println!("{} {}", outcome.package_path, outcome.action.as_str());
/// ```
pub fn publish(
    config: &LayoutPublishConfig,
    cancellation: &CancellationToken,
) -> Result<LayoutPublishOutcome, PublishError> {
    let package = Package::read(&config.source_dir)?;
    // Stage the payload once: the content hash compared against an existing
    // version directory and the bytes written are the same tree.
    let staging = tempfile::TempDir::new().map_err(|e| PublishError::Io {
        path: std::env::temp_dir(),
        message: format!("creating staging dir: {e}"),
    })?;
    let payload = staging.path().join("payload");
    crate::git_publish::copy_dir_controlled(&config.source_dir, &payload, cancellation)?;
    let staged = Staged {
        package,
        content_hash: content_hash(&payload)?,
        payload,
    };
    match &config.target {
        LayoutTarget::Localdir { root } => localdir::publish(config, root, &staged, cancellation),
        LayoutTarget::Monorepo { url, branch } => {
            monorepo::publish(config, url, branch, &staged, cancellation)
        }
    }
}

struct Package {
    kind: PackageKind,
    group: String,
    name: String,
    version: semver::Version,
}

impl Package {
    fn read(source_dir: &Path) -> Result<Self, PublishError> {
        let invalid = |reason: String| PublishError::SourceInvalid {
            path: source_dir.to_path_buf(),
            reason,
        };
        let manifest = Manifest::read(source_dir.join(Manifest::FILENAME))
            .map_err(|e| invalid(format!("cannot read vibe.toml: {e}")))?;
        let meta = manifest
            .package
            .ok_or_else(|| invalid("vibe.toml has no [package] table".into()))?;
        Ok(Package {
            kind: meta.kind,
            group: meta.group.as_str().to_string(),
            name: meta.name.to_string(),
            version: meta.version,
        })
    }

    fn path(&self) -> String {
        format!("{}/{}/v{}", self.group, self.name, self.version)
    }
}

struct Staged {
    package: Package,
    payload: PathBuf,
    content_hash: String,
}

impl Staged {
    fn outcome(
        &self,
        action: LayoutAction,
        commit: Option<String>,
        retried: bool,
        dry_run: bool,
    ) -> LayoutPublishOutcome {
        LayoutPublishOutcome {
            kind: self.package.kind,
            group: self.package.group.clone(),
            name: self.package.name.clone(),
            version: self.package.version.clone(),
            package_path: self.package.path(),
            action,
            commit,
            retried,
            content_hash: self.content_hash.clone(),
            dry_run,
        }
    }

    /// Compare against an existing version directory: `None` when absent,
    /// `Some(Unchanged)` when identical, `Some(Replaced)` when different and
    /// `replace` allows it, the refusal otherwise.
    fn decide(
        &self,
        existing: &Path,
        replace: bool,
        location: &str,
    ) -> Result<LayoutAction, PublishError> {
        if !existing.exists() {
            return Ok(LayoutAction::Created);
        }
        if content_hash(existing)? == self.content_hash {
            return Ok(LayoutAction::Unchanged);
        }
        if replace {
            return Ok(LayoutAction::Replaced);
        }
        Err(PublishError::VersionExists {
            package: format!(
                "{}/{}@{}",
                self.package.group, self.package.name, self.package.version
            ),
            location: location.to_string(),
        })
    }
}

fn content_hash(dir: &Path) -> Result<String, PublishError> {
    vibe_registry::compute_content_hash(dir).map_err(|e| PublishError::Io {
        path: dir.to_path_buf(),
        message: format!("hashing package tree: {e}"),
    })
}

fn checkpoint(cancellation: &CancellationToken) -> Result<(), PublishError> {
    cancellation
        .checkpoint()
        .map_err(|error| PublishError::Git(error.to_string()))
}

fn io(path: &Path, what: &str, error: std::io::Error) -> PublishError {
    PublishError::Io {
        path: path.to_path_buf(),
        message: format!("{what}: {error}"),
    }
}

#[cfg(test)]
#[path = "layout_publish/tests.rs"]
mod tests;
