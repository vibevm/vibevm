use std::cell::RefCell;
use std::collections::BTreeSet;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use vibe_core::progress::ProgressTask;
use vibe_publish::release_manifest::{
    AggregateDistributionManifest, BundleDistributionManifest, DISTRIBUTION_MANIFEST_FILENAME,
    DISTRIBUTION_SOURCE_ARCHIVE_FILENAME, DistributionAsset, DistributionComponent,
    DistributionComponentName, DistributionSourceArchive, PlatformDistributionFragment,
    SUPPORTED_DISTRIBUTION_TARGETS,
};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use super::archive::install_bundle;
use super::{
    Downloader, activate_install, bootstrap_with_downloader, current_target,
    install_release_version, move_to_newest_release,
};
use crate::cli::VvmBootstrapArgs;
use crate::commands::vvm::VvmEnv;
use crate::commands::vvm::env::{EnvPersister, Persisted};
use crate::commands::vvm::model::{InstallRecord, Kind, Origin, VersionId};
use crate::commands::vvm::store::{BINARY_NAME, INDEX_BINARY_NAME, VersionStore};

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
const TREE: &str = "89abcdef0123456789abcdef0123456789abcdef";

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn zip_bytes(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default();
    for (name, bytes) in entries {
        writer.start_file(*name, options).unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn source_zip(extra: Option<(&str, &[u8])>) -> Vec<u8> {
    let mut entries = vec![
        ("Cargo.toml", b"[workspace]\n".as_slice()),
        (
            "crates/vibe-cli/Cargo.toml",
            b"[package]\nname='vibe-cli'\n".as_slice(),
        ),
    ];
    if let Some(extra) = extra {
        entries.push(extra);
    }
    zip_bytes(&entries)
}

fn symlink_source_zip() -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default();
    writer.start_file("Cargo.toml", options).unwrap();
    writer.write_all(b"[workspace]\n").unwrap();
    writer
        .start_file("crates/vibe-cli/Cargo.toml", options)
        .unwrap();
    writer.write_all(b"[package]\nname='vibe-cli'\n").unwrap();
    writer
        .add_symlink("source-link", "Cargo.toml", options)
        .unwrap();
    writer.finish().unwrap().into_inner()
}

struct Fixture {
    path: PathBuf,
    manifest: BundleDistributionManifest,
    asset: DistributionAsset,
}

fn write_bundle(
    root: &Path,
    actual_vibe: &[u8],
    declared_vibe: &[u8],
    source: &[u8],
    extra_outer: bool,
) -> Fixture {
    write_versioned_bundle(
        root,
        semver::Version::new(1, 0, 0),
        actual_vibe,
        declared_vibe,
        source,
        extra_outer,
    )
}

/// A published bundle under an arbitrary release number. Two bundles of the
/// SAME version with different payloads are the rebuilt-release case, so the
/// local file is named by its own digest while the release asset name — the
/// one the download URL is built from — stays the canonical per-version one.
fn write_versioned_bundle(
    root: &Path,
    version: semver::Version,
    actual_vibe: &[u8],
    declared_vibe: &[u8],
    source: &[u8],
    extra_outer: bool,
) -> Fixture {
    let vibe_index = b"index-binary";
    let manifest = BundleDistributionManifest::new(
        version.clone(),
        COMMIT,
        current_target().unwrap(),
        vec![
            DistributionComponent {
                name: DistributionComponentName::Vibe,
                path: BINARY_NAME.to_string(),
                size: declared_vibe.len() as u64,
                digest: digest(declared_vibe),
            },
            DistributionComponent {
                name: DistributionComponentName::VibeIndex,
                path: INDEX_BINARY_NAME.to_string(),
                size: vibe_index.len() as u64,
                digest: digest(vibe_index),
            },
        ],
        DistributionSourceArchive {
            path: DISTRIBUTION_SOURCE_ARCHIVE_FILENAME.to_string(),
            size: source.len() as u64,
            digest: digest(source),
            tree_oid: TREE.to_string(),
        },
    )
    .unwrap();
    let manifest_bytes = manifest.to_json_bytes().unwrap();
    let mut entries = vec![
        (BINARY_NAME, actual_vibe),
        (INDEX_BINARY_NAME, vibe_index.as_slice()),
        (DISTRIBUTION_SOURCE_ARCHIVE_FILENAME, source),
        (DISTRIBUTION_MANIFEST_FILENAME, manifest_bytes.as_slice()),
        ("LICENSE.md", b"license".as_slice()),
        ("README.md", b"readme".as_slice()),
    ];
    if extra_outer {
        entries.push(("surprise", b"no"));
    }
    let bundle = zip_bytes(&entries);
    let outer = digest(&bundle);
    let path = root.join(format!("bundle-{version}-{}.zip", &outer[7..15]));
    std::fs::write(&path, &bundle).unwrap();
    let asset = DistributionAsset {
        name: format!("vibevm-{version}-{}.zip", current_target().unwrap()),
        size: bundle.len() as u64,
        digest: outer,
    };
    Fixture {
        path,
        manifest,
        asset,
    }
}

fn platform_bundle(
    target: &str,
    version: &semver::Version,
    source_archive: &DistributionSourceArchive,
) -> BundleDistributionManifest {
    let windows = target == "x86_64-pc-windows-msvc";
    BundleDistributionManifest::new(
        version.clone(),
        COMMIT,
        target,
        vec![
            DistributionComponent {
                name: DistributionComponentName::Vibe,
                path: if windows { "vibe.exe" } else { "vibe" }.to_string(),
                size: 1,
                digest: digest(b"v"),
            },
            DistributionComponent {
                name: DistributionComponentName::VibeIndex,
                path: if windows {
                    "vibe-index.exe"
                } else {
                    "vibe-index"
                }
                .to_string(),
                size: 1,
                digest: digest(b"i"),
            },
        ],
        source_archive.clone(),
    )
    .unwrap()
}

fn aggregate_for(fixture: &Fixture) -> AggregateDistributionManifest {
    let version = semver::Version::parse(&fixture.manifest.version).unwrap();
    let platforms = SUPPORTED_DISTRIBUTION_TARGETS
        .iter()
        .map(|target| {
            let (bundle, asset) = if *target == current_target().unwrap() {
                (fixture.manifest.clone(), fixture.asset.clone())
            } else {
                (
                    platform_bundle(target, &version, &fixture.manifest.source_archive),
                    DistributionAsset {
                        name: format!("vibevm-{version}-{target}.zip"),
                        size: 1,
                        digest: digest(b"x"),
                    },
                )
            };
            let vibe = bundle
                .components
                .iter()
                .find(|component| component.name == DistributionComponentName::Vibe)
                .unwrap();
            let bootstrap = DistributionAsset {
                name: format!(
                    "vibe-bootstrap-{target}{}",
                    if target.contains("windows") {
                        ".exe"
                    } else {
                        ""
                    }
                ),
                size: vibe.size,
                digest: vibe.digest.clone(),
            };
            PlatformDistributionFragment::new(asset, bootstrap, bundle).unwrap()
        })
        .collect();
    AggregateDistributionManifest::from_platforms(platforms).unwrap()
}

struct LocalDownloader {
    bundle: PathBuf,
    urls: RefCell<Vec<String>>,
}

impl Downloader for LocalDownloader {
    fn download(
        &self,
        url: &str,
        destination: &Path,
        maximum_bytes: u64,
        expected_bytes: Option<u64>,
        progress: &ProgressTask,
    ) -> anyhow::Result<()> {
        self.urls.borrow_mut().push(url.to_string());
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let copied = std::fs::copy(&self.bundle, destination)?;
        assert!(copied <= maximum_bytes);
        progress.set_progress(copied, expected_bytes, "bytes");
        Ok(())
    }
}

struct ReleaseDownloader {
    aggregate: PathBuf,
    bundle: PathBuf,
    urls: RefCell<Vec<String>>,
}

impl Downloader for ReleaseDownloader {
    fn download(
        &self,
        url: &str,
        destination: &Path,
        maximum_bytes: u64,
        expected_bytes: Option<u64>,
        progress: &ProgressTask,
    ) -> anyhow::Result<()> {
        self.urls.borrow_mut().push(url.to_string());
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let source = if url
            .split('?')
            .next()
            .is_some_and(|url| url.ends_with("/DISTRIBUTIONS.json"))
        {
            &self.aggregate
        } else {
            &self.bundle
        };
        let copied = std::fs::copy(source, destination)?;
        assert!(copied <= maximum_bytes);
        progress.set_progress(copied, expected_bytes, "bytes");
        Ok(())
    }
}

fn quiet() -> crate::output::Context {
    crate::output::Context::from_flags(true, false, None, true, crate::cli::AgentModeArg::Auto)
}

fn active_selector(store: &VersionStore) -> String {
    store.active().unwrap().unwrap().selector().to_string()
}

fn write_test_executable(path: &Path, bytes: &[u8]) {
    std::fs::write(path, bytes).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).unwrap();
    }
}

