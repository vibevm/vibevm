//! px in the shared tool store (PROP-019 `#companion-tools`): the pinned
//! archive is verified against the pin and unpacked into
//! `<root>/vibevm/tools/px/<version>/`; a release without it installs as
//! before; bytes under the pinned name that are not the pinned archive stop
//! the command before activation; the error of a missing px names the store
//! and the asset.

#![cfg(test)]

use super::super::ReleaseDir;
use super::super::RemoteContext;
use super::super::companions::{self, PxOutcome};
use super::*;
use vibe_core::progress::{Progress, ProgressTask};
use vibe_net::px_store::{PinnedAsset, PxPin};

/// A release server: the aggregate, the bundle, and optionally one more
/// named asset. Anything else answers "not found".
struct Server {
    aggregate: PathBuf,
    bundle: PathBuf,
    extra: Option<(String, PathBuf)>,
    urls: RefCell<Vec<String>>,
}

impl Downloader for Server {
    fn download(
        &self,
        url: &str,
        destination: &Path,
        maximum_bytes: u64,
        expected_bytes: Option<u64>,
        progress: &ProgressTask,
    ) -> anyhow::Result<()> {
        self.urls.borrow_mut().push(url.to_string());
        let path = url.split('?').next().unwrap_or(url);
        let source = if path.ends_with("/DISTRIBUTIONS.json") {
            Some(&self.aggregate)
        } else if let Some((name, file)) = &self.extra
            && path.ends_with(&format!("/{name}"))
        {
            Some(file)
        } else if path.ends_with(".zip") && path.contains("/vibevm-") {
            Some(&self.bundle)
        } else {
            None
        };
        let Some(source) = source else {
            anyhow::bail!("`{path}` answered 404");
        };
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let copied = std::fs::copy(source, destination)?;
        assert!(copied <= maximum_bytes);
        progress.set_progress(copied, expected_bytes, "bytes");
        Ok(())
    }
}

fn server(root: &Path, fixture: &Fixture, extra: Option<(&str, &[u8])>) -> Server {
    let aggregate = root.join("DISTRIBUTIONS.json");
    std::fs::write(&aggregate, aggregate_for(fixture).to_json_bytes().unwrap()).unwrap();
    let extra = extra.map(|(name, bytes)| {
        let file = root.join(name);
        std::fs::write(&file, bytes).unwrap();
        (name.to_owned(), file)
    });
    Server {
        aggregate,
        bundle: fixture.path.clone(),
        extra,
        urls: RefCell::new(Vec::new()),
    }
}

fn install(store: &VersionStore, env: &VvmEnv, downloader: &dyn Downloader) -> anyhow::Result<()> {
    let ctx = quiet();
    let progress = Progress::default();
    let persister = FakePersister::new(false);
    install_release_version(
        &RemoteContext {
            source: Default::default(),
            locator: vibe_core::remap::active(),
            ctx: &ctx,
            env,
            store,
            downloader,
            persister: &persister,
            command: "self:install",
            progress: &progress,
        },
        "1.0.12",
        false,
    )
    .map(|_| ())
}

fn machine(temp: &Path) -> (VersionStore, VvmEnv) {
    let store = VersionStore::new(temp.join("opt"));
    let env = VvmEnv {
        root: Some(temp.join("opt")),
        ..VvmEnv::default()
    };
    (store, env)
}

/// A pin for test bytes: the same shape as the embedded one.
fn pin_for(bytes: &[u8], archive: &str, executable: &str) -> (PxPin, PinnedAsset) {
    let asset = PinnedAsset {
        target: current_target().unwrap().into(),
        url: "https://example.invalid/px".into(),
        sha256: format!("{:x}", Sha256::digest(bytes)),
        archive: archive.into(),
        executable: executable.into(),
    };
    let pin = PxPin {
        assets: vec![asset.clone()],
        version: "0.0.1-test".into(),
        ..PxPin::embedded().clone()
    };
    (pin, asset)
}

