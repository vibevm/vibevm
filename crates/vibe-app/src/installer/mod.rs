//! Installer children: authored shell commands, verified archives, Windows
//! native installers and hooks. Children inherit stdin and stream their
//! output live; in JSON mode child stdout is relayed to stderr.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#child-output");

pub mod archive;
pub mod hooks;
pub mod native;
pub mod shell;

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use sha2::{Digest, Sha256};

use crate::error::InstallerError;

/// Run one child to completion, streaming its output; non-zero is an error.
pub fn run_child(command: &mut Command, what: &str, json: bool) -> Result<(), InstallerError> {
    command.stdin(Stdio::inherit()).stderr(Stdio::inherit());
    let spawn_error = |error: io::Error| InstallerError::Spawn {
        what: what.into(),
        reason: error.to_string(),
    };
    let status = if json {
        let mut child = command
            .stdout(Stdio::piped())
            .spawn()
            .map_err(spawn_error)?;
        if let Some(mut stdout) = child.stdout.take() {
            let mut stderr = io::stderr().lock();
            let mut buffer = [0u8; 8192];
            loop {
                let read = stdout.read(&mut buffer).map_err(spawn_error)?;
                if read == 0 {
                    break;
                }
                let _ = stderr.write_all(&buffer[..read]);
                let _ = stderr.flush();
            }
        }
        child.wait().map_err(spawn_error)?
    } else {
        command
            .stdout(Stdio::inherit())
            .status()
            .map_err(spawn_error)?
    };
    if status.success() {
        return Ok(());
    }
    Err(InstallerError::Exited {
        what: what.into(),
        code: status.code().map_or_else(
            || "without a code".into(),
            |code| format!("with code {code}"),
        ),
    })
}

/// Copy a declared installer source into `destination`, verifying its
/// SHA-256. A source is an `https://` URL or a path under `package_root`.
pub fn fetch_verified(
    source: &str,
    sha256: &str,
    package_root: &Path,
    destination: &Path,
    offline: bool,
) -> Result<(), InstallerError> {
    let fetch_error = |reason: String| InstallerError::Fetch {
        source_ref: source.into(),
        reason,
    };
    let mut input: Box<dyn Read> =
        if source.starts_with("https://") || source.starts_with("http://") {
            if offline {
                return Err(InstallerError::Offline {
                    source_ref: source.into(),
                });
            }
            let response = reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(600))
                .build()
                .and_then(|client| client.get(source).send())
                .and_then(reqwest::blocking::Response::error_for_status)
                .map_err(|error| fetch_error(error.to_string()))?;
            Box::new(response)
        } else {
            let path = package_file(package_root, source).map_err(fetch_error)?;
            Box::new(File::open(&path).map_err(|error| fetch_error(error.to_string()))?)
        };
    let mut output =
        File::create(destination).map_err(|source| io("creating", destination, source))?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = input
            .read(&mut buffer)
            .map_err(|error| fetch_error(error.to_string()))?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
        output
            .write_all(&buffer[..read])
            .map_err(|source| io("writing", destination, source))?;
    }
    output
        .sync_all()
        .map_err(|source| io("writing", destination, source))?;
    if hex(&hash.finalize()) != sha256.to_ascii_lowercase() {
        let _ = fs::remove_file(destination);
        return Err(InstallerError::Digest {
            source_ref: source.into(),
        });
    }
    Ok(())
}

/// A package-relative file that stays inside the package tree.
fn package_file(package_root: &Path, relative: &str) -> Result<PathBuf, String> {
    if relative.contains("://") || Path::new(relative).is_absolute() {
        return Err("the source must be package-relative or an HTTP(S) URL".into());
    }
    let candidate = fs::canonicalize(package_root.join(relative))
        .map_err(|error| format!("reading the package file: {error}"))?;
    let root = fs::canonicalize(package_root)
        .map_err(|error| format!("reading the package tree: {error}"))?;
    if !candidate.starts_with(&root) || !candidate.is_file() {
        return Err("the source escapes the package tree or is not a file".into());
    }
    Ok(candidate)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(crate) fn io(action: &'static str, path: &Path, source: io::Error) -> InstallerError {
    InstallerError::Io {
        action,
        path: path.display().to_string(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_relative_sources_are_verified_and_contained() {
        let dir = tempfile::tempdir().unwrap();
        let package = dir.path().join("pkg");
        fs::create_dir_all(&package).unwrap();
        fs::write(package.join("payload.bin"), b"payload").unwrap();
        fs::write(dir.path().join("outside.bin"), b"payload").unwrap();
        let digest = hex(&Sha256::digest(b"payload"));
        let out = dir.path().join("out");
        fetch_verified("payload.bin", &digest, &package, &out, true).unwrap();
        assert_eq!(fs::read(&out).unwrap(), b"payload");
        assert!(matches!(
            fetch_verified("payload.bin", &"0".repeat(64), &package, &out, true),
            Err(InstallerError::Digest { .. })
        ));
        assert!(fetch_verified("../outside.bin", &digest, &package, &out, true).is_err());
        assert!(matches!(
            fetch_verified("https://example.invalid/x", &digest, &package, &out, true),
            Err(InstallerError::Offline { .. })
        ));
    }
}
