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
        admitted: vec!["corp".into()],
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
    // The refusal names what WAS asked beside what was not.
    assert!(rendered.contains("allowed registries: corp"), "{rendered}");
    let nothing_allowed = SourcePolicyError::ExcludedOnly {
        package: "org.policy/demo".into(),
        endpoints: vec!["https://github.com".into()],
        admitted: Vec::new(),
    };
    assert!(
        nothing_allowed.to_string().contains(
            "allowed registries: (none — the source policy disables every configured registry)"
        ),
        "{nothing_allowed}"
    );
}

/// The registry-enumeration decision (`PROP-073#SOURCE-POLICY`): a registry
/// URL in, "is this hosting excluded by the effective policy" out. Nothing is
/// opened, resolved or contacted to decide it.
#[test]
fn an_excluded_hosting_disables_exactly_the_registry_urls_it_names() {
    let site: SourcePolicyDecl = toml::from_str("exclude_hosting=['github','gitverse']").unwrap();
    let empty = SourcePolicyDecl::default();
    let policy = EffectiveSourcePolicy::merge(&site, &empty, &empty).unwrap();

    for excluded in [
        "https://github.com/vibespecs",
        "https://GitHub.com/vibespecs",
        "git+https://github.com/vibespecs",
        "git@github.com:vibespecs/node.git",
        "ssh://git@ssh.github.com:443/vibespecs/node.git",
        "https://raw.githubusercontent.com/vibespecs/node",
        "https://gitverse.ru/vibespecs",
        "https://api.gitverse.ru/vibespecs",
    ] {
        assert!(
            policy.excludes_endpoint(excluded),
            "`{excluded}` must be disabled by the policy"
        );
    }
    assert_eq!(
        policy
            .excluded_by("https://gitverse.ru/vibespecs")
            .map(ToString::to_string),
        Some("gitverse".to_owned())
    );

    // The site registry, and every spelling that names no excluded hosting,
    // stays allowed. A suffix of an excluded host is a different host.
    for allowed in [
        "ssh://git@mono.example.invalid/corp/mono.git",
        "git@mono.example.invalid:corp/mono.git",
        "file:///srv/corp/mono.git",
        "/srv/corp/mono.git",
        "C:\\srv\\corp\\mono.git",
        "https://github.com.attacker.invalid/vibespecs",
        "https://notgithub.com/vibespecs",
        "",
    ] {
        assert!(
            !policy.excludes_endpoint(allowed),
            "`{allowed}` must stay allowed"
        );
        assert!(policy.excluded_by(allowed).is_none(), "{allowed}");
    }
}

/// An exact host selector disables that one host and nothing around it, and
/// an empty policy disables nothing at all.
#[test]
fn an_exact_host_exclusion_and_an_empty_policy_decide_narrowly() {
    let site: SourcePolicyDecl =
        toml::from_str("exclude_hosting=['host:MONO.example.invalid.']").unwrap();
    let empty = SourcePolicyDecl::default();
    let policy = EffectiveSourcePolicy::merge(&site, &empty, &empty).unwrap();
    assert!(policy.excludes_endpoint("ssh://git@mono.example.invalid/corp.git"));
    assert!(policy.excludes_endpoint("git@mono.example.invalid:corp.git"));
    assert!(!policy.excludes_endpoint("https://github.com/vibespecs"));
    assert!(!policy.excludes_endpoint("ssh://git@other.example.invalid/corp.git"));
    assert_eq!(
        policy
            .excluded_selectors()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        vec!["host:mono.example.invalid".to_owned()]
    );

    let neutral = EffectiveSourcePolicy::merge(&empty, &empty, &empty).unwrap();
    assert!(!neutral.excludes_endpoint("https://github.com/vibespecs"));
    assert_eq!(neutral.excluded_selectors().count(), 0);
}
