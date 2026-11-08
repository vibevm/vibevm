//! The release lane through a release source (`PROP-019#release-sources`):
//! a local release drop, a bare platform zip, the verification that a drop
//! never weakens, the offline posture over a drop, and the source policy
//! keeping `self update` away from an excluded host. Bytes move through the
//! real downloader; no test reaches the network.

#![cfg(test)]

use super::super::RemoteContext;
use super::super::download::HttpDownloader;
use super::super::release_source::{EffectiveReleaseSource, ReleaseSource, SourceOrigin};
use super::release_tests::{bundle_for, seed_machine};
use super::*;
use vibe_core::manifest::{HostingSelector, Manifest, SourcePolicyDecl};
use vibe_core::progress::Progress;
use vibe_core::remap::{Layer, LoadedSource, Locator};
use vibe_core::{GlobalRegistryConfig, GlobalRegistrySource};
use vibe_package_source::{PackageSourceOptions, SourceStorage};

#[cfg(test)]
fn operation(root: &Path, exclude_github: bool) -> vibe_package_source::SourceOperation {
    let storage = root.join("acquisition");
    std::fs::create_dir_all(&storage).unwrap();
    vibe_package_source::source_operation(
        &PackageSourceOptions {
            storage: Some(SourceStorage {
                acquisition_root: storage,
                read_only_store: None,
            }),
            source_policy: SourcePolicyDecl {
                exclude_hosting: if exclude_github {
                    vec![HostingSelector::Github]
                } else {
                    Vec::new()
                },
                ..SourcePolicyDecl::default()
            },
            ..PackageSourceOptions::default()
        },
        &Manifest::default(),
        root,
        GlobalRegistrySource::ready(GlobalRegistryConfig::default()),
    )
    .unwrap()
}

#[cfg(test)]
fn localdir(drop: &Path) -> (EffectiveReleaseSource, Locator) {
    let effective = EffectiveReleaseSource {
        source: ReleaseSource::Localdir {
            path: drop.to_path_buf(),
        },
        origin: SourceOrigin::SelfTable(None),
    };
    let locator = Locator::from_sources(
        vec![LoadedSource {
            layer: Layer::Local,
            source: "[self] source".into(),
            rows: effective.row().into_iter().collect(),
            policy: Default::default(),
        }],
        Vec::new(),
    );
    (effective, locator)
}

/// Lay one release out the way an administrator copies it from the
/// release page: `<drop>/v<version>/DISTRIBUTIONS.json` and the zip.
#[cfg(test)]
fn publish(drop: &Path, fixture: &Fixture) {
    let dir = drop.join(format!("v{}", fixture.manifest.version));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("DISTRIBUTIONS.json"),
        aggregate_for(fixture).to_json_bytes().unwrap(),
    )
    .unwrap();
    std::fs::copy(&fixture.path, dir.join(&fixture.asset.name)).unwrap();
}

