//! The trust set: the operating system's own certificate verifier, plus
//! whatever certificates this installation adds to it (`PROP-068#proxy`).
//!
//! The operating system decides first, through its own verifier — on
//! Windows the CryptoAPI chain engine, which reads **every** certificate
//! store the machine and the logged-in user have, including the
//! intermediate `CA` ones, and follows enterprise policy and AIA. That is
//! the whole of Part 1: a chain Edge and every other Windows application
//! builds is a chain vibe builds.
//!
//! On top of that, and never instead of it, come the certificates the
//! operator names — `[tls] extra_ca`, or its older spelling `[proxy]
//! ca_file`. Each entry is a certificate file or a directory of them. A
//! certificate authority among them becomes an extra trust anchor; any
//! certificate among them, authority or not, also becomes a **pin**: if a
//! server presents exactly those bytes, the connection is accepted after
//! checking that the certificate is current and is for that host. That is
//! how a self-signed server certificate is trusted at all — webpki will
//! not take an end-entity certificate as an anchor.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rustls::pki_types::{CertificateDer, ServerName, UnixTime};

mod bundle;
mod load;
mod verifier;
mod x509;

pub use bundle::{SYSTEM_STORE_NAMES, system_certificates};
pub(crate) use verifier::provider;

/// The application protocols every vibe TLS client offers.
///
/// HTTP/1.1 only: see [`TrustSet::client_config`]. Adding `h2` here is
/// safe only together with `reqwest`'s `http2` feature in the workspace
/// manifest, and the two must move in one change.
pub const ALPN: [&[u8]; 1] = [b"http/1.1"];

/// Why the configured extra certificates cannot be trusted.
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
        "could not read the extra certificate `{}`: {source} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-068#proxy; \
          fix: point `[tls] extra_ca` (or `[proxy] ca_file`) at a readable \
          certificate file, or at a directory of `*.pem` / `*.crt` / `*.cer` files)",
        path.display()
    )]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "the extra certificate `{}` holds no certificate \
         (violates spec://org.vibevm.core/vibevm/common/PROP-068#proxy; \
          fix: export the certificate as PEM (`-----BEGIN CERTIFICATE-----`) or as DER)",
        path.display()
    )]
    Empty { path: PathBuf },
    #[error(
        "the extra-certificate directory `{}` holds no `*.pem`, `*.crt` or `*.cer` file \
         (violates spec://org.vibevm.core/vibevm/common/PROP-068#proxy; \
          fix: export the certificates into that directory, or name the file itself)",
        path.display()
    )]
    EmptyDirectory { path: PathBuf },
    #[error(
        "the extra certificate `{}` is not a valid certificate: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-068#proxy; \
          fix: export the certificate as PEM or DER)",
        path.display()
    )]
    Invalid { path: PathBuf, reason: String },
    #[error(
        "the trust set is unusable: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-068#proxy; \
          fix: check that this machine has a certificate store, or name the \
          certificates with `[tls] extra_ca`)"
    )]
    Unusable { reason: String },
}

/// Which part of the trust set accepted a server's chain.
///
/// ```
/// use vibe_net::TrustSource;
/// assert_eq!(TrustSource::System.describe(), "trusted by system");
/// let extra = TrustSource::ExtraCa("/etc/corp-ca.pem".into());
/// assert_eq!(extra.describe(), "trusted by extra CA: /etc/corp-ca.pem");
/// assert_eq!(extra.kind(), "extra_ca");
/// let pinned = TrustSource::Pinned("/home/me/dev.crt".into());
/// assert_eq!(pinned.describe(), "pinned self-signed: /home/me/dev.crt");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustSource {
    /// The operating system's verifier (or, where vibe still consults
    /// them, the web roots it ships).
    System,
    /// An extra certificate authority, and the file it was read from.
    ExtraCa(PathBuf),
    /// The server's own certificate, named in the trust set.
    Pinned(PathBuf),
}

impl TrustSource {
    /// The stable one-word form a machine reads (`vibe dl --json`).
    pub fn kind(&self) -> &'static str {
        match self {
            Self::System => "system",
            Self::ExtraCa(_) => "extra_ca",
            Self::Pinned(_) => "pinned",
        }
    }

    /// The file this trust came from, when it came from a file.
    pub fn source(&self) -> Option<&Path> {
        match self {
            Self::System => None,
            Self::ExtraCa(path) | Self::Pinned(path) => Some(path),
        }
    }

    /// The sentence a person reads.
    pub fn describe(&self) -> String {
        match self {
            Self::System => "trusted by system".to_owned(),
            Self::ExtraCa(path) => format!("trusted by extra CA: {}", path.display()),
            Self::Pinned(path) => format!("pinned self-signed: {}", path.display()),
        }
    }
}

