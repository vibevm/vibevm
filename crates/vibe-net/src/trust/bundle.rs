//! The CA bundle a child `git` reads, and the system certificates that go
//! into it (`PROP-068#proxy`).
//!
//! A child `git` cannot be handed a verifier; it is handed a file. On
//! Windows that file *replaces* the Windows certificate store rather than
//! adding to it — Git's own documentation says so of
//! `http.schannelUseSSLCAInfo`: "the Secure Channel backend can use the
//! certificate bundle provided via `http.sslCAInfo`, but that would
//! override the Windows Certificate Store". So when vibe hands a bundle
//! over at all, the bundle has to carry the whole store, not just the
//! extra certificates — and the whole store means the intermediate `CA`
//! stores too, which is exactly where a corporate subordinate authority
//! is deployed and exactly what the earlier bundle was missing.
//!
//! The bundle is named after its own content and kept in
//! `<settings-dir>/tls`, so a child reads a file that outlives the call
//! that wrote it and two runs with the same trust set write one file.

use super::{TrustError, TrustSet};
use base64::Engine;
use std::path::PathBuf;

/// The Windows certificate stores a bundle is exported from: the trusted
/// roots and the intermediate authorities, of the machine and of the
/// logged-in user. `rustls-native-certs` reads only the first of the four,
/// which is why a corporate chain that ends at a subordinate authority
/// deployed to `CA` could not be completed from its bundle.
pub const SYSTEM_STORE_NAMES: [&str; 2] = ["ROOT", "CA"];

/// Certificates in DER, as one PEM bundle.
pub(crate) fn to_pem<'a>(certificates: impl Iterator<Item = &'a [u8]>) -> Vec<u8> {
    let mut out = Vec::new();
    for der in certificates {
        out.extend_from_slice(b"-----BEGIN CERTIFICATE-----\n");
        let encoded = base64::engine::general_purpose::STANDARD.encode(der);
        for line in encoded.as_bytes().chunks(64) {
            out.extend_from_slice(line);
            out.push(b'\n');
        }
        out.extend_from_slice(b"-----END CERTIFICATE-----\n");
    }
    out
}

/// Every certificate this operating system's stores hold, in DER.
///
/// On Windows that is `ROOT` and `CA` of both `CurrentUser` and
/// `LocalMachine`; elsewhere it is the platform bundle
/// `rustls-native-certs` reads (which honours `SSL_CERT_FILE` and
/// `SSL_CERT_DIR`).
#[cfg(windows)]
pub fn system_certificates() -> (Vec<Vec<u8>>, Vec<String>) {
    let mut certificates: Vec<Vec<u8>> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    let mut push =
        |store: Result<schannel::cert_store::CertStore, std::io::Error>, name: &str| match store {
            Ok(store) => {
                for certificate in store.certs() {
                    let der = certificate.to_der().to_vec();
                    if !certificates.contains(&der) {
                        certificates.push(der);
                    }
                }
            }
            Err(error) => errors.push(format!("{name}: {error}")),
        };
    for name in SYSTEM_STORE_NAMES {
        push(
            schannel::cert_store::CertStore::open_current_user(name),
            &format!("CurrentUser\\{name}"),
        );
        push(
            schannel::cert_store::CertStore::open_local_machine(name),
            &format!("LocalMachine\\{name}"),
        );
    }
    (certificates, errors)
}

/// Every certificate this operating system's stores hold, in DER.
#[cfg(not(windows))]
pub fn system_certificates() -> (Vec<Vec<u8>>, Vec<String>) {
    let native = rustls_native_certs::load_native_certs();
    let errors = native.errors.iter().map(ToString::to_string).collect();
    let mut certificates: Vec<Vec<u8>> = Vec::new();
    for certificate in native.certs {
        let der = certificate.as_ref().to_vec();
        if !certificates.contains(&der) {
            certificates.push(der);
        }
    }
    (certificates, errors)
}

/// The system's certificates followed by the configured extra ones, as one
/// PEM bundle. Naming an extra certificate never takes the system's away.
pub(crate) fn combined(set: &TrustSet) -> Result<Vec<u8>, TrustError> {
    let (system, errors) = system_certificates();
    for error in &errors {
        tracing::debug!("operating-system certificate store: {error}");
    }
    let extra = set.pem();
    let mut out = to_pem(system.iter().map(Vec::as_slice));
    out.extend_from_slice(&extra);
    if !out.ends_with(b"\n") {
        out.push(b'\n');
    }
    Ok(out)
}