struct FakePersister {
    fail_path: bool,
    fail_home: bool,
    homes: RefCell<Vec<PathBuf>>,
    paths: RefCell<Vec<PathBuf>>,
}

impl FakePersister {
    fn new(fail_path: bool) -> Self {
        Self {
            fail_path,
            fail_home: false,
            homes: RefCell::new(Vec::new()),
            paths: RefCell::new(Vec::new()),
        }
    }

    fn home_failure() -> Self {
        Self {
            fail_path: false,
            fail_home: true,
            homes: RefCell::new(Vec::new()),
            paths: RefCell::new(Vec::new()),
        }
    }
}

impl EnvPersister for FakePersister {
    fn set_vibevm_home(&self, home: &Path) -> anyhow::Result<Persisted> {
        self.homes.borrow_mut().push(home.to_path_buf());
        if self.fail_home {
            anyhow::bail!("injected advisory HOME persistence failure");
        }
        Ok(Persisted::Changed)
    }

    fn ensure_on_path(&self, path: &Path) -> anyhow::Result<Persisted> {
        self.paths.borrow_mut().push(path.to_path_buf());
        if self.fail_path {
            anyhow::bail!("injected PATH persistence failure");
        }
        Ok(Persisted::Changed)
    }

    fn activation_hint(&self) -> String {
        "test activation".into()
    }
}

