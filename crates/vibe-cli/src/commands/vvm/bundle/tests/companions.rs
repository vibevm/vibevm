//! px in the shared tool store (PROP-019 `#companion-tools`): the archive
//! is unpacked into `<root>/vibevm/tools/px/<version>/` with the
//! provenance record beside it; a release without it installs as before;
//! any px the operator supplies is taken as it is, while bytes that are
//! not a px at all still stop the command before activation; the error of
//! a missing px names the store and the asset.

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
fn an_archive_of_either_kind_is_unpacked_into_the_versioned_store_with_its_record() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("opt/vibevm/tools/px");
    let zip = zip_bytes(&[
        ("px.exe", b"px for windows".as_slice()),
        ("Lib/x.pyd", b"lib".as_slice()),
        ("LICENSE.txt", b"MIT".as_slice()),
    ]);
    let (pin, asset) = pin_for(&zip, "zip", "px.exe");
    let program = companions::install_bytes(&root, &pin, &asset, &zip, "the drop").unwrap();
    assert_eq!(program, root.join("0.0.1-test/px.exe"));
    assert_eq!(std::fs::read(&program).unwrap(), b"px for windows");
    assert!(root.join("0.0.1-test/Lib/x.pyd").is_file());
    assert!(root.join("0.0.1-test/NOTICE").is_file());
    // The record states the provenance: where the bytes were read and the
    // SHA-256 vibe measured of them — information, never a gate
    // (owner decision 2026-11-09).
    let found = companions::installed(&root).expect("the store offers px");
    assert_eq!(found.program, program);
    assert_eq!(found.source.as_deref(), Some("the drop"));
    assert_eq!(found.sha256.as_deref(), Some(asset.sha256.as_str()));
    assert!(found.installed_at.is_some());

    let gz = tar_gz(&[
        ("./px", b"px for linux".as_slice()),
        ("./LICENSE.txt", b"MIT".as_slice()),
    ]);
    let (pin, asset) = pin_for(&gz, "tar.gz", "px");
    let program = companions::install_bytes(&root, &pin, &asset, &gz, "the drop").unwrap();
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

