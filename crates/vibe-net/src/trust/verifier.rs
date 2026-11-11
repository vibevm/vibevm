//! The one certificate verifier every TLS client of vibe uses
//! (`PROP-068#proxy`).
//!
//! Three sources of trust, asked in this order:
//!
//! 1. **A pinned certificate** — the server's own certificate, byte for
//!    byte, is one the operator put in the trust set. webpki will not take
//!    an end-entity certificate as a trust anchor, so a self-signed server
//!    certificate can only be trusted by naming it; the two checks a path
//!    build would have made are made here instead — the certificate must
//!    be current and must be for the host being contacted.
//! 2. **The operating system's verifier**, augmented with the extra
//!    certificate authorities. On Windows that is the CryptoAPI chain
//!    engine — every store including the intermediate `CA` ones,
//!    enterprise policy and AIA issuer fetching; on macOS the Security
//!    framework; elsewhere webpki over the platform's root bundle.
//! 3. **The web roots vibe ships**, on the platforms where the operating
//!    system's verifier is itself webpki over a root bundle. They were
//!    part of the trust set before this verifier existed and a minimal
//!    container that installs no CA bundle still has them. On Windows and
//!    macOS they are *not* consulted: there the operating system's own
//!    root programme decides, including which roots it distrusts.

use rustls::client::WebPkiServerVerifier;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::CryptoProvider;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, RootCertStore, SignatureScheme};
use std::path::PathBuf;
use std::sync::Arc;

use super::TrustError;
use super::x509::Facts;

/// Whether this build consults the web roots vibe ships as well as the
/// operating system's verifier. See the module note: only where the
/// platform verifier is itself webpki over a root bundle.
pub(crate) const CONSULTS_BUNDLED_ROOTS: bool = !cfg!(any(windows, target_vendor = "apple"));

/// One certificate the operator named, ready to be matched byte for byte.
#[derive(Debug, Clone)]
pub(crate) struct Pin {
    pub(crate) der: CertificateDer<'static>,
    pub(crate) facts: Facts,
    pub(crate) source: PathBuf,
}

/// The crypto provider every vibe TLS session uses — the same `ring` build
/// `reqwest` links, so one handshake cannot disagree with another about
/// which cipher suites exist.
pub(crate) fn provider() -> Arc<CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

/// The web roots vibe ships, as a verifier.
pub(crate) fn bundled_verifier(
    provider: &Arc<CryptoProvider>,
) -> Result<Arc<WebPkiServerVerifier>, TrustError> {
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    WebPkiServerVerifier::builder_with_provider(Arc::new(roots), provider.clone())
        .build()
        .map_err(|error| TrustError::Unusable {
            reason: error.to_string(),
        })
}