#[path = "network_tests.rs"]
mod network_tests;

#[path = "release_tests.rs"]
mod release_tests;

#[test]
fn verified_bundle_installs_both_tools_source_shims_and_force_refresh() {
    let temp = tempfile::tempdir().unwrap();
    let source = source_zip(None);
    let fixture = write_bundle(temp.path(), b"vibe-binary", b"vibe-binary", &source, false);
    let store = VersionStore::new(temp.path().join("opt"));

    let first = install_bundle(
        &store,
        &fixture.path,
        &fixture.manifest,
        &fixture.asset,
        false,
    )
    .unwrap();
    assert!(!first.reused);
    assert_eq!(first.record.selector().to_string(), "tag:1.0.0#1");
    assert_eq!(
        std::fs::read(store.binary_path(&first.record.version_id(), 1)).unwrap(),
        b"vibe-binary"
    );
    assert_eq!(
        std::fs::read(store.index_binary_path(&first.record.version_id(), 1)).unwrap(),
        b"index-binary"
    );
    assert!(
        store
            .instance_source_dir(&first.record.version_id(), 1)
            .join("Cargo.toml")
            .is_file()
    );
    assert_eq!(first.record.commit, COMMIT);
    assert_eq!(
        PathBuf::from(first.record.source_path.as_deref().unwrap()),
        store
            .instance_source_dir(&first.record.version_id(), 1)
            .canonicalize()
            .unwrap()
    );
    let stored_manifest = BundleDistributionManifest::from_json_slice(
        &std::fs::read(first.home.join(DISTRIBUTION_MANIFEST_FILENAME)).unwrap(),
    )
    .unwrap();
    assert_eq!(stored_manifest.source_archive.tree_oid, TREE);
    assert!(super::archive::installed_bundle_intact(
        &store,
        &first.record
    ));
    let mut swapped_identity = first.record.clone();
    swapped_identity.commit = "f".repeat(40);
    assert!(!super::archive::installed_bundle_intact(
        &store,
        &swapped_identity
    ));
    let failing = FakePersister::new(true);
    let error = activate_install(&store, &first, &failing, false)
        .unwrap_err()
        .to_string();
    assert!(error.contains("injected PATH persistence failure"));
    assert!(store.active().unwrap().is_none(), "pointer flips last");
    assert!(
        failing.homes.borrow().is_empty(),
        "version-specific HOME is not written before generic PATH succeeds"
    );

    let persister = FakePersister::new(false);
    let activation = activate_install(&store, &first, &persister, false).unwrap();
    assert!(activation.durable_path_changed);
    assert!(!activation.path_on_current_process);
    assert!(store.shim_dir().join("vibe").is_file());
    assert!(store.shim_dir().join("vibe-index").is_file());
    assert_eq!(store.active().unwrap().unwrap().instance, 1);

    let reused = install_bundle(
        &store,
        &fixture.path,
        &fixture.manifest,
        &fixture.asset,
        false,
    )
    .unwrap();
    assert!(reused.reused);
    assert_eq!(reused.record.instance, 1);
    activate_install(&store, &reused, &persister, true).unwrap();

    let forced = install_bundle(
        &store,
        &fixture.path,
        &fixture.manifest,
        &fixture.asset,
        true,
    )
    .unwrap();
    assert!(!forced.reused);
    let advisory_failure = FakePersister::home_failure();
    let activation = activate_install(&store, &forced, &advisory_failure, true).unwrap();
    assert!(activation.advisory_home_warning.is_some());
    assert_eq!(store.active().unwrap().unwrap().instance, 2);
    assert_eq!(forced.record.selector().to_string(), "tag:1.0.0#2");
    assert_eq!(store.previous().unwrap().unwrap().instance, 1);
    assert!(
        store
            .instance_dir(&VersionId::new(Kind::Tag, "1.0.0"), 1)
            .is_dir()
    );
    store.write_current(&first.home).unwrap();
    assert_eq!(store.active().unwrap().unwrap().instance, 1);
    assert_eq!(store.previous().unwrap().unwrap().instance, 2);
}

