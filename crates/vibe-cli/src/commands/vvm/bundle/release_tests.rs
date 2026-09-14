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

/// A server that must never be asked anything: under the offline posture
/// every release-lane verb is refused BEFORE its first request, so a call
/// arriving here is the bug the refusal exists to prevent.
struct UnaskedServer;

impl Downloader for UnaskedServer {
    fn download(&self, url: &str, _destination: &Path, _maximum_bytes: u64) -> anyhow::Result<()> {
        panic!("the offline posture must refuse before requesting `{url}`")
    }
}

/// The offline posture, with the store already holding the release: every
/// release-lane verb refuses, names itself and the address it wanted, and
/// cites the rule — without one request and without touching the inventory.
///
/// The seeded machine is the load-bearing part. `install_selected` can
/// recognise an unchanged release from its manifest and reuse the instance
/// it already holds, costing no download — so a weaker implementation could
/// look like it honoured the posture here. It does not: the manifest that
/// would recognise the instance is itself a fetch, so the verb stops before
/// it, and the held instance rescues nothing.
#[test]
fn the_offline_posture_refuses_every_release_verb_before_its_first_request() {
    let temp = tempfile::tempdir().unwrap();
    let one = bundle_for(temp.path(), semver::Version::new(1, 0, 0), b"vibe-1.0.0");
    let (store, _online, installed) = seed_machine(temp.path(), &one);
    let env = VvmEnv {
        root: Some(temp.path().join("opt")),
        offline: true,
        ..VvmEnv::default()
    };

    // `{:#}` — the rendering the CLI itself prints, so these read the whole
    // chain a caller would see and not just its outermost link.
    let refused = |command: &str, address: &str, error: anyhow::Error| {
        let error = format!("{error:#}");
        assert!(
            error.contains(&format!("`vibe {}`", command.replace(':', " "))),
            "the refusal names the verb: {error}"
        );
        assert!(
            error.contains(address),
            "the refusal names the address: {error}"
        );
        assert!(
            error.contains("spec://org.vibevm.core/vibevm/common/PROP-019#surface"),
            "the refusal cites the rule: {error}"
        );
    };

    // `self update` and `self install stable` are one function — `stable` IS
    // the newest release — so both stop at the newest-release manifest.
    let newest = "https://github.com/vibevm/vibevm/releases/latest/download/DISTRIBUTIONS.json";
    for command in ["self:update", "self:install"] {
        refused(
            command,
            newest,
            drive(&store, &env, &UnaskedServer, command, |remote| {
                move_to_newest_release(remote, &installed, false)
            })
            .unwrap_err(),
        );
    }

    // `self install X.Y.Z` and `self reinstall` ask a named release's own
    // manifest rather than what is newest, and stop at the same seam.
    let named = "https://github.com/vibevm/vibevm/releases/download/v1.0.0/DISTRIBUTIONS.json";
    for (command, force) in [("self:install", false), ("self:reinstall", true)] {
        refused(
            command,
            named,
            drive(&store, &env, &UnaskedServer, command, |remote| {
                install_release_version(remote, "1.0.0", force)
            })
            .unwrap_err(),
        );
    }

    // Nothing moved: no generation allocated, no pointer repointed.
    assert_eq!(store.load_state().unwrap().installs.len(), 1);
    assert_eq!(active_selector(&store), "tag:1.0.0#1");
}

/// `--force` asks for a fresh generation, which is exactly the case that
/// cannot be served from what is already on disk. It is refused the same
/// way rather than falling through to a download the posture forbids.
#[test]
fn a_forced_offline_update_is_refused_rather_than_allocating_a_generation() {
    let temp = tempfile::tempdir().unwrap();
    let one = bundle_for(temp.path(), semver::Version::new(1, 0, 0), b"vibe-1.0.0");
    let (store, _online, installed) = seed_machine(temp.path(), &one);
    let env = VvmEnv {
        root: Some(temp.path().join("opt")),
        offline: true,
        ..VvmEnv::default()
    };

    let error = format!(
        "{:#}",
        drive(&store, &env, &UnaskedServer, "self:update", |remote| {
            move_to_newest_release(remote, &installed, true)
        })
        .unwrap_err()
    );
    assert!(error.contains("this run is offline"), "{error}");
    assert_eq!(store.load_state().unwrap().installs.len(), 1);
    assert_eq!(active_selector(&store), "tag:1.0.0#1");
}

/// The posture is a posture, not a mode the lane is compiled in: with it
/// resolved false the same call against the same store does its ordinary
/// work. Pinned so a guard placed too widely shows up as a red here rather
/// than as a version manager nobody can update.
#[test]
fn an_online_run_is_untouched_by_the_offline_guard() {
    let temp = tempfile::tempdir().unwrap();
    let one = bundle_for(temp.path(), semver::Version::new(1, 0, 0), b"vibe-1.0.0");
    let (store, env, installed) = seed_machine(temp.path(), &one);
    assert!(!env.offline);

    let server = release_server(temp.path(), "online", &one);
    drive(&store, &env, &server, "self:update", |remote| {
        move_to_newest_release(remote, &installed, false)
    })
    .unwrap();
    assert_eq!(server.urls.borrow().len(), 1);
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
