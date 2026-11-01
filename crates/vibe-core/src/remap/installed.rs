//! The remap packages a home has installed (`PROP-066#package`).
//!
//! `<home>/remap/packages/` holds one directory per installed remap
//! package — the package tree as it was published, so `remap export` can
//! hand it on and `remap list` can show the offer without a registry — and
//! `<home>/remap/installed.json` is the index over them. The index carries
//! the offer itself (description, priority, rows, policy) because every
//! command that asks "what would this package apply?" asks it on a path
//! where reading and parsing N manifests would be the slow part of a
//! `vibe install`.
//!
//! The trees beside it are the authority for what each offer says — the
//! index is a cache of it, rewritten whole on every mutation of the home —
//! and the content hash each record carries is the registry's, computed
//! over the published tree, never a second hash computed over the copy.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#package");

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specmark::spec;

use crate::manifest::Manifest;

use super::consent::{ConsentError, remap_dir};
use super::package::RemapPackage;

/// `<home>/remap/packages` — one subdirectory per installed package.
pub fn packages_dir(home: &Path) -> PathBuf {
    remap_dir(home).join("packages")
}

/// `<home>/remap/packages/<group>__<name>` — the tree of one installed
/// package. The group and the name are joined rather than nested so the
/// directory name is the package key and a listing of the directory is a
/// listing of the installed set.
pub fn package_dir(home: &Path, key: &str) -> PathBuf {
    packages_dir(home).join(key.replace('/', "__"))
}

/// `<home>/remap/installed.json`.
pub fn installed_path(home: &Path) -> PathBuf {
    remap_dir(home).join("installed.json")
}

/// One installed package's record: the offer plus where its tree is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#package")]
pub struct InstalledRemap {
    /// The offer — key, version, content hash, rows, policy.
    #[serde(flatten)]
    pub package: RemapPackage,
    /// The registry that served it, when one is known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
    /// When it was installed, RFC 3339.
    pub installed: String,
}

/// The index of one home's installed remap packages.
///
/// ```
/// use vibe_core::remap::{InstalledRemaps, InstalledRemap, RemapPackage, RemapSection};
///
/// let mut index = InstalledRemaps::default();
/// assert!(index.packages.is_empty());
/// index.upsert(InstalledRemap {
///     package: RemapPackage {
///         key: "org.corp/intranet".into(),
///         version: "1.0.0".into(),
///         content_hash: "sha256:ab".into(),
///         section: RemapSection::default(),
///         policy: Default::default(),
///     },
///     registry: None,
///     installed: "2026-11-01T00:00:00Z".into(),
/// });
/// assert!(index.find("org.corp/intranet").is_some());
/// // A second install of the same key replaces the record rather than
/// // adding a second one: a home holds one version of a package.
/// assert_eq!(index.packages.len(), 1);
/// assert!(index.remove("org.corp/intranet").is_some());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-066#package")]
pub struct InstalledRemaps {
    /// The installed packages, sorted by key.
    #[serde(default)]
    pub packages: Vec<InstalledRemap>,
}

impl InstalledRemaps {
    /// Read the index of `home`. A missing file is an empty index.
    pub fn load(home: &Path) -> Result<Self, ConsentError> {
        let path = installed_path(home);
        match std::fs::read_to_string(&path) {
            Ok(body) => serde_json::from_str(&body).map_err(|source| ConsentError::Malformed {
                path: path.display().to_string(),
                source,
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(ConsentError::Io {
                action: "reading",
                path: path.display().to_string(),
                source,
            }),
        }
    }

    /// Write the index of `home`, replacing it atomically.
    pub fn store(&self, home: &Path) -> Result<(), ConsentError> {
        let path = installed_path(home);
        let body =
            serde_json::to_string_pretty(self).map_err(|source| ConsentError::Malformed {
                path: path.display().to_string(),
                source,
            })?;
        super::consent::write_map(&path, format!("{body}\n").as_bytes())
    }

    /// The record for `key`, if the home has one.
    pub fn find(&self, key: &str) -> Option<&InstalledRemap> {
        self.packages.iter().find(|entry| entry.package.key == key)
    }

    /// Add or replace a record, keeping the index sorted by key.
    pub fn upsert(&mut self, entry: InstalledRemap) {
        self.packages
            .retain(|existing| existing.package.key != entry.package.key);
        self.packages.push(entry);
        self.packages
            .sort_by(|left, right| left.package.key.cmp(&right.package.key));
    }

    /// Drop the record for `key`, returning it.
    pub fn remove(&mut self, key: &str) -> Option<InstalledRemap> {
        let found = self
            .packages
            .iter()
            .position(|entry| entry.package.key == key)?;
        Some(self.packages.remove(found))
    }

    /// Just the offers, in index order — what the Locator and the provider
    /// map are built from.
    pub fn offers(&self) -> Vec<RemapPackage> {
        let mut offers: Vec<RemapPackage> =
            self.packages.iter().map(|e| e.package.clone()).collect();
        // Higher `priority` wins among accepted packages
        // (`PROP-066#package`), and `PROP-066#precedence` breaks a tie
        // within one layer by the later row — so the highest priority is
        // loaded last.
        offers.sort_by(|left, right| {
            left.section
                .priority
                .cmp(&right.section.priority)
                .then_with(|| left.key.cmp(&right.key))
        });
        offers
    }
}

/// Read the offer out of a package tree: its `[[remap]]` rows, its
/// `[remap]` description and priority, and its `[policy]`
/// (`PROP-066#package`).
///
/// `content_hash` is the caller's, because the hash that acceptance binds
/// to is the one the registry computed over the published tree — not a
/// second hash computed here over a copy.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#package")]
pub fn offer_from_tree(tree: &Path, content_hash: &str) -> Result<RemapPackage, String> {
    let manifest = Manifest::read(tree.join(Manifest::FILENAME))
        .map_err(|error| format!("reading the package manifest: {error}"))?;
    offer_from_manifest(&manifest, content_hash)
}

/// The same, from an already parsed manifest.
pub fn offer_from_manifest(
    manifest: &Manifest,
    content_hash: &str,
) -> Result<RemapPackage, String> {
    let meta = manifest
        .package
        .as_ref()
        .ok_or_else(|| "the manifest carries no [package] table".to_owned())?;
    Ok(RemapPackage {
        key: format!("{}/{}", meta.group.as_str(), meta.name),
        version: meta.version.to_string(),
        content_hash: content_hash.to_owned(),
        section: manifest.remap.clone(),
        policy: manifest.policy,
    })
}
