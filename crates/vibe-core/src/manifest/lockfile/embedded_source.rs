//! Portable authenticated provenance of one locked upstream source.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#lockfile");

use specmark::spec;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::content_hash::ContentHash;
use crate::provenance::SourceUrl;

/// One immutable external source authenticated for a locked package. The
/// record contains portable provenance only: cache paths and credentials are
/// deliberately absent.
///
/// ```
/// use vibe_core::manifest::LockedEmbeddedSource;
///
/// let locked: LockedEmbeddedSource = toml::from_str(r#"
/// name = "upstream"
/// source_url = "https://github.com/example/upstream.git"
/// source_ref = "refs/tags/v1.2.3"
/// resolved_commit = "0123456789abcdef0123456789abcdef01234567"
/// tree_oid = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
/// content_hash = "sha256-tree/1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
/// upstream_authors = ["Example Upstream Authors"]
/// upstream_license = "MIT"
/// license_path = "LICENSE"
/// license_url = "https://github.com/example/upstream/blob/0123456789abcdef0123456789abcdef01234567/LICENSE"
/// license_file_sha256 = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
/// "#).unwrap();
/// assert_eq!(locked.source_ref.as_deref(), Some("refs/tags/v1.2.3"));
/// assert_eq!(locked.upstream_authors, ["Example Upstream Authors"]);
/// let rendered = toml::to_string(&locked).unwrap();
/// assert_eq!(locked, toml::from_str::<LockedEmbeddedSource>(&rendered).unwrap());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockedEmbeddedSource {
    pub name: String,
    pub source_url: SourceUrl,
    /// Optional advertised ref used to obtain the exact commit. It is fetch
    /// provenance, never the source identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_ref: Option<String>,
    /// Exact commit verified at fetch time.
    pub resolved_commit: String,
    /// Git tree object owned by `resolved_commit`.
    pub tree_oid: String,
    /// Independent SHA-256 identity of the upstream tree.
    pub content_hash: ContentHash,
    /// Authors of the referenced upstream bytes. This remains distinct from
    /// the owning package's `[package].authors`, which records only package
    /// authorship.
    pub upstream_authors: Vec<String>,
    /// SPDX expression for the upstream bytes. This is distinct from the
    /// adapter package's own `[package].license`.
    pub upstream_license: String,
    /// Portable path of the verified licence file inside the source tree.
    pub license_path: PathBuf,
    /// Immutable public upstream license URL at the resolved commit.
    pub license_url: String,
    /// Hash of the licence file bytes verified inside the authenticated tree.
    pub license_file_sha256: ContentHash,
}

impl LockedEmbeddedSource {
    /// Whether this portable lock row authenticates exactly one authored
    /// declaration. Cache paths are deliberately absent from both sides.
    #[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#lockfile")]
    pub fn matches_declaration(&self, declaration: &crate::manifest::EmbeddedSourceDecl) -> bool {
        self.name == declaration.name
            && self.source_url.as_str() == declaration.url
            && self.source_ref == declaration.ref_hint
            && self.resolved_commit == declaration.commit
            && self.content_hash == declaration.content_hash
            && self.upstream_authors == declaration.upstream_authors
            && self.upstream_license == declaration.upstream_license
            && self.license_path == declaration.license_path
            && self.license_url == declaration.license_url
    }
}
