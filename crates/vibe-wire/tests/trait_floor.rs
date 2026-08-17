//! Proofs for the trait floor the codegen post-processing puts on every
//! generated type — `Debug`, `Clone`, `PartialEq`, `Eq` beside the serde
//! pair jtd-codegen writes.
//!
//! These are the guard the re-export needs. jtd-codegen's own emission
//! derives `Serialize, Deserialize` and nothing else, while the
//! hand-written types about to be replaced carry the floor everywhere,
//! so a re-export of an unfloored type would strip `{:?}` from every log
//! line, `.clone()` from every caller and `assert_eq!` from every test
//! that touches an index record. The floor's absence is a COMPILE
//! failure rather than a wrong answer, which is why the proof lives in a
//! test that would not build without it: the red form of this file is
//! `error[E0369]` / `error[E0277]`, taken by disabling the pass and
//! regenerating.
//!
//! They also pin the half that is easy to lose: an OPENED vocabulary
//! hands its serde derive over to hand-rolled impls, and the pass that
//! does that must take the serde pair only — an opened enum has no
//! reason to be the one generated type that cannot be printed or
//! compared.

use vibe_wire::generated::index::e1::entry::PackageKind;
use vibe_wire::generated::index::e1::repomd::{NamingConvention, Repomd};

/// A plain generated struct carries the whole floor: it can be cloned,
/// compared and printed. Each of the three is a distinct trait, so the
/// assertions are written to need all three rather than to look tidy.
#[test]
fn a_generated_struct_clones_compares_and_prints() {
    let repomd = Repomd {
        files: Default::default(),
        generated_at: "2026-08-17T00:00:00Z".parse().expect("an RFC 3339 instant"),
        generator: "vibe-index".to_string(),
        naming: NamingConvention::Fqdn,
        package_count: 0,
        registry: "example".to_string(),
        registry_url: "https://example.invalid".to_string(),
        schema_version: 1,
        version_count: 0,
    };

    let copy = repomd.clone();
    assert_eq!(repomd, copy, "the floor's PartialEq compares by value");

    let printed = format!("{repomd:?}");
    assert!(
        printed.contains("vibe-index"),
        "the floor's Debug prints the fields: {printed}"
    );

    let mut other = copy;
    other.schema_version = 2;
    assert_ne!(repomd, other, "a changed field must compare unequal");
}

/// A CLOSED vocabulary is replayed by the vocabulary pass rather than
/// rewritten, so it takes the floor through a different code path — and
/// the two paths have to agree.
#[test]
fn a_closed_vocabulary_carries_the_floor() {
    let naming = NamingConvention::Fqdn;
    assert_eq!(naming.clone(), NamingConvention::Fqdn);
    assert_ne!(naming, NamingConvention::Name);
    assert!(format!("{naming:?}").contains("Fqdn"));
}

/// An OPENED vocabulary loses the derived serde pair to hand-rolled
/// impls, and this is the assertion that it loses ONLY that: the
/// `Unknown` arm compares and prints like every other variant.
#[test]
fn an_opened_vocabulary_keeps_the_floor_it_did_not_have_to_lose() {
    let unknown: PackageKind =
        serde_json::from_str("\"plugin\"").expect("an open vocabulary takes an unknown value");
    assert_eq!(
        unknown.clone(),
        PackageKind::Unknown("plugin".to_string()),
        "the Unknown arm compares by its carried string"
    );
    assert_ne!(unknown, PackageKind::Flow);
    assert!(
        format!("{unknown:?}").contains("plugin"),
        "the Unknown arm prints the value it carries"
    );
}
