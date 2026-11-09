//! The seam between the index and the sidecar that carries the declared
//! version files (PROP-075 `#VERSION-FILE-FORMATS`).
//!
//! The declarations are persisted beside `index.json`, not inside it, so a
//! home that declares a version file keeps writing
//! `vibe-user-applications/6` and every existing PATH proxy goes on reading
//! it (PROP-059 `#proxy-index-upgrade`). The two documents cannot diverge:
//! a load reads the sidecar into the records, and every publication rewrites
//! it from them.

use super::*;
use crate::versionfile::Declared;

impl Index {
    /// Fill each record's declared version files from the sidecar. A key the
    /// sidecar names and the index does not is ignored; a missing or corrupt
    /// document is an empty declared set, which degrades to "this slot has
    /// no version file" and never fails a prompt.
    pub(super) fn read_declared_version_files(&mut self, settings: &SettingsRoot) {
        let declared = Declared::load(settings);
        if declared.installations.is_empty() {
            return;
        }
        for (key, record) in &mut self.installations {
            if let Some((_, rows)) = declared.installations.get(key) {
                record.version_file = rows.clone();
            }
        }
    }

    /// The declared version files of these records, in the shape the sidecar
    /// persists.
    pub(crate) fn declared_version_files(&self) -> Declared {
        Declared {
            installations: self
                .installations
                .iter()
                .filter(|(_, record)| !record.version_file.is_empty())
                .map(|(key, record)| {
                    (
                        key.clone(),
                        (record.slot.clone(), record.version_file.clone()),
                    )
                })
                .collect(),
        }
    }
}
