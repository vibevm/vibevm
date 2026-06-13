//! Content-addressed package identity.
//!
//! Spec: [PROP-008 §2.2](../../../spec/modules/vibe-registry/PROP-008-qualified-naming.md#identity)
//! (the `(group, name, version, content_hash)` identity tuple),
//! [PROP-002 §2.1](../../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#identity)
//! (content addressing).

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-008#identity");

use std::fmt;

use serde::{Deserialize, Serialize};

/// The `sha256:<hex>` content hash over a package's file tree — the
/// **identity** component of the `(group, name, version, content_hash)`
/// tuple (PROP-002 §2.1). It is what an integrity check keys off, so a
/// mirror-switch or host-migration that changes `source_url` but not the
/// bytes leaves identity intact.
///
/// `serde(transparent)`: the wire form is the bare `sha256:…` string the
/// lockfile already carries. The newtype's value is keeping the identity
/// hash from being confused with the many other strings around it
/// (`source_url`, `source_ref`, `resolved_commit`); [`parse`] checks the
/// algorithm prefix, while [`from_validated`] wraps a hash a trusted
/// producer (`vibe-index`'s `compute_content_hash`) already emitted.
///
/// ```
/// use vibe_core::ContentHash;
///
/// let h = ContentHash::parse("sha256:e3b0c44298fc1c14").unwrap();
/// assert_eq!(h.as_str(), "sha256:e3b0c44298fc1c14");
/// assert!(ContentHash::parse("md5:whatever").is_err()); // wrong algorithm
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContentHash(String);

impl ContentHash {
    /// The required algorithm prefix. vibevm hashes package trees with
    /// SHA-256; the prefix makes the algorithm explicit and future-proofs
    /// the format against an algorithm change.
    pub const PREFIX: &'static str = "sha256:";

    /// Parse a `sha256:<hex>` hash, checking the algorithm prefix and that
    /// the digest is non-empty lowercase hex. Lenient on length — test
    /// fixtures and truncated-display hashes are accepted as long as the
    /// shape is right.
    pub fn parse(input: &str) -> Result<Self, crate::Error> {
        let Some(hex) = input.strip_prefix(Self::PREFIX) else {
            return Err(crate::Error::BadContentHash {
                input: input.to_owned(),
                reason: format!("missing the `{}` algorithm prefix", Self::PREFIX),
            });
        };
        if hex.is_empty() || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(crate::Error::BadContentHash {
                input: input.to_owned(),
                reason: "the digest after the prefix must be non-empty hexadecimal".into(),
            });
        }
        Ok(ContentHash(input.to_owned()))
    }

    /// Wrap a hash already produced by a trusted hasher
    /// (`vibe-index::compute_content_hash`), skipping the re-check.
    pub fn from_validated(hash: String) -> Self {
        ContentHash(hash)
    }

    /// The full `sha256:<hex>` string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::ops::Deref for ContentHash {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl From<String> for ContentHash {
    fn from(s: String) -> Self {
        ContentHash(s)
    }
}

impl From<ContentHash> for String {
    fn from(h: ContentHash) -> String {
        h.0
    }
}

impl AsRef<str> for ContentHash {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl PartialEq<str> for ContentHash {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for ContentHash {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}
