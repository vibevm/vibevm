//! The version files the home's installed applications declare, kept beside
//! the index as `applications/version-files.json`
//! (PROP-075 `#VERSION-FILE-FORMATS`).
//!
//! Why beside and not inside: the index's wire form is a closed schema whose
//! protocol tag gates every PATH proxy that reads it
//! (PROP-059 `#proxy-index-upgrade`), and a home that declares a version file
//! must not force `vibe-user-applications/7` on proxies that cannot read it.
//! The sidecar is written from the index records on every publication and
//! read back into them on every load, so it never diverges: an uninstalled
//! key simply stops being written, and a key the document names but the index
//! does not is ignored.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS");

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use specmark::spec;
use vibe_core::manifest::VersionFileDecl;
use vibe_wire::generated::application_version_files as wire;

use crate::error::StateError;
use crate::settings::{SettingsRoot, io, write_atomic};

/// The protocol tag of the sidecar document.
pub const VERSION_FILES_PROTOCOL: &str = "vibe-app-version-files/1";

/// `<settings>/applications/version-files.json`.
pub fn path(settings: &SettingsRoot) -> PathBuf {
    settings.applications_dir().join("version-files.json")
}

/// The declared rows by installation key, with each key's slot.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Declared {
    pub installations: BTreeMap<String, (String, Vec<VersionFileDecl>)>,
}

impl Declared {
    /// Read the sidecar; an absent or unreadable document is an empty set,
    /// because a missing declaration degrades to "this slot has no version
    /// file" and never fails a prompt (PROP-075 `#ACTIVATION-HOOK`).
    pub fn load(settings: &SettingsRoot) -> Self {
        let file = path(settings);
        let Ok(bytes) = fs::read(&file) else {
            return Self::default();
        };
        let Ok(document) = serde_json::from_slice::<wire::ApplicationVersionFiles>(&bytes) else {
            return Self::default();
        };
        if document.protocol != VERSION_FILES_PROTOCOL {
            return Self::default();
        }
        Self {
            installations: document
                .installations
                .into_iter()
                .map(|(key, declaration)| {
                    (
                        key,
                        (
                            declaration.slot,
                            declaration.rows.into_iter().map(from_wire).collect(),
                        ),
                    )
                })
                .collect(),
        }
    }

    /// Write the sidecar, or remove it when nothing declares a version file,
    /// so a home that never had one keeps no empty document.
    pub fn save(&self, settings: &SettingsRoot) -> Result<(), StateError> {
        let file = path(settings);
        if self.installations.is_empty() {
            return match fs::remove_file(&file) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(source) => Err(io("removing", &file, source)),
            };
        }
        let document = wire::ApplicationVersionFiles {
            protocol: VERSION_FILES_PROTOCOL.to_owned(),
            installations: self
                .installations
                .iter()
                .map(|(key, (slot, rows))| {
                    (
                        key.clone(),
                        wire::Declaration {
                            slot: slot.clone(),
                            rows: rows.iter().map(to_wire).collect(),
                        },
                    )
                })
                .collect(),
        };
        let mut bytes =
            serde_json::to_vec_pretty(&document).map_err(|error| StateError::Index {
                path: file.display().to_string(),
                reason: error.to_string(),
            })?;
        bytes.push(b'\n');
        write_atomic(&file, &bytes)
    }

    /// The rows of one slot, in the declaration order of the newest
    /// installation that declares them: one slot has one file-name set, and
    /// two providers of the same slot that disagree are merged in key order
    /// with the first declaration of a name winning.
    pub fn for_slot(&self, slot: &str) -> Vec<VersionFileDecl> {
        let mut rows: Vec<VersionFileDecl> = Vec::new();
        for (declared_slot, declared) in self.installations.values() {
            if declared_slot != slot {
                continue;
            }
            for row in declared {
                if !rows.contains(row) {
                    rows.push(row.clone());
                }
            }
        }
        rows
    }

    /// Every slot that declares at least one version file.
    pub fn slots(&self) -> Vec<String> {
        let mut slots: Vec<String> = self
            .installations
            .values()
            .filter(|(_, rows)| !rows.is_empty())
            .map(|(slot, _)| slot.clone())
            .collect();
        slots.sort();
        slots.dedup();
        slots
    }

    /// Every file name any installed application declares, deduplicated, in
    /// the order the declarations give them. This is the fixed list the
    /// prompt hook's builtin walk tests (PROP-075 `#ACTIVATION-HOOK`).
    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        for (_, rows) in self.installations.values() {
            for row in rows {
                for name in row.names() {
                    if !names.iter().any(|seen| seen == name) {
                        names.push(name.to_owned());
                    }
                }
            }
        }
        names
    }

    /// A short, stable fingerprint of the whole declared set, so the
    /// resolution cache and the hook's managed block can tell that the
    /// declared set changed without storing it twice.
    pub fn fingerprint(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        for (key, (slot, rows)) in &self.installations {
            hasher.update(key.as_bytes());
            hasher.update(b"\0");
            hasher.update(slot.as_bytes());
            for row in rows {
                hasher.update(b"\0");
                hasher.update(row.format().as_bytes());
                if let Some(key) = row.key() {
                    hasher.update(b":");
                    hasher.update(key.as_bytes());
                }
                for name in row.names() {
                    hasher.update(b",");
                    hasher.update(name.as_bytes());
                }
            }
            hasher.update(b"\n");
        }
        format!("{:x}", hasher.finalize())
    }
}

fn to_wire(row: &VersionFileDecl) -> wire::Row {
    match row {
        VersionFileDecl::Plain { names } => wire::Row::Plain(Box::new(wire::RowPlain {
            names: names.clone(),
        })),
        VersionFileDecl::ToolVersions { key } => {
            wire::Row::ToolVersions(Box::new(wire::RowToolVersions { key: key.clone() }))
        }
    }
}

fn from_wire(row: wire::Row) -> VersionFileDecl {
    match row {
        wire::Row::Plain(plain) => VersionFileDecl::Plain { names: plain.names },
        wire::Row::ToolVersions(asdf) => VersionFileDecl::ToolVersions { key: asdf.key },
    }
}
