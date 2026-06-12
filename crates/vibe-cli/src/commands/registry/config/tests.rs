//! Unit tests for the registry-config helpers.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#registry");

use super::{adapter_for_host, parse_naming};
use vibe_core::manifest::NamingConvention;

#[test]
fn adapter_for_host_picks_github() {
    assert_eq!(adapter_for_host("github.com"), Some("github"));
    assert_eq!(adapter_for_host("api.github.com"), Some("github"));
    assert_eq!(adapter_for_host("GITHUB.com"), Some("github"));
}

#[test]
fn adapter_for_host_picks_gitverse() {
    assert_eq!(adapter_for_host("gitverse.ru"), Some("gitverse"));
    assert_eq!(adapter_for_host("api.gitverse.ru"), Some("gitverse"));
}

#[test]
fn adapter_for_host_returns_none_for_unknown_host() {
    assert_eq!(adapter_for_host("example.invalid"), None);
    assert_eq!(adapter_for_host(""), None);
}

#[test]
fn parse_naming_accepts_canonical_spellings() {
    assert!(matches!(
        parse_naming("kind-name").unwrap(),
        NamingConvention::KindName
    ));
    assert!(matches!(
        parse_naming("name").unwrap(),
        NamingConvention::Name
    ));
    assert!(matches!(
        parse_naming("kind/name").unwrap(),
        NamingConvention::KindSlashName
    ));
}

#[test]
fn parse_naming_rejects_unknown_value() {
    let err = parse_naming("KindName").unwrap_err();
    // Spelling mismatch — must match the serde rename exactly.
    assert!(err.to_string().contains("unknown naming convention"));
}
