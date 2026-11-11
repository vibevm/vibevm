//! The trust set end to end (`PROP-068#proxy`): what the configured extra
//! certificates are read from, that they are *added* to the operating
//! system's own verifier rather than replacing it, that an incomplete
//! chain can be completed from them, and that a self-signed server
//! certificate is accepted only when its own bytes are named — for the
//! right host, at a time it is valid.
//!
//! Every certificate here is made on the spot by the local test authority;
//! no certificate of any real network appears in this repository.

#![cfg(unix)]

mod support;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use support::{FIXTURE_HOST, as_der, https_server, self_signed, test_ca, test_chain};
use vibe_core::net::ProxyRoute;
use vibe_net::probe::{FailureClass, ProbeOptions};
use vibe_net::{NetworkSession, TrustSet, TrustSource};

fn admit_everything(_: &url::Url) -> Result<(), String> {
    Ok(())
}

/// Probe `https://127.0.0.1:<port>/<file>` over a direct route whose trust
/// set is exactly `extra_ca`.
fn probe_with(extra_ca: Vec<PathBuf>, port: u16, file: &str) -> vibe_net::probe::Probe {
    let session = Arc::new(NetworkSession::new(
        ProxyRoute::direct().with_extra_ca(extra_ca),
    ));
    let url = url::Url::parse(&format!("https://127.0.0.1:{port}/{file}")).unwrap();
    vibe_net::probe::probe(&session, &url, &ProbeOptions::default(), &admit_everything)
}

fn scratch() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("vibe-trust-")
        .tempdir()
        .unwrap()
}

fn payload(dir: &Path) -> &'static str {
    std::fs::write(dir.join("payload.bin"), b"trust").unwrap();
    "payload.bin"
}

#[test]
fn a_certificate_the_machine_does_not_know_is_refused_until_its_authority_is_named() {
    let dir = scratch();
    let ca = test_ca(dir.path(), "additive");
    let file = payload(dir.path());
    let server = https_server(dir.path(), Some(&ca), dir.path());

    // Nothing configured: the operating system has never heard of this
    // authority, and the chain is refused by name.
    let bare = probe_with(Vec::new(), server.port, file);
    let failure = bare.failure.expect("an unknown authority must be refused");
    assert_eq!(
        failure.class,
        FailureClass::UntrustedCertificate,
        "{failure:?}"
    );
    assert!(
        failure
            .detail
            .contains("unable to get local issuer certificate"),
        "{}",
        failure.detail
    );
    // It says who refused it, and the class's fix says how to export the
    // issuer on this operating system and which key to name it in.
    assert!(
        failure.detail.contains("the system store does not trust"),
        "{}",
        failure.detail
    );
    assert!(
        FailureClass::UntrustedCertificate
            .fix()
            .contains("extra_ca"),
        "{}",
        FailureClass::UntrustedCertificate.fix()
    );

    // The same authority named: accepted, and reported as the extra one.
    let trusted = probe_with(vec![ca.ca_pem.clone()], server.port, file);
    assert!(trusted.failure.is_none(), "{:?}", trusted.failure);
    let tls = trusted.hops[0].tls.as_ref().unwrap();
    assert_eq!(
        tls.trusted_by,
        Some(TrustSource::ExtraCa(ca.ca_pem.clone()))
    );
    assert_eq!(
        tls.trusted_by.as_ref().unwrap().describe(),
        format!("trusted by extra CA: {}", ca.ca_pem.display())
    );
}

