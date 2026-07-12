use specmark::verifies;

use super::*;

#[test]
fn kind_roundtrip() {
    for kind in PackageKind::ALL {
        let s = kind.to_string();
        let parsed: PackageKind = s.parse().unwrap();
        assert_eq!(kind, parsed);
    }
}

#[test]
fn kind_rejects_unknown() {
    let err = "widget".parse::<PackageKind>().unwrap_err();
    assert!(matches!(err, Error::BadPackageKind(_)));
}

#[test]
fn name_accepts_valid_kebab() {
    for name in ["wal", "welcome-page", "auth-email", "x", "a-b-c", "h2o"] {
        validate_package_name(name).unwrap_or_else(|_| panic!("should accept `{name}`"));
    }
}

#[test]
fn name_rejects_invalid() {
    for name in [
        "",
        "-leading",
        "trailing-",
        "Double--hyphen",
        "Upper",
        "with_underscore",
        "with space",
        "unicode-😊",
    ] {
        assert!(
            validate_package_name(name).is_err(),
            "should reject `{name}`"
        );
    }
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-008#group", r = 1)]
fn group_accepts_valid() {
    for g in [
        "org.vibevm",
        "com.acme",
        "a",
        "x.y.z",
        "dev.example-team",
        "org.vibevm_internal",
        "h2o.mol",
    ] {
        Group::parse(g).unwrap_or_else(|_| panic!("should accept `{g}`"));
    }
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-008#group", r = 1)]
fn group_rejects_invalid() {
    for g in [
        "",
        "   ",
        ".org",
        "org.",
        "org..vibevm",
        "Org.Vibevm",
        "org vibevm",
        "org/vibevm",
        "org.vibevm!",
        "org:vibevm",
    ] {
        assert!(Group::parse(g).is_err(), "should reject `{g}`");
    }
}

#[test]
fn group_display_roundtrips() {
    let g = Group::parse("org.vibevm").unwrap();
    assert_eq!(g.to_string(), "org.vibevm");
    assert_eq!(g.as_str(), "org.vibevm");
    let back: Group = g.to_string().parse().unwrap();
    assert_eq!(g, back);
}

#[test]
fn group_trims_whitespace() {
    let g = Group::parse("  org.vibevm  ").unwrap();
    assert_eq!(g.as_str(), "org.vibevm");
}

#[test]
fn group_serde_via_string() {
    let g = Group::parse("com.acme").unwrap();
    let json = serde_json::to_string(&g).unwrap();
    assert_eq!(json, r#""com.acme""#);
    let back: Group = serde_json::from_str(&json).unwrap();
    assert_eq!(g, back);
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-008#pkgref", r = 1)]
fn parse_short_bare() {
    let r = PackageRef::parse("wal").unwrap();
    assert_eq!(r.kind, None);
    assert_eq!(r.group, None);
    assert_eq!(r.name, "wal");
    assert_eq!(r.version, VersionSpec::Latest);
    assert_eq!(r.to_string(), "wal");
    assert!(!r.is_qualified());
}

#[test]
fn parse_short_with_kind() {
    let r = PackageRef::parse("flow:wal").unwrap();
    assert_eq!(r.kind, Some(PackageKind::Flow));
    assert_eq!(r.group, None);
    assert_eq!(r.name, "wal");
    assert_eq!(r.to_string(), "flow:wal");
}

#[test]
fn parse_qualified() {
    let r = PackageRef::parse("org.vibevm.world/wal").unwrap();
    assert_eq!(r.kind, None);
    assert_eq!(r.group.as_ref().unwrap().as_str(), "org.vibevm");
    assert_eq!(r.name, "wal");
    assert!(r.is_qualified());
    assert_eq!(r.to_string(), "org.vibevm.world/wal");
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-008#pkgref", r = 1)]
fn parse_qualified_with_kind() {
    let r = PackageRef::parse("flow:org.vibevm.world/wal").unwrap();
    assert_eq!(r.kind, Some(PackageKind::Flow));
    assert_eq!(r.group.as_ref().unwrap().as_str(), "org.vibevm");
    assert_eq!(r.name, "wal");
    assert_eq!(r.to_string(), "flow:org.vibevm.world/wal");
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-008#pkgref", r = 1)]
fn parse_bare_semver_is_caret_per_cargo() {
    // Cargo / npm / Poetry semantics: a bare semver like `0.3.0` is
    // shorthand for `^0.3.0` (caret — compatible release). To pin
    // strictly equal, write `=0.3.0`. Holds across every pkgref form.
    for s in [
        "wal@0.3.0",
        "flow:wal@0.3.0",
        "org.vibevm.world/wal@0.3.0",
        "flow:org.vibevm.world/wal@0.3.0",
    ] {
        let r = PackageRef::parse(s).unwrap();
        assert_eq!(r.name, "wal");
        assert!(r.version.matches(&semver::Version::parse("0.3.0").unwrap()));
        assert!(
            r.version.matches(&semver::Version::parse("0.3.5").unwrap()),
            "{s}: 0.3.0 caret must accept 0.3.5"
        );
        assert!(
            !r.version.matches(&semver::Version::parse("0.4.0").unwrap()),
            "{s}: 0.3.0 caret must reject 0.4.0"
        );
    }
}

#[test]
fn parse_eq_version_is_exact() {
    let r = PackageRef::parse("org.vibevm.world/wal@=0.3.0").unwrap();
    assert!(r.version.matches(&semver::Version::parse("0.3.0").unwrap()));
    assert!(
        !r.version.matches(&semver::Version::parse("0.3.1").unwrap()),
        "=0.3.0 must reject 0.3.1"
    );
}

#[test]
fn parse_range_and_tilde_versions() {
    let caret = PackageRef::parse("org.vibevm.world/wal@^0.3").unwrap();
    assert!(
        caret
            .version
            .matches(&semver::Version::parse("0.3.5").unwrap())
    );
    assert!(
        !caret
            .version
            .matches(&semver::Version::parse("0.4.0").unwrap())
    );
    let tilde = PackageRef::parse("org.vibevm.world/wal@~0.3.1").unwrap();
    assert!(
        tilde
            .version
            .matches(&semver::Version::parse("0.3.5").unwrap())
    );
    assert!(
        !tilde
            .version
            .matches(&semver::Version::parse("0.4.0").unwrap())
    );
}

#[test]
fn parse_all_kinds_in_prefix() {
    for kind in PackageKind::ALL {
        let r = PackageRef::parse(&format!("{kind}:org.vibevm/thing")).unwrap();
        assert_eq!(r.kind, Some(kind));
    }
}

#[test]
fn parse_rejects_bad_kind() {
    assert!(matches!(
        PackageRef::parse("widget:wal").unwrap_err(),
        Error::BadPackageKind(_)
    ));
    assert!(matches!(
        PackageRef::parse("widget:org.vibevm.world/wal").unwrap_err(),
        Error::BadPackageKind(_)
    ));
}

#[test]
fn parse_rejects_bad_group() {
    // Uppercase in the group segment — `Group::parse` rejects it.
    assert!(matches!(
        PackageRef::parse("Org.Vibevm/wal").unwrap_err(),
        Error::BadGroup { .. }
    ));
}

#[test]
fn parse_rejects_bad_name() {
    // Empty name after the group separator.
    assert!(PackageRef::parse("org.vibevm/").is_err());
    // Empty name after the kind prefix.
    assert!(matches!(
        PackageRef::parse("flow:").unwrap_err(),
        Error::BadPackageName(_)
    ));
    // A dot in the name — no `:`/`/`, so the whole token is the name,
    // and kebab-case forbids the dot.
    assert!(matches!(
        PackageRef::parse("flow.wal").unwrap_err(),
        Error::BadPackageName(_)
    ));
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-008#pkgref", r = 1)]
fn display_round_trips_every_form() {
    for s in [
        "wal",
        "flow:wal",
        "org.vibevm.world/wal",
        "flow:org.vibevm.world/wal",
        "org.vibevm.world/wal@^0.3",
        "flow:org.vibevm.world/wal@=0.3.0",
    ] {
        let r = PackageRef::parse(s).unwrap();
        let r2 = PackageRef::parse(&r.to_string()).unwrap();
        assert_eq!(r, r2, "round-trip failed for `{s}`");
    }
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-008#identity", r = 1)]
fn qualified_name_is_the_identity_string() {
    // kind and version drop; `<group>/<name>` is the identity.
    let q = PackageRef::parse("flow:org.vibevm.world/wal@0.1.0").unwrap();
    assert_eq!(q.qualified_name(), "org.vibevm.world/wal");
    // No group yet — the bare name is the best identity available.
    let short = PackageRef::parse("wal@0.1.0").unwrap();
    assert_eq!(short.qualified_name(), "wal");
}

#[test]
fn empty_input_rejected() {
    assert!(PackageRef::parse("").is_err());
    assert!(PackageRef::parse("   ").is_err());
}

#[test]
fn serde_round_trips_via_string() {
    let r = PackageRef::parse("flow:org.vibevm.world/wal@^0.3").unwrap();
    let json = serde_json::to_string(&r).unwrap();
    assert_eq!(json, r#""flow:org.vibevm.world/wal@^0.3""#);
    let back: PackageRef = serde_json::from_str(&json).unwrap();
    assert_eq!(r, back);
}