#[test]
fn tampered_immutable_instance_is_never_reused_or_repaired() {
    for mutation in 0..5 {
        let temp = tempfile::tempdir().unwrap();
        let source = source_zip(None);
        let fixture = write_bundle(temp.path(), b"vibe-binary", b"vibe-binary", &source, false);
        let store = VersionStore::new(temp.path().join("opt"));
        let first = install_bundle(
            &store,
            &fixture.path,
            &fixture.manifest,
            &fixture.asset,
            false,
        )
        .unwrap();
        let home = &first.home;
        let tampered = match mutation {
            0 => store.binary_path(&first.record.version_id(), 1),
            1 => store.index_binary_path(&first.record.version_id(), 1),
            2 => home.join(DISTRIBUTION_MANIFEST_FILENAME),
            3 => home.join("source/Cargo.toml"),
            4 => home.join(DISTRIBUTION_SOURCE_ARCHIVE_FILENAME),
            _ => unreachable!(),
        };
        std::fs::write(&tampered, b"tampered").unwrap();

        let replacement = install_bundle(
            &store,
            &fixture.path,
            &fixture.manifest,
            &fixture.asset,
            false,
        )
        .unwrap();
        assert!(!replacement.reused, "mutation {mutation} was reused");
        assert_eq!(replacement.record.instance, 2);
        assert_eq!(std::fs::read(&tampered).unwrap(), b"tampered");
    }
}