#[test]
fn an_intermediate_the_server_never_sends_can_come_from_the_trust_set() {
    // The corporate shape: the root is deployed, the subordinate authority
    // is deployed, and the server sends only its own certificate.
    let dir = scratch();
    let chain = test_chain(dir.path(), "corp");
    let file = payload(dir.path());
    let leaf_only = support::Ca {
        ca_pem: chain.root_pem.clone(),
        cert: chain.leaf_only.clone(),
        key: chain.key.clone(),
    };
    let server = https_server(dir.path(), Some(&leaf_only), dir.path());

    // The root alone cannot complete a chain whose middle is missing.
    let rootless = probe_with(vec![chain.root_pem.clone()], server.port, file);
    assert!(
        rootless.failure.is_some(),
        "an incomplete chain must not verify"
    );

    // Naming the subordinate authority completes it.
    let completed = probe_with(
        vec![chain.root_pem.clone(), chain.intermediate_pem.clone()],
        server.port,
        file,
    );
    assert!(completed.failure.is_none(), "{:?}", completed.failure);
    assert_eq!(
        completed.hops[0].tls.as_ref().unwrap().trusted_by,
        Some(TrustSource::ExtraCa(chain.intermediate_pem.clone()))
    );
}

#[test]
fn a_self_signed_server_certificate_is_trusted_only_by_naming_its_own_bytes() {
    let dir = scratch();
    let pinned = self_signed(dir.path(), "dev", FIXTURE_HOST);
    let other = self_signed(dir.path(), "other", "elsewhere.test");
    let file = payload(dir.path());
    let server = https_server(dir.path(), Some(&pinned), dir.path());

    // Unnamed, it is just an untrusted certificate.
    assert!(probe_with(Vec::new(), server.port, file).failure.is_some());

    // A different self-signed certificate does not stand in for it: the
    // match is on bytes, not on shape.
    assert!(
        probe_with(vec![other.cert.clone()], server.port, file)
            .failure
            .is_some()
    );

    // Its own bytes, and the probe reports it as a pin rather than as an
    // authority — nothing was signed by it.
    let probe = probe_with(vec![pinned.cert.clone()], server.port, file);
    assert!(probe.failure.is_none(), "{:?}", probe.failure);
    let source = probe.hops[0].tls.as_ref().unwrap().trusted_by.clone();
    assert_eq!(source, Some(TrustSource::Pinned(pinned.cert.clone())));
    assert_eq!(
        source.unwrap().describe(),
        format!("pinned self-signed: {}", pinned.cert.display())
    );
}

#[test]
fn a_pin_holds_only_for_the_host_it_names_and_only_while_it_is_valid() {
    let dir = scratch();
    // A certificate for a name this connection does not use.
    let elsewhere = self_signed(dir.path(), "elsewhere", "elsewhere.test");
    let der = std::fs::read(&elsewhere.cert).unwrap();
    let der: Vec<rustls::pki_types::CertificateDer<'static>> =
        rustls::pki_types::pem::PemObject::pem_slice_iter(&der)
            .collect::<Result<_, _>>()
            .unwrap();
    let set = TrustSet::from_paths(std::slice::from_ref(&elsewhere.cert)).unwrap();

    let now = rustls::pki_types::UnixTime::now();
    let right = rustls::pki_types::ServerName::try_from("elsewhere.test").unwrap();
    let wrong = rustls::pki_types::ServerName::try_from("other.test").unwrap();
    assert_eq!(
        set.attribute(&der, &right, now),
        Some(TrustSource::Pinned(elsewhere.cert.clone())),
        "the host it names is accepted"
    );
    assert_eq!(
        set.attribute(&der, &wrong, now),
        None,
        "another host is not"
    );

    // The same bytes, ten years on: the certificate has expired and the
    // pin no longer stands.
    let later = rustls::pki_types::UnixTime::since_unix_epoch(std::time::Duration::from_secs(
        now.as_secs() + 10 * 365 * 86_400,
    ));
    assert_eq!(set.attribute(&der, &right, later), None);
}

