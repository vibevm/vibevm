//! Captured source-selection identity, separate from serving route and content.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY");
use super::RegistryPin;
use crate::{ContentHash, SourceUrl};
use std::path::PathBuf;

/// Runtime candidate binding. A value is not contact or acquisition authority;
/// the registry retains the actual route in a separate non-forgeable token.
///
/// ```
/// use std::path::PathBuf;
/// use vibe_core::manifest::{CanonicalSourceBinding, RegistryPin};
/// use vibe_core::SourceUrl;
///
/// let pinned = CanonicalSourceBinding::Registry {
///     registry: RegistryPin::new("team".into()).unwrap(),
///     source_url: SourceUrl::new("https://example.invalid/org/pkg.git"),
/// };
/// assert_eq!(pinned.registry_name(), Some("team"));
/// let local = CanonicalSourceBinding::Local { root: PathBuf::from("/srv/pkgs") };
/// assert_eq!(local.registry_name(), None);
/// ```
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum CanonicalSourceBinding {
    Registry {
        registry: RegistryPin,
        source_url: SourceUrl,
    },
    DirectoryRegistry {
        root: PathBuf,
    },
    Local {
        root: PathBuf,
    },
    Embedded {
        root: PathBuf,
    },
    Path {
        root: PathBuf,
    },
    Git {
        source_url: SourceUrl,
        source_ref: Option<String>,
    },
    Override {
        source_url: SourceUrl,
        source_ref: Option<String>,
    },
    Locked {
        registry: Option<String>,
        source_url: SourceUrl,
        source_ref: Option<String>,
        source_kind: Option<crate::manifest::SourceKind>,
        resolved_commit: Option<String>,
        content_hash: ContentHash,
        via_redirect: Option<SourceUrl>,
    },
}
impl CanonicalSourceBinding {
    /// Capture all held source constraints, retaining authoritative absence.
    pub fn from_locked(row: &crate::manifest::LockedPackage) -> Self {
        Self::Locked {
            registry: row.registry.clone(),
            source_url: row.source_url.clone(),
            source_ref: row.source_ref.clone(),
            source_kind: row.source_kind,
            resolved_commit: row.resolved_commit.clone(),
            content_hash: row.content_hash.clone(),
            via_redirect: row.via_redirect.clone().map(SourceUrl::new),
        }
    }

    /// Registry constraint identity, including a held alias absent from today's catalog.
    pub fn registry_name(&self) -> Option<&str> {
        match self {
            Self::Registry { registry, .. } => Some(registry.as_str()),
            Self::Locked { registry, .. } => registry.as_deref(),
            _ => None,
        }
    }
}
impl std::fmt::Debug for CanonicalSourceBinding {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self {
            Self::Registry { .. } => "registry",
            Self::DirectoryRegistry { .. } => "directory-registry",
            Self::Local { .. } => "local",
            Self::Embedded { .. } => "embedded",
            Self::Path { .. } => "path",
            Self::Git { .. } => "git",
            Self::Override { .. } => "override",
            Self::Locked { .. } => "locked",
        };
        formatter
            .debug_struct("CanonicalSourceBinding")
            .field("kind", &kind)
            .field("registry", &self.registry_name())
            .finish_non_exhaustive()
    }
}
