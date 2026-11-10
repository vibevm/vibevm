//! Installer children: authored shell commands, verified archives, Windows
//! native installers and hooks. Children inherit stdin and stream their
//! output live; in JSON mode child stdout is relayed to stderr, and under a
//! step panel they get neither (`PROP-065#display`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#child-output");

pub mod archive;
mod child;
mod declared_input;
mod drop;
mod fetch;
pub mod native;
pub mod programs;
pub mod shell;
pub mod source_build;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use vibe_core::progress::ProgressTask;
use vibe_core::remap::{
    Artifact, ByteLocation, Candidate, Locator, PathLookup, RemapRow, RemapTarget, Resolution,
};

use crate::error::InstallerError;

pub(crate) use child::{ChildIo, Relay, Released, run_attached, run_child};
pub use child::{Interrupt, StepLogs, StepPanel};
pub use declared_input::{DeclaredInput, fetch_declared_input};
pub(crate) use drop::{copy_from_drop, copy_from_drop_named};
use drop::{declared_origin_refused, first_rung};
pub(crate) use fetch::{TransferFacts, fetch_verified};

/// What one fetch reports and refuses (`PROP-065#sizes`): the declared
/// size, the step task its transfer's samples go to, the interrupt it
/// stops on between chunks, what the journal says the bytes are for, and
/// the URL the manifest named when a remap row fetched another
/// (`PROP-068#redaction`).
#[derive(Clone, Copy, Default)]
pub(crate) struct FetchWatch<'a> {
    pub http_client: Option<&'a vibe_package_source::OperationHttpClient>,
    pub size: Option<u64>,
    pub task: Option<&'a ProgressTask>,
    pub interrupt: Option<&'a dyn Interrupt>,
    pub facts: Option<&'a TransferFacts>,
    pub purpose: Option<vibe_core::progress::transfer::Purpose>,
    pub remapped_from: Option<&'a str>,
}

/// Which candidate served one input's bytes, and the row that chose it
/// (`PROP-066#provenance`). Recorded in `steps.json` and the run record so
/// the canonical origin in the package stays untouched and the place the
/// bytes actually came from is still recoverable afterwards.
///
/// ```
/// let served = vibe_app::ServedBy {
///     input: "upstream".into(),
///     target: "/srv/artifacts".into(),
///     row: None,
/// };
/// assert!(served.row.is_none(), "the declared origin carries no row");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServedBy {
    /// The input's name — a `sources` entry's `name`, else the step id.
    pub input: String,
    /// The candidate the bytes came from.
    pub target: String,
    /// The row that chose it; `None` for the declared origin.
    pub row: Option<ServedRow>,
}

/// The row a [`ServedBy`] credits, flattened for the record.
///
/// ```
/// let row = vibe_app::ServedRow {
///     layer: "local".into(),
///     source: "~/.vibe/settings.toml".into(),
///     index: 0,
///     reason: Some("air-gapped build host".into()),
/// };
/// assert_eq!(row.index, 0);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServedRow {
    /// The layer label of `PROP-066#layers`.
    pub layer: String,
    /// The file or package that declared the row.
    pub source: String,
    /// The row's position in that source.
    pub index: usize,
    /// The row's `reason`.
    pub reason: Option<String>,
}

/// What one input is, as the Locator is asked about it: the resolution
/// point, the artifact's identity, the declared source as the package
/// wrote it, the digest every candidate is verified against, and the file
/// name a `path` candidate looks for under `<path>/<sha256>/`.
pub(crate) struct Located<'a> {
    pub locator: &'a Locator,
    pub artifact: &'a Artifact,
    pub declared: &'a str,
    pub sha256: &'a str,
    pub filename: Option<&'a str>,
}

