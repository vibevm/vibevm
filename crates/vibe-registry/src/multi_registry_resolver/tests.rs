//! Tests for the resolver's own surface — the mirror-chain filter and
//! sort exposed via [`MultiRegistryResolver::mirrors_for`].

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#registry-model");

use super::*;
use tempfile::tempdir;

use crate::multi_registry_resolver::test_support::*;

#[test]
fn mirrors_for_filters_and_sorts() {
    let cache = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());

    let mirrors = vec![
        MirrorSection {
            of: "vibespecs".to_string(),
            url: "https://a".to_string(),
            priority: 2,
        },
        MirrorSection {
            of: "vibespecs".to_string(),
            url: "https://b".to_string(),
            priority: 1,
        },
        MirrorSection {
            of: "*".to_string(),
            url: "https://catchall".to_string(),
            priority: 99,
        },
        MirrorSection {
            of: "other".to_string(),
            url: "https://unrelated".to_string(),
            priority: 0,
        },
    ];
    let r = build_resolver(
        cache.path(),
        vec![registry_section("vibespecs", "git@host:org")],
        mirrors,
        vec![],
        fake,
    );

    let m = r.mirrors_for("vibespecs");
    assert_eq!(m.len(), 3);
    assert_eq!(m[0].url, "https://b");
    assert_eq!(m[1].url, "https://a");
    assert_eq!(m[2].url, "https://catchall");
}

/// A `[[registry]]` whose `url` is a local, **non-git** directory is served
/// by `LocalRegistry` (filesystem read), not `git clone` — so a plain
/// directory laid out `<group>/<name>/v<version>/` resolves and fetches with
/// no git backend. This is the fix for the "file:// registry → not found"
/// regression where every `[[registry]]` was git-cloned.
#[test]
fn local_directory_registry_resolves_and_fetches_without_git() {
    use std::fs;

    let cache = tempdir().unwrap();
    // The registry root: a plain directory (NO `.git`) laid out as a registry.
    let root = tempdir().unwrap();
    let group = "org.vibevm";
    let name = "wal";
    let pkg_dir = root.path().join(group).join(name).join("v0.1.0");
    fs::create_dir_all(&pkg_dir).unwrap();
    fs::write(
        pkg_dir.join("vibe.toml"),
        manifest_text(name, "flow", "0.1.0"),
    )
    .unwrap();
    fs::write(pkg_dir.join("README.md"), "wal").unwrap();
    assert!(
        !root.path().join(".git").exists(),
        "sanity: the registry is not a git repo"
    );

    // A [[registry]] whose url is the explicit `file://` form for that
    // directory. (A bare path stays on the git backend — the historical
    // behaviour; only `file:` opens a LocalRegistry.)
    let root_str = root.path().display().to_string().replace('\\', "/");
    let url = if root_str.starts_with('/') {
        format!("file://{root_str}")
    } else {
        format!("file:///{root_str}")
    };
    let r = build_resolver(
        cache.path(),
        vec![registry_section("local", &url)],
        vec![],
        vec![],
        Arc::new(FakeBackend::default()),
    );

    // The local source is NOT in the git subset; it is a `Local` source.
    assert_eq!(
        r.registries().len(),
        0,
        "git subset empty — a local dir is not git"
    );
    assert_eq!(r.sources().len(), 1);
    assert!(matches!(r.sources()[0], RegistrySource::Local(_)));

    let g = Group::parse(group).unwrap();
    let versions = r.list_versions(&g, name).unwrap();
    assert_eq!(versions, vec![semver::Version::parse("0.1.0").unwrap()]);

    let pkgref = PackageRef::parse(&format!("{group}/{name}@0.1.0")).unwrap();
    let resolution = r.resolve(&pkgref).unwrap();
    assert_eq!(resolution.registry_name.as_deref(), Some("local"));
    assert_eq!(resolution.source_url, url);
    assert!(
        resolution.source_ref.is_none(),
        "no git ref for a local source"
    );
    assert_eq!(
        resolution.resolved.version,
        semver::Version::parse("0.1.0").unwrap()
    );

    let proj_cache = tempdir().unwrap();
    let cached = r.fetch(&resolution, proj_cache.path()).unwrap();
    assert!(cached.cache_dir.join("vibe.toml").is_file());
    assert!(cached.cache_dir.join("README.md").is_file());
    assert!(cached.content_hash.starts_with("sha256:"));
}

/// A `git+file://` url is a local *git* repo to clone (the `git+` transport),
/// NOT a plain directory — it stays on the git backend, never routed to
/// `LocalRegistry`. Regression guard: the local-directory fix initially sent
/// `git+file://` to `LocalRegistry` and broke git-clone local registries
/// (caught by `cli_pkg_cycle::install_from_git_registry` under the floor).
#[test]
fn git_transport_url_stays_on_the_git_backend_not_local() {
    let cache = tempdir().unwrap();
    let r = build_resolver(
        cache.path(),
        vec![registry_section("local-git", "git+file:///C:/some/repo")],
        vec![],
        vec![],
        Arc::new(FakeBackend::default()),
    );
    assert!(
        r.sources()
            .iter()
            .all(|s| !matches!(s, RegistrySource::Local(_))),
        "git+file:// is a git transport — must stay on the git backend, not LocalRegistry"
    );
}

/// A bare-path url (no scheme) stays on the git backend too — it is the
/// historical "local git repo to clone" form the multi-registry tests and
/// local-git workflows rely on. Only an explicit `file:` scheme opens a
/// LocalRegistry. Regression guard: an earlier draft of the local-directory
/// fix routed bare paths to LocalRegistry and broke
/// `differential_oracle::provider_pair_agrees_on_solvable_inputs`.
#[test]
fn bare_path_url_stays_on_the_git_backend_not_local() {
    let cache = tempdir().unwrap();
    let bare = cache.path().join("org"); // a bare filesystem path, no scheme
    let r = build_resolver(
        cache.path(),
        vec![registry_section("local-git", &bare.display().to_string())],
        vec![],
        vec![],
        Arc::new(FakeBackend::default()),
    );
    assert!(
        r.sources()
            .iter()
            .all(|s| !matches!(s, RegistrySource::Local(_))),
        "a bare path (no scheme) is a git registry historically — only file:// opens LocalRegistry"
    );
}