/// The operating system's verifier, augmented with `anchors`.
pub(crate) fn platform_verifier(
    anchors: &[CertificateDer<'static>],
    provider: &Arc<CryptoProvider>,
) -> Result<rustls_platform_verifier::Verifier, TrustError> {
    let built = if anchors.is_empty() {
        rustls_platform_verifier::Verifier::new(provider.clone())
    } else {
        rustls_platform_verifier::Verifier::new_with_extra_roots(
            anchors.iter().cloned(),
            provider.clone(),
        )
    };
    built.map_err(|error| TrustError::Unusable {
        reason: error.to_string(),
    })
}

/// The composed verifier described in the module note.
#[derive(Debug)]
pub(crate) struct VibeVerifier {
    /// `None` only where the platform verifier could not be built *and*
    /// the bundled roots can stand in for it — a container with no CA
    /// bundle installed, which is the one case the fallback exists for.
    platform: Option<Arc<rustls_platform_verifier::Verifier>>,
    bundled: Option<Arc<WebPkiServerVerifier>>,
    pins: Vec<Pin>,
}

impl VibeVerifier {
    pub(crate) fn new(
        anchors: &[CertificateDer<'static>],
        pins: Vec<Pin>,
        provider: &Arc<CryptoProvider>,
    ) -> Result<Self, TrustError> {
        let bundled = if CONSULTS_BUNDLED_ROOTS {
            Some(bundled_verifier(provider)?)
        } else {
            None
        };
        let platform = match platform_verifier(anchors, provider) {
            Ok(verifier) => Some(Arc::new(verifier)),
            // Without a platform root store there is nothing to augment;
            // where the bundled roots stand in, the session still has a
            // trust set and says so in the log rather than failing here.
            Err(error) if bundled.is_some() => {
                tracing::debug!("operating-system certificate verifier: {error}");
                None
            }
            Err(error) => return Err(error),
        };
        Ok(Self {
            platform,
            bundled,
            pins,
        })
    }

    /// The pin whose bytes are this certificate, current and for this
    /// host. `None` when nothing is pinned for this connection.
    fn pinned(&self, end_entity: &CertificateDer<'_>, host: &str, now: UnixTime) -> Option<&Pin> {
        let now = i64::try_from(now.as_secs()).unwrap_or(i64::MAX);
        self.pins.iter().find(|pin| {
            pin.der.as_ref() == end_entity.as_ref()
                && pin.facts.is_current(now)
                && pin.facts.is_for(host)
        })
    }
}

/// The host name a `ServerName` stands for, as a certificate's own names
/// are written.
pub(crate) fn host_of(name: &ServerName<'_>) -> String {
    match name {
        ServerName::DnsName(dns) => dns.as_ref().to_ascii_lowercase(),
        ServerName::IpAddress(address) => std::net::IpAddr::from(*address).to_string(),
        _ => String::new(),
    }
}

impl ServerCertVerifier for VibeVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        // A pin is matched first: it is a byte comparison over a list the
        // operator wrote, and it spares a self-signed certificate the
        // platform verifier's issuer lookups, which on Windows may go to
        // the network before failing.
        if let Some(pin) = self.pinned(end_entity, &host_of(server_name), now) {
            tracing::debug!(source = %pin.source.display(), "accepted a pinned server certificate");
            return Ok(ServerCertVerified::assertion());
        }
        let mut refusal = None;
        for verifier in [
            self.platform
                .as_ref()
                .map(|v| v.as_ref() as &dyn ServerCertVerifier),
            self.bundled
                .as_ref()
                .map(|v| v.as_ref() as &dyn ServerCertVerifier),
        ]
        .into_iter()
        .flatten()
        {
            match verifier.verify_server_cert(
                end_entity,
                intermediates,
                server_name,
                ocsp_response,
                now,
            ) {
                Ok(accepted) => return Ok(accepted),
                // The first refusal is the one reported: it is the
                // operating system's, and it is the one an operator can
                // act on.
                Err(error) => refusal = refusal.or(Some(error)),
            }
        }
        Err(refusal.unwrap_or(rustls::Error::InvalidCertificate(
            rustls::CertificateError::UnknownIssuer,
        )))
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        match (&self.platform, &self.bundled) {
            (Some(platform), _) => platform.verify_tls12_signature(message, cert, dss),
            (None, Some(bundled)) => bundled.verify_tls12_signature(message, cert, dss),
            (None, None) => Err(rustls::Error::General("no trust set".into())),
        }
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        match (&self.platform, &self.bundled) {
            (Some(platform), _) => platform.verify_tls13_signature(message, cert, dss),
            (None, Some(bundled)) => bundled.verify_tls13_signature(message, cert, dss),
            (None, None) => Err(rustls::Error::General("no trust set".into())),
        }
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        match (&self.platform, &self.bundled) {
            (Some(platform), _) => platform.supported_verify_schemes(),
            (None, Some(bundled)) => bundled.supported_verify_schemes(),
            (None, None) => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_roots_stand_in_only_where_the_platform_verifier_is_webpki() {
        // Windows and macOS have a root programme of their own, including
        // the roots they distrust; nowhere else does.
        assert_eq!(
            CONSULTS_BUNDLED_ROOTS,
            !(cfg!(windows) || cfg!(target_vendor = "apple"))
        );
    }

    #[test]
    fn a_server_name_reads_as_the_host_a_certificate_would_name() {
        let dns = ServerName::try_from("Example.Test".to_owned()).unwrap();
        assert_eq!(host_of(&dns), "example.test");
        let ip = ServerName::try_from("127.0.0.1").unwrap();
        assert_eq!(host_of(&ip), "127.0.0.1");
    }
}
