//! Content-addressed package identity.
//!
//! Spec: [PROP-008 §2.2](../../../vibevm/vibespecs/modules/vibe-registry/PROP-008-qualified-naming.xml#identity)
//! (the `(group, name, version, content_hash)` identity tuple),
//! [PROP-002 §2.1](../../../vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml#identity)
//! (content addressing).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#identity");

use std::fmt;

use serde::{Deserialize, Serialize};

/// The `sha256:<hex>` content hash over a package's file tree — the
/// **identity** component of the `(group, name, version, content_hash)`
/// tuple (PROP-002 §2.1). It is what an integrity check keys off, so a
/// mirror-switch or host-migration that changes `source_url` but not the
/// bytes leaves identity intact.
///
/// The wire form is the bare `sha256:…` string the lockfile already carries,
/// and `serde(try_from = "String", into = "String")` keeps it exactly that
/// while running [`parse`] at the boundary — the spelling `Group` has used all
/// along. It was `serde(transparent)` until 2026-08-05, on a reason that only
/// ever justified the wire SHAPE: `transparent` and `try_from`/`into` emit the
/// same bare string, and only one of them notices a malformed value arriving.
/// [`parse`] accepts TWO prefixes now (PROP-044 §4.7): the bare `sha256:`
/// (recipe 0, what every lockfile in existence already carries) and
/// `sha256-tree/1:` (recipe 1, the index-side hasher's new label); a reader
/// accepts both for the same reason the recipe id exists at all — a value
/// must say how it was computed, and old values said it by omission.
/// The newtype's other job is keeping the identity hash from being confused
/// with the many other strings around it (`source_url`, `source_ref`,
/// `resolved_commit`); [`from_validated`] still wraps a hash a trusted producer
/// (`vibe-index`'s `compute_content_hash`) already emitted, and there is
/// deliberately no `From<String>` — an unchecked constructor next to a checked
/// one is the back door the boundary check exists to close.
///
/// ```
/// use vibe_core::ContentHash;
///
/// let h = ContentHash::parse("sha256:e3b0c44298fc1c14").unwrap();
/// assert_eq!(h.as_str(), "sha256:e3b0c44298fc1c14");
/// // Recipe 1 (the index-side label) parses too — old and new forms are both
/// // readable, and the value records which recipe made it.
/// assert!(ContentHash::parse("sha256-tree/1:e3b0c44298fc1c14").is_ok());
/// assert!(ContentHash::parse("md5:whatever").is_err()); // unknown algorithm
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ContentHash(String);

impl ContentHash {
    /// The required algorithm prefix. vibevm hashes package trees with
    /// SHA-256; the prefix makes the algorithm explicit and future-proofs
    /// the format against an algorithm change.
    pub const PREFIX: &'static str = "sha256:";