#[cfg(test)]
fn run<T>(
    store: &VersionStore,
    env: &VvmEnv,
    source: EffectiveReleaseSource,
    locator: &Locator,
    downloader: &dyn Downloader,
    call: impl FnOnce(&RemoteContext<'_>) -> anyhow::Result<T>,
) -> anyhow::Result<T> {
    let ctx = quiet();
    let progress = Progress::default();
    let persister = FakePersister::new(false);
    call(&RemoteContext {
        source,
        locator,
        ctx: &ctx,
        env,
        store,
        downloader,
        persister: &persister,
        command: "self:update",
        progress: &progress,
    })
}

#[test]
fn update_moves_to_the_newest_release_in_a_local_drop() {
    let temp = tempfile::tempdir().unwrap();
    let one = bundle_for(temp.path(), semver::Version::new(1, 0, 0), b"vibe-1.0.0");
    let (store, env, installed) = seed_machine(temp.path(), &one);
    let drop = temp.path().join("drop");
    for version in [(1, 5, 0), (2, 0, 0)] {
        let version = semver::Version::new(version.0, version.1, version.2);
        let payload = format!("vibe-{version}");
        publish(&drop, &bundle_for(temp.path(), version, payload.as_bytes()));
    }
    let (source, locator) = localdir(&drop);
    let downloader = HttpDownloader::new(&operation(temp.path(), false)).unwrap();
    run(&store, &env, source, &locator, &downloader, |remote| {
        move_to_newest_release(remote, &installed, false)
    })
    .unwrap();
    assert!(active_selector(&store).starts_with("tag:2.0.0#"));
}

#[test]
fn a_drop_read_offline_needs_no_network() {
    let temp = tempfile::tempdir().unwrap();
    let one = bundle_for(temp.path(), semver::Version::new(1, 0, 0), b"vibe-1.0.0");
    let (store, mut env, installed) = seed_machine(temp.path(), &one);
    env.offline = true;
    let drop = temp.path().join("drop");
    publish(
        &drop,
        &bundle_for(temp.path(), semver::Version::new(1, 1, 0), b"vibe-1.1.0"),
    );
    let (source, locator) = localdir(&drop);
    let downloader = HttpDownloader::new(&operation(temp.path(), false)).unwrap();
    run(&store, &env, source, &locator, &downloader, |remote| {
        move_to_newest_release(remote, &installed, false)
    })
    .unwrap();
    assert!(active_selector(&store).starts_with("tag:1.1.0#"));
}

#[test]
fn a_bare_platform_zip_installs_by_its_embedded_manifest() {
    let temp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(temp.path().join("opt"));
    let env = VvmEnv {
        root: Some(temp.path().join("opt")),
        ..VvmEnv::default()
    };
    let drop = temp.path().join("drop");
    std::fs::create_dir_all(&drop).unwrap();
    let three = bundle_for(temp.path(), semver::Version::new(3, 0, 0), b"vibe-3.0.0");
    std::fs::copy(&three.path, drop.join(&three.asset.name)).unwrap();
    let (source, locator) = localdir(&drop);
    let downloader = HttpDownloader::new(&operation(temp.path(), false)).unwrap();
    run(
        &store,
        &env,
        source.clone(),
        &locator,
        &downloader,
        |remote| install_release_version(remote, "3.0.0", false),
    )
    .unwrap();
    assert!(active_selector(&store).starts_with("tag:3.0.0#"));

    // A zip whose payload contradicts its own manifest is still refused.
    let four = write_versioned_bundle(
        temp.path(),
        semver::Version::new(4, 0, 0),
        b"tampered-vibe",
        b"vibe-4.0.0",
        &source_zip(None),
        false,
    );
    std::fs::copy(&four.path, drop.join(&four.asset.name)).unwrap();
    let error = run(&store, &env, source, &locator, &downloader, |remote| {
        install_release_version(remote, "4.0.0", false)
    })
    .unwrap_err();
    assert!(
        format!("{error:#}").contains("digest") || format!("{error:#}").contains("size"),
        "{error:#}"
    );
    assert!(active_selector(&store).starts_with("tag:3.0.0#"));
}

#[test]
fn a_drop_never_weakens_the_published_digests() {
    let temp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(temp.path().join("opt"));
    let env = VvmEnv {
        root: Some(temp.path().join("opt")),
        ..VvmEnv::default()
    };
    let drop = temp.path().join("drop");
    let five = bundle_for(temp.path(), semver::Version::new(5, 0, 0), b"vibe-5.0.0");
    publish(&drop, &five);
    // Someone replaced the zip with a different build of the same name:
    // the published size or digest refuses it.
    let other = bundle_for(temp.path(), semver::Version::new(5, 0, 0), b"vibe-5.0.X");
    std::fs::copy(&other.path, drop.join("v5.0.0").join(&five.asset.name)).unwrap();
    let (source, locator) = localdir(&drop);
    let downloader = HttpDownloader::new(&operation(temp.path(), false)).unwrap();
    let error = run(&store, &env, source, &locator, &downloader, |remote| {
        install_release_version(remote, "5.0.0", false)
    })
    .unwrap_err();
    let text = format!("{error:#}");
    assert!(
        text.contains("digest") || text.contains("byte limit"),
        "{text}"
    );
    assert!(store.active().unwrap().is_none(), "nothing was installed");
}

#[test]
fn an_excluded_github_is_never_contacted_by_self_update() {
    let temp = tempfile::tempdir().unwrap();
    let one = bundle_for(temp.path(), semver::Version::new(1, 0, 0), b"vibe-1.0.0");
    let (store, env, installed) = seed_machine(temp.path(), &one);
    let downloader = HttpDownloader::new(&operation(temp.path(), true)).unwrap();
    let locator = Locator::default();
    let error = run(
        &store,
        &env,
        EffectiveReleaseSource::default(),
        &locator,
        &downloader,
        |remote| move_to_newest_release(remote, &installed, false),
    )
    .unwrap_err();
    let text = format!("{error:#}");
    assert!(text.contains("github.com"), "{text}");
    assert!(
        text.contains("exclu") || text.contains("refused"),
        "the policy refused before any connection: {text}"
    );
}