fn tar_gz(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
        Vec::new(),
        flate2::Compression::default(),
    ));
    for (name, bytes) in entries {
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder.append_data(&mut header, name, *bytes).unwrap();
    }
    builder.into_inner().unwrap().finish().unwrap()
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools")]
fn a_pinned_archive_of_either_kind_is_verified_and_unpacked_into_the_versioned_store() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("opt/vibevm/tools/px");
    let zip = zip_bytes(&[
        ("px.exe", b"px for windows".as_slice()),
        ("Lib/x.pyd", b"lib".as_slice()),
        ("LICENSE.txt", b"MIT".as_slice()),
    ]);
    let (pin, asset) = pin_for(&zip, "zip", "px.exe");
    let program = companions::install_bytes(&root, &pin, &asset, &zip).unwrap();
    assert_eq!(program, root.join("0.0.1-test/px.exe"));
    assert_eq!(std::fs::read(&program).unwrap(), b"px for windows");
    assert!(root.join("0.0.1-test/Lib/x.pyd").is_file());
    assert!(root.join("0.0.1-test/NOTICE").is_file());
    assert_eq!(companions::installed(&root, &pin, &asset), Some(program));

    let gz = tar_gz(&[
        ("./px", b"px for linux".as_slice()),
        ("./LICENSE.txt", b"MIT".as_slice()),
    ]);
    let (pin, asset) = pin_for(&gz, "tar.gz", "px");
    let program = companions::install_bytes(&root, &pin, &asset, &gz).unwrap();
    assert_eq!(std::fs::read(&program).unwrap(), b"px for linux");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_ne!(
            std::fs::metadata(&program).unwrap().permissions().mode() & 0o111,
            0
        );
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools")]
fn bytes_that_are_not_the_pinned_archive_or_escape_are_refused_and_leave_nothing() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("store");
    let good = zip_bytes(&[("px.exe", b"px".as_slice())]);
    let (pin, asset) = pin_for(&good, "zip", "px.exe");
    let error = companions::install_bytes(&root, &pin, &asset, b"another archive").unwrap_err();
    assert!(error.to_string().contains("integrity mismatch"), "{error}");
    assert!(
        error.to_string().contains("PROP-019#companion-tools"),
        "{error}"
    );
    for bytes in [
        zip_bytes(&[("../evil", b"x".as_slice())]),
        zip_bytes(&[("README", b"x".as_slice())]),
    ] {
        let (pin, asset) = pin_for(&bytes, "zip", "px.exe");
        assert!(companions::install_bytes(&root, &pin, &asset, &bytes).is_err());
        assert!(!root.join(&pin.version).exists());
    }
    assert!(!temp.path().join("evil").exists());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools")]