/// One installation's trust set, read once.
///
/// ```
/// use vibe_net::TrustSet;
/// // Without configured certificates the set is the operating system's.
/// let set = TrustSet::from_paths(&[]).unwrap();
/// assert!(set.is_empty());
/// assert!(set.client_config().is_ok());
/// ```
#[derive(Debug, Clone)]
pub struct TrustSet {
    /// The certificate authorities among the configured certificates:
    /// extra trust anchors.
    anchors: Vec<load::Loaded>,
    /// Every configured certificate, authority or not: exact-byte pins.
    pins: Vec<load::Loaded>,
    /// The entries as configured, for reporting and for the git bundle.
    sources: Vec<PathBuf>,
}

impl TrustSet {
    /// The trust set of `route` — `[proxy] ca_file` together with
    /// `[tls] extra_ca`, as the composition root merged them.
    pub fn of_route(route: &vibe_core::net::ProxyRoute) -> Result<Self, TrustError> {
        Self::from_paths(route.extra_ca())
    }

    /// The trust set these configured entries describe.
    pub fn from_paths(paths: &[PathBuf]) -> Result<Self, TrustError> {
        let loaded = load::certificates_at(paths)?;
        let anchors = loaded
            .iter()
            .filter(|certificate| {
                x509::Facts::parse(certificate.der.as_ref()).is_some_and(|facts| facts.is_ca)
            })
            .cloned()
            .collect();
        Ok(Self {
            anchors,
            pins: loaded,
            sources: paths.to_vec(),
        })
    }

    /// Whether this installation adds nothing to the operating system.
    pub fn is_empty(&self) -> bool {
        self.pins.is_empty()
    }

    /// The files and directories the configuration named.
    pub fn sources(&self) -> &[PathBuf] {
        &self.sources
    }

    /// How many extra anchors and how many pins the set holds — what
    /// `vibe self doctor` reports.
    pub fn counts(&self) -> (usize, usize) {
        (self.anchors.len(), self.pins.len() - self.anchors.len())
    }