    /// Every algorithm prefix `parse` accepts, newest first. `sha256:` is
    /// recipe 0 — the pre-recipe form, still written by the registry-side
    /// hasher and by every lockfile in existence; `sha256-tree/1:` is recipe 1
    /// (PROP-044 §4.7), emitted by the index. A reader accepts both for the
    /// same reason the recipe id exists at all: a value must say how it was
    /// computed, and old values said it by omission.
    pub const ACCEPTED_PREFIXES: &'static [&'static str] = &["sha256-tree/1:", "sha256:"];

    /// Parse a `<algo-prefix><hex>` hash, checking the algorithm prefix
    /// (one of [`ACCEPTED_PREFIXES`]) and that the digest is non-empty hex.
    /// Lenient on length — test fixtures and truncated-display hashes are
    /// accepted as long as the shape is right.
    pub fn parse(input: &str) -> Result<Self, crate::Error> {
        for prefix in Self::ACCEPTED_PREFIXES {
            if let Some(hex) = input.strip_prefix(prefix) {
                if !Self::hex_tail_is_valid(hex) {
                    return Err(crate::Error::BadContentHash {
                        input: input.to_owned(),
                        reason: "the digest after the prefix must be non-empty hexadecimal".into(),
                    });
                }
                return Ok(ContentHash(input.to_owned()));
            }
        }
        Err(crate::Error::BadContentHash {
            input: input.to_owned(),
            reason: format!(
                "missing a recognised algorithm prefix; expected one of {}",
                Self::ACCEPTED_PREFIXES.join(" / ")
            ),
        })
    }

    /// Whether one borrowed spelling is a legal `<algo-prefix><hex>` hash.
    ///
    /// The same grammar [`parse`](Self::parse) enforces, as a borrowed
    /// predicate: it accepts exactly the spellings `parse` accepts (both
    /// recipe prefixes, non-empty hexadecimal tail) and refuses exactly the
    /// rest, sharing the law through [`Self::hex_tail_is_valid`] so the two
    /// cannot drift. Revalidating callers that already hold a
    /// `ContentHash`-shaped `&str` — the R4.1 plan refusal path, where the
    /// spelling can be attacker-sized — call this instead of `parse` and
    /// never allocate the error clone `parse` would build.
    pub fn is_valid_spelling(input: &str) -> bool {
        for prefix in Self::ACCEPTED_PREFIXES {
            if let Some(hex) = input.strip_prefix(prefix) {
                return Self::hex_tail_is_valid(hex);
            }
        }
        false
    }

    /// The one grammar core shared by `parse` and `is_valid_spelling`: a
    /// digest tail is legal when non-empty and fully hexadecimal.
    fn hex_tail_is_valid(hex: &str) -> bool {
        !hex.is_empty() && hex.bytes().all(|b| b.is_ascii_hexdigit())
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

impl From<ContentHash> for String {
    fn from(h: ContentHash) -> String {
        h.0
    }
}

impl TryFrom<String> for ContentHash {
    type Error = crate::Error;

    fn try_from(s: String) -> std::result::Result<Self, Self::Error> {
        ContentHash::parse(&s)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::LockedPackage;

    #[test]
    fn parse_accepts_both_recipe_labels() {
        // Recipe 0 (legacy) and recipe 1 (tree) both parse — old values stay
        // readable, new values carry their recipe (PROP-044 §4.7).
        let legacy = ContentHash::parse("sha256:e3b0c44298fc1c14").unwrap();
        assert_eq!(legacy.as_str(), "sha256:e3b0c44298fc1c14");
        let tree = ContentHash::parse("sha256-tree/1:e3b0c44298fc1c14").unwrap();
        assert_eq!(tree.as_str(), "sha256-tree/1:e3b0c44298fc1c14");
    }

    #[test]
    fn parse_rejects_unknown_algorithm() {
        // A third algorithm — or no prefix at all — is rejected.
        assert!(ContentHash::parse("md5:d41d8cd98f00b204").is_err());
        assert!(ContentHash::parse("e3b0c44298fc1c14").is_err());
    }

    #[test]
    fn parse_rejects_empty_or_non_hex_tail() {
        assert!(ContentHash::parse("sha256:").is_err());
        assert!(ContentHash::parse("sha256-tree/1:").is_err());
        assert!(ContentHash::parse("sha256:nothex").is_err());
    }

    #[test]
    fn locked_package_with_legacy_hash_reads_as_today() {
        // A lockfile entry carrying the pre-recipe `sha256:` form deserialises
        // unchanged — the old value said its recipe by omission, and the
        // reader still honours that. This is the shape every lockfile already
        // on disk has.
        let toml_src = r#"
            kind = "flow"
            name = "wal"
            group = "org.vibevm"
            version = "0.3.0"
            registry = "vibespecs"
            source_url = "git@gitverse.ru:vibespecs/flow-wal.git"
            content_hash = "sha256:abc"
            source_kind = "registry"
        "#;
        let p: LockedPackage = toml::from_str(toml_src).unwrap();
        assert_eq!(p.content_hash.as_str(), "sha256:abc");
    }

    #[test]
    fn locked_package_accepts_recipe_1_hash() {
        // The new label round-trips through the lockfile shape too.
        let toml_src = r#"
            kind = "flow"
            name = "wal"
            group = "org.vibevm"
            version = "0.3.0"
            registry = "vibespecs"
            source_url = "git@gitverse.ru:vibespecs/flow-wal.git"
            content_hash = "sha256-tree/1:abc"
            source_kind = "registry"
        "#;
        let p: LockedPackage = toml::from_str(toml_src).unwrap();
        assert_eq!(p.content_hash.as_str(), "sha256-tree/1:abc");
    }

    #[test]
    fn each_form_survives_a_string_round_trip() {
        // `into = "String"` emits exactly the bytes that arrived, so a hash's
        // recipe survives a lockfile write+read cycle.
        for input in ["sha256:abc", "sha256-tree/1:abc"] {
            let h = ContentHash::parse(input).unwrap();
            let wire: String = h.into();
            assert_eq!(wire, input);
        }
    }

    #[test]
    fn is_valid_spelling_agrees_with_parse_on_every_corpus_spelling() {
        // The borrowed predicate accepts exactly what `parse` accepts and
        // refuses exactly the rest — the shared grammar core cannot drift.
        let corpus = [
            ("sha256:abc", true),
            ("sha256:", false),
            ("sha256:nothex", false),
            ("sha256:A0b1C2", true),
            ("sha256-tree/1:abc", true),
            ("sha256-tree/1:", false),
            ("sha256-tree/1:zz", false),
            ("md5:d41d8cd98f00b204", false),
            ("e3b0c44298fc1c14", false),
            ("", false),
        ];
        for (input, legal) in corpus {
            assert_eq!(
                ContentHash::is_valid_spelling(input),
                legal,
                "spelling {input:?} classified wrong"
            );
            assert_eq!(
                ContentHash::parse(input).is_ok(),
                legal,
                "parse disagrees with is_valid_spelling on {input:?}"
            );
        }
    }
}
