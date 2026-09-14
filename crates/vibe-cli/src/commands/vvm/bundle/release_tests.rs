//! The newest-release lane: what `self update` and `self install stable` do
//! on a binary execution, and what `self reinstall` refetches instead
//! (PROP-019 `##CMD-UPDATE`, `##CMD-REINSTALL`). Every case runs against a
//! substituted `Downloader`, so the whole lane is exercised without a network.

use super::super::RemoteContext;
use super::*;

/// A mocked release server: one aggregate manifest and the bundle it names.
fn release_server(root: &Path, label: &str, fixture: &Fixture) -> ReleaseDownloader {
    let aggregate = root.join(format!("DISTRIBUTIONS-{label}.json"));
    std::fs::write(&aggregate, aggregate_for(fixture).to_json_bytes().unwrap()).unwrap();
    ReleaseDownloader {
        aggregate,
        bundle: fixture.path.clone(),
        urls: RefCell::new(Vec::new()),
    }
}

/// Run one release-lane call against a mocked server, the way the command
/// surface does once it has resolved the running binary's own identity.
fn drive<T>(
    store: &VersionStore,
    env: &VvmEnv,
    downloader: &dyn Downloader,
    command: &str,
    call: impl FnOnce(&RemoteContext<'_>) -> anyhow::Result<T>,
) -> anyhow::Result<T> {
    let ctx = quiet();
    let persister = FakePersister::new(false);
    call(&RemoteContext {
        ctx: &ctx,
        env,
        store,
        downloader,
        persister: &persister,
        command,
    })
}

/// A machine already holding `fixture`'s release as its running binary
/// instance — the state every `update` / `reinstall` case starts from.
fn seed_machine(temp: &Path, fixture: &Fixture) -> (VersionStore, VvmEnv, InstallRecord) {
    let store = VersionStore::new(temp.join("opt"));
    let env = VvmEnv {
        root: Some(temp.join("opt")),
        ..VvmEnv::default()
    };
    let server = release_server(temp, "seed", fixture);
    drive(&store, &env, &server, "self:install", |remote| {
        install_release_version(remote, &fixture.manifest.version, false)
    })
    .unwrap();
    let record = store.active().unwrap().unwrap();
    (store, env, record)
}

fn bundle_for(temp: &Path, version: semver::Version, payload: &[u8]) -> Fixture {
    write_versioned_bundle(temp, version, payload, payload, &source_zip(None), false)
}

fn asks_what_is_newest(url: &str) -> bool {
    url.starts_with(
        "https://github.com/vibevm/vibevm/releases/latest/download/DISTRIBUTIONS.json?vvm_nonce=",
    )
}

fn fetched(urls: &[String], asset_name: &str) -> bool {
    urls.iter().any(|url| {
        url.split('?')
            .next()
            .is_some_and(|url| url.ends_with(asset_name))
    })
}

#[test]
fn an_unchanged_newest_release_is_settled_by_the_manifest_without_a_download() {
    let temp = tempfile::tempdir().unwrap();
    let one = bundle_for(temp.path(), semver::Version::new(1, 0, 0), b"vibe-1.0.0");
    let (store, env, installed) = seed_machine(temp.path(), &one);
    assert_eq!(installed.selector().to_string(), "tag:1.0.0#1");

    let server = release_server(temp.path(), "same", &one);
    drive(&store, &env, &server, "self:update", |remote| {
        move_to_newest_release(remote, &installed, false)
    })
    .unwrap();

    let urls = server.urls.borrow().clone();
    assert_eq!(urls.len(), 1, "the bundle is never fetched: {urls:?}");
    assert!(asks_what_is_newest(&urls[0]), "{}", urls[0]);
    assert_eq!(store.load_state().unwrap().installs.len(), 1);
    assert_eq!(active_selector(&store), "tag:1.0.0#1");

    // `--force` still buys a fresh immutable generation of the same release,
    // and really refetches the bytes to build it from.
    let forced = release_server(temp.path(), "forced", &one);
    drive(&store, &env, &forced, "self:update", |remote| {
        move_to_newest_release(remote, &installed, true)
    })
    .unwrap();
    assert_eq!(active_selector(&store), "tag:1.0.0#2");
    assert!(fetched(&forced.urls.borrow(), &one.asset.name));
}

#[test]
fn a_release_rebuilt_under_its_own_number_is_refetched_and_installed() {
    let temp = tempfile::tempdir().unwrap();
    let one = bundle_for(temp.path(), semver::Version::new(1, 0, 0), b"vibe-1.0.0");
    let (store, env, installed) = seed_machine(temp.path(), &one);

    let rebuilt = bundle_for(
        temp.path(),
        semver::Version::new(1, 0, 0),
        b"vibe-1.0.0-rebuilt",
    );
    assert_ne!(rebuilt.asset.digest, one.asset.digest);
    assert_eq!(
        rebuilt.asset.name, one.asset.name,
        "same release asset name"
    );

    let server = release_server(temp.path(), "rebuilt", &rebuilt);
    drive(&store, &env, &server, "self:update", |remote| {
        move_to_newest_release(remote, &installed, false)
    })
    .unwrap();

    assert_eq!(store.load_state().unwrap().installs.len(), 2);
    assert_eq!(active_selector(&store), "tag:1.0.0#2");
    let urls = server.urls.borrow();
    assert!(asks_what_is_newest(&urls[0]), "{}", urls[0]);
    assert!(fetched(&urls, &rebuilt.asset.name), "{urls:?}");
    let active = store.active().unwrap().unwrap();
    assert_eq!(
        std::fs::read(store.binary_path(&active.version_id(), active.instance)).unwrap(),
        b"vibe-1.0.0-rebuilt"
    );
}

#[test]
fn a_newer_release_is_installed_and_activated_by_the_verified_path() {
    let temp = tempfile::tempdir().unwrap();
    let one = bundle_for(temp.path(), semver::Version::new(1, 0, 0), b"vibe-1.0.0");
    let (store, env, installed) = seed_machine(temp.path(), &one);

    let two = bundle_for(temp.path(), semver::Version::new(1, 1, 0), b"vibe-1.1.0");
    let server = release_server(temp.path(), "forward", &two);
    drive(&store, &env, &server, "self:update", |remote| {
        move_to_newest_release(remote, &installed, false)
    })
    .unwrap();

    let active = store.active().unwrap().unwrap();
    assert_eq!(active.id, "1.1.0");
    assert_eq!(active.origin, Origin::Binary);
    assert_eq!(
        std::fs::read(store.binary_path(&active.version_id(), active.instance)).unwrap(),
        b"vibe-1.1.0"
    );
    // The older generation is preserved, so `self rollback` still has a home.
    assert_eq!(store.previous().unwrap().unwrap().id, "1.0.0");
    let urls = server.urls.borrow();
    assert!(asks_what_is_newest(&urls[0]), "{}", urls[0]);
    assert!(
        urls.iter().any(|url| url.starts_with(
            "https://github.com/vibevm/vibevm/releases/download/v1.1.0/vibevm-1.1.0-"
        )),
        "the bundle comes from the new version's own release directory: {urls:?}"
    );
}

#[test]
fn a_withdrawn_newest_release_never_walks_the_machine_backwards() {
    let temp = tempfile::tempdir().unwrap();
    let two = bundle_for(temp.path(), semver::Version::new(1, 1, 0), b"vibe-1.1.0");
    let (store, env, installed) = seed_machine(temp.path(), &two);

    let one = bundle_for(temp.path(), semver::Version::new(1, 0, 0), b"vibe-1.0.0");
    let server = release_server(temp.path(), "withdrawn", &one);
    drive(&store, &env, &server, "self:update", |remote| {
        move_to_newest_release(remote, &installed, false)
    })
    .unwrap();

    assert_eq!(server.urls.borrow().len(), 1, "nothing but the manifest");
    assert_eq!(store.load_state().unwrap().installs.len(), 1);
    assert_eq!(active_selector(&store), "tag:1.1.0#1");
}

struct UnreachableServer;

impl Downloader for UnreachableServer {
    fn download(&self, url: &str, _destination: &Path, _maximum_bytes: u64) -> anyhow::Result<()> {
        anyhow::bail!("release server unreachable for `{url}`")
    }
}

#[test]
fn an_unreadable_newest_release_manifest_fails_the_command_and_changes_nothing() {
    let temp = tempfile::tempdir().unwrap();
    let one = bundle_for(temp.path(), semver::Version::new(1, 0, 0), b"vibe-1.0.0");
    let (store, env, installed) = seed_machine(temp.path(), &one);

    let error = drive(&store, &env, &UnreachableServer, "self:update", |remote| {
        move_to_newest_release(remote, &installed, false)
    })
    .unwrap_err();
    let error = format!("{error:#}");
    assert!(
        error.contains("learning the newest published release"),
        "{error}"
    );
    assert!(error.contains("release server unreachable"), "{error}");
    assert_eq!(store.load_state().unwrap().installs.len(), 1);
    assert_eq!(active_selector(&store), "tag:1.0.0#1");
}

#[test]
fn reinstall_refetches_the_running_version_without_asking_what_is_newest() {
    let temp = tempfile::tempdir().unwrap();
    let one = bundle_for(temp.path(), semver::Version::new(1, 0, 0), b"vibe-1.0.0");
    let (store, env, installed) = seed_machine(temp.path(), &one);

    let server = release_server(temp.path(), "reinstall", &one);
    let reused = drive(&store, &env, &server, "self:reinstall", |remote| {
        install_release_version(remote, &installed.id, true)
    })
    .unwrap();

    assert!(!reused, "reinstall always lands a fresh immutable instance");
    assert_eq!(active_selector(&store), "tag:1.0.0#2");
    let urls = server.urls.borrow();
    assert!(
        urls.iter().any(|url| url.starts_with(
            "https://github.com/vibevm/vibevm/releases/download/v1.0.0/DISTRIBUTIONS.json?"
        )),
        "{urls:?}"
    );
    assert!(fetched(&urls, &one.asset.name), "{urls:?}");
    assert!(
        !urls.iter().any(|url| url.contains("/releases/latest/")),
        "reinstall never asks what the newest release is: {urls:?}"
    );
}
