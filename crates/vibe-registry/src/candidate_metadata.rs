//! Evidence captured at the source's actual metadata read.
use crate::RegistryError;
use crate::shippable::compute_content_hash_checked as compute_content_hash;
use sha2::{Digest, Sha256};
use std::path::Path;
use vibe_core::manifest::Manifest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateIdentity {
    pub source: String,
    /// `git:<commit>` or the incumbent shippable-tree hash.
    pub content: String,
    pub definition: String,
}

#[derive(Debug, Clone)]
pub struct CandidateMetadata {
    pub identity: CandidateIdentity,
    pub manifest: Manifest,
}

impl CandidateMetadata {
    pub fn read_local_registry(directory: &Path) -> Result<Self, RegistryError> {
        Self::read_local(
            directory,
            crate::local_registry::source_uri_for_local(directory),
        )
    }

    pub(crate) fn from_bytes(
        source: String,
        content: String,
        bytes: &[u8],
    ) -> Result<Self, RegistryError> {
        let source = crate::git_backend::shell::without_userinfo(&source);
        let text = std::str::from_utf8(bytes).map_err(|e| invalid(&source, e))?;
        let manifest = Manifest::parse_str(text).map_err(|e| invalid(&source, e))?;
        manifest
            .require_package()
            .map_err(|e| invalid(&source, e))?;
        let mut hash = Sha256::new();
        hash.update(b"vibe-manifest-legacy-r1\0");
        hash.update(bytes);
        Ok(Self {
            identity: CandidateIdentity {
                source,
                content,
                definition: format!("sha256:{:x}", hash.finalize()),
            },
            manifest,
        })
    }

    /// Check acquired bytes before a store insertion. Immutable Git objects
    /// authenticate the tree; local sources use the incumbent tree hash.
    pub(crate) fn verify_checkout(
        &self,
        directory: &Path,
        commit: Option<&str>,
    ) -> Result<(), RegistryError> {
        let content = match self.identity.content.strip_prefix("git:") {
            Some(expected) if commit == Some(expected) => self.identity.content.clone(),
            Some(_) => {
                return Err(invalid(
                    &self.identity.source,
                    "checkout differs from catalog commit",
                ));
            }
            None => compute_content_hash(directory)?,
        };
        let path = directory.join(Manifest::FILENAME);
        let bytes = std::fs::read(&path).map_err(|source| RegistryError::Io { path, source })?;
        let current = Self::from_bytes(self.identity.source.clone(), content, &bytes)?;
        if current.identity != self.identity {
            return Err(invalid(
                &self.identity.source,
                "acquired content differs from catalog",
            ));
        }
        Ok(())
    }

    /// Capture a local definition and verify its content binding across the read.
    /// No store insertion or install effect occurs here.
    pub fn read_local(directory: &Path, source: String) -> Result<Self, RegistryError> {
        let before = compute_content_hash(directory)?;
        let path = directory.join(Manifest::FILENAME);
        let bytes = std::fs::read(&path).map_err(|source| RegistryError::Io {
            path: path.clone(),
            source,
        })?;
        let metadata = Self::from_bytes(source.clone(), before.clone(), &bytes)?;
        let again = std::fs::read(&path).map_err(|source| RegistryError::Io { path, source })?;
        if bytes != again || before != compute_content_hash(directory)? {
            return Err(invalid(
                &source,
                "candidate changed during metadata capture",
            ));
        }
        Ok(metadata)
    }
}

fn invalid(source: &str, reason: impl std::fmt::Display) -> RegistryError {
    RegistryError::MalformedMeta {
        path: source.into(),
        reason: reason.to_string(),
    }
}

/// Stable length-delimited encoding of the inspected, ordered candidate rows.
pub fn catalog_digest(
    rows: &[(String, CandidateIdentity)],
    inventory: &[(String, String)],
) -> String {
    let mut hash = Sha256::new();
    hash.update(b"vibe-candidate-catalog-r1\0");
    hash.update((rows.len() as u64).to_be_bytes());
    for (coordinate, identity) in rows {
        for field in [
            coordinate,
            &identity.source,
            &identity.content,
            &identity.definition,
        ] {
            hash.update((field.len() as u64).to_be_bytes());
            hash.update(field.as_bytes());
        }
    }
    hash.update((inventory.len() as u64).to_be_bytes());
    for (coordinate, availability) in inventory {
        for field in [coordinate, availability] {
            hash.update((field.len() as u64).to_be_bytes());
            hash.update(field.as_bytes());
        }
    }
    format!("sha256:{:x}", hash.finalize())
}