#[cfg(unix)]
#[test]
fn executable_mode_and_matching_byte_symlink_tamper_never_reuse() {
    use std::os::unix::fs::{PermissionsExt, symlink};

    let temp = tempfile::tempdir().unwrap();
    let source = source_zip(None);
    let fixture = write_bundle(temp.path(), b"vibe-binary", b"vibe-binary", &source, false);
    let store = VersionStore::new(temp.path().join("opt"));
    let first = install_bundle(
        &store,
        &fixture.path,
        &fixture.manifest,
        &fixture.asset,
        false,
    )
    .unwrap();
    let vibe = store.binary_path(&first.record.version_id(), first.record.instance);
    let mut permissions = std::fs::metadata(&vibe).unwrap().permissions();
    permissions.set_mode(0o644);
    std::fs::set_permissions(&vibe, permissions).unwrap();
    let second = install_bundle(
        &store,
        &fixture.path,
        &fixture.manifest,
        &fixture.asset,
        false,
    )
    .unwrap();
    assert_eq!(second.record.instance, 2);

    let cargo = second.home.join("source/Cargo.toml");
    let copy = second.home.join("same-bytes");
    std::fs::copy(&cargo, &copy).unwrap();
    std::fs::remove_file(&cargo).unwrap();
    symlink(&copy, &cargo).unwrap();
    let third = install_bundle(
        &store,
        &fixture.path,
        &fixture.manifest,
        &fixture.asset,
        false,
    )
    .unwrap();
    assert_eq!(third.record.instance, 3);
}

#[test]
fn component_hash_mismatch_never_publishes_or_activates() {
    let temp = tempfile::tempdir().unwrap();
    let source = source_zip(None);
    let fixture = write_bundle(temp.path(), b"tampered", b"expected", &source, false);
    let store = VersionStore::new(temp.path().join("opt"));

    let error = install_bundle(
        &store,
        &fixture.path,
        &fixture.manifest,
        &fixture.asset,
        false,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("integrity mismatch"), "{error}");
    assert!(store.active().unwrap().is_none());
    assert!(store.load_state().unwrap().installs.is_empty());
}

#[test]
fn traversal_and_extra_outer_entries_are_rejected_before_activation() {
    let temp = tempfile::tempdir().unwrap();
    let malicious_source = source_zip(Some(("../escaped", b"escape")));
    let traversal = write_bundle(temp.path(), b"vibe", b"vibe", &malicious_source, false);
    let store = VersionStore::new(temp.path().join("opt"));
    let error = install_bundle(
        &store,
        &traversal.path,
        &traversal.manifest,
        &traversal.asset,
        false,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("unsafe source ZIP entry"), "{error}");
    assert!(!temp.path().join("escaped").exists());
    assert!(store.active().unwrap().is_none());

    let safe_source = source_zip(None);
    let extra = write_bundle(temp.path(), b"vibe", b"vibe", &safe_source, true);
    let error = install_bundle(&store, &extra.path, &extra.manifest, &extra.asset, false)
        .unwrap_err()
        .to_string();
    assert!(error.contains("structure mismatch"), "{error}");
    assert!(store.active().unwrap().is_none());

    let symlink_source = symlink_source_zip();
    let symlink = write_bundle(temp.path(), b"vibe", b"vibe", &symlink_source, false);
    let error = install_bundle(
        &store,
        &symlink.path,
        &symlink.manifest,
        &symlink.asset,
        false,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("symlink"), "{error}");
    assert!(store.active().unwrap().is_none());

    for unsafe_name in ["café.rs", "CONIN$", "CONOUT$.txt"] {
        let unsafe_source = source_zip(Some((unsafe_name, b"unsafe")));
        let fixture = write_bundle(temp.path(), b"vibe", b"vibe", &unsafe_source, false);
        let error = install_bundle(
            &store,
            &fixture.path,
            &fixture.manifest,
            &fixture.asset,
            false,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("unsafe source ZIP entry"), "{error}");
        assert!(store.active().unwrap().is_none());
    }
}
