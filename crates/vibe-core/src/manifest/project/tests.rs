//! Tests for the `[[registry]]` / `[[mirror]]` / `[[override]]` sections —
//! extracted from `project.rs` to keep that file within the length budget.

use super::*;

// ----- `[project].group` — the self-coordinate group half (B-031) -----------

#[test]
fn project_section_parses_with_group() {
    // A project carrying its self-coordinate group round-trips through serde.
    let raw = r#"
name = "vibevm"
group = "org.vibevm.core"
version = "0.1.0-dev"
"#;
    let p: ProjectSection = toml::from_str(raw).unwrap();
    assert_eq!(p.name, "vibevm");
    assert_eq!(p.group.as_ref().unwrap().as_str(), "org.vibevm.core");
    // The group survives a serialize round-trip (and absent fields stay absent).
    let back: ProjectSection = toml::from_str(&toml::to_string(&p).unwrap()).unwrap();
    assert_eq!(p, back);
}

#[test]
fn project_section_parses_without_group() {
    // `group` is optional — a project with no self coordinate is legal.
    let raw = r#"
name = "my-app"
version = "0.1.0"
"#;
    let p: ProjectSection = toml::from_str(raw).unwrap();
    assert_eq!(p.name, "my-app");
    assert!(p.group.is_none());
    // A groupless project serializes without a `group` key.
    let rendered = toml::to_string(&p).unwrap();
    assert!(!rendered.contains("group"));
}

#[test]
fn project_section_rejects_a_malformed_group() {
    // Group validation runs on parse (PROP-008): an uppercase segment is bad.
    let raw = r#"
name = "x"
group = "Org.Bad"
version = "0.1.0"
"#;
    assert!(toml::from_str::<ProjectSection>(raw).is_err());
}

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
        "org.vibevm.wal"
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
        index_url: None,
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
        index_url: None,
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
        index_url: None,
    };
    assert_eq!(
        r.resolve_token_env_name().as_deref(),
        Some("VIBEVM_REGISTRY_TOKEN_GITLAB_COMPANY_COM")
    );
}

// ----- `[[registry]].index_url` — the PROP-005 index-location key (B-083) --

/// The TOML block from PROP-005 `##INDEX-URL-CONFIG`, verbatim — comments
/// included — plus the two alternative `index_url` lines the block carries
/// de-commented. B-083's whole defect was this block being a parse
/// refusal: `RegistrySection` is strict (`deny_unknown_fields`) and did
/// not know the key, so a reader copying the spec's example got a
/// manifest-load error. Field-level asserts live below with the key
/// itself; this test's job is the block parsing at all.
#[test]
fn prop005_index_url_example_parses() {
    let verbatim = r#"
name = "vibespecs"
url = "https://github.com/vibespecs"
naming = "kind-name"
index_url = "https://github.com/vibespecs/index"  # default; explicit override allowed
# or, to point at a hosted server:
# index_url = "https://index.vibespecs.dev"
# or, to disable index lookup entirely:
# index_url = "none"
"#;
    let r: RegistrySection = toml::from_str(verbatim).unwrap();
    assert_eq!(r.name, "vibespecs");
    assert_eq!(
        r.index_url.as_deref(),
        Some("https://github.com/vibespecs/index")
    );

    let hosted = r#"
name = "vibespecs"
url = "https://github.com/vibespecs"
naming = "kind-name"
index_url = "https://index.vibespecs.dev"
"#;
    let r: RegistrySection = toml::from_str(hosted).unwrap();
    assert_eq!(r.index_url.as_deref(), Some("https://index.vibespecs.dev"));

    let disabled = r#"
name = "vibespecs"
url = "https://github.com/vibespecs"
naming = "kind-name"
index_url = "none"
"#;
    let r: RegistrySection = toml::from_str(disabled).unwrap();
    // `"none"` is carried verbatim — the disable semantics live in the
    // resolution ladder (vibe-registry), not in the schema.
    assert_eq!(r.index_url.as_deref(), Some("none"));
}

#[test]
fn registry_section_without_index_url_is_none() {
    let r: RegistrySection = toml::from_str(
        r#"
name = "r"
url = "https://github.com/vibespecs"
"#,
    )
    .unwrap();
    assert!(r.index_url.is_none());
    // An unset key serializes away — a round-trip adds nothing.
    let rendered = toml::to_string(&r).unwrap();
    assert!(!rendered.contains("index_url"));
}

#[test]
fn registry_section_still_refuses_unknown_fields_alongside_index_url() {
    // The section stays strict with the new key present: a garbage
    // extra key is a refusal, exactly as before the key existed.
    let raw = r#"
name = "r"
url = "https://github.com/vibespecs"
index_url = "https://github.com/vibespecs/index"
bogus = 1
"#;
    assert!(toml::from_str::<RegistrySection>(raw).is_err());
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
        index_url: None,
    };
    assert!(r.resolve_token_env_name().is_none());
}
