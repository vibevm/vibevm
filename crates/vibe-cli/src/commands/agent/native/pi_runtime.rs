//! Resolve the Pi package and interpreter belonging to the selected executable.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-HARNESS-EXECUTABLE");

use anyhow::{Result, bail};
use std::path::{Path, PathBuf};

const PACKAGE: &str = "@earendil-works/pi-coding-agent";
pub(super) const VERIFIED_VERSION: &str = "1.5.2";

#[derive(Debug)]
pub(super) struct PiRuntime {
    pub(super) package: PathBuf,
    pub(super) node: PathBuf,
}

pub(super) fn discover(program: &Path) -> Result<PiRuntime> {
    let resolved = std::fs::canonicalize(program)?;
    let mut candidates = resolved
        .ancestors()
        .take(5)
        .map(Path::to_path_buf)
        .collect::<Vec<_>>();
    if let Some(bin) = program.parent() {
        candidates.push(bin.join("node_modules").join(PACKAGE));
        candidates.push(bin.join("../lib/node_modules").join(PACKAGE));
    }
    let package = candidates.into_iter().find(|candidate| {
        std::fs::read(candidate.join("package.json")).ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
            .is_some_and(|meta| meta["name"] == PACKAGE && meta["version"] == VERIFIED_VERSION)
    }).ok_or_else(|| anyhow::anyhow!("selected Pi installation is not the admitted {PACKAGE}@{VERIFIED_VERSION}; no native carrier attached"))?;
    let executable = if cfg!(windows) { "node.exe" } else { "node" };
    for ancestor in package.ancestors() {
        if ancestor.file_name().and_then(|name| name.to_str()) != Some("node_modules") {
            continue;
        }
        let Some(parent) = ancestor.parent() else {
            continue;
        };
        let prefix = if parent.file_name().and_then(|name| name.to_str()) == Some("lib") {
            parent.parent()
        } else {
            Some(parent)
        };
        if let Some(prefix) = prefix {
            for node in [prefix.join("bin").join(executable), prefix.join(executable)] {
                if node.is_file() {
                    return Ok(PiRuntime { package, node });
                }
            }
        }
    }
    // An env-node entrypoint explicitly delegates interpreter selection to PATH.
    let header = std::fs::read(&resolved)?;
    if header.starts_with(b"#!/usr/bin/env node") {
        return Ok(PiRuntime {
            package,
            node: super::super::program::resolve("node")?,
        });
    }
    bail!(
        "the selected Pi installation does not identify its Node interpreter; no native carrier attached"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mismatched_package_refuses_before_runtime_execution() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("pi");
        std::fs::write(&path, "#!/usr/bin/env node\n").unwrap();
        std::fs::write(
            temp.path().join("package.json"),
            r#"{"name":"foreign","version":"1.5.2"}"#,
        )
        .unwrap();
        assert!(
            discover(&path)
                .unwrap_err()
                .to_string()
                .contains("not the admitted")
        );
    }
}