/// px is the operator's choice and vibe does not pin its content (owner
/// decision 2026-11-09): a digest that does not match the pin file's is
/// NOT a refusal. What still stops an installation is bytes that are not
/// a px at all — an archive that escapes its directory, or one with no px
/// program in it.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools")]
fn an_archive_whose_digest_is_not_the_pins_installs_anyway_and_records_what_it_was() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("store");
    let good = zip_bytes(&[("px.exe", b"px".as_slice())]);
    let (pin, asset) = pin_for(&good, "zip", "px.exe");
    let other = zip_bytes(&[("px.exe", b"a px of my own".as_slice())]);
    let program = companions::install_bytes(&root, &pin, &asset, &other, "mine.zip").unwrap();
    assert_eq!(std::fs::read(&program).unwrap(), b"a px of my own");
    let found = companions::installed(&root).expect("installed");
    assert_eq!(
        found.sha256.as_deref(),
        Some(companions::measure(&other).as_str())
    );
    assert_ne!(found.sha256.as_deref(), Some(asset.sha256.as_str()));

    for bytes in [
        zip_bytes(&[("../evil", b"x".as_slice())]),
        zip_bytes(&[("README", b"x".as_slice())]),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("store");
        let (pin, asset) = pin_for(&bytes, "zip", "px.exe");
        assert!(
            companions::install_bytes(&root, &pin, &asset, &bytes, "mine.zip").is_err(),
            "bytes that are not a px are still refused"
        );
        assert!(!root.join(&pin.version).exists());
        assert!(!temp.path().join("evil").exists());
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools")]
fn a_release_without_px_installs_as_before_and_unusable_px_bytes_stop_it() {
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

    // A digest that is not the pin file's is no longer a refusal (owner
    // decision 2026-11-09) — but bytes that are not an archive at all
    // cannot be unpacked, and a release source serving those still stops
    // the install before activation.
    let temp = tempfile::tempdir().unwrap();
    let fixture = super::release_tests::bundle_for(
        temp.path(),
        semver::Version::new(1, 0, 12),
        b"vibe 1.0.12",
    );
    let server_wrong = server(temp.path(), &fixture, Some((&name, b"not px")));
    let (store, env) = machine(temp.path());
    let error = install(&store, &env, &server_wrong).unwrap_err();
    assert!(format!("{error:#}").contains("px archive"), "{error:#}");
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

/// A release unpacked by hand: the px archive beside the imported binary,
/// or in `tools/px/` there, is placed in the shared store; its absence is
/// reported, and the bytes are installed as they are — a px from the
/// operator's own files is their choice (owner decision 2026-11-09).
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

    // A different archive under the same name is installed, not refused;
    // what it was is recorded.
    let other = tempfile::tempdir().unwrap();
    let (fresh, _) = machine(other.path());
    let mine = zip_bytes(&[("px.exe", b"a px of my own".as_slice())]);
    std::fs::write(other.path().join(&name), &mine).unwrap();
    let placed = px_beside_with(&fresh, &pin, other.path(), &progress).unwrap();
    let PxOutcome::Installed { program, .. } = placed else {
        panic!("{placed:?}");
    };
    assert_eq!(std::fs::read(&program).unwrap(), b"a px of my own");
    assert_eq!(
        companions::installed(&companions::store_root(&fresh)).and_then(|found| found.sha256),
        Some(companions::measure(&mine))
    );

    // Bytes that are not a px at all are still refused.
    let junk = tempfile::tempdir().unwrap();
    let (empty, _) = machine(junk.path());
    std::fs::write(junk.path().join(&name), b"not an archive").unwrap();
    assert!(px_beside_with(&empty, &pin, junk.path(), &progress).is_err());
}

/// `vibe self px install --from <archive|directory|executable>`: ANY px
/// the operator names is installed — a zip, a tar.gz, a directory they
/// unpacked themselves, or the program alone — of any version, on any
/// target, including one the pin has no default asset for. Nothing is
/// checked against a digest; the SHA-256 is measured and recorded so the
/// report can name the exact bytes (owner decision 2026-11-09).
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#companion-tools")]
fn any_local_px_is_installed_and_recorded_whatever_its_shape_or_target() {
    let temp = tempfile::tempdir().unwrap();
    let (store, _) = machine(temp.path());
    let root = companions::store_root(&store);
    let files = temp.path().join("files");
    std::fs::create_dir_all(&files).unwrap();
    let pin = PxPin::embedded();

    // A zip named `px-<version>-<target>.zip` lands in that version's
    // directory, so an operator's px and a default one can live together.
    let zip = zip_bytes(&[("px.exe", b"zipped px".as_slice())]);
    let named = match vibe_net::px_store::running_target() {
        Some(target) => files.join(format!("px-9.9.9-{target}.zip")),
        None => files.join("px.zip"),
    };
    std::fs::write(&named, &zip).unwrap();
    let placed = companions::install_local(&store, pin, &named, false).unwrap();
    let PxOutcome::Installed { program, .. } = placed else {
        panic!("{placed:?}");
    };
    assert_eq!(std::fs::read(&program).unwrap(), b"zipped px");
    let found = vibe_net::px_store::stored_in(program.parent().unwrap()).unwrap();
    assert_eq!(
        found.sha256.as_deref(),
        Some(companions::measure(&zip).as_str())
    );
    assert_eq!(
        found.source.as_deref(),
        Some(named.display().to_string().as_str())
    );
    if vibe_net::px_store::running_target().is_some() {
        assert!(
            program.starts_with(root.join("9.9.9")),
            "{}",
            program.display()
        );
    }

    // The same file again changes nothing; `--force` replaces it.
    assert!(matches!(
        companions::install_local(&store, pin, &named, false).unwrap(),
        PxOutcome::AlreadyInstalled(_)
    ));
    assert!(matches!(
        companions::install_local(&store, pin, &named, true).unwrap(),
        PxOutcome::Installed { .. }
    ));

    // A tar.gz whose px sits in a subdirectory is found.
    let gz = tar_gz(&[("px-0.1/px", b"tarred px".as_slice())]);
    let tarred = files.join("mine.tar.gz");
    std::fs::write(&tarred, &gz).unwrap();
    let placed = companions::install_local(&store, pin, &tarred, false).unwrap();
    let PxOutcome::Installed { program, .. } = placed else {
        panic!("{placed:?}");
    };
    assert_eq!(std::fs::read(&program).unwrap(), b"tarred px");
    assert!(
        program.starts_with(root.join(vibe_net::px_store::CUSTOM_VERSION)),
        "a name that states no version is `custom`: {}",
        program.display()
    );

    // The program alone.
    let bare = files.join("px-of-my-own");
    std::fs::write(&bare, b"bare px").unwrap();
    let placed = companions::install_local(&store, pin, &bare, true).unwrap();
    let PxOutcome::Installed { program, .. } = placed else {
        panic!("{placed:?}");
    };
    assert_eq!(std::fs::read(&program).unwrap(), b"bare px");
    assert_eq!(program.file_name().unwrap(), vibe_net::PX_EXE);

    // A directory the operator unpacked themselves.
    let unpacked = files.join("unpacked");
    std::fs::create_dir_all(unpacked.join("bin")).unwrap();
    std::fs::write(unpacked.join("bin").join(vibe_net::PX_EXE), b"unpacked px").unwrap();
    std::fs::write(unpacked.join("README"), b"notes").unwrap();
    let placed = companions::install_local(&store, pin, &unpacked, true).unwrap();
    let PxOutcome::Installed { program, .. } = placed else {
        panic!("{placed:?}");
    };
    assert_eq!(std::fs::read(&program).unwrap(), b"unpacked px");
    // The whole layout is copied, and the program is found inside it.
    let directory = root.join(vibe_net::px_store::CUSTOM_VERSION);
    assert_eq!(program, directory.join("bin").join(vibe_net::PX_EXE));
    assert!(directory.join("README").is_file());

    // And a target the pin has no default asset for is no obstacle: the
    // same call with a pin that offers nothing installs the same px.
    let no_asset = PxPin {
        assets: Vec::new(),
        ..pin.clone()
    };
    assert!(companions::pinned_asset(&no_asset).is_none());
    let placed = companions::install_local(&store, &no_asset, &bare, true).unwrap();
    assert!(matches!(placed, PxOutcome::Installed { .. }), "{placed:?}");

    // What is not a px at all is still refused, by name.
    let junk = files.join("notes.txt");
    std::fs::write(&junk, b"this is not px").unwrap();
    let error = companions::install_local(&store, pin, &files.join("missing"), false).unwrap_err();
    assert!(format!("{error:#}").contains("missing"), "{error:#}");
    let error = companions::install_local(&store, pin, &unpacked.join("README"), true);
    // A single file with no archive magic is taken as the program itself,
    // which is the operator's call to make.
    assert!(error.is_ok(), "{error:?}");
}
