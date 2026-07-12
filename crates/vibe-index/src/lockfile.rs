//! Minimal `vibe.lock` reader — just enough to drive `vibe-index
//! outdated`. Only the `(group, name, version)` identity tuple per
//! `[[package]]` is consumed (PROP-008 §2.2); every other field is
//! parsed tolerantly via `#[serde(flatten)]` into a catch-all.

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-008#identity");

use std::path::Path;

use semver::Version;
use serde::Deserialize;
use vibe_core::Group;

use crate::error::{Error, Result};
use crate::types::PackageKind;

#[derive(Debug, Deserialize)]
pub struct Lockfile {
    #[serde(default)]
    pub package: Vec<LockedPackage>,
    /// Other top-level fields (`meta`, etc.) are accepted but unused.
    #[serde(flatten)]
    pub _extras: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct LockedPackage {
    pub kind: PackageKind,
    /// Reverse-FQDN group — mandatory as of the schema-v5 lockfile
    /// (PROP-008). With `name` it is the package identity.
    pub group: Group,
    pub name: String,
    pub version: Version,
    /// All other fields (registry, source_url, content_hash, …) pass
    /// through into this catch-all so the reader stays tolerant of the
    /// fields it does not itself consume.
    #[serde(flatten)]
    pub _extras: serde_json::Map<String, serde_json::Value>,
}

pub fn read(path: &Path) -> Result<Lockfile> {
    let bytes = std::fs::read(path).map_err(|e| Error::Io {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;
    let s = std::str::from_utf8(&bytes)
        .map_err(|e| Error::Malformed(format!("vibe.lock not UTF-8: {e}")))?;
    toml::from_str::<Lockfile>(s).map_err(|e| Error::Malformed(format!("vibe.lock: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parses_minimal_lockfile() {
        let body = br#"
[meta]
generated_by = "vibe 0.2.0"
schema_version = 5

[[package]]
kind = "flow"
group = "org.vibevm"
name = "wal"
version = "0.1.0"
registry = "vibespecs"
source_url = "git@gitverse.ru:vibespecs/org.vibevm_wal.git"
content_hash = "sha256:abc"

[[package]]
kind = "stack"
group = "org.vibevm"
name = "rust"
version = "0.3.0"
"#;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vibe.lock");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(body).unwrap();
        f.sync_all().unwrap();
        let lock = read(&path).unwrap();
        assert_eq!(lock.package.len(), 2);
        assert_eq!(lock.package[0].kind, PackageKind::Flow);
        assert_eq!(lock.package[0].group.as_str(), "org.vibevm");
        assert_eq!(lock.package[0].name, "wal");
        assert_eq!(lock.package[0].version.to_string(), "0.1.0");
        assert_eq!(lock.package[1].kind, PackageKind::Stack);
    }

    #[test]
    fn parses_empty_lockfile() {
        let body = b"";
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vibe.lock");
        std::fs::write(&path, body).unwrap();
        let lock = read(&path).unwrap();
        assert!(lock.package.is_empty());
    }
}
