//! Unit tests for [`super`], out-of-line per the file-length budget.
//! Included via `#[cfg(test)] #[path] mod tests;`, so the module-tree
//! position — and therefore `use super::*` — is unchanged from the
//! inline form. Non-`#[test]` helpers carry `#[cfg(test)]` so
//! file-grain scanners (the conform frontend) scope their `unwrap`s
//! as test code.

use super::*;

/// The canonical group every fixture package in these tests belongs to.
#[cfg(test)]
fn org() -> Group {
    Group::parse("org.vibevm").unwrap()
}

#[test]
fn publish_posture_default_is_all_true() {
    assert!(PublishPosture::default().is_default());
    assert!(!PublishPosture::default().is_never());
    assert!(PublishPosture::default().includes("anything"));
}

#[test]
fn publish_posture_roundtrips_all_forms() {
    // `publish = false`
    let never: PublishPosture = toml::from_str("v = false").map(|w: Wrap| w.v).unwrap();
    assert!(never.is_never());
    assert!(!never.includes("vibespecs"));
    // `publish = true`
    let all: PublishPosture = toml::from_str("v = true").map(|w: Wrap| w.v).unwrap();
    assert!(all.is_default());
    // `publish = ["a", "b"]`
    let some: PublishPosture = toml::from_str("v = [\"a\", \"b\"]")
        .map(|w: Wrap| w.v)
        .unwrap();
    assert!(some.includes("a"));
    assert!(some.includes("b"));
    assert!(!some.includes("c"));
    assert!(!some.is_never());
}

#[derive(Deserialize)]
struct Wrap {
    v: PublishPosture,
}

#[test]
fn compatibility_is_empty() {
    assert!(Compatibility::default().is_empty());
    let c = Compatibility {
        min_vibe_version: Some("0.1.0".into()),
        requires_kinds: vec![],
    };
    assert!(!c.is_empty());
}

#[test]
fn package_meta_as_package_ref_pins_exact() {
    let meta = PackageMeta {
        name: "wal".into(),
        group: Group::parse("org.vibevm").unwrap(),
        kind: PackageKind::Flow,
        version: semver::Version::parse("0.3.0").unwrap(),
        authors: vec![],
        license: None,
        description: None,
        homepage: None,
        keywords: vec![],
        describes: None,
        publish: PublishPosture::default(),
    };
    let r = meta.as_package_ref().unwrap();
    assert_eq!(r.kind, Some(PackageKind::Flow));
    assert_eq!(r.group, Some(org()));
    assert_eq!(r.name, "wal");
    assert!(r.version.matches(&semver::Version::parse("0.3.0").unwrap()));
    assert!(!r.version.matches(&semver::Version::parse("0.3.1").unwrap()));
}

// --- PROP-009 §2.4 / §2.5 — inclusion type + boot category ----------

#[test]
fn link_type_default_is_static() {
    assert_eq!(LinkType::default(), LinkType::Static);
}

#[test]
fn boot_snippet_parses_category_and_link() {
    let bs: BootSnippet = toml::from_str(
        r#"source = "boot/10-flow-wal.md"
category = "flow"
link = "inline"
"#,
    )
    .unwrap();
    assert_eq!(bs.category, Some(BootCategory::Flow));
    assert_eq!(bs.link, Some(LinkType::Inline));
}

#[test]
fn boot_snippet_minimal_form_parses() {
    // `source` is the only required field; `category`, `link`, and
    // `when` are optional and absent here.
    let bs: BootSnippet = toml::from_str("source = \"boot/10-flow-wal.md\"\n").unwrap();
    assert!(bs.category.is_none());
    assert!(bs.link.is_none());
    assert!(bs.when.is_none());
}

#[test]
fn boot_category_user_override_is_kebab_case() {
    let bs: BootSnippet = toml::from_str(
        r#"source = "boot/90-user.md"
category = "user-override"
"#,
    )
    .unwrap();
    assert_eq!(bs.category, Some(BootCategory::UserOverride));
}

#[test]
fn boot_snippet_round_trips_with_category_and_link() {
    let bs: BootSnippet = toml::from_str(
        r#"source = "boot/20-stack-rust.md"
category = "stack"
link = "dynamic"
"#,
    )
    .unwrap();
    let rendered = toml::to_string_pretty(&bs).unwrap();
    let back: BootSnippet = toml::from_str(&rendered).unwrap();
    assert_eq!(bs, back);
}

// --- PROP-009 §2.4 / §2.6 — the `when` OS gate ----------------------

#[test]
fn boot_snippet_parses_when() {
    let bs: BootSnippet = toml::from_str(
        r#"source = "boot/win.md"
when = "os:windows"
"#,
    )
    .unwrap();
    assert_eq!(bs.when, Some(WhenCondition::Os(TargetOs::Windows)));
}

#[test]
fn boot_snippet_rejects_a_malformed_when() {
    let err = toml::from_str::<BootSnippet>(
        r#"source = "boot/win.md"
when = "os:plan9"
"#,
    )
    .unwrap_err();
    assert!(err.to_string().contains("plan9"), "{err}");
}

#[test]
fn boot_snippet_round_trips_with_when() {
    let bs: BootSnippet = toml::from_str(
        r#"source = "boot/mac.md"
category = "stack"
when = "os:macos"
"#,
    )
    .unwrap();
    let rendered = toml::to_string_pretty(&bs).unwrap();
    assert!(rendered.contains("when = \"os:macos\""), "{rendered}");
    let back: BootSnippet = toml::from_str(&rendered).unwrap();
    assert_eq!(bs, back);
}
