//! The probe's TLS session: the trust set every vibe client uses, a
//! verifier that records the server's chain, and which part of the trust
//! set accepted it.

use super::der::summarize;
use super::*;
use crate::TrustSet;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{CertificateError, DigitallySignedStruct, SignatureScheme};
use std::net::TcpStream;
use std::sync::{Mutex, PoisonError};

/// The one trust set, wrapped so the probe keeps the chain the server
/// sent even when verification refuses it — a refusal an operator can act
/// on names the certificate it ended at.
#[derive(Debug)]
struct Recording {
    inner: Arc<dyn ServerCertVerifier>,
    chain: Arc<Mutex<Vec<CertificateDer<'static>>>>,
}

impl ServerCertVerifier for Recording {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        {
            let mut chain = self.chain.lock().unwrap_or_else(PoisonError::into_inner);
            chain.clear();
            chain.push(end_entity.clone().into_owned());
            chain.extend(intermediates.iter().map(|c| c.clone().into_owned()));
        }
        self.inner
            .verify_server_cert(end_entity, intermediates, server_name, ocsp_response, now)
    }
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls12_signature(message, cert, dss)
    }
    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls13_signature(message, cert, dss)
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.inner.supported_verify_schemes()
    }
}

pub(super) fn handshake(
    session: &NetworkSession,
    host: &str,
    tcp: TcpStream,
) -> Result<
    (
        rustls::StreamOwned<rustls::ClientConnection, TcpStream>,
        TlsInfo,
    ),
    Failure,
> {
    let trust = TrustSet::of_route(session.route())
        .map_err(|error| fail(FailureClass::Tls, error.to_string()))?;
    let chain = Arc::new(Mutex::new(Vec::new()));
    let recording = Arc::new(Recording {
        inner: trust
            .verifier()
            .map_err(|error| fail(FailureClass::Tls, error.to_string()))?,
        chain: chain.clone(),
    });
    // The same configuration every client of this operation gets, with
    // the recording verifier in place of the plain one and no ALPN: the
    // probe speaks HTTP/1.1 by hand.
    let config = rustls::ClientConfig::builder_with_provider(crate::trust::provider())
        .with_safe_default_protocol_versions()
        .map_err(|error| fail(FailureClass::Tls, error.to_string()))?
        .dangerous()
        .with_custom_certificate_verifier(recording)
        .with_no_client_auth();
    let name = ServerName::try_from(
        host.trim_start_matches('[')
            .trim_end_matches(']')
            .to_owned(),
    )
    .map_err(|_| {
        fail(
            FailureClass::Tls,
            format!("`{host}` is not a TLS server name"),
        )
    })?;
    let connection = rustls::ClientConnection::new(Arc::new(config), name.clone())
        .map_err(|error| fail(FailureClass::Tls, error.to_string()))?;
    let mut stream = rustls::StreamOwned::new(connection, tcp);
    let shaken = loop {
        if !stream.conn.is_handshaking() {
            break Ok(());
        }
        match stream.conn.complete_io(&mut stream.sock) {
            Ok(_) => {}
            Err(error) => break Err(error),
        }
    };
    let recorded: Vec<CertificateDer<'static>> =
        chain.lock().unwrap_or_else(PoisonError::into_inner).clone();
    let summaries: Vec<CertSummary> = recorded.iter().map(|c| summarize(c.as_ref())).collect();
    if let Err(error) = shaken {
        let issuer = summaries
            .last()
            .map(|cert| cert.issuer.clone())
            .unwrap_or_default();
        let inner = error
            .get_ref()
            .and_then(|inner| inner.downcast_ref::<rustls::Error>())
            .cloned();
        let failure = match inner {
            Some(rustls::Error::InvalidCertificate(CertificateError::UnknownIssuer)) => fail(
                FailureClass::UntrustedCertificate,
                format!(
                    "unable to get local issuer certificate: the chain ends at `{issuer}`, {}",
                    trust.untrusted_by()
                ),
            ),
            Some(rustls::Error::InvalidCertificate(
                CertificateError::NotValidForName | CertificateError::NotValidForNameContext { .. },
            )) => fail(
                FailureClass::NameMismatch,
                format!(
                    "the certificate `{}` is not valid for `{host}`",
                    summaries.first().map(|c| c.subject.as_str()).unwrap_or("?")
                ),
            ),
            Some(other) => fail(FailureClass::Tls, other.to_string()),
            None => handshake_io_failure(&error),
        };
        return Err(Failure {
            detail: if summaries.is_empty() {
                failure.detail
            } else {
                format!("{}; chain: {}", failure.detail, chain_text(&summaries))
            },
            ..failure
        });
    }
    let trusted_by = trust.attribute(&recorded, &name, UnixTime::now());
    let version = stream
        .conn
        .protocol_version()
        .map(|v| format!("{v:?}").replace("TLSv1_", "TLS 1."))
        .unwrap_or_default();
    let cipher = stream
        .conn
        .negotiated_cipher_suite()
        .map(|suite| format!("{:?}", suite.suite()))
        .unwrap_or_default();
    Ok((
        stream,
        TlsInfo {
            version,
            cipher,
            chain: summaries,
            trusted_by,
        },
    ))
}

fn chain_text(chain: &[CertSummary]) -> String {
    chain
        .iter()
        .map(|cert| format!("`{}` issued by `{}`", cert.subject, cert.issuer))
        .collect::<Vec<_>>()
        .join(" ← ")
}

/// The failure of a handshake the transport ended, after the TCP connection
/// itself was made: a timeout stays a timeout; a reset or an early close is
/// the TLS phase's, since the server or a filter on the path refused the
/// session for this name (PROP-068 `#dl`). It is not a refused connection.
fn handshake_io_failure(error: &std::io::Error) -> Failure {
    use std::io::ErrorKind;
    match error.kind() {
        ErrorKind::TimedOut | ErrorKind::WouldBlock => fail(
            FailureClass::ConnectTimeout,
            format!("TLS handshake: {error}"),
        ),
        ErrorKind::ConnectionReset
        | ErrorKind::ConnectionAborted
        | ErrorKind::UnexpectedEof
        | ErrorKind::BrokenPipe => fail(
            FailureClass::Tls,
            format!(
                "TLS handshake: the connection was closed before the handshake finished ({error}); \
                 the server, or a filter on the network path, refused the session for this name"
            ),
        ),
        _ => fail(FailureClass::Tls, format!("TLS handshake: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Error, ErrorKind};

    #[test]
    fn a_reset_during_the_handshake_is_a_tls_failure_not_a_refused_connection() {
        for kind in [
            ErrorKind::ConnectionReset,
            ErrorKind::ConnectionAborted,
            ErrorKind::UnexpectedEof,
        ] {
            let failure = handshake_io_failure(&Error::new(kind, "closed by the remote host"));
            assert_eq!(failure.class, FailureClass::Tls, "{kind:?}");
            assert!(
                failure
                    .detail
                    .contains("closed before the handshake finished"),
                "{}",
                failure.detail
            );
        }
    }

    #[test]
    fn the_untrusted_issuer_sentence_says_who_refused_the_chain() {
        let bare = TrustSet::from_paths(&[]).unwrap();
        assert_eq!(bare.untrusted_by(), "which the system store does not trust");
        assert!(TrustSet::untrusted_help().contains("extra_ca"));
    }

    #[test]
    fn a_timeout_during_the_handshake_stays_a_timeout() {
        let failure = handshake_io_failure(&Error::new(ErrorKind::TimedOut, "timed out"));
        assert_eq!(failure.class, FailureClass::ConnectTimeout);
    }
}
