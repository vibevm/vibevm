//! Public native capability construction controls.
#![cfg(test)]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-PREPARATION");
use super::*;
#[test]
fn direct_public_https_constructor_retains_useful_anonymous_preparation() {
    let f = Fixture::new();
    let temp = tempfile::tempdir().unwrap();
    let registry = crate::GitPerPackageRegistry::open_single_package(
        "public",
        &format!("{}/repo.git", f.get("a")),
        "main",
        temp.path(),
        Arc::new(f.backend()),
        0,
        vibe_core::manifest::AuthKind::None,
        None,
    )
    .unwrap();
    let selected = crate::ResolvedPackage {
        group: vibe_core::Group::parse("org.fixture").unwrap(),
        name: "demo".into(),
        version: semver::Version::new(1, 0, 0),
        source_dir: PathBuf::new(),
    };
    assert_eq!(
        registry
            .prepare_in_place(&selected, &temp.path().join("slot"))
            .unwrap()
            .resolved_commit(),
        f.get("current_head")
    );
    assert_eq!(f.contacts("b"), 0);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES-LAW")]
fn child_rewrite_uses_first_equal_length_match_and_longest_prefix() {
    for (first, second) in [("first", "second"), ("second", "first")] {
        let entries = vec![
            (
                format!("url.https://{first}.invalid/.insteadof"),
                "https://raw.invalid/".into(),
            ),
            (
                format!("url.https://{second}.invalid/.insteadof"),
                "https://raw.invalid/".into(),
            ),
        ];
        assert_eq!(
            super::super::gitlinks::rewritten_url("https://raw.invalid/repo.git", &entries),
            format!("https://{first}.invalid/repo.git")
        );
        let mut longer = entries;
        longer.push((
            "url.https://longest.invalid/.insteadof".into(),
            "https://raw.invalid/team/".into(),
        ));
        assert_eq!(
            super::super::gitlinks::rewritten_url("https://raw.invalid/team/repo.git", &longer),
            "https://longest.invalid/repo.git"
        );
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES-LAW")]
fn relative_scp_child_preserves_remote_home_and_rejects_authority_escape() {
    for (parent, child, expected) in [
        (
            "git@example.invalid:parent.git",
            "../child.git",
            "git@example.invalid:child.git",
        ),
        (
            "git@example.invalid:owner/parent.git",
            "../../other/child.git",
            "git@example.invalid:other/child.git",
        ),
    ] {
        assert_eq!(
            super::super::gitlinks::resolve_url(parent, child).unwrap(),
            expected
        );
    }
    assert!(
        super::super::gitlinks::resolve_url(
            "git@example.invalid:owner/parent.git",
            "../../../escape.git"
        )
        .is_err()
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES-LAW")]
fn denied_mirror_has_no_contact_before_independently_admitted_next_route() {
    use crate::{GitPerPackageRegistry, ResolvedPackage};
    use vibe_core::manifest::NamingConvention;
    let f = Fixture::new();
    let temp = tempfile::tempdir().unwrap();
    assert_eq!(
        f.backend()
            .list_tags(&format!("{}/public/org.fixture.demo.git", f.get("a")))
            .unwrap(),
        ["v1.0.0", "v2.0.0"]
    );
    std::fs::write(Path::new(f.get("root")).join("a.log"), "").unwrap();
    let registry = GitPerPackageRegistry::open_with_explicit_token(
        "fixture",
        &format!("{}/missing", f.get("a")),
        "main",
        NamingConvention::Fqdn,
        vec![
            format!("{}/public", f.get("b")),
            format!("{}/public", f.get("a")),
        ],
        &temp.path().join("cache"),
        Arc::new(f.backend()),
        3600,
        vibe_core::manifest::AuthKind::None,
        None,
    )
    .unwrap();
    let selected = ResolvedPackage {
        group: vibe_core::Group::parse("org.fixture").unwrap(),
        name: "demo".into(),
        version: semver::Version::new(1, 0, 0),
        source_dir: PathBuf::new(),
    };
    let slot = temp.path().join("slot");
    let prepared = registry.prepare_in_place(&selected, &slot).unwrap();
    assert_eq!(prepared.resolved_commit(), f.get("head"));
    assert!(!slot.exists());
    assert_eq!(f.contacts("b"), 0);
    assert_eq!(f.connections("b"), 0);
    let requests = std::fs::read_to_string(Path::new(f.get("root")).join("a.log")).unwrap();
    assert!(requests.contains("/missing/"));
    assert!(requests.contains("/public/"));
    let excluded_only = GitPerPackageRegistry::open_with_explicit_token(
        "excluded-only",
        &format!("{}/missing", f.get("a")),
        "main",
        NamingConvention::Fqdn,
        vec![format!("{}/public", f.get("b"))],
        &temp.path().join("excluded-cache"),
        Arc::new(f.backend()),
        0,
        vibe_core::manifest::AuthKind::None,
        None,
    )
    .unwrap();
    let denied_slot = temp.path().join("denied-slot");
    assert!(matches!(
        excluded_only.prepare_in_place(&selected, &denied_slot),
        Err(crate::RegistryError::ContactRefused(
            crate::ContactRefusal::ExcludedHost { .. }
        ))
    ));
    assert_eq!(f.connections("b"), 0);
    assert!(!denied_slot.exists());
}
