//! Artifact drops and `path` suffix handling (`PROP-066#drops`).

use std::path::{Path, PathBuf};

use super::drop::{RELEASE_DOWNLOAD_ROOT, package_drop_row, package_lookup, release_source_row};
use super::*;

fn locator(rows: Vec<RemapRow>) -> Locator {
    Locator::from_sources(
        vec![LoadedSource {
            layer: Layer::Local,
            source: "test".into(),
            rows,
            policy: UnmatchedPolicy::Allow,
        }],
        Vec::new(),
    )
}

fn decl(body: &str) -> RemapRow {
    toml::from_str::<RowDecl>(body)
        .unwrap()
        .into_row(Provenance::new(Layer::Local, "settings.toml", 0))
        .unwrap()
}

/// A `prefix` row with a `path` target mirrors the URL layout: the
/// remainder is joined, its query dropped, and no segment climbs out.
#[test]
fn a_prefix_path_row_joins_the_url_remainder() {
    let located = locator(vec![decl(
        "prefix = \"https://h.example/rel\"\npath = \"/srv/mirror\"",
    )]);
    let artifact = Artifact::new(ArtifactKind::Distribution)
        .with_url("https://h.example/rel/v1.0.0/../a.zip?vvm_nonce=1");
    assert_eq!(
        locator_target(&located, &artifact),
        RemapTarget::Path(PathBuf::from("/srv/mirror/v1.0.0/a.zip"))
    );
    // A non-prefix key keeps the target whole.
    let digest = "ab".repeat(32);
    let whole = locator(vec![decl(&format!(
        "sha256 = \"{digest}\"\npath = \"/srv/by-digest\""
    ))]);
    let input = Artifact::new(ArtifactKind::AppInput)
        .with_url("https://h.example/x/t.tgz")
        .with_digest(&digest);
    assert_eq!(
        locator_target(&whole, &input),
        RemapTarget::Path(PathBuf::from("/srv/by-digest"))
    );
}

fn locator_target(locator: &Locator, artifact: &Artifact) -> RemapTarget {
    locator.choose_candidate(artifact).unwrap().target
}

/// The package drop serves application inputs only, keeps its root whole,
/// loses to any user row that matches, and still yields to the origin
/// as the last candidate.
#[test]
fn the_package_drop_is_the_weakest_app_input_row() {
    let drop = package_drop_row(Path::new("/srv/pay"), Layer::Local, "config.toml");
    let user = decl("prefix = \"https://h.example/\"\nurl = \"https://mirror.example/\"");
    let located = locator(vec![drop, user]);
    let input = Artifact::new(ArtifactKind::AppInput).with_url("https://other.example/t.tgz");
    let Resolution::Candidates(candidates) = located.locate(&input) else {
        panic!("nothing denies")
    };
    assert_eq!(
        candidates[0].target,
        RemapTarget::Path(PathBuf::from("/srv/pay"))
    );
    assert_eq!(
        candidates.last().unwrap().target.display(),
        "https://other.example/t.tgz"
    );
    let mirrored = Artifact::new(ArtifactKind::AppInput).with_url("https://h.example/t.tgz");
    assert_eq!(
        locator_target(&located, &mirrored).display(),
        "https://mirror.example/t.tgz"
    );
    // Not a distribution, not a registry clone.
    let release = Artifact::new(ArtifactKind::Distribution).with_url("https://other.example/x");
    assert_eq!(
        locator_target(&located, &release).display(),
        "https://other.example/x"
    );
    assert_eq!(located.package_drops(), vec![Path::new("/srv/pay")]);
}

/// A `deny` row still refuses an input a drop could serve.
#[test]
fn a_deny_row_outranks_the_drop() {
    let digest = "cd".repeat(32);
    let located = locator(vec![
        package_drop_row(Path::new("/srv/pay"), Layer::Local, "config.toml"),
        decl(&format!("sha256 = \"{digest}\"\ndeny = true")),
    ]);
    let input = Artifact::new(ArtifactKind::AppInput)
        .with_url("https://h.example/t.tgz")
        .with_digest(&digest);
    assert!(matches!(located.locate(&input), Resolution::Denied { .. }));
}

/// The release row maps the download root and nothing else.
#[test]
fn the_release_row_maps_only_distributions_under_the_download_root() {
    let located = locator(vec![release_source_row(
        RemapTarget::Url("http://nexus.example/repository/vibe".into()),
        Layer::Local,
        "[self] source",
        "nexus",
    )]);
    let manifest = Artifact::new(ArtifactKind::Distribution).with_url(format!(
        "{RELEASE_DOWNLOAD_ROOT}/v1.0.13/DISTRIBUTIONS.json"
    ));
    assert_eq!(
        locator_target(&located, &manifest).display(),
        "http://nexus.example/repository/vibe/v1.0.13/DISTRIBUTIONS.json"
    );
    let input = Artifact::new(ArtifactKind::AppInput)
        .with_url(format!("{RELEASE_DOWNLOAD_ROOT}/v1.0.13/x.zip"));
    assert!(
        locator_target(&located, &input)
            .display()
            .starts_with("https://github.com/")
    );
}

