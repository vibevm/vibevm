use super::*;

#[test]
fn old_documents_omit_policy_and_explicit_empty_preference_survives() {
    let old =
        crate::manifest::Manifest::parse_str("[project]\nname='demo'\nversion='1.0.0'").unwrap();
    assert!(old.source_policy.is_empty());
    assert!(!toml::to_string(&old).unwrap().contains("source_policy"));
    let policy: SourcePolicyDecl = toml::from_str("prefer_hosting=[]").unwrap();
    assert_eq!(policy.prefer_hosting, Some(vec![]));
    assert!(toml::to_string(&policy).unwrap().contains("prefer_hosting"));
    assert!(serde_json::from_str::<SourcePolicyDecl>(r#"{"prefer_hosting":null}"#).is_err());
    assert!(serde_json::from_str::<SourcePolicyDecl>(r#"{"exclude_hosting":null}"#).is_err());
}

#[test]
fn highest_explicit_preference_wins_exclusions_remain_monotonic() {
    let user =
        toml::from_str("prefer_hosting=['github']\nexclude_hosting=['host:EXAMPLE.invalid.']")
            .unwrap();
    let project =
        toml::from_str("prefer_hosting=['gitverse']\nexclude_hosting=['github']").unwrap();
    let clear = toml::from_str("prefer_hosting=[]").unwrap();
    let effective = EffectiveSourcePolicy::merge(&user, &project, &clear).unwrap();
    assert!(!effective.has_preference());
    assert!(effective.denied_host_keys().contains("example.invalid"));
    assert!(
        effective
            .denied_host_keys()
            .contains("raw.githubusercontent.com")
    );
    let effective =
        EffectiveSourcePolicy::merge(&user, &project, &SourcePolicyDecl::default()).unwrap();
    assert!(
        effective.preference_rank("https://gitverse.ru/o/r")
            < effective.preference_rank("https://github.com/o/r")
    );
}

#[test]
fn verified_family_keys_have_exact_boundaries_and_generic_hosts_have_no_port_semantics() {
    for address in [
        "https://github.com/o/r",
        "https://api.github.com/repos/o/r",
        "https://raw.githubusercontent.com/o/r/main/a",
        "https://codeload.github.com/o/r/zip/main",
        "https://release-assets.githubusercontent.com/a",
    ] {
        assert!(HostingSelector::Github.matches_endpoint(address));
    }
    for address in [
        "https://evilgithub.com/o/r",
        "https://api.github.com.attacker.invalid/a",
        "https://raw.githubusercontent.com.attacker.invalid/a",
        "https://evilraw.githubusercontent.com/a",
        "ssh://alias/r",
    ] {
        assert!(!HostingSelector::Github.matches_endpoint(address));
    }
    let exact = HostingSelector::try_from("host:EXAMPLE.invalid.".to_owned()).unwrap();
    assert!(exact.matches_endpoint("https://example.invalid:8443/a"));
    assert!(!exact.matches_endpoint("https://sub.example.invalid/a"));
    assert!(HostingSelector::try_from("host:example.invalid:443".to_owned()).is_err());
    assert!(HostingSelector::try_from("host:example.invalid..".to_owned()).is_err());
}

#[test]
fn duplicates_and_unknown_selectors_refuse_before_construction() {
    let duplicate = toml::from_str("prefer_hosting=['github','github']").unwrap();
    assert!(matches!(
        EffectiveSourcePolicy::merge(
            &duplicate,
            &SourcePolicyDecl::default(),
            &SourcePolicyDecl::default()
        ),
        Err(SourcePolicyError::DuplicateHostingSelector)
    ));
    assert!(toml::from_str::<SourcePolicyDecl>("exclude_hosting=['guess-company']").is_err());
}

#[test]
fn registry_pin_roundtrip_preserves_inline_metadata_and_version_var() {
    let requires: crate::manifest::Requires = toml::from_str("[packages]\n'org.example/a'={version='^1',registry='team',optional=false}\n'org.example/b'={version.var='shared',registry='mirror'}").unwrap();
    let roundtrip: crate::manifest::Requires =
        toml::from_str(&toml::to_string(&requires).unwrap()).unwrap();
    assert_eq!(requires, roundtrip);
    assert_eq!(roundtrip.registry_pins["org.example/a"].as_str(), "team");
    assert_eq!(roundtrip.registry_pins["org.example/b"].as_str(), "mirror");
    for declaration in [
        "registry='team',path='../a'",
        "registry='team',git='https://example.invalid/a',tag='v1.0.0'",
        "registry='team',auth='ssh'",
    ] {
        assert!(
            toml::from_str::<crate::manifest::Requires>(&format!(
                "[packages]\n'org.example/a'={{{declaration}}}"
            ))
            .is_err()
        );
    }
    assert!(
        serde_json::from_str::<crate::manifest::Requires>(
            r#"{"packages":{"org.example/a":{"version":"^1","registry":null}}}"#
        )
        .is_err()
    );
}

#[test]
fn github_family_includes_documented_ssh_effective_host_without_suffix_expansion() {
    let host = HostingSelector::Github;
    assert!(host.matches_endpoint("ssh://git@ssh.github.com:443/team/repo.git"));
    assert!(!host.matches_endpoint("ssh://git@ssh.github.com.attacker.invalid:443/team/repo.git"));
}

#[test]
fn excluded_only_requirement_names_the_package_and_refused_endpoints() {
    let error = SourcePolicyError::ExcludedOnly {
        package: "org.policy/demo".into(),
        endpoints: vec!["http://127.0.0.1:9".into(), "https://github.com".into()],
    };
    assert_eq!(error.code(), "source_host_excluded");
    let rendered = error.to_string();
    assert!(rendered.starts_with("source_host_excluded: "), "{rendered}");
    assert!(rendered.contains("org.policy/demo"), "{rendered}");
    assert!(
        rendered.contains("http://127.0.0.1:9, https://github.com"),
        "{rendered}"
    );
    assert!(
        rendered.contains("PROP-073#SOURCE-POLICY-LAW"),
        "{rendered}"
    );
}