/// Fetch one declared input through the Locator (`PROP-066#locator`): the
/// remapped candidates are tried in winning order, the declared origin
/// last, and each one is verified against the **unchanged** `sha256`, so a
/// remap can only change where the bytes came from.
///
/// A `deny` row — or `unmatched = "deny"` with nothing matching — refuses
/// before any connection with the code `remap_denied`. When every
/// candidate fails, the error names each one and why.
pub(crate) fn fetch_located(
    input: &Located<'_>,
    package_root: &Path,
    destination: &Path,
    offline: bool,
    watch: &FetchWatch<'_>,
) -> Result<ServedBy, InstallerError> {
    let Located {
        locator,
        artifact,
        declared,
        sha256,
        filename,
    } = *input;
    let candidates = match locator.locate(artifact) {
        Resolution::Denied { row, reason } => {
            return Err(InstallerError::RemapDenied {
                source_ref: if watch.http_client.is_some() {
                    vibe_core::progress::transfer::safe_remote(declared)
                } else {
                    declared.into()
                },
                row: row.map_or_else(
                    || "the active `[policy] unmatched = \"deny\"`".to_owned(),
                    |row| {
                        format!(
                            "row {} of `{}` ({})",
                            row.provenance.index + 1,
                            row.provenance.source,
                            row.provenance.layer.label()
                        )
                    },
                ),
                reason: reason.unwrap_or_else(|| "no reason given".into()),
            });
        }
        Resolution::Candidates(candidates) => candidates,
    };
    let mut failures: Vec<String> = Vec::new();
    let mut all_offline = true;
    let mut first_exclusion = None;
    // The path the reader would put the file at, remembered from the drop
    // candidate itself, so the refusal of an excluded declared origin can
    // name it instead of advising another endpoint (`PROP-066#drops`).
    let mut drop_rung: Option<String> = None;
    for candidate in &candidates {
        let outcome = match &candidate.target {
            // One location rule (`PROP-066#locations`): a `file://` target
            // is read like a `path` target, never handed to HTTP.
            RemapTarget::Url(url) if url.to_ascii_lowercase().starts_with("file://") => {
                match ByteLocation::parse(url) {
                    Ok(ByteLocation::Local(path)) => {
                        copy_verified(&path, sha256, filename, destination)
                    }
                    Ok(ByteLocation::Remote(_)) => continue,
                    Err(error) => Err(InstallerError::Fetch {
                        source_ref: url.clone(),
                        reason: error.to_string(),
                    }),
                }
            }
            RemapTarget::Url(url) => {
                let watch = FetchWatch {
                    remapped_from: (url != declared).then_some(declared),
                    ..*watch
                };
                fetch_verified(url, sha256, package_root, destination, offline, &watch)
            }
            RemapTarget::Path(root) => {
                let row = candidate.row.as_ref().and_then(|row| locator.row(row));
                match row {
                    Some(row) if row.lookup == PathLookup::PackageDrop => {
                        if drop_rung.is_none() {
                            drop_rung = first_rung(root, artifact, sha256);
                        }
                        copy_from_drop(root, artifact, sha256, destination)
                    }
                    // A mirror row joined the URL remainder onto its
                    // directory; the directory itself is still searched
                    // by digest, as it always was.
                    Some(RemapRow {
                        target: RemapTarget::Path(original),
                        ..
                    }) if original != root => copy_verified(root, sha256, filename, destination)
                        .or_else(|_| copy_verified(original, sha256, filename, destination)),
                    _ => copy_verified(root, sha256, filename, destination),
                }
            }
            // The Locator never puts a refusal in a candidate list.
            RemapTarget::Deny => continue,
        };
        match outcome {
            Ok(()) => return Ok(served(candidate, locator, artifact)),
            Err(InstallerError::ContactRefused(
                refusal @ vibe_registry::ContactRefusal::ExcludedHost { .. },
            )) => {
                all_offline = false;
                first_exclusion.get_or_insert(refusal);
            }
            Err(
                error @ (InstallerError::ContactRefused(_)
                | InstallerError::SourcePolicy(_)
                | InstallerError::Cancelled(_)),
            ) => return Err(error),
            Err(error) => {
                all_offline &= matches!(error, InstallerError::Offline { .. });
                let target = if watch.http_client.is_some() {
                    vibe_core::progress::transfer::safe_remote(&candidate.target.display())
                } else {
                    candidate.target.display()
                };
                failures.push(format!("{target}: {error}"));
            }
        }
    }
    if let Some(refusal) = first_exclusion {
        // A drop or mirror that missed explains more than the excluded
        // origin does: what file to put where comes first.
        if failures.is_empty() {
            return Err(InstallerError::ContactRefused(refusal));
        }
        failures.push(declared_origin_refused(&refusal, drop_rung.as_deref()));
    }
    // When every candidate was refused for the same reason — the network
    // posture — the posture is the answer, not a list of four URLs that
    // all said so.
    if all_offline && !failures.is_empty() {
        return Err(InstallerError::Offline {
            source_ref: if watch.http_client.is_some() {
                vibe_core::progress::transfer::safe_remote(declared)
            } else {
                declared.into()
            },
        });
    }
    Err(InstallerError::Fetch {
        source_ref: if watch.http_client.is_some() {
            vibe_core::progress::transfer::safe_remote(declared)
        } else {
            declared.into()
        },
        reason: if failures.is_empty() {
            "no candidate source was available".to_owned()
        } else {
            format!("every candidate failed — {}", failures.join("; "))
        },
    })
}

/// Credit the candidate that served the bytes.
fn served(candidate: &Candidate, locator: &Locator, artifact: &Artifact) -> ServedBy {
    ServedBy {
        input: artifact
            .step
            .clone()
            .or_else(|| artifact.package.clone())
            .unwrap_or_default(),
        target: candidate.target.display(),
        row: candidate.row.as_ref().map(|row| ServedRow {
            layer: row.provenance.layer.label().to_owned(),
            source: row.provenance.source.clone(),
            index: row.provenance.index,
            reason: locator.row(row).and_then(|row| row.reason.clone()),
        }),
    }
}

