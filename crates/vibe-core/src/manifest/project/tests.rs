//! Tests for the `[[registry]]` / `[[mirror]]` / `[[override]]` sections —
//! extracted from `project.rs` to keep that file within the length budget.

use super::*;

#[test]
fn registry_section_rejects_unknown_field() {
    let raw = r#"
name = "r"
url = "git@host:org"
bogus = 1
"#;
    assert!(toml::from_str::<RegistrySection>(raw).is_err());
}

#[test]
fn registry_section_defaults() {
    let raw = r#"
name = "vibespecs"
url = "https://github.com/vibespecs"
"#;
    let r: RegistrySection = toml::from_str(raw).unwrap();
    assert_eq!(r.r#ref, DEFAULT_REGISTRY_REF);
    assert_eq!(r.naming, NamingConvention::Fqdn);
    assert_eq!(r.auth, AuthKind::None);
    assert!(r.token_env.is_none());
    assert!(r.enabled); // on by default
    // Defaults skip on serialize — no spurious diffs.
    let rendered = toml::to_string_pretty(&r).unwrap();
    assert!(!rendered.contains("auth ="));
    assert!(!rendered.contains("naming ="));
    assert!(!rendered.contains("enabled ="));
}

#[test]
fn registry_section_enabled_false_round_trips() {
    // `enabled = false` parses, survives a serialize round-trip, and —
    // unlike the default `true` — is written out so the switch-off is
    // visible in the file (PROP-002 §2.2.3 #enabled).
    let raw = "name = \"r\"\nurl = \"https://x/y\"\nenabled = false\n";
    let r: RegistrySection = toml::from_str(raw).unwrap();
    assert!(!r.enabled);
    let rendered = toml::to_string_pretty(&r).unwrap();
    assert!(rendered.contains("enabled = false"));
    let back: RegistrySection = toml::from_str(&rendered).unwrap();
    assert_eq!(r, back);
}

#[test]
fn auth_kind_variants_roundtrip() {
    for (raw_value, expected) in [
        ("none", AuthKind::None),
        ("token-env", AuthKind::TokenEnv),
        ("credential-helper", AuthKind::CredentialHelper),
        ("ssh", AuthKind::Ssh),
    ] {
        let raw = format!("name = \"r\"\nurl = \"https://x/y\"\nauth = \"{raw_value}\"\n");
        let r: RegistrySection = toml::from_str(&raw).unwrap();
        assert_eq!(r.auth, expected);
        let back: RegistrySection = toml::from_str(&toml::to_string_pretty(&r).unwrap()).unwrap();
        assert_eq!(r, back);
    }
}

#[test]
fn auth_kind_rejects_unknown_value() {
    let raw = "name = \"r\"\nurl = \"https://x/y\"\nauth = \"bogus\"\n";
    assert!(toml::from_str::<RegistrySection>(raw).is_err());
}

#[test]
fn naming_convention_repo_name() {
    use crate::package_ref::{Group, PackageKind};
    let org = Group::parse("org.vibevm").unwrap();
    assert_eq!(
        NamingConvention::Fqdn.repo_name(None, &org, "wal").unwrap(),
        "org.vibevm_wal"
    );
    assert_eq!(
        NamingConvention::KindName
            .repo_name(Some(PackageKind::Flow), &org, "wal")
            .unwrap(),
        "flow-wal"
    );
    assert_eq!(
        NamingConvention::Name
            .repo_name(Some(PackageKind::Stack), &org, "rust-cli")
            .unwrap(),
        "rust-cli"
    );
    assert_eq!(
        NamingConvention::KindSlashName
            .repo_name(Some(PackageKind::Feat), &org, "welcome-page")
            .unwrap(),
        "feat/welcome-page"
    );
    // A legacy `kind-*` convention without a kind is an error.
    assert!(
        NamingConvention::KindName
            .repo_name(None, &org, "wal")
            .is_err()
    );
}

#[test]
fn resolve_token_env_name_derives_from_host() {
    let r = RegistrySection {
        name: "r".into(),
        url: "https://gitlab.company.com/vibespecs".into(),
        r#ref: "main".into(),
        naming: NamingConvention::KindName,
        auth: AuthKind::TokenEnv,
        token_env: None,
        enabled: true,
    };
    assert_eq!(
        r.resolve_token_env_name().as_deref(),
        Some("VIBEVM_REGISTRY_TOKEN_GITLAB_COMPANY_COM")
    );
}

#[test]
fn resolve_token_env_name_honours_explicit_override() {
    let r = RegistrySection {
        name: "r".into(),
        url: "https://gitlab.company.com/vibespecs".into(),
        r#ref: "main".into(),
        naming: NamingConvention::KindName,
        auth: AuthKind::TokenEnv,
        token_env: Some("MY_CUSTOM_TOKEN".to_string()),
        enabled: true,
    };
    assert_eq!(
        r.resolve_token_env_name().as_deref(),
        Some("MY_CUSTOM_TOKEN")
    );
}

#[test]
fn resolve_token_env_name_handles_scp_form() {
    let r = RegistrySection {
        name: "r".into(),
        url: "git@gitlab.company.com:vibespecs".into(),
        r#ref: "main".into(),
        naming: NamingConvention::KindName,
        auth: AuthKind::Ssh,
        token_env: None,
        enabled: true,
    };
    assert_eq!(
        r.resolve_token_env_name().as_deref(),
        Some("VIBEVM_REGISTRY_TOKEN_GITLAB_COMPANY_COM")
    );
}

#[test]
fn resolve_token_env_name_returns_none_for_file_url() {
    let r = RegistrySection {
        name: "r".into(),
        url: "file:///tmp/registry".into(),
        r#ref: "main".into(),
        naming: NamingConvention::KindName,
        auth: AuthKind::TokenEnv,
        token_env: None,
        enabled: true,
    };
    assert!(r.resolve_token_env_name().is_none());
}
