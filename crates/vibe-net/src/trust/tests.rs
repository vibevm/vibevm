//! What the trust set says about itself, without a network.

use super::*;

#[test]
fn nothing_configured_is_the_operating_systems_decision_alone() {
    let set = TrustSet::from_paths(&[]).unwrap();
    assert!(set.is_empty());
    assert_eq!(set.counts(), (0, 0));
    assert!(set.sources().is_empty());
    assert!(set.pem().is_empty());
    // A client can still be built: the operating system is the trust set.
    assert!(set.client_config().is_ok());
    // And a child `git` is handed no file, so it uses that same store.
    assert!(set.git_bundle().unwrap().is_none());
}

#[test]
fn the_refusal_names_who_refused_and_what_to_do_about_it() {
    assert_eq!(
        TrustSet::from_paths(&[]).unwrap().untrusted_by(),
        "which the system store does not trust"
    );
    let help = TrustSet::untrusted_help();
    assert!(help.contains("extra_ca"), "{help}");
    // Generic, by construction: it names stores and commands, never a
    // particular network.
    assert!(!help.contains("corp"), "{help}");
}

#[test]
fn a_trust_source_reads_the_same_way_for_a_person_and_for_a_machine() {
    let system = TrustSource::System;
    assert_eq!((system.kind(), system.source()), ("system", None));
    assert_eq!(system.describe(), "trusted by system");

    let extra = TrustSource::ExtraCa(PathBuf::from("/etc/corp-ca.pem"));
    assert_eq!(extra.kind(), "extra_ca");
    assert_eq!(extra.source(), Some(Path::new("/etc/corp-ca.pem")));
    assert_eq!(extra.describe(), "trusted by extra CA: /etc/corp-ca.pem");

    let pinned = TrustSource::Pinned(PathBuf::from("/home/me/dev.crt"));
    assert_eq!(pinned.kind(), "pinned");
    assert_eq!(pinned.describe(), "pinned self-signed: /home/me/dev.crt");
}

#[test]
fn an_installation_that_names_certificates_says_so_in_its_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ca.pem");
    // A certificate is not needed to assert the sentence; an unreadable
    // path is, because that is the other half of the contract.
    std::fs::write(&path, "-----BEGIN CERTIFICATE-----\nnot base64\n").unwrap();
    assert!(TrustSet::from_paths(&[path]).is_err());
}

#[test]
fn the_offered_protocols_are_only_what_this_build_of_reqwest_can_speak() {
    // A pre-configured TLS configuration replaces reqwest's own, ALPN
    // included. The workspace builds reqwest without its `http2` feature,
    // so offering `h2` would let a server that prefers HTTP/2 — which
    // every large host does — select a protocol `hyper-util` then panics
    // on ("http2 feature is not enabled"), turning every download against
    // a real host into a crash. Found on Windows against a live host, not
    // by a local fixture: no local fixture here negotiates HTTP/2.
    assert_eq!(ALPN, [b"http/1.1".as_slice()]);
    let config = TrustSet::from_paths(&[]).unwrap().client_config().unwrap();
    assert_eq!(config.alpn_protocols, vec![b"http/1.1".to_vec()]);
    assert!(
        !config.alpn_protocols.iter().any(|name| name == b"h2"),
        "offering h2 needs reqwest's `http2` feature in the same change"
    );
}
