//! The **directory root** each registry source serves packages from —
//! the place a template is read off the manifests on disk (PROP-075
//! `#TEMPLATE-DISCOVERY`, `#VERSIONS-CHANNEL-DISCOVERY`).
//!
//! The three variants answer three different ways, and a caller that
//! matched them itself got the monorepo wrong: a home whose only
//! `[[registry]]` is a mirrored monorepo — the corporate shape — then had
//! no directory root at all, so no template was ever discovered and every
//! `vibe app versions` answered the version union with an empty channel
//! table. The decision therefore lives beside the variants, and this is
//! its test.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");

use std::fs;

use tempfile::tempdir;

use super::test_support::*;
use super::*;

/// A monorepo `[[registry]]` over `url`, which the fake backend serves.
fn monorepo_section(name: &str, url: &str) -> RegistrySection {
    let mut section = registry_section(name, url);
    section.r#type = Some(vibe_core::manifest::RegistryType::Monorepo);
    section
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-DISCOVERY")]
fn a_monorepo_source_offers_its_working_copy_as_a_directory_root() {
    let cache = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    let url = "git@host:owner/mono.git";
    // The repository the backend clones: one package laid out the way
    // every directory-shaped registry lays one out.
    let upstream = tempdir().unwrap();
    let package = upstream.path().join("org.vibevm/demo/v1.0.0");
    fs::create_dir_all(&package).unwrap();
    fs::write(
        package.join("vibe.toml"),
        manifest_text("demo", "flow", "1.0.0"),
    )
    .unwrap();
    fake.seed_bootstrap(url, upstream.path().to_owned());

    let resolver = build_resolver(
        cache.path(),
        vec![monorepo_section("corp", url)],
        vec![],
        vec![],
        fake,
    );
    let sources = resolver.sources().expect("the declared sources");
    assert_eq!(sources.len(), 1);
    let root = sources[0]
        .directory_root(false)
        .expect("a monorepo serves a directory root");
    // The root is the working copy, and it is a directory registry's
    // layout — which is the whole claim the template layer relies on.
    assert!(
        root.join("org.vibevm/demo/v1.0.0/vibe.toml").is_file(),
        "{} is not laid out <group>/<name>/v<version>/",
        root.display()
    );

    // Offline reads only a *local* repository, exactly as a resolution
    // under `--offline` does: this one is remote, so it offers no root
    // rather than reaching for the network to produce one.
    assert_eq!(sources[0].directory_root(true), None);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-DISCOVERY")]
fn a_localdir_source_offers_its_own_directory_and_a_git_registry_offers_none() {
    let cache = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    let directory = tempdir().unwrap();
    let local = crate::vendor::file_url_for_dir(directory.path());
    let resolver = build_resolver(
        cache.path(),
        vec![
            registry_section("drop", &local),
            registry_section("hub", "git@host:owner"),
        ],
        vec![],
        vec![],
        fake,
    );
    let sources = resolver.sources().expect("the declared sources");
    assert_eq!(sources.len(), 2);
    // A `localdir` registry *is* the directory, offline or not.
    for offline in [false, true] {
        assert_eq!(
            sources[0].directory_root(offline).as_deref(),
            Some(directory.path()),
        );
    }
    // A per-package git registry has no such root: its packages are
    // repositories, and there is no tree on disk to read manifests off.
    assert_eq!(sources[1].directory_root(false), None);
    assert_eq!(sources[1].directory_root(true), None);
}
