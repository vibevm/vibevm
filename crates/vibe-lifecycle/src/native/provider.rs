//! Provider identity/root projection for native candidates.

use std::path::{Path, PathBuf};

use crate::{ExtensionProvider, ExtensionRegistryRow};

/// Which relative-root vocabulary a native provider owns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProviderHome {
    Dependency,
    Host,
}

/// Owned provider facts retained across grouping and Cargo execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProviderFacts {
    pub(super) identity: String,
    pub(super) root: PathBuf,
    pub(super) version: String,
    pub(super) content_hash: Option<String>,
    pub(super) home: ProviderHome,
}

impl ProviderFacts {
    pub(super) fn root(&self) -> &Path {
        &self.root
    }
}

pub(super) fn facts(row: &ExtensionRegistryRow) -> ProviderFacts {
    match row.provider() {
        ExtensionProvider::Dependency(provider) => ProviderFacts {
            identity: provider.id.to_string(),
            root: provider.root.clone(),
            version: provider.version.clone(),
            content_hash: Some(provider.content_hash.to_string()),
            home: ProviderHome::Dependency,
        },
        ExtensionProvider::Host(provider) => ProviderFacts {
            identity: provider.identity.to_string(),
            root: provider.root.clone(),
            version: provider.version.clone(),
            content_hash: provider.content_hash.as_ref().map(ToString::to_string),
            home: ProviderHome::Host,
        },
    }
}
