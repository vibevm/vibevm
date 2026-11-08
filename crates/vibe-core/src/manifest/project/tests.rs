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
    assert!(p.spec_format.is_none());
    // A groupless project serializes without a `group` key.
    let rendered = toml::to_string(&p).unwrap();
    assert!(!rendered.contains("group"));
}

#[test]
fn project_section_parses_spec_format() {
    let raw = r#"
name = "my-app"
version = "0.1.0"
spec_format = "xml"
"#;
    let p: ProjectSection = toml::from_str(raw).unwrap();
    assert_eq!(p.spec_format, Some(SpecFormat::Xml));
    let rendered = toml::to_string(&p).unwrap();
    assert!(rendered.contains("spec_format = \"xml\""), "{rendered}");
}

#[test]
fn spec_format_rejects_unknown_value() {
    let raw = "name = \"x\"\nversion = \"0.1.0\"\nspec_format = \"html\"\n";
    assert!(toml::from_str::<ProjectSection>(raw).is_err());
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
    assert_eq!(r.auth, None);
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
        assert_eq!(r.auth, Some(expected));
        let rendered = toml::to_string_pretty(&r).unwrap();
        assert!(
            rendered.contains(&format!("auth = \"{raw_value}\"")),
            "{rendered}"
        );
        let back: RegistrySection = toml::from_str(&rendered).unwrap();
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
        auth: Some(AuthKind::TokenEnv),
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
        auth: Some(AuthKind::TokenEnv),
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
        auth: Some(AuthKind::Ssh),
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
        auth: Some(AuthKind::TokenEnv),
        token_env: None,
        enabled: true,
        index_url: None,
    };
    assert!(r.resolve_token_env_name().is_none());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-HOST-POLICY",
    r = 1
)]
fn m_06_a_4_project_sources_presence_and_strict_grammar() {
    let base = serde_json::json!({"name":"host","version":"1.0.0"});
    let legacy: ProjectSection = serde_json::from_value(base.clone()).unwrap();
    assert!(legacy.sources.is_none());
    assert!(
        serde_json::to_value(&legacy)
            .unwrap()
            .get("sources")
            .is_none()
    );
    let mut empty = base.clone();
    empty["sources"] = serde_json::json!([]);
    let empty: ProjectSection = serde_json::from_value(empty).unwrap();
    assert_eq!(empty.sources, Some(vec![]));
    assert_eq!(
        serde_json::to_value(empty).unwrap()["sources"],
        serde_json::json!([])
    );
    let row = serde_json::json!({"file":"native.payload","media":"application/xml","interpretation":"vibe-spec","role":"spec","loading":"explicit"});
    let mut declared = base.clone();
    declared["sources"] = serde_json::json!([row.clone()]);
    let parsed: ProjectSection = serde_json::from_value(declared.clone()).unwrap();
    assert!(
        parsed.group.is_none(),
        "declarations do not invent or require a group"
    );
    assert_eq!(parsed.sources.as_ref().unwrap().len(), 1);
    for bad in [
        serde_json::Value::Null,
        serde_json::json!([{"file":"x","unexpected":true}]),
        serde_json::json!([{"file":"../x","media":"application/xml","interpretation":"vibe-spec","role":"spec","loading":"explicit"}]),
    ] {
        declared["sources"] = bad;
        assert!(serde_json::from_value::<ProjectSection>(declared.clone()).is_err());
    }
    let mut unsupported = base;
    unsupported["source_exports"] = serde_json::json!([]);
    assert!(serde_json::from_value::<ProjectSection>(unsupported).is_err());
    assert!(
        super::super::Manifest::new_project("host", "1.0.0")
            .project
            .unwrap()
            .sources
            .is_none()
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-auth"
)]
fn registry_auth_presence_is_declaration_data_independent_of_transport() {
    for url in [
        "https://example.invalid/org",
        "ssh://git@example.invalid/org",
        "git@example.invalid:org",
        "file:///fixture/registry",
    ] {
        let base = format!("name='r'\nurl='{url}'\n");
        let omitted: RegistrySection = toml::from_str(&base).unwrap();
        let restricted: RegistrySection = toml::from_str(&format!("{base}auth='none'\n")).unwrap();
        assert_eq!(omitted.auth, None);
        assert_eq!(restricted.auth, Some(AuthKind::None));
        assert_ne!(omitted, restricted);
        for row in [omitted, restricted] {
            let rendered = toml::to_string(&row).unwrap();
            assert_eq!(rendered.contains("auth ="), row.auth.is_some());
            assert_eq!(toml::from_str::<RegistrySection>(&rendered).unwrap(), row);
            let json = serde_json::to_value(&row).unwrap();
            assert_eq!(json.get("auth").is_some(), row.auth.is_some());
            assert_eq!(
                serde_json::from_value::<RegistrySection>(json).unwrap(),
                row
            );
        }
    }
    // AuthKind's package/redirect/backend default remains unchanged.
    assert_eq!(AuthKind::default(), AuthKind::None);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-auth"
)]
fn present_registry_auth_rejects_null_and_non_enum_values() {
    for auth in [
        serde_json::Value::Null,
        serde_json::json!(false),
        serde_json::json!(0),
        serde_json::json!([]),
        serde_json::json!({}),
        serde_json::json!("auto"),
    ] {
        let value =
            serde_json::json!({"name":"r","url":"ssh://git@example.invalid/org","auth":auth});
        assert!(serde_json::from_value::<RegistrySection>(value).is_err());
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#global-config"
)]
fn project_and_global_registry_roundtrips_preserve_auth_presence() {
    let rows = "[[registry]]\nname='omitted'\nurl='ssh://git@example.invalid/org'\n[[registry]]\nname='restricted'\nurl='ssh://git@example.invalid/org'\nauth='none'\n";
    let project = super::super::Manifest::parse_str(&format!(
        "[project]\nname='host'\nversion='1.0.0'\n{rows}"
    ))
    .unwrap();
    assert_eq!(project.registries[0].auth, None);
    assert_eq!(project.registries[1].auth, Some(AuthKind::None));
    let rendered = project.render_with_comments(None).unwrap();
    assert_eq!(
        super::super::Manifest::parse_str(&rendered).unwrap(),
        project
    );
    let global: crate::GlobalRegistryConfig = toml::from_str(rows).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("registry.toml");
    global.write_to(&path).unwrap();
    assert_eq!(
        crate::GlobalRegistryConfig::load_from(&path).unwrap(),
        global
    );
    let encoded: toml::Value = toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert!(encoded["registry"][0].get("auth").is_none());
    assert_eq!(encoded["registry"][1]["auth"].as_str(), Some("none"));
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#global-config"
)]
fn project_shadow_keeps_auth_omission_and_restriction_distinct() {
    for (project_auth, global_auth, expected) in [
        ("", "auth='none'\n", None),
        ("auth='none'\n", "", Some(AuthKind::None)),
    ] {
        let section = "[[registry]]\nname='same'\nurl='ssh://git@example.invalid/org'\n";
        let project = super::super::Manifest::parse_str(&format!(
            "[project]\nname='host'\nversion='1.0.0'\n{section}{project_auth}"
        ))
        .unwrap();
        let global: crate::GlobalRegistryConfig =
            toml::from_str(&format!("{section}{global_auth}")).unwrap();
        let effective = crate::merge_effective(&project, &global);
        assert_eq!(effective.registries.len(), 1);
        assert_eq!(effective.registries[0].auth, expected);
    }
    assert!(
        crate::default_registries()
            .iter()
            .all(|row| row.auth.is_none())
    );
}