fn a_release_without_px_installs_as_before_and_wrong_px_bytes_stop_it() {
    // No px asset: the bundle installs and activates, the store stays empty.
    let temp = tempfile::tempdir().unwrap();
    let fixture = super::release_tests::bundle_for(
        temp.path(),
        semver::Version::new(1, 0, 12),
        b"vibe 1.0.12",
    );
    let server_without = server(temp.path(), &fixture, None);
    let (store, env) = machine(temp.path());
    install(&store, &env, &server_without).unwrap();
    assert!(store.active().unwrap().is_some());
    assert!(!companions::store_root(&store).exists());
    let pin = PxPin::embedded();
    let asset = companions::pinned_asset(pin).unwrap();
    let name = pin.release_asset_name(asset);
    assert!(
        server_without
            .urls
            .borrow()
            .iter()
            .any(|url| url.contains(&name))
    );

    // Bytes under the pinned name that are not the pinned archive: refused
    // before activation.
    let temp = tempfile::tempdir().unwrap();
    let fixture = super::release_tests::bundle_for(
        temp.path(),
        semver::Version::new(1, 0, 12),
        b"vibe 1.0.12",
    );
    let server_wrong = server(temp.path(), &fixture, Some((&name, b"not px")));
    let (store, env) = machine(temp.path());
    let error = install(&store, &env, &server_wrong).unwrap_err();
    assert!(
        format!("{error:#}").contains("integrity mismatch"),
        "{error:#}"
    );
    assert!(store.active().unwrap().is_none());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools")]
fn an_already_installed_px_costs_no_download() {
    let temp = tempfile::tempdir().unwrap();
    let fixture = super::release_tests::bundle_for(
        temp.path(),
        semver::Version::new(1, 0, 12),
        b"vibe 1.0.12",
    );
    let (store, env) = machine(temp.path());
    let pin = PxPin::embedded();
    let asset = companions::pinned_asset(pin).unwrap();
    let program = vibe_net::px_store::stored_dir(&companions::store_root(&store), pin)
        .join(&asset.executable);
    std::fs::create_dir_all(program.parent().unwrap()).unwrap();
    std::fs::write(&program, b"px").unwrap();
    let server = server(temp.path(), &fixture, None);
    install(&store, &env, &server).unwrap();
    let name = pin.release_asset_name(asset);
    assert!(!server.urls.borrow().iter().any(|url| url.contains(&name)));
    let ctx = quiet();
    let progress = Progress::default();
    let persister = FakePersister::new(false);
    let remote = RemoteContext {
        source: Default::default(),
        locator: vibe_core::remap::active(),
        ctx: &ctx,
        env: &env,
        store: &store,
        downloader: &server,
        persister: &persister,
        command: "self:px",
        progress: &progress,
    };
    assert_eq!(
        companions::install_from_dirs(&remote, pin, &[], None, false).unwrap(),
        PxOutcome::AlreadyInstalled(program)
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools")]
fn a_drop_serves_px_to_any_installation_from_its_version_directory_or_root() {
    let temp = tempfile::tempdir().unwrap();
    let archive = zip_bytes(&[("px.exe", b"px".as_slice()), ("px", b"px".as_slice())]);
    let (mut pin, asset) = pin_for(&archive, "zip", vibe_net::PX_EXE);
    pin.version = "0.0.2-drop".into();
    let drop = temp.path().join("drop");
    let version_dir = drop.join("v1.0.12");
    std::fs::create_dir_all(&version_dir).unwrap();
    // The asset sits at the drop root, not in the version directory.
    std::fs::write(drop.join(pin.release_asset_name(&asset)), &archive).unwrap();
    let base = |path: &Path| {
        ReleaseDir::Base(reqwest::Url::from_directory_path(path).unwrap().to_string())
    };
    let (store, env) = machine(temp.path());
    let ctx = quiet();
    let progress = Progress::default();
    let persister = FakePersister::new(false);
    let downloader = super::super::HttpDownloader::for_invocation().unwrap();
    let remote = RemoteContext {
        source: Default::default(),
        locator: vibe_core::remap::active(),
        ctx: &ctx,
        env: &env,
        store: &store,
        downloader: &downloader,
        persister: &persister,
        command: "self:px",
        progress: &progress,
    };
    let dirs = super::super::px_lane::px_dirs(&base(&version_dir));
    assert_eq!(dirs.len(), 2, "the version directory, then the drop root");
    let outcome = companions::install_from_dirs(&remote, &pin, &dirs, None, false).unwrap();
    let PxOutcome::Installed { program, .. } = outcome else {
        panic!("{outcome:?}");
    };
    assert_eq!(
        program,
        companions::store_root(&store)
            .join("0.0.2-drop")
            .join(vibe_net::PX_EXE)
    );
    // A second installation of the same root shares it without a download.
    assert!(matches!(
        companions::install_from_dirs(&remote, &pin, &dirs, None, false).unwrap(),
        PxOutcome::AlreadyInstalled(_)
    ));
    // An empty drop reports the asset it looked for.
    let empty = temp.path().join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    let PxOutcome::Unavailable {
        asset: named,
        tried,
    } = companions::install_from_dirs(&remote, &pin, &[base(&empty)], None, true).unwrap()
    else {
        panic!("an empty drop has no px");
    };
    assert_eq!(named, pin.release_asset_name(&asset));
    assert_eq!(tried.len(), 1);
}

/// A release unpacked by hand: the pinned px archive beside the imported
/// binary, or in `tools/px/` there, is placed in the shared store; its
/// absence is reported, and bytes under the pinned name that are not the
/// pinned archive stop the import.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools")]
fn px_beside_an_unpacked_release_is_placed_in_the_store() {
    use super::super::px_lane::px_beside_with;
    let temp = tempfile::tempdir().unwrap();
    let (store, _) = machine(temp.path());
    let dir = temp.path().join("unpacked");
    std::fs::create_dir_all(dir.join("tools/px")).unwrap();
    let archive = zip_bytes(&[("px.exe", b"px beside".as_slice())]);
    let (pin, asset) = pin_for(&archive, "zip", "px.exe");
    let name = pin.release_asset_name(&asset);
    let progress = Progress::default();

    let absent = px_beside_with(&store, &pin, &dir, &progress).unwrap();
    assert!(
        matches!(absent, PxOutcome::Unavailable { .. }),
        "{absent:?}"
    );
    assert!(!companions::store_root(&store).exists());

    std::fs::write(dir.join("tools/px").join(&name), &archive).unwrap();
    let placed = px_beside_with(&store, &pin, &dir, &progress).unwrap();
    let PxOutcome::Installed { program, .. } = placed else {
        panic!("{placed:?}");
    };
    assert_eq!(std::fs::read(&program).unwrap(), b"px beside");
    assert_eq!(
        px_beside_with(&store, &pin, &dir, &progress).unwrap(),
        PxOutcome::AlreadyInstalled(program)
    );

    let other = tempfile::tempdir().unwrap();
    let (fresh, _) = machine(other.path());
    std::fs::write(other.path().join(&name), b"not the pinned archive").unwrap();
    let error = px_beside_with(&fresh, &pin, other.path(), &progress).unwrap_err();
    assert!(error.to_string().contains("integrity mismatch"), "{error}");
}
