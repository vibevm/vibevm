specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-LAYOUT");
use super::*;
use crate::{layout as names, settings};
use std::path::Path;
fn r(text: &str) -> Reference {
    Reference::new(text).unwrap()
}
fn instance(text: &str) -> TargetInstanceId {
    TargetInstanceId::new(text, "admitting-owner").unwrap()
}
fn path(parts: &[&[u8]]) -> AbsoluteTargetPath {
    AbsoluteTargetPath::new(
        TargetPathPolicy::LinuxBytes,
        TargetAnchor::PosixRoot,
        parts.iter().map(|s| s.to_vec()).collect(),
    )
    .unwrap()
}
fn relative(parts: &[&[u8]]) -> TargetRelativePath {
    TargetRelativePath::new(
        TargetPathPolicy::LinuxBytes,
        parts.iter().map(|s| s.to_vec()).collect(),
    )
    .unwrap()
}
fn refs(text: &str) -> RootBindingReferences {
    RootBindingReferences {
        binding: r(text),
        producer: r("storage-owner"),
        physical_evidence: Some(r("claimed/unverified/evidence")),
    }
}
fn spec(parts: &[&[u8]], id: &str) -> RootSpec {
    RootSpec {
        references: refs(id),
        path: path(parts),
    }
}
fn input() -> SettingsLayoutInputs {
    SettingsLayoutInputs {
        instance: instance("instance"),
        principal: r("user"),
        policy: TargetPathPolicy::LinuxBytes,
        mode: TargetMode::WorkspaceProjection,
        settings: Some(spec(&[b"settings"], "settings-binding")),
        store_references: refs("immutable-binding"),
        state_references: refs("mutable-binding"),
        profile: None,
        cache: CacheSelection::Unused,
        home: None,
        config_file_override: None,
    }
}
fn bound(layout: &EnvironmentLayout, role: RootRole) -> &DeclaredRootBinding {
    let RootSlot::Bound(binding) = layout.slot(role) else {
        panic!("root is not bound")
    };
    binding
}
fn bytes(p: &AbsoluteTargetPath) -> Vec<Vec<u8>> {
    p.components().iter().map(|c| c.bytes().to_vec()).collect()
}
fn resource(
    root: &DeclaredRootBinding,
    parts: &[&[u8]],
    comparison: CaseComparison,
) -> ResourceIdentity {
    ResourceIdentity::new(
        root.instance().clone(),
        root,
        root.principal().clone(),
        relative(parts),
        comparison,
    )
    .unwrap()
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-LAYOUT-DEFAULTS",
    r = 1
)]
fn settings_table_missing_s_keeps_required_roots_unresolved() {
    let mut inputs = input();
    inputs.settings = None;
    inputs.home = Some(path(&[b"home", b"user"]));
    let layout = EnvironmentLayout::settings_anchored(inputs).unwrap();
    for role in [
        RootRole::Configuration,
        RootRole::ImmutableStore,
        RootRole::MutableState,
    ] {
        assert_eq!(layout.slot(role), &RootSlot::MissingRequired);
    }
    assert_eq!(
        layout.missing_required(),
        [
            RootRole::Configuration,
            RootRole::ImmutableStore,
            RootRole::MutableState
        ]
    );
    assert_eq!(layout.slot(RootRole::Profile), &RootSlot::Inapplicable);
    assert_eq!(layout.home(), Some(&path(&[b"home", b"user"])));
    let choices = [
        ExplicitRootChoice::Required(None),
        ExplicitRootChoice::Inapplicable,
        ExplicitRootChoice::Inapplicable,
        ExplicitRootChoice::Required(None),
        ExplicitRootChoice::Inapplicable,
    ];
    let explicit = EnvironmentLayout::explicit(
        instance("explicit"),
        r("user"),
        TargetPathPolicy::LinuxBytes,
        choices,
        None,
        None,
    )
    .unwrap();
    assert_eq!(
        explicit.missing_required(),
        [RootRole::Configuration, RootRole::MutableState]
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-LAYOUT-DEFAULTS",
    r = 1
)]
fn explicit_profile_cache_and_home_bindings_do_not_fall_back() {
    let mut inputs = input();
    inputs.mode = TargetMode::UserProfile;
    inputs.home = Some(path(&[b"home", b"other"]));
    inputs.config_file_override = Some(path(&[b"overrides", b"chosen.toml"]));
    inputs.cache = CacheSelection::Requested(None);
    let unresolved = EnvironmentLayout::settings_anchored(inputs.clone()).unwrap();
    assert_eq!(
        unresolved.slot(RootRole::Profile),
        &RootSlot::MissingRequired
    );
    assert_eq!(
        unresolved.slot(RootRole::DisposableCache),
        &RootSlot::MissingRequired
    );
    inputs.profile = Some(spec(&[b"profiles", b"explicit"], "profile-id"));
    inputs.cache = CacheSelection::Requested(Some(spec(&[b"temp-cache"], "cache-id")));
    inputs.store_references = refs("../opaque/instance\nreference");
    let layout = EnvironmentLayout::settings_anchored(inputs).unwrap();
    assert!(layout.missing_required().is_empty());
    assert_eq!(
        bytes(bound(&layout, RootRole::Configuration).path()),
        [b"settings".to_vec()]
    );
    assert_eq!(
        bytes(bound(&layout, RootRole::ImmutableStore).path()),
        [b"settings".to_vec(), b"store".to_vec()]
    );
    assert_eq!(
        bytes(bound(&layout, RootRole::MutableState).path()),
        [b"settings".to_vec(), b"state".to_vec()]
    );
    assert_eq!(
        bound(&layout, RootRole::ImmutableStore)
            .references()
            .binding
            .as_str(),
        "../opaque/instance\nreference"
    );
    assert_eq!(
        layout.config_file_override(),
        Some(&path(&[b"overrides", b"chosen.toml"]))
    );
    assert_eq!(layout.home(), Some(&path(&[b"home", b"other"])));
    let root = Path::new("selected-settings");
    assert_eq!(
        settings::user_config_path_from(root),
        root.join("config.toml")
    );
    assert_eq!(
        settings::settings_toml_path_from(root),
        root.join("settings.toml")
    );
    assert_eq!(
        settings::registry_config_path_from(root),
        root.join("registry.toml")
    );
    assert_eq!(
        settings::registries_cache_dir_from(root),
        root.join("registries")
    );
    assert_eq!(
        settings::search_cache_dir_from(root),
        root.join("search-cache")
    );
    assert_eq!(
        (
            names::IMMUTABLE_STORE_DIR,
            names::LAUNCHER_BIN_DIR,
            names::MUTABLE_STATE_DIR,
            names::DEPLOY_STATE_RELATIVE
        ),
        ("store", "bin", "state", "state/deployments")
    );
    assert_eq!(
        settings::settings_dir_from(Some(root.into()), Some(Path::new("different-home").into())),
        Some(root.into())
    );
    assert_eq!(
        settings::settings_dir_from(None, Some(root.into())),
        Some(root.join(".vibe"))
    );
    assert_eq!(names::USER_SETTINGS_DIR, ".vibe");
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#TARGET-PATH-POLICY",
    r = 2
)]
fn linux_non_utf8_components_survive_windows_control_plane() {
    let windows_executor = PlatformFacts::new(
        Fact::Known(crate::manifest::TargetOs::Windows),
        Fact::Known(r("x86_64")),
        Fact::Unknown,
        Fact::NotApplicable,
    );
    assert_eq!(
        windows_executor.os(),
        &Fact::Known(crate::manifest::TargetOs::Windows)
    );
    let raw = vec![0xff, b':', b'\\', 0x80];
    let component = TargetComponent::new(TargetPathPolicy::LinuxBytes, raw.clone()).unwrap();
    assert_eq!(component.bytes(), raw);
    for literal in [
        b"COM1".as_slice(),
        b"name:colon",
        b"C:\\literal",
        b"trailing.",
        b"trailing ",
        b"line\nname",
    ] {
        assert!(TargetComponent::new(TargetPathPolicy::LinuxBytes, literal.to_vec()).is_ok());
    }
    let layout = EnvironmentLayout::settings_anchored(input()).unwrap();
    let root = bound(&layout, RootRole::ImmutableStore);
    let identity = resource(root, &[&raw], CaseComparison::ByteSensitive);
    assert_eq!(identity.path().components()[0].bytes(), raw);
    assert_eq!(
        identity.lexical_compare(&identity),
        ResourceComparison::SameDeclaredIdentity
    );
    assert!(TargetComponent::new(TargetPathPolicy::WindowsPortableWin32Utf8, raw).is_err());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#TARGET-PATH-POLICY",
    r = 2
)]
fn traversal_and_windows_reserved_rules_use_only_target_policy() {
    for policy in [
        TargetPathPolicy::LinuxBytes,
        TargetPathPolicy::MacosPosixBytes,
        TargetPathPolicy::WindowsPortableWin32Utf8,
    ] {
        for invalid in [b"".as_slice(), b".", b"..", b"a/b", b"a\0b"] {
            assert!(TargetComponent::new(policy, invalid.to_vec()).is_err());
        }
    }
    for invalid in [
        "COM1.txt",
        "LPT³.log",
        "CONIN$",
        "trailing.",
        "trailing ",
        "a:b",
        "a\\b",
        "a?b",
    ] {
        assert!(
            TargetComponent::new(
                TargetPathPolicy::WindowsPortableWin32Utf8,
                invalid.as_bytes().to_vec()
            )
            .is_err()
        );
        assert!(
            TargetComponent::new(TargetPathPolicy::LinuxBytes, invalid.as_bytes().to_vec()).is_ok()
        );
    }
    let win = TargetPathPolicy::WindowsPortableWin32Utf8;
    assert!(
        AbsoluteTargetPath::new(
            win,
            TargetAnchor::WindowsDrive(b'C'),
            vec![b"valid".to_vec()]
        )
        .is_ok()
    );
    assert!(AbsoluteTargetPath::new(win, TargetAnchor::WindowsDrive(b'1'), vec![]).is_err());
    assert!(
        AbsoluteTargetPath::new(
            win,
            TargetAnchor::WindowsUnc {
                server: b"server".to_vec(),
                share: b"share".to_vec()
            },
            vec![]
        )
        .is_ok()
    );
    assert!(
        AbsoluteTargetPath::new(
            win,
            TargetAnchor::WindowsUnc {
                server: b"server/path".to_vec(),
                share: b"share".to_vec()
            },
            vec![]
        )
        .is_err()
    );
    assert!(
        AbsoluteTargetPath::new(
            TargetPathPolicy::LinuxBytes,
            TargetAnchor::WindowsDrive(b'C'),
            vec![]
        )
        .is_err()
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-RESOURCE-IDENTITY",
    r = 1
)]
fn distinct_instances_share_immutable_content_without_sharing_resource_identity() {
    let first = EnvironmentLayout::settings_anchored(input()).unwrap();
    let mut next = input();
    next.instance = instance("other-instance");
    next.state_references = refs("other-mutable-binding");
    let second = EnvironmentLayout::settings_anchored(next).unwrap();
    let a = bound(&first, RootRole::ImmutableStore);
    let b = bound(&second, RootRole::ImmutableStore);
    assert_eq!(a.references(), b.references());
    assert_eq!(a.path(), b.path());
    let content = crate::ContentHash::parse("sha256:abc").unwrap();
    let name = content.as_str().as_bytes();
    let ia = resource(a, &[name], CaseComparison::ByteSensitive);
    let ib = resource(b, &[name], CaseComparison::ByteSensitive);
    assert_eq!(ia.path(), ib.path());
    assert_eq!(
        ia.lexical_compare(&ib),
        ResourceComparison::DifferentDeclaredIdentity
    );
    let ma = resource(
        bound(&first, RootRole::MutableState),
        &[b"data"],
        CaseComparison::ByteSensitive,
    );
    let mb = resource(
        bound(&second, RootRole::MutableState),
        &[b"data"],
        CaseComparison::ByteSensitive,
    );
    assert_eq!(
        ma.lexical_compare(&mb),
        ResourceComparison::DifferentDeclaredIdentity
    );
    let different_role = DeclaredRootBinding::new(
        a.instance().clone(),
        RootRole::MutableState,
        a.references().clone(),
        a.principal().clone(),
        a.path().clone(),
    );
    assert_eq!(
        ia.lexical_compare(&resource(
            &different_role,
            &[name],
            CaseComparison::ByteSensitive
        )),
        ResourceComparison::DifferentDeclaredIdentity
    );
    let different_principal = DeclaredRootBinding::new(
        a.instance().clone(),
        a.role(),
        a.references().clone(),
        r("another-user"),
        a.path().clone(),
    );
    assert_eq!(
        ia.lexical_compare(&resource(
            &different_principal,
            &[name],
            CaseComparison::ByteSensitive
        )),
        ResourceComparison::DifferentDeclaredIdentity
    );
    assert!(
        ResourceIdentity::new(
            instance("foreign"),
            a,
            a.principal().clone(),
            relative(&[name]),
            CaseComparison::ByteSensitive
        )
        .is_err()
    );
    assert!(
        ResourceIdentity::new(
            a.instance().clone(),
            a,
            r("foreign"),
            relative(&[name]),
            CaseComparison::ByteSensitive
        )
        .is_err()
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#TARGET-PATH-POLICY",
    r = 2
)]
fn lexical_collision_ascii_limits_and_unknown_policy_are_explicit() {
    let layout = EnvironmentLayout::settings_anchored(input()).unwrap();
    let root = bound(&layout, RootRole::ImmutableStore);
    let a = resource(root, &[b"Tool"], CaseComparison::ByteSensitive);
    let b = resource(root, &[b"tool"], CaseComparison::ByteSensitive);
    assert_eq!(
        a.lexical_compare(&b),
        ResourceComparison::DifferentDeclaredIdentity
    );
    let a = resource(root, &[b"Tool"], CaseComparison::AsciiInsensitive);
    let b = resource(root, &[b"tool"], CaseComparison::AsciiInsensitive);
    assert_eq!(
        a.lexical_compare(&b),
        ResourceComparison::SameDeclaredIdentity
    );
    assert_ne!(a.path(), b.path());
    let raw = resource(root, &[&[0xff]], CaseComparison::AsciiInsensitive);
    assert_eq!(raw.lexical_compare(&raw), ResourceComparison::Unresolved);
    let unknown = resource(root, &[b"same"], CaseComparison::Unknown);
    assert_eq!(
        unknown.lexical_compare(&unknown),
        ResourceComparison::Unresolved
    );
    assert_eq!(
        path(&[]).lexical_relation(&path(&[]), CaseComparison::Unknown),
        LexicalRelation::Unresolved
    );
    let unicode = resource(root, &["é".as_bytes()], CaseComparison::AsciiInsensitive);
    assert_eq!(
        unicode.lexical_compare(&unicode),
        ResourceComparison::Unresolved
    );
    assert_eq!(
        path(&[b"Tool"]).lexical_relation(&path(&[b"tool"]), CaseComparison::AsciiInsensitive),
        LexicalRelation::Equal
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#PAYLOAD-PREFIX-COMPATIBILITY",
    r = 1
)]
fn prefix_bound_or_missing_relocation_evidence_refuses_relocation_claim() {
    let usr = path(&[b"usr"]);
    let home = path(&[b"home", b"user"]);
    let bound = PrefixCompatibility::PrefixBound {
        exact_target_prefix: usr.clone(),
    };
    assert_eq!(
        bound.assess(&usr, &usr, CaseComparison::ByteSensitive, &[]),
        PrefixAssessment::DeclarativeMatch
    );
    assert_eq!(
        bound.assess(&usr, &home, CaseComparison::ByteSensitive, &[]),
        PrefixAssessment::PrefixMismatch
    );
    assert_eq!(
        bound.assess(&usr, &usr, CaseComparison::Unknown, &[]),
        PrefixAssessment::UnresolvedComparison
    );
    let mut claim = PrefixCompatibility::DeclaredRelocatable {
        producer: r("producer"),
        evidence: None,
        conditions: vec![r("relative-loader-path")],
    };
    assert_eq!(
        claim.assess(&usr, &home, CaseComparison::ByteSensitive, &[]),
        PrefixAssessment::NeedsEvidence
    );
    if let PrefixCompatibility::DeclaredRelocatable { evidence, .. } = &mut claim {
        *evidence = Some(r("declared-evidence"));
    }
    assert_eq!(
        claim.assess(&usr, &home, CaseComparison::ByteSensitive, &[]),
        PrefixAssessment::NeedsConditions(vec![r("relative-loader-path")])
    );
    assert_eq!(
        claim.assess(
            &usr,
            &home,
            CaseComparison::ByteSensitive,
            &[r("relative-loader-path")]
        ),
        PrefixAssessment::DeclarativeMatch
    );
    assert_eq!(claim.evidence_verification(), ClaimVerification::Unverified);
    let unknown = PrefixCompatibility::Unsupported {
        reason: r("relocatability-unknown"),
    };
    assert!(matches!(
        unknown.assess(&usr, &home, CaseComparison::ByteSensitive, &[]),
        PrefixAssessment::Unsupported(_)
    ));
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-RESOURCE-IDENTITY",
    r = 1
)]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#RESERVED-ENVIRONMENT-RESOURCES",
    r = 1
)]
fn configuration_cache_and_reserved_resources_never_claim_physical_or_effect_authority() {
    let mut inputs = input();
    inputs.cache = CacheSelection::Requested(Some(spec(&[b"settings", b"cache"], "cache")));
    let separate = EnvironmentLayout::settings_anchored(inputs.clone()).unwrap();
    assert_eq!(
        separate.protected_configuration_paths_cache_relation(CaseComparison::ByteSensitive),
        ConfigurationCacheRelation::LexicallySeparate
    );
    assert_eq!(
        separate.protected_configuration_paths_cache_relation(CaseComparison::Unknown),
        ConfigurationCacheRelation::Unresolved
    );
    assert_eq!(
        bound(&separate, RootRole::DisposableCache).evidence_verification(),
        ClaimVerification::Unverified
    );
    inputs.cache = CacheSelection::Requested(Some(spec(&[b"settings"], "cache")));
    let overlap = EnvironmentLayout::settings_anchored(inputs.clone()).unwrap();
    assert_eq!(
        overlap.protected_configuration_paths_cache_relation(CaseComparison::ByteSensitive),
        ConfigurationCacheRelation::LexicalOverlap
    );
    inputs.cache = CacheSelection::Requested(Some(spec(&[b"override-cache"], "cache")));
    inputs.config_file_override = Some(path(&[b"override-cache", b"private-config.toml"]));
    let override_overlap = EnvironmentLayout::settings_anchored(inputs).unwrap();
    assert_eq!(
        override_overlap
            .protected_configuration_paths_cache_relation(CaseComparison::ByteSensitive),
        ConfigurationCacheRelation::LexicalOverlap
    );
    for kind in [
        ReservedResourceKind::Account,
        ReservedResourceKind::Service,
        ReservedResourceKind::Mount,
        ReservedResourceKind::Boot,
    ] {
        assert_eq!(
            ReservedEnvironmentResource::new(kind, r("named-resource")).operation_support(),
            ReservedOperationSupport::UnsupportedOperation
        );
    }
}