    fn anchor_ders(&self) -> Vec<CertificateDer<'static>> {
        self.anchors
            .iter()
            .map(|certificate| certificate.der.clone())
            .collect()
    }

    fn pins(&self) -> Result<Vec<verifier::Pin>, TrustError> {
        Ok(self
            .pins
            .iter()
            .filter_map(|certificate| {
                x509::Facts::parse(certificate.der.as_ref()).map(|facts| verifier::Pin {
                    der: certificate.der.clone(),
                    facts,
                    source: certificate.source.clone(),
                })
            })
            .collect())
    }

    /// The verifier every TLS client of this installation uses.
    pub fn verifier(
        &self,
    ) -> Result<Arc<dyn rustls::client::danger::ServerCertVerifier>, TrustError> {
        let provider = verifier::provider();
        Ok(Arc::new(verifier::VibeVerifier::new(
            &self.anchor_ders(),
            self.pins()?,
            &provider,
        )?))
    }

    /// The `rustls` client configuration every TLS client of this
    /// installation is built from — the one construction point, so a new
    /// client cannot be given a different trust set by accident.
    ///
    /// A pre-configured TLS configuration is used by `reqwest` exactly as
    /// given, so this one also owns the ALPN list — and it must offer
    /// only what this build of `reqwest` can actually speak. The
    /// workspace builds `reqwest` without its `http2` feature, so
    /// offering `h2` would let a server that prefers HTTP/2 (every large
    /// host does) select a protocol `hyper-util` then panics on. Offering
    /// `http/1.1` alone is exactly what `reqwest` offers for itself under
    /// these features; [`ALPN`] is asserted against that.
    pub fn client_config(&self) -> Result<rustls::ClientConfig, TrustError> {
        let provider = verifier::provider();
        let mut config = rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .map_err(|error| TrustError::Unusable {
                reason: error.to_string(),
            })?
            .dangerous()
            .with_custom_certificate_verifier(self.verifier()?)
            .with_no_client_auth();
        config.alpn_protocols = ALPN.iter().map(|name| name.to_vec()).collect();
        Ok(config)
    }

    /// A client configuration that trusts exactly `roots` and nothing else
    /// — a caller that replaces the trust set deliberately, rather than
    /// adding to it (a registry given its own `ca`).
    pub fn pinned_roots_config(pem: &[u8]) -> Result<rustls::ClientConfig, TrustError> {
        let path = Path::new("<captured registry trust>");
        let loaded = load::certificates_in_bytes(pem, path)?;
        let provider = verifier::provider();
        let mut roots = rustls::RootCertStore::empty();
        for certificate in &loaded {
            roots
                .add(certificate.der.clone())
                .map_err(|error| TrustError::Invalid {
                    path: path.to_path_buf(),
                    reason: error.to_string(),
                })?;
        }
        let verifier = rustls::client::WebPkiServerVerifier::builder_with_provider(
            Arc::new(roots),
            provider.clone(),
        )
        .build()
        .map_err(|error| TrustError::Unusable {
            reason: error.to_string(),
        })?;
        let mut config = rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .map_err(|error| TrustError::Unusable {
                reason: error.to_string(),
            })?
            .dangerous()
            .with_custom_certificate_verifier(verifier)
            .with_no_client_auth();
        config.alpn_protocols = ALPN.iter().map(|name| name.to_vec()).collect();
        Ok(config)
    }

    /// Which part of the trust set accepts this chain: the operating
    /// system first, then each configured file in the order it was
    /// written, then the pins. Asking the system first means a server the
    /// machine already trusts is reported as the machine's, whatever else
    /// also names it.
    pub fn attribute(
        &self,
        chain: &[CertificateDer<'static>],
        name: &ServerName<'static>,
        now: UnixTime,
    ) -> Option<TrustSource> {
        let (leaf, intermediates) = chain.split_first()?;
        let provider = verifier::provider();
        let accepts = |anchors: &[CertificateDer<'static>]| {
            verifier::VibeVerifier::new(anchors, Vec::new(), &provider).is_ok_and(|verifier| {
                use rustls::client::danger::ServerCertVerifier;
                verifier
                    .verify_server_cert(leaf, intermediates, name, &[], now)
                    .is_ok()
            })
        };
        if accepts(&[]) {
            return Some(TrustSource::System);
        }
        for source in &self.sources {
            let anchors: Vec<CertificateDer<'static>> = self
                .anchors
                .iter()
                .filter(|certificate| certificate.source.starts_with(source))
                .map(|certificate| certificate.der.clone())
                .collect();
            if !anchors.is_empty() && accepts(&anchors) {
                return Some(TrustSource::ExtraCa(source.clone()));
            }
        }
        let host = verifier::host_of(name);
        let now = i64::try_from(now.as_secs()).unwrap_or(i64::MAX);
        self.pins
            .iter()
            .find(|certificate| {
                certificate.der.as_ref() == leaf.as_ref()
                    && x509::Facts::parse(certificate.der.as_ref())
                        .is_some_and(|facts| facts.is_current(now) && facts.is_for(&host))
            })
            .map(|certificate| TrustSource::Pinned(certificate.source.clone()))
    }

    /// The configured certificates as one PEM bundle.
    pub fn pem(&self) -> Vec<u8> {
        bundle::to_pem(self.pins.iter().map(|certificate| certificate.der.as_ref()))
    }

    /// The combined bundle a child `git` reads, written into the
    /// content-addressed cache. `None` when nothing is configured: the
    /// child then uses the platform's own trust, which is what it should.
    pub fn git_bundle(&self) -> Result<Option<PathBuf>, TrustError> {
        if self.is_empty() {
            return Ok(None);
        }
        bundle::cached(&bundle::combined(self)?).map(Some)
    }

    /// [`Self::git_bundle`] written into an explicit cache directory — the
    /// seam a test drives without touching the operator's own settings
    /// directory, and the one [`Self::git_bundle`] itself calls.
    pub fn git_bundle_in(&self, cache: &Path) -> Result<Option<PathBuf>, TrustError> {
        if self.is_empty() {
            return Ok(None);
        }
        bundle::cached_in(cache, &bundle::combined(self)?).map(Some)
    }

    /// How an operator adds the missing issuer of an untrusted chain, on
    /// this operating system.
    pub fn untrusted_help() -> &'static str {
        bundle::UNTRUSTED_HELP
    }

    /// The sentence that says who refused an untrusted chain.
    pub fn untrusted_by(&self) -> &'static str {
        if self.is_empty() {
            "which the system store does not trust"
        } else {
            "which neither the system store nor the extra certificates trust"
        }
    }
}

#[cfg(test)]
mod tests;
