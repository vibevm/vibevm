//! Installer children: authored shell commands, verified archives, Windows
//! native installers and hooks. Children inherit stdin and stream their
//! output live; in JSON mode child stdout is relayed to stderr.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#child-output");

pub mod archive;
mod child;
pub mod native;
pub mod programs;
pub mod shell;
pub mod source_build;

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use sha2::{Digest, Sha256};
use vibe_core::progress::ProgressTask;

use crate::error::InstallerError;

pub use child::Interrupt;
pub(crate) use child::{ChildIo, Relay, run_attached, run_child};

/// What one fetch reports and refuses (`PROP-065#sizes`): the declared
/// size, the step task its byte measurements go to, and the interrupt it
/// stops on between chunks.
#[derive(Clone, Copy, Default)]
pub(crate) struct FetchWatch<'a> {
    pub size: Option<u64>,
    pub task: Option<&'a ProgressTask>,
    pub interrupt: Option<&'a dyn Interrupt>,
}

/// Bytes between two measurements of one fetch.
const MEASURE_EVERY: u64 = 1 << 20;

/// Copy a declared installer source into `destination`, verifying its
/// SHA-256 and its declared size. A source is an `https://` URL or a path
/// under `package_root`.
pub(crate) fn fetch_verified(
    source: &str,
    sha256: &str,
    package_root: &Path,
    destination: &Path,
    offline: bool,
    watch: &FetchWatch<'_>,
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
    let (mut total, mut measured) = (0u64, 0u64);
    let stop = |error: InstallerError| {
        let _ = fs::remove_file(destination);
        error
    };
    loop {
        if watch
            .interrupt
            .is_some_and(|interrupt| interrupt.poll(None))
        {
            return Err(stop(InstallerError::Interrupted {
                what: format!("fetching `{source}`"),
            }));
        }
        let read = input
            .read(&mut buffer)
            .map_err(|error| stop(fetch_error(error.to_string())))?;
        if read == 0 {
            break;
        }
        total += read as u64;
        if let Some(declared) = watch.size.filter(|declared| total > *declared) {
            return Err(stop(InstallerError::Oversize {
                source_ref: source.into(),
                declared,
            }));
        }
        hash.update(&buffer[..read]);
        output
            .write_all(&buffer[..read])
            .map_err(|source| io("writing", destination, source))?;
        if let Some(task) = watch.task
            && total - measured >= MEASURE_EVERY
        {
            task.set_progress(total, watch.size, "bytes");
            measured = total;
        }
    }
    if let Some(task) = watch.task {
        task.set_progress(total, watch.size, "bytes");
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
        let watch = FetchWatch::default();
        fetch_verified("payload.bin", &digest, &package, &out, true, &watch).unwrap();
        assert_eq!(fs::read(&out).unwrap(), b"payload");
        assert!(matches!(
            fetch_verified("payload.bin", &"0".repeat(64), &package, &out, true, &watch),
            Err(InstallerError::Digest { .. })
        ));
        assert!(fetch_verified("../outside.bin", &digest, &package, &out, true, &watch).is_err());
        assert!(matches!(
            fetch_verified(
                "https://example.invalid/x",
                &digest,
                &package,
                &out,
                true,
                &watch
            ),
            Err(InstallerError::Offline { .. })
        ));
    }

    #[test]
    fn a_declared_size_is_measured_and_never_exceeded() {
        use std::sync::{Arc, Mutex};
        use vibe_core::progress::{Progress, ProgressEvent, ProgressEventKind, ProgressObserver};

        struct Bytes(Mutex<Vec<(u64, Option<u64>)>>);
        impl ProgressObserver for Bytes {
            fn observe(&self, event: ProgressEvent) {
                if let ProgressEventKind::Progress {
                    completed, total, ..
                } = event.kind
                {
                    self.0.lock().unwrap().push((completed, total));
                }
            }
        }
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("payload.bin"), b"payload").unwrap();
        let digest = hex(&Sha256::digest(b"payload"));
        let out = dir.path().join("out");
        let bytes = Arc::new(Bytes(Mutex::new(Vec::new())));
        let task = Progress::new(bytes.clone()).task("fetch");
        let exact = FetchWatch {
            size: Some(7),
            task: Some(&task),
            interrupt: None,
        };
        fetch_verified("payload.bin", &digest, dir.path(), &out, true, &exact).unwrap();
        assert_eq!(bytes.0.lock().unwrap().as_slice(), [(7, Some(7))]);
        let small = FetchWatch {
            size: Some(6),
            ..exact
        };
        let error = fetch_verified("payload.bin", &digest, dir.path(), &out, true, &small);
        assert!(matches!(
            error,
            Err(InstallerError::Oversize { declared: 6, .. })
        ));
        assert!(!out.exists(), "an oversize download is not kept");
    }
}
