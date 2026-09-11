use super::super::{CONNECT_TIMEOUT, RemoteContext, TOTAL_TIMEOUT, copy_download, write_download};
use super::*;
use vibe_publish::release_manifest::DISTRIBUTION_BUNDLE_MAX_BYTES;

#[test]
fn download_copy_stops_at_declared_size_plus_one() {
    assert_eq!(CONNECT_TIMEOUT, std::time::Duration::from_secs(20));
    assert_eq!(TOTAL_TIMEOUT, std::time::Duration::from_secs(30 * 60));
    let mut source = std::io::Cursor::new(vec![1_u8; 32]);
    let mut destination = Vec::new();
    let error = copy_download(&mut source, &mut destination, 8)
        .unwrap_err()
        .to_string();
    assert!(error.contains("8-byte limit"));
    assert_eq!(destination.len(), 9);
}

#[test]
fn download_writer_never_deletes_or_truncates_an_unowned_collision() {
    let temp = tempfile::tempdir().unwrap();
    let destination = temp.path().join("download.tmp");
    std::fs::write(&destination, b"owner bytes").unwrap();
    let mut source = std::io::Cursor::new(b"new bytes");
    assert!(write_download(&destination, 1024, &mut source).is_err());
    assert_eq!(std::fs::read(destination).unwrap(), b"owner bytes");
}

#[test]
fn oversized_bundle_declaration_is_rejected_before_downloader_runs() {
    let temp = tempfile::tempdir().unwrap();
    let source = source_zip(None);
    let fixture = write_bundle(temp.path(), b"vibe-binary", b"vibe-binary", &source, false);
    let mut aggregate = aggregate_for(&fixture);
    aggregate
        .platforms
        .iter_mut()
        .find(|platform| platform.target == current_target().unwrap())
        .unwrap()
        .asset
        .size = DISTRIBUTION_BUNDLE_MAX_BYTES + 1;
    let manifest = temp.path().join("oversized.json");
    std::fs::write(&manifest, serde_json::to_vec(&aggregate).unwrap()).unwrap();
    let downloader = LocalDownloader {
        bundle: fixture.path,
        urls: RefCell::new(Vec::new()),
    };
    let env = VvmEnv {
        root: Some(temp.path().join("opt")),
        ..VvmEnv::default()
    };
    let bootstrap = temp.path().join("vibe-bootstrap.exe");
    write_test_executable(&bootstrap, b"vibe-binary");
    let error = bootstrap_with_downloader(
        &quiet(),
        &env,
        &VvmBootstrapArgs {
            manifest,
            version: "1.0.0".into(),
            release_base: "https://github.com/vibevm/vibevm/releases/download/v1.0.0".into(),
            force: false,
        },
        &downloader,
        &bootstrap,
        &FakePersister::new(false),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("exceeding the independent"), "{error}");
    assert!(downloader.urls.borrow().is_empty());
}

#[test]
fn hosted_bootstrap_cross_checks_manifest_and_downloads_selected_bundle_only() {
    let temp = tempfile::tempdir().unwrap();
    let source = source_zip(None);
    let fixture = write_bundle(temp.path(), b"vibe-binary", b"vibe-binary", &source, false);
    let aggregate_path = temp.path().join("DISTRIBUTIONS.json");
    std::fs::write(
        &aggregate_path,
        aggregate_for(&fixture).to_json_bytes().unwrap(),
    )
    .unwrap();
    let downloader = LocalDownloader {
        bundle: fixture.path.clone(),
        urls: RefCell::new(Vec::new()),
    };
    let env = VvmEnv {
        root: Some(temp.path().join("opt")),
        ..VvmEnv::default()
    };
    let persister = FakePersister::new(false);
    let args = VvmBootstrapArgs {
        manifest: aggregate_path,
        version: "1.0.0".into(),
        release_base: "https://github.com/vibevm/vibevm/releases/download/v1.0.0".into(),
        force: false,
    };

    let mismatched = VvmBootstrapArgs {
        manifest: args.manifest.clone(),
        version: "2.0.0".into(),
        release_base: args.release_base.clone(),
        force: false,
    };
    let bootstrap = temp.path().join("vibe-bootstrap.exe");
    write_test_executable(&bootstrap, b"vibe-binary");
    let error = bootstrap_with_downloader(
        &quiet(),
        &env,
        &mismatched,
        &downloader,
        &bootstrap,
        &persister,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("does not match requested"), "{error}");
    assert!(downloader.urls.borrow().is_empty());

    write_test_executable(&bootstrap, b"wrong-bootstrap");
    let error =
        bootstrap_with_downloader(&quiet(), &env, &args, &downloader, &bootstrap, &persister)
            .unwrap_err()
            .to_string();
    assert!(error.contains("expected bounded regular file"), "{error}");
    assert!(downloader.urls.borrow().is_empty());
    write_test_executable(&bootstrap, b"vibe-binary");
    bootstrap_with_downloader(&quiet(), &env, &args, &downloader, &bootstrap, &persister).unwrap();
    let urls = downloader.urls.borrow();
    assert_eq!(urls.len(), 1);
    assert!(
        urls[0]
            .split('?')
            .next()
            .is_some_and(|url| url.ends_with(&fixture.asset.name))
    );
    assert!(urls[0].contains("?vvm_nonce="));
    assert_eq!(persister.homes.borrow().len(), 1);
    assert_eq!(persister.paths.borrow().len(), 1);
    assert_eq!(
        active_selector(&VersionStore::new(temp.path().join("opt"))),
        "tag:1.0.0#1"
    );
}

#[test]
fn binary_update_uses_mutable_release_assets_and_force_allocates_a_fresh_instance() {
    let temp = tempfile::tempdir().unwrap();
    let source = source_zip(None);
    let fixture = write_bundle(temp.path(), b"vibe-binary", b"vibe-binary", &source, false);
    let aggregate_path = temp.path().join("DISTRIBUTIONS.json");
    std::fs::write(
        &aggregate_path,
        aggregate_for(&fixture).to_json_bytes().unwrap(),
    )
    .unwrap();
    let downloader = ReleaseDownloader {
        aggregate: aggregate_path,
        bundle: fixture.path.clone(),
        urls: RefCell::new(Vec::new()),
    };
    let store = VersionStore::new(temp.path().join("opt"));
    let env = VvmEnv {
        root: Some(temp.path().join("opt")),
        ..VvmEnv::default()
    };
    let persister = FakePersister::new(false);
    let ctx = quiet();
    let remote = RemoteContext {
        ctx: &ctx,
        env: &env,
        store: &store,
        downloader: &downloader,
        persister: &persister,
        command: "self:update",
    };

    update_binary(&remote, "1.0.0", false).unwrap();
    update_binary(&remote, "1.0.0", false).unwrap();
    assert_eq!(store.load_state().unwrap().installs.len(), 1);
    update_binary(&remote, "1.0.0", true).unwrap();
    assert_eq!(store.load_state().unwrap().installs.len(), 2);
    assert_eq!(active_selector(&store), "tag:1.0.0#2");
    let urls = downloader.urls.borrow();
    assert!(urls.iter().any(|url| url.starts_with(
        "https://github.com/vibevm/vibevm/releases/download/v1.0.0/DISTRIBUTIONS.json?vvm_nonce="
    )));
    assert!(urls.iter().all(|url| url.contains("?vvm_nonce=")));
    assert_eq!(
        urls.iter().cloned().collect::<BTreeSet<_>>().len(),
        urls.len(),
        "every mutable release request bypasses stable-URL caches"
    );
}
