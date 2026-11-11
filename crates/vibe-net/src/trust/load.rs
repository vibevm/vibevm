//! Reading the extra certificates off disk (`PROP-068#proxy`).
//!
//! An entry of the list is a **file** — PEM holding one or many
//! certificates, or a single DER — or a **directory**, whose `*.pem`,
//! `*.crt` and `*.cer` files are all read, in name order so two runs of
//! the same directory produce the same bundle. Every certificate keeps the
//! path it came from: that path is what a report names and what a fix
//! points at.

use super::TrustError;
use rustls::pki_types::CertificateDer;
use rustls::pki_types::pem::PemObject;
use std::path::{Path, PathBuf};

/// The file extensions a certificate directory is read for.
const CERTIFICATE_EXTENSIONS: [&str; 3] = ["pem", "crt", "cer"];

/// One certificate and the file it was read from.
#[derive(Debug, Clone)]
pub(crate) struct Loaded {
    pub(crate) der: CertificateDer<'static>,
    pub(crate) source: PathBuf,
}

/// Whether a directory entry is one of the certificate files a directory
/// is read for.
///
/// The extension is matched without case, because a directory filled by
/// an export tool holds `Root.CER` as readily as `root.cer`.
fn is_certificate_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            CERTIFICATE_EXTENSIONS
                .iter()
                .any(|known| extension.eq_ignore_ascii_case(known))
        })
}

/// The certificate files of a directory, in name order.
fn directory_files(path: &Path) -> Result<Vec<PathBuf>, TrustError> {
    let entries = std::fs::read_dir(path).map_err(|source| TrustError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let mut files: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| TrustError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        let file = entry.path();
        if file.is_file() && is_certificate_file(&file) {
            files.push(file);
        }
    }
    files.sort();
    Ok(files)
}

/// Every certificate in one file: a PEM bundle, or a single DER when the
/// bytes carry no PEM header at all.
///
/// A file that parses as neither is an error rather than an empty list:
/// an operator who named a file meant it to be trusted, and silently
/// trusting nothing is the failure this whole path exists to end.
pub(crate) fn certificates_in_file(path: &Path) -> Result<Vec<Loaded>, TrustError> {
    let bytes = std::fs::read(path).map_err(|source| TrustError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    certificates_in_bytes(&bytes, path)
}

/// [`certificates_in_file`] over bytes already in hand.
pub(crate) fn certificates_in_bytes(bytes: &[u8], path: &Path) -> Result<Vec<Loaded>, TrustError> {
    let text = String::from_utf8_lossy(bytes);
    if text.contains("-----BEGIN CERTIFICATE-----") {
        let mut out = Vec::new();
        for certificate in CertificateDer::pem_slice_iter(bytes) {
            let der = certificate.map_err(|error| TrustError::Invalid {
                path: path.to_path_buf(),
                reason: error.to_string(),
            })?;
            out.push(Loaded {
                der,
                source: path.to_path_buf(),
            });
        }
        if out.is_empty() {
            return Err(TrustError::Empty {
                path: path.to_path_buf(),
            });
        }
        return Ok(out);
    }
    // No PEM header: the file is a single DER certificate, which is what
    // the Windows and macOS export dialogs write by default.
    if super::x509::Facts::parse(bytes).is_none() {
        return Err(TrustError::Empty {
            path: path.to_path_buf(),
        });
    }
    Ok(vec![Loaded {
        der: CertificateDer::from(bytes.to_vec()),
        source: path.to_path_buf(),
    }])
}

/// Every certificate the configured list names, in the order written and
/// without repeats (the same certificate reached through two entries is
/// one trust decision, and is reported under the first path that named
/// it).
pub(crate) fn certificates_at(paths: &[PathBuf]) -> Result<Vec<Loaded>, TrustError> {
    let mut out: Vec<Loaded> = Vec::new();
    for path in paths {
        let found = if path.is_dir() {
            let files = directory_files(path)?;
            if files.is_empty() {
                return Err(TrustError::EmptyDirectory {
                    path: path.to_path_buf(),
                });
            }
            let mut all = Vec::new();
            for file in files {
                all.extend(certificates_in_file(&file)?);
            }
            all
        } else {
            certificates_in_file(path)?
        };
        for certificate in found {
            if !out.iter().any(|seen| seen.der == certificate.der) {
                out.push(certificate);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_three_certificate_extensions_are_read_from_a_directory() {
        assert!(is_certificate_file(Path::new("/a/root.pem")));
        assert!(is_certificate_file(Path::new("/a/root.CRT")));
        assert!(is_certificate_file(Path::new("/a/Root.Cer")));
        assert!(!is_certificate_file(Path::new("/a/README.md")));
        assert!(!is_certificate_file(Path::new("/a/root")));
        assert!(!is_certificate_file(Path::new("/a/root.key")));
    }

    #[test]
    fn a_file_that_holds_no_certificate_is_an_error_not_an_empty_trust_set() {
        let error = certificates_in_bytes(b"hello", Path::new("/a/notes.txt")).unwrap_err();
        assert!(matches!(error, TrustError::Empty { .. }), "{error}");
        assert!(error.to_string().contains("/a/notes.txt"));
    }
}