#[test]
fn the_configured_list_reads_files_directories_pem_and_der() {
    let dir = scratch();
    let ca = test_ca(dir.path(), "shapes");
    let chain = test_chain(dir.path(), "shapes2");

    // One file.
    let one = TrustSet::from_paths(std::slice::from_ref(&ca.ca_pem)).unwrap();
    assert_eq!(one.counts(), (1, 0));

    // A list of files.
    let many = TrustSet::from_paths(&[ca.ca_pem.clone(), chain.root_pem.clone()]).unwrap();
    assert_eq!(many.counts(), (2, 0));

    // A directory, PEM and DER side by side, and a file that is neither.
    let certs = dir.path().join("certs");
    std::fs::create_dir(&certs).unwrap();
    std::fs::copy(&ca.ca_pem, certs.join("root.pem")).unwrap();
    as_der(&chain.root_pem, &certs.join("other.cer"));
    std::fs::write(certs.join("README.md"), "not a certificate").unwrap();
    let from_dir = TrustSet::from_paths(std::slice::from_ref(&certs)).unwrap();
    assert_eq!(
        from_dir.counts(),
        (2, 0),
        "both certificates, and only those"
    );

    // A certificate reached twice is one decision.
    let twice = TrustSet::from_paths(&[ca.ca_pem.clone(), certs.join("root.pem")]).unwrap();
    assert_eq!(twice.counts(), (1, 0));

    // A self-signed end-entity certificate is a pin, not an authority.
    let pin = self_signed(dir.path(), "pin", "pin.test");
    assert_eq!(TrustSet::from_paths(&[pin.cert]).unwrap().counts(), (0, 1));
}

#[test]
fn a_named_file_that_holds_no_certificate_is_refused_rather_than_trusted_empty() {
    let dir = scratch();
    let empty = dir.path().join("notes.txt");
    std::fs::write(&empty, "nothing here").unwrap();
    let error = TrustSet::from_paths(std::slice::from_ref(&empty))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("notes.txt") && error.contains("fix:"),
        "{error}"
    );

    let missing = dir.path().join("absent.pem");
    let error = TrustSet::from_paths(&[missing]).unwrap_err().to_string();
    assert!(
        error.contains("absent.pem") && error.contains("fix:"),
        "{error}"
    );

    let bare = dir.path().join("certs");
    std::fs::create_dir(&bare).unwrap();
    let error = TrustSet::from_paths(&[bare]).unwrap_err().to_string();
    assert!(
        error.contains("`*.pem`") && error.contains("fix:"),
        "{error}"
    );
}

#[test]
fn the_bundle_a_child_git_reads_carries_the_machines_own_certificates_too() {
    let dir = scratch();
    // The bundle lives in the settings directory; the test gives it one of
    // its own rather than the operator's.
    let cache = dir.path().join("tls");

    let ca = test_ca(dir.path(), "bundle");
    let set = TrustSet::from_paths(std::slice::from_ref(&ca.ca_pem)).unwrap();
    let path = set
        .git_bundle_in(&cache)
        .unwrap()
        .expect("a configured set has a bundle");
    assert!(path.starts_with(&cache), "{}", path.display());
    assert_eq!(path.extension().unwrap(), "pem");

    let bundle = std::fs::read(&path).unwrap();
    let certificates = rustls::pki_types::pem::PemObject::pem_slice_iter(&bundle)
        .collect::<Result<Vec<rustls::pki_types::CertificateDer<'static>>, _>>()
        .unwrap();
    let (system, _) = vibe_net::trust::system_certificates();
    assert_eq!(
        certificates.len(),
        system.len() + 1,
        "the machine's own certificates, and then the configured one"
    );
    let named = std::fs::read(&ca.ca_pem).unwrap();
    let named: Vec<rustls::pki_types::CertificateDer<'static>> =
        rustls::pki_types::pem::PemObject::pem_slice_iter(&named)
            .collect::<Result<_, _>>()
            .unwrap();
    assert_eq!(certificates.last(), named.first());

    // The name is the content, so asking twice writes one file.
    let again = TrustSet::from_paths(&[ca.ca_pem])
        .unwrap()
        .git_bundle_in(&cache)
        .unwrap()
        .unwrap();
    assert_eq!(again, path);
    assert_eq!(std::fs::read_dir(&cache).unwrap().count(), 1);

    // Nothing configured, nothing handed over: the child uses the
    // platform's own trust, which on Windows is the whole store.
    assert_eq!(
        TrustSet::from_paths(&[])
            .unwrap()
            .git_bundle_in(&cache)
            .unwrap(),
        None
    );
}
