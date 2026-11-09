//! The probe's TLS session: the trust set every vibe client uses, a
//! verifier that records the server's chain, and which part of the trust
//! set accepted it.

use super::der::summarize;
use super::hop::io_class;
use super::*;
use rustls::client::WebPkiServerVerifier;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{CertificateError, DigitallySignedStruct, RootCertStore, SignatureScheme};
use std::net::TcpStream;
use std::sync::{Mutex, PoisonError};

/// The roots every vibe client trusts besides `ca_file`: the bundled web
/// roots and the operating system's store.
fn system_roots() -> RootCertStore {
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let native = rustls_native_certs::load_native_certs();
    roots.add_parsable_certificates(native.certs);
    roots
}

fn ca_file_roots(session: &NetworkSession) -> Result<Option<RootCertStore>, Failure> {
    let Some(path) = session.route().ca_file() else {
        return Ok(None);
    };
    let bytes = crate::trust::read_ca_file(path)
        .map_err(|error| fail(FailureClass::Tls, error.to_string()))?;
    let mut roots = RootCertStore::empty();
    let certs: Vec<CertificateDer<'static>> = CertificateDer::pem_slice_iter(&bytes)
        .filter_map(Result::ok)
        .collect();
    roots.add_parsable_certificates(certs);
    Ok(Some(roots))
}

#[derive(Debug)]
struct Recording {
    inner: Arc<WebPkiServerVerifier>,
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

fn verifier(
    roots: RootCertStore,
    provider: &Arc<rustls::crypto::CryptoProvider>,
) -> Result<Arc<WebPkiServerVerifier>, Failure> {
    WebPkiServerVerifier::builder_with_provider(Arc::new(roots), provider.clone())
        .build()
        .map_err(|error| fail(FailureClass::Tls, format!("trust set unusable: {error}")))
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
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let system = system_roots();
    let extra = ca_file_roots(session)?;
    let mut all = system.clone();
    if let Some(extra) = &extra {
        all.roots.extend(extra.roots.iter().cloned());
    }
    let chain = Arc::new(Mutex::new(Vec::new()));
    let recording = Arc::new(Recording {
        inner: verifier(all, &provider)?,
        chain: chain.clone(),
    });
    let config = rustls::ClientConfig::builder_with_provider(provider.clone())
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
                    "unable to get local issuer certificate: the chain ends at `{issuer}`, which neither the system store{} trusts",
                    if extra.is_some() {
                        " nor `[proxy] ca_file`"
                    } else {
                        ""
                    }
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
            None => fail(io_class(&error), format!("TLS handshake: {error}")),
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
    let trusted_by = attribute(&recorded, &name, system, extra, &provider);
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

fn attribute(
    chain: &[CertificateDer<'static>],
    name: &ServerName<'static>,
    system: RootCertStore,
    extra: Option<RootCertStore>,
    provider: &Arc<rustls::crypto::CryptoProvider>,
) -> Option<TrustSource> {
    let (leaf, rest) = chain.split_first()?;
    let accepts = |roots: RootCertStore| {
        verifier(roots, provider).is_ok_and(|v| {
            v.verify_server_cert(leaf, rest, name, &[], UnixTime::now())
                .is_ok()
        })
    };
    if accepts(system) {
        return Some(TrustSource::System);
    }
    if extra.is_some_and(accepts) {
        return Some(TrustSource::CaFile);
    }
    None
}