/// `<settings-dir>/tls/<sha256>.pem`, written once.
///
/// Content addressing is what makes the file safe to leave behind: the
/// name *is* the bytes, so a stale file is never read as a current trust
/// set, and a concurrent run writing the same bundle writes the same
/// bytes. The write goes through a temporary file in the same directory
/// so a reader never sees a half-written bundle.
pub(crate) fn cached(bytes: &[u8]) -> Result<PathBuf, TrustError> {
    let dir = vibe_core::settings::tls_cache_dir().ok_or_else(|| TrustError::Unusable {
        reason: "this account has no settings directory for the CA bundle".to_owned(),
    })?;
    cached_in(&dir, bytes)
}

/// [`cached`] in an explicit directory — the seam a test drives without
/// touching the operator's own settings directory.
pub(crate) fn cached_in(dir: &std::path::Path, bytes: &[u8]) -> Result<PathBuf, TrustError> {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    let name = format!("{digest:x}.pem");
    let dir = dir.to_path_buf();
    let path = dir.join(&name);
    if path.is_file() {
        return Ok(path);
    }
    std::fs::create_dir_all(&dir).map_err(|source| TrustError::Read {
        path: dir.clone(),
        source,
    })?;
    let scratch = dir.join(format!("{}.{}.tmp", name, std::process::id()));
    std::fs::write(&scratch, bytes).map_err(|source| TrustError::Read {
        path: scratch.clone(),
        source,
    })?;
    match std::fs::rename(&scratch, &path) {
        Ok(()) => Ok(path),
        // Another process wrote the same content first: the bytes are the
        // name, so its file is this file.
        Err(_) if path.is_file() => {
            let _ = std::fs::remove_file(&scratch);
            Ok(path)
        }
        Err(source) => {
            let _ = std::fs::remove_file(&scratch);
            Err(TrustError::Read { path, source })
        }
    }
}

/// How an operator exports the missing issuer of an untrusted chain and
/// hands it to vibe, on this operating system. Generic: it names the
/// stores and commands, never a particular network.
pub(crate) const UNTRUSTED_HELP: &str = if cfg!(windows) {
    "export the issuer and every certificate above it, then name the file with \
     `[tls] extra_ca`. In PowerShell, the whole of both stores: \
     `Get-ChildItem Cert:\\CurrentUser\\Root, Cert:\\CurrentUser\\CA, \
     Cert:\\LocalMachine\\Root, Cert:\\LocalMachine\\CA | ForEach-Object { \
     [Convert]::ToBase64String($_.RawData, 'InsertLineBreaks') | \
     ForEach-Object { \"-----BEGIN CERTIFICATE-----`n$_`n-----END CERTIFICATE-----\" } } | \
     Set-Content -Encoding ascii $HOME\\.vibe\\corp-ca.pem`"
} else if cfg!(target_vendor = "apple") {
    "export the issuer and every certificate above it, then name the file with \
     `[tls] extra_ca`. From the system keychain: \
     `security find-certificate -a -p /Library/Keychains/System.keychain > ~/.vibe/corp-ca.pem` \
     (add `-a -p ~/Library/Keychains/login.keychain-db` for your own), or export the \
     certificate from Keychain Access as PEM"
} else {
    "export the issuer and every certificate above it, then name the file with \
     `[tls] extra_ca`; or install it where this distribution keeps certificate \
     authorities (`/usr/local/share/ca-certificates` then `update-ca-certificates`, \
     or `/etc/pki/ca-trust/source/anchors` then `update-ca-trust`)"
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pem_bundle_is_written_in_the_shape_every_tool_reads() {
        let text = String::from_utf8(to_pem([b"abc".as_slice()].into_iter())).unwrap();
        assert!(text.starts_with("-----BEGIN CERTIFICATE-----\n"));
        assert!(text.ends_with("-----END CERTIFICATE-----\n"));
        assert!(text.contains("YWJj"));
    }

    #[test]
    fn the_help_names_this_platform_and_no_particular_network() {
        assert!(UNTRUSTED_HELP.contains("extra_ca"));
        if cfg!(windows) {
            assert!(UNTRUSTED_HELP.contains("CurrentUser\\CA"));
        } else if cfg!(target_vendor = "apple") {
            assert!(UNTRUSTED_HELP.contains("security find-certificate"));
        } else {
            assert!(UNTRUSTED_HELP.contains("update-ca-certificates"));
        }
    }
}