/// A package spelling that could climb out of the drop contributes no rung.
#[test]
fn hostile_names_never_leave_the_drop() {
    assert!(package_lookup(Path::new("/d"), None, "ab", Some("../x")).is_empty());
    let rungs = package_lookup(Path::new("/d"), Some("../g/n@1"), "ab", Some("t"));
    assert_eq!(rungs, vec![PathBuf::from("/d/ab/t"), PathBuf::from("/d/t")]);
}

#[test]
fn byte_locations_follow_one_rule() {
    assert!(matches!(
        ByteLocation::parse("https://h.example/a"),
        Ok(ByteLocation::Remote(_))
    ));
    assert!(matches!(
        ByteLocation::parse("HTTP://h.example/a"),
        Ok(ByteLocation::Remote(_))
    ));
    assert!(matches!(
        ByteLocation::parse("relative/dir"),
        Err(LocationError::Relative { .. })
    ));
    assert!(matches!(
        ByteLocation::parse("git+ssh://h/a"),
        Err(LocationError::Unsupported { .. })
    ));
    assert!(ByteLocation::of_target(&RemapTarget::Deny).is_err());
}

/// A drop path a person typed with forward slashes is joined with the
/// platform's own separator, so a rung carries both styles; the shown form
/// settles on one (`PROP-066#drops`). The rewrite is parameterised, so the
/// Windows spelling is asserted on every platform.
#[test]
fn a_shown_drop_rung_carries_one_separator_style() {
    use super::drop::{shown_path, with_separator};

    let mixed = r"C:/users/o/ck/drop\org.vibevm.codkit\codkit\1.0.0\file.zip";
    assert_eq!(
        with_separator(mixed, '\\'),
        r"C:\users\o\ck\drop\org.vibevm.codkit\codkit\1.0.0\file.zip"
    );
    assert_eq!(
        with_separator(mixed, '/'),
        "C:/users/o/ck/drop/org.vibevm.codkit/codkit/1.0.0/file.zip"
    );

    // What the miss message and `vibe artifacts where` actually print: the
    // first rung of the convention, in this platform's one style.
    let rungs = package_lookup(
        Path::new("C:/users/o/ck/drop"),
        Some("org.vibevm.codkit/codkit@1.0.0"),
        &"ab".repeat(32),
        Some("file.zip"),
    );
    let shown = shown_path(&rungs[0]);
    let foreign = if cfg!(windows) { '/' } else { '\\' };
    assert!(
        !shown.contains(foreign),
        "one separator style throughout: {shown}"
    );
}

/// The by-name lookup is the package convention minus the one rung the
/// digest keys, and nothing else moves: the reader that has no digest
/// document (`vibe app versions`' `in drop` mark) searches the same
/// places in the same order, and simply never looks under
/// `<drop>/<sha256>/<file>` (`PROP-066#drops`).
#[test]
fn the_by_name_lookup_is_the_convention_without_the_digest_rung() {
    use super::drop::package_lookup_by_name;

    let drop = Path::new("/d");
    let digest = "ab".repeat(32);
    let full = package_lookup(drop, Some("org.example/tool@1.2.0"), &digest, Some("t.tgz"));
    let by_name = package_lookup_by_name(drop, Some("org.example/tool@1.2.0"), Some("t.tgz"));
    assert_eq!(
        by_name,
        vec![
            PathBuf::from("/d/org.example/tool/1.2.0/t.tgz"),
            PathBuf::from("/d/org.example/tool/t.tgz"),
            PathBuf::from("/d/t.tgz"),
        ]
    );
    let kept: Vec<PathBuf> = full
        .into_iter()
        .filter(|rung| rung != &drop.join(&digest).join("t.tgz"))
        .collect();
    assert_eq!(kept, by_name, "the one dropped rung is the digest's");

    // The rungs a part is unknown for are skipped the same way: no
    // package coordinate leaves the flat drop alone, no version leaves the
    // version-less package directory, and no file name is no answer.
    assert_eq!(
        package_lookup_by_name(drop, None, Some("t.tgz")),
        vec![PathBuf::from("/d/t.tgz")]
    );
    assert_eq!(
        package_lookup_by_name(drop, Some("org.example/tool"), Some("t.tgz")),
        vec![
            PathBuf::from("/d/org.example/tool/t.tgz"),
            PathBuf::from("/d/t.tgz"),
        ]
    );
    assert!(package_lookup_by_name(drop, Some("org.example/tool@1.2.0"), None).is_empty());
    // A name that could climb out of the drop is no answer either.
    assert!(package_lookup_by_name(drop, None, Some("../t.tgz")).is_empty());
}