/// Read a `path` candidate: the file itself, or `<path>/<sha256>/<filename>`
/// the way the application download cache is laid out
/// (`PROP-066#row`). The bytes are verified before they are placed.
///
/// The file name is a hint, not a requirement. A `sources` input knows the
/// name it was published under, but an `archive` or `native` input is kept
/// under its step's id, and the person who laid out the directory had the
/// upstream name — so a lone file inside `<path>/<sha256>/` is taken
/// whatever it is called, and `<path>/<sha256>` is read as a file too. The
/// digest decides either way; the name only narrows the search.
fn copy_verified(
    root: &Path,
    sha256: &str,
    filename: Option<&str>,
    destination: &Path,
) -> Result<(), InstallerError> {
    let shown = root.display().to_string();
    let fetch_error = |reason: String| InstallerError::Fetch {
        source_ref: shown.clone(),
        reason,
    };
    let digest = sha256.to_ascii_lowercase();
    let source = if root.is_file() {
        root.to_path_buf()
    } else {
        let by_digest = root.join(&digest);
        match filename.map(|name| by_digest.join(name)) {
            Some(named) if named.is_file() => named,
            _ if by_digest.is_file() => by_digest,
            _ => match sole_file(&by_digest) {
                Some(found) => found,
                None => {
                    return Err(fetch_error(format!(
                        "`{}` holds neither `{}` nor exactly one file",
                        by_digest.display(),
                        filename.unwrap_or("<filename>")
                    )));
                }
            },
        }
    };
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| io("creating", parent, error))?;
    }
    fs::copy(&source, destination).map_err(|error| io("copying", destination, error))?;
    let actual =
        source_build::digest_of(destination).map_err(|error| io("reading", destination, error))?;
    if actual != digest {
        let _ = fs::remove_file(destination);
        return Err(InstallerError::Digest {
            source_ref: source.display().to_string(),
        });
    }
    Ok(())
}

/// The one regular file `directory` holds, when it holds exactly one.
fn sole_file(directory: &Path) -> Option<PathBuf> {
    let mut files = fs::read_dir(directory)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file());
    let only = files.next()?;
    files.next().is_none().then_some(only)
}

/// A package-relative file that stays inside the package tree.
pub(crate) fn package_file(package_root: &Path, relative: &str) -> Result<PathBuf, String> {
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

pub(crate) fn hex(bytes: &[u8]) -> String {
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
    use sha2::{Digest, Sha256};

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

    /// A `path` row serves an input from a directory laid out by digest,
    /// and the file name inside it is a hint rather than a requirement
    /// (`PROP-066#row`): an `archive` input is kept under its step's id,
    /// not under the name it was published as.
    #[test]
    fn a_path_row_serves_an_input_by_digest_whatever_the_file_is_called() {
        use vibe_core::remap::{
            Artifact, ArtifactKind, Layer, LoadedSource, Locator, Provenance, RowDecl,
        };

        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("store");
        let digest = hex(&Sha256::digest(b"payload"));
        let entry = store.join(&digest);
        fs::create_dir_all(&entry).unwrap();
        fs::write(entry.join("published-name.bin"), b"payload").unwrap();

        let decl: RowDecl = toml::from_str(&format!(
            "sha256 = \"{digest}\"\npath = \"{}\"",
            store.display()
        ))
        .unwrap();
        let locator = Locator::from_sources(
            vec![LoadedSource {
                layer: Layer::Local,
                source: "settings.toml".into(),
                rows: vec![
                    decl.into_row(Provenance::new(Layer::Local, "settings.toml", 0))
                        .unwrap(),
                ],
                policy: Default::default(),
            }],
            Vec::new(),
        );
        let artifact = Artifact::new(ArtifactKind::AppInput)
            .with_url("https://example.invalid/unreachable")
            .with_digest(&digest);
        let out = dir.path().join("inputs/install");
        let served = fetch_located(
            &Located {
                locator: &locator,
                artifact: &artifact,
                declared: "https://example.invalid/unreachable",
                sha256: &digest,
                // The step's own file name, which the store does not use.
                filename: Some("install"),
            },
            dir.path(),
            &out,
            true,
            &FetchWatch::default(),
        )
        .unwrap();
        assert_eq!(fs::read(&out).unwrap(), b"payload");
        assert_eq!(served.target, store.display().to_string());
        let row = served.row.expect("the row that served it");
        assert_eq!((row.layer.as_str(), row.index), ("local", 0));

        // A `deny` row refuses the same input before any connection.
        let deny: RowDecl = toml::from_str(&format!("sha256 = \"{digest}\"\ndeny = true")).unwrap();
        let refusing = Locator::from_sources(
            vec![LoadedSource {
                layer: Layer::Invocation,
                source: "closed.toml".into(),
                rows: vec![
                    deny.into_row(Provenance::new(Layer::Invocation, "closed.toml", 0))
                        .unwrap(),
                ],
                policy: Default::default(),
            }],
            Vec::new(),
        );
        let refused = fetch_located(
            &Located {
                locator: &refusing,
                artifact: &artifact,
                declared: "https://example.invalid/unreachable",
                sha256: &digest,
                filename: Some("install"),
            },
            dir.path(),
            &dir.path().join("inputs/denied"),
            true,
            &FetchWatch::default(),
        );
        assert!(matches!(refused, Err(InstallerError::RemapDenied { .. })));
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
            ..FetchWatch::default()
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
