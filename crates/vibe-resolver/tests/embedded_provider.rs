//! Behaviour oracle for the `EmbeddedProvider` cell (PROP-030 §3): drive the
//! composed provider over a real embedded local-directory registry, so the
//! cell has an integration-level characterization and not merely the unit
//! tests beside it. The `declared = None` shape is the one PROP-030 leans on
//! to lift the "no registry configured" bail — the embedded registry answering
//! a project that declares no `[[registry]]` of its own.

use std::path::Path;

use specmark::verifies;
use vibe_core::{Group, PackageRef};
use vibe_registry::LocalRegistry;
use vibe_resolver::{
    DepProvider, EmbeddedPrecedence, EmbeddedProvider, LocalCompositeProvider,
    LocalRegistryProvider, VersionEnumerator,
};

fn v(s: &str) -> semver::Version {
    semver::Version::parse(s).unwrap()
}

/// Lay a package down in the local-directory shape a `LocalRegistry` reads:
/// `<root>/org.vibevm/<name>/v<semver>/vibe.toml`.
fn seed(root: &Path, name: &str, versions: &[&str]) {
    for ver in versions {
        let dir = root.join("org.vibevm").join(name).join(format!("v{ver}"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("vibe.toml"),
            format!(
                "[package]\ngroup = \"org.vibevm\"\nname = \"{name}\"\nkind = \"flow\"\nversion = \"{ver}\"\n"
            ),
        )
        .unwrap();
    }
}

#[test]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-030#precedence")]
fn embedded_registry_answers_alone_when_no_declared_walk() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("embedded");
    std::fs::create_dir_all(&root).unwrap();
    seed(&root, "wal", &["0.1.0", "0.2.0"]);

    let registry = LocalRegistry::new(&root).unwrap();
    // The local family is now a composite (PROP-030 §3.3): a single-element
    // composite is equivalent to the pre-§3.3 single-provider shape.
    let provider = EmbeddedProvider::new(
        LocalCompositeProvider::new(vec![LocalRegistryProvider::new(&registry)]),
        None,
        EmbeddedPrecedence::EmbeddedFirst,
        false,
    );
    let group = Group::parse("org.vibevm").unwrap();

    // Every embedded version is enumerated.
    let mut versions = provider.list_versions(&group, "wal").unwrap();
    versions.sort();
    assert_eq!(versions, vec![v("0.1.0"), v("0.2.0")]);

    // resolve_version returns a version the embedded registry actually holds.
    let picked = provider
        .resolve_version(&PackageRef::parse("org.vibevm/wal").unwrap())
        .unwrap();
    assert!(versions.contains(&picked), "picked {picked}");

    // fetch_manifest reads the requested version out of the embedded tree.
    let manifest = provider.fetch_manifest(&group, "wal", &v("0.1.0")).unwrap();
    assert_eq!(manifest.require_package().unwrap().name, "wal");

    // A coordinate the embedded registry does not carry is reported absent,
    // not fabricated — so the combiner would fall through to a declared walk.
    assert!(provider.list_versions(&group, "nope").is_err());
}
