//! Unit tests for [`super`], out-of-line per the file-length budget.
//! Included via `#[cfg(test)] #[path] mod tests;`, so the module-tree
//! position — and therefore `use super::*` — is unchanged from the
//! inline form. Non-`#[test]` helpers carry `#[cfg(test)]` so
//! file-grain scanners (the conform frontend) scope their `unwrap`s
//! as test code.

use specmark::verifies;

use super::*;

/// The canonical group every fixture package in these tests belongs to.
#[cfg(test)]
fn org() -> Group {
    Group::parse("org.vibevm").unwrap()
}

#[test]
fn link_type_parses_static_transitive_wire_form() {
    #[derive(serde::Deserialize)]
    struct L {
        v: LinkType,
    }
    // PROP-035 §12 — the kebab wire form.
    let lt: LinkType = toml::from_str("v = \"static-transitive\"")
        .map(|w: L| w.v)
        .unwrap();
    assert_eq!(lt, LinkType::StaticTransitive);
    // The base forms.
    let base: LinkType = toml::from_str("v = \"static\"").map(|w: L| w.v).unwrap();
    assert_eq!(base, LinkType::Static);
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
        materialization: Materialization::default(),
        bridge: false,
        format: PackageFormat::default(),
    };
    let r = meta.as_package_ref().unwrap();
    assert_eq!(r.kind, Some(PackageKind::Flow));
    assert_eq!(r.group, Some(org()));
    assert_eq!(r.name, "wal");
    assert!(r.version.matches(&semver::Version::parse("0.3.0").unwrap()));
    assert!(!r.version.matches(&semver::Version::parse("0.3.1").unwrap()));
}

#[test]
#[verifies("spec://vibevm/modules/vibe-workspace/PROP-035#formats")]
fn package_format_parses_and_defaults_to_simple() {
    // PROP-035 §3: `format` is optional and defaults to `simple` — the
    // fail-safe posture (a forgotten format over-loads, visibly working,
    // rather than silently loading nothing). A manifest opts into `normal`
    // explicitly.
    #[derive(Deserialize)]
    struct F {
        v: PackageFormat,
    }
    let normal: PackageFormat = toml::from_str("v = \"normal\"").map(|w: F| w.v).unwrap();
    assert_eq!(normal, PackageFormat::Normal);
    assert!(normal.is_normal());

    // Absent on a `[package]`, it defaults to `simple`.
    let pkg: PackageMeta = toml::from_str(
        "name = \"greeter\"\ngroup = \"com.example.hello\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    assert_eq!(pkg.format, PackageFormat::Simple);
    assert!(pkg.format.is_default());

    // Explicit `format = "normal"` on a `[package]` is carried through, and
    // `deny_unknown_fields` still stands (a bogus field is rejected).
    let pkg_n: PackageMeta = toml::from_str(
        "name = \"greeter\"\ngroup = \"com.example.hello\"\nkind = \"flow\"\nversion = \"0.1.0\"\nformat = \"normal\"\n",
    )
    .unwrap();
    assert!(pkg_n.format.is_normal());
}

// --- PROP-009 §2.4 / §2.5 — inclusion type + boot category ----------

#[test]
fn link_type_default_is_static() {
    assert_eq!(LinkType::default(), LinkType::Dynamic);
}

#[test]
fn boot_snippet_parses_category_and_link() {
    let bs: BootSnippet = toml::from_str(
        r#"source = "boot/10-flow-wal.md"
category = "flow"
link = "static"
"#,
    )
    .unwrap();
    assert_eq!(bs.category, Some(BootCategory::Flow));
    assert_eq!(bs.link, Some(LinkType::Static));
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

// --- bridge-packages design: PROP-020/022/023 + PROP-015 ------------

#[derive(Serialize, Deserialize)]
struct MatWrap {
    v: Materialization,
}

#[test]
fn materialization_default_is_snapshot() {
    assert_eq!(Materialization::default(), Materialization::Snapshot);
    assert!(Materialization::default().is_default());
    assert!(!Materialization::Snapshot.is_in_place());
    assert!(Materialization::InPlace.is_in_place());
}

#[test]
fn materialization_roundtrips_kebab_case() {
    for (text, mode) in [
        ("snapshot", Materialization::Snapshot),
        ("hardlink", Materialization::Hardlink),
        ("in-place", Materialization::InPlace),
    ] {
        let parsed: Materialization = toml::from_str(&format!("v = \"{text}\""))
            .map(|w: MatWrap| w.v)
            .unwrap();
        assert_eq!(parsed, mode);
        // and back out in the same kebab form
        let rendered = toml::to_string_pretty(&MatWrap { v: mode }).unwrap();
        assert!(rendered.contains(text), "{rendered}");
    }
}

#[test]
fn package_meta_parses_materialization_and_bridge() {
    let p: PackageMeta = toml::from_str(
        r#"name = "chromium"
group = "org.example"
kind = "tool"
version = "1.0.0"
materialization = "in-place"
bridge = true
"#,
    )
    .unwrap();
    assert_eq!(p.materialization, Materialization::InPlace);
    assert!(p.bridge);
}

#[test]
fn package_meta_defaults_skip_serialize() {
    // A package that sets neither new field serialises without them.
    let p: PackageMeta = toml::from_str(
        r#"name = "wal"
group = "org.vibevm"
kind = "feat"
version = "0.1.0"
"#,
    )
    .unwrap();
    assert!(p.materialization.is_default());
    assert!(!p.bridge);
    let rendered = toml::to_string_pretty(&p).unwrap();
    assert!(!rendered.contains("materialization"), "{rendered}");
    assert!(!rendered.contains("bridge"), "{rendered}");
}

#[test]
fn hooks_parse_both_phases() {
    let h: HooksDecl = toml::from_str(
        r#"pre-install = "hooks/prepare"
post-install = "hooks/finalise"
"#,
    )
    .unwrap();
    assert_eq!(
        h.pre_install.as_deref().and_then(|p| p.to_str()),
        Some("hooks/prepare")
    );
    assert_eq!(
        h.post_install.as_deref().and_then(|p| p.to_str()),
        Some("hooks/finalise")
    );
    assert!(!h.is_empty());
    assert!(HooksDecl::default().is_empty());
}

#[test]
fn skill_decl_parses_include() {
    let s: SkillDecl = toml::from_str(
        r#"name = "vim"
path = "upstream/skills/vim"
include = ["SKILL.md", "references/**/*.md"]
"#,
    )
    .unwrap();
    assert_eq!(
        s.include,
        vec!["SKILL.md".to_string(), "references/**/*.md".to_string()]
    );
}
