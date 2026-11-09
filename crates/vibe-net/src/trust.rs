//! The trust set: the operating system's root store plus `[proxy] ca_file`.

use base64::Engine;
use std::path::{Path, PathBuf};

/// Why `[proxy] ca_file` cannot be trusted.
///
/// ```
/// use vibe_net::TrustError;
/// let text = TrustError::Empty { path: "/etc/corp-ca.pem".into() }.to_string();
/// assert!(text.contains("/etc/corp-ca.pem") && text.contains("fix:"));
/// ```
#[derive(Debug, thiserror::Error)]
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-068#proxy")]
pub enum TrustError {
    #[error(
        "could not read `[proxy] ca_file` `{}`: {source} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-068#proxy; \
          fix: point `ca_file` at a readable PEM bundle of the corporate CA)",
        path.display()
    )]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "`[proxy] ca_file` `{}` holds no PEM certificate \
         (violates spec://org.vibevm.core/vibevm/common/PROP-068#proxy; \
          fix: export the corporate CA as PEM (`-----BEGIN CERTIFICATE-----`), not DER)",
        path.display()
    )]
    Empty { path: PathBuf },
    #[error(
        "`[proxy] ca_file` `{}` is not a valid PEM certificate bundle: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-068#proxy; \
          fix: export the corporate CA as PEM)",
        path.display()
    )]
    Invalid { path: PathBuf, reason: String },
}

/// The PEM bytes of `ca_file`, checked to hold at least one certificate.
pub(crate) fn read_ca_file(path: &Path) -> Result<Vec<u8>, TrustError> {
    let bytes = std::fs::read(path).map_err(|source| TrustError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    if !String::from_utf8_lossy(&bytes).contains("-----BEGIN CERTIFICATE-----") {
        return Err(TrustError::Empty {
            path: path.to_path_buf(),
        });
    }
    Ok(bytes)
}

/// `ca_file` as reqwest root certificates.
pub(crate) fn extra_roots(path: &Path) -> Result<Vec<reqwest::Certificate>, TrustError> {
    let bytes = read_ca_file(path)?;
    reqwest::Certificate::from_pem_bundle(&bytes).map_err(|error| TrustError::Invalid {
        path: path.to_path_buf(),
        reason: error.to_string(),
    })
}

/// One PEM bundle: the operating system's roots followed by `ca_file` —
/// what a child `git` reads as `http.sslCAInfo`, so naming the corporate CA
/// never takes the public roots away.
pub(crate) fn combined_bundle(ca_file: &Path) -> Result<Vec<u8>, TrustError> {
    let extra = read_ca_file(ca_file)?;
    let mut out = Vec::new();
    let native = rustls_native_certs::load_native_certs();
    for error in &native.errors {
        tracing::debug!("operating-system root store: {error}");
    }
    for cert in native.certs {
        out.extend_from_slice(b"-----BEGIN CERTIFICATE-----\n");
        let encoded = base64::engine::general_purpose::STANDARD.encode(cert.as_ref());
        for line in encoded.as_bytes().chunks(64) {
            out.extend_from_slice(line);
            out.push(b'\n');
        }
        out.extend_from_slice(b"-----END CERTIFICATE-----\n");
    }
    out.extend_from_slice(&extra);
    if !extra.ends_with(b"\n") {
        out.push(b'\n');
    }
    Ok(out)
}

/// How many roots the operating system's store offers (for `doctor`).
pub(crate) fn native_root_count() -> (usize, usize) {
    let native = rustls_native_certs::load_native_certs();
    (native.certs.len(), native.errors.len())
}
