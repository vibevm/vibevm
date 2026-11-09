//! Local import: bare payloads, companions beside them, and releases
//! unpacked by hand.

use super::*;
use crate::commands::vvm::env::{self, EnvPersister, Persisted};
use sha2::Digest;
use std::cell::Cell;
use std::fs;

fn quiet() -> output::Context {
    output::Context::from_flags(true, false, None, true, crate::cli::AgentModeArg::Auto)
}

fn write_payload(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }
}

fn request(executable: &Path, replace_candidate: bool) -> ImportRequest<'_> {
    ImportRequest {
        executable,
        tag: "1.2.3",
        commit: Some("abc1234"),
        profile: Profile::Release,
        replace_candidate,
        now: "2026-08-20T00:00:00Z",
    }
}

#[test]
fn import_reuses_same_hash_and_preserves_each_distinct_mutable_version_payload() {
    let tmp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(tmp.path().join("opt"));
    let payload = tmp.path().join("ready-vibe.exe");
    write_payload(&payload, b"candidate-one");

    perform_import(&quiet(), &store, &request(&payload, false)).unwrap();
    assert!(
        store.read_current().unwrap().is_none(),
        "import is inactive by default"
    );

    perform_import(&quiet(), &store, &request(&payload, false)).unwrap();
    let id = VersionId::new(Kind::Tag, "1.2.3");
    assert_eq!(store.instances_of(&id).unwrap().len(), 1);
    assert_eq!(store.load_state().unwrap().next_instance, 2);

    write_payload(&payload, b"candidate-two");
    perform_import(&quiet(), &store, &request(&payload, false)).unwrap();
    let records = store.instances_of(&id).unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].selector().to_string(), "tag:1.2.3#1");
    assert_eq!(records[1].selector().to_string(), "tag:1.2.3#2");
    assert_eq!(
        fs::read(store.binary_path(&id, 1)).unwrap(),
        b"candidate-one"
    );
    assert_eq!(
        fs::read(store.binary_path(&id, 2)).unwrap(),
        b"candidate-two"
    );
}

#[test]
fn imported_outcome_uses_the_common_durable_activation_path() {
    let tmp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(tmp.path().join("opt"));
    let payload = tmp.path().join("ready-vibe.exe");
    write_payload(&payload, b"candidate-one");
    perform_import(&quiet(), &store, &request(&payload, false)).unwrap();

    write_payload(&payload, b"candidate-two");
    let outcome = perform_import(&quiet(), &store, &request(&payload, true)).unwrap();
    let persister = FakePersister::default();
    env::activate_instance(&store, &outcome.home, &persister).unwrap();

    let id = VersionId::new(Kind::Tag, "1.2.3");
    assert_eq!(store.instances_of(&id).unwrap().len(), 2);
    assert_eq!(
        store.read_current().unwrap().unwrap(),
        store.instance_dir(&id, 2)
    );
    assert!(persister.path.get() && persister.home.get());
    assert!(store.shim_dir().join("vibe").is_file());
    let shim = fs::read_to_string(store.shim_dir().join("vibe")).unwrap();
    assert!(shim.contains("vibe self use"));
    assert!(!shim.contains("vibe man use"));
    if cfg!(windows) {
        assert!(store.shim_dir().join("vibe.cmd").is_file());
    }
}

#[derive(Default)]
struct FakePersister {
    path: Cell<bool>,
    home: Cell<bool>,
}

impl EnvPersister for FakePersister {
    fn set_install_root(&self, _root: &Path) -> Result<Persisted> {
        Ok(Persisted::Unchanged)
    }

    fn set_vibevm_home(&self, _home: &Path) -> Result<Persisted> {
        self.home.set(true);
        Ok(Persisted::Changed)
    }

    fn ensure_on_path(&self, _dir: &Path) -> Result<Persisted> {
        self.path.set(true);
        Ok(Persisted::Changed)
    }

    fn activation_hint(&self) -> String {
        "test".into()
    }
}

const RELEASE_COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

fn sha(bytes: &[u8]) -> String {
    format!("sha256:{:x}", sha2::Sha256::digest(bytes))
}

fn source_zip() -> Vec<u8> {
    use std::io::Write;
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    for (name, bytes) in [
        ("Cargo.toml", b"[workspace]\n".as_slice()),
        (
            "crates/vibe-cli/Cargo.toml",
            b"[package]\nname='vibe-cli'\n",
        ),
    ] {
        writer.start_file(name, options).unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

const VIBE: &[u8] = b"release-vibe";
const PROXY: &[u8] = b"release-proxy";
const INDEX: &[u8] = b"release-index";
/// A tenant of the shared directory the release was unpacked into.
const FOREIGN: (&str, &[u8]) = ("corp-tool", b"#!/bin/sh\necho corp\n");

/// A release ZIP unpacked by hand into `dir`, beside a foreign tool: the
/// executables, the packaged source and the `DISTRIBUTION.json` naming
/// them. `declared_vibe` is what the manifest says `vibe` is.
fn unpack_release(dir: &Path, declared_vibe: &[u8]) -> PathBuf {
    use vibe_publish::release_manifest::{
        BundleDistributionManifest, DISTRIBUTION_SOURCE_ARCHIVE_FILENAME, DistributionComponent,
        DistributionComponentName, DistributionSourceArchive,
    };
    fs::create_dir_all(dir).unwrap();
    let source = source_zip();
    let component = |name, path: &str, bytes: &[u8]| DistributionComponent {
        name,
        path: path.to_string(),
        size: bytes.len() as u64,
        digest: sha(bytes),
    };
    let manifest = BundleDistributionManifest::new(
        semver::Version::new(1, 2, 3),
        RELEASE_COMMIT,
        crate::commands::vvm::bundle::current_target().unwrap(),
        vec![
            component(DistributionComponentName::Vibe, BINARY_NAME, declared_vibe),
            component(
                DistributionComponentName::VibeAppProxy,
                APP_PROXY_BINARY_NAME,
                PROXY,
            ),
            component(
                DistributionComponentName::VibeIndex,
                INDEX_BINARY_NAME,
                INDEX,
            ),
        ],
        DistributionSourceArchive {
            path: DISTRIBUTION_SOURCE_ARCHIVE_FILENAME.to_string(),
            size: source.len() as u64,
            digest: sha(&source),
            tree_oid: "a".repeat(40),
        },
    )
    .unwrap();
    write_payload(&dir.join(BINARY_NAME), VIBE);
    write_payload(&dir.join(APP_PROXY_BINARY_NAME), PROXY);
    write_payload(&dir.join(INDEX_BINARY_NAME), INDEX);
    fs::write(dir.join(DISTRIBUTION_SOURCE_ARCHIVE_FILENAME), &source).unwrap();
    fs::write(
        dir.join(DISTRIBUTION_MANIFEST_FILENAME),
        manifest.to_json_bytes().unwrap(),
    )
    .unwrap();
    write_payload(&dir.join(FOREIGN.0), FOREIGN.1);
    dir.join(BINARY_NAME)
}

fn release_request(executable: &Path) -> ImportRequest<'_> {
    let mut request = request(executable, false);
    request.commit = None;
    request
}

fn refused(result: Result<ImportOutcome>) -> String {
    match result {
        Ok(outcome) => panic!("unexpected import of {}", outcome.record.selector()),
        Err(error) => format!("{error:#}"),
    }
}

fn instance_files(home: &Path) -> std::collections::BTreeSet<String> {
    fs::read_dir(home)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect()
}

/// A release unpacked by hand is imported member by member — the instance
/// is the one the release ZIP installs, and the foreign tool sharing the
/// directory is neither copied nor touched.
#[test]
fn an_unpacked_release_imports_exactly_its_verified_members() {
    let tmp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(tmp.path().join("opt"));
    let payload = unpack_release(&tmp.path().join("unpacked"), VIBE);

    let outcome = perform_import(&quiet(), &store, &release_request(&payload)).unwrap();
    assert!(!outcome.reused);
    let record = &outcome.record;
    assert_eq!(record.commit, RELEASE_COMMIT);
    assert!(record.distribution_manifest_sha256.is_some());
    let id = record.version_id();
    assert_eq!(
        fs::read(store.binary_path(&id, record.instance)).unwrap(),
        VIBE
    );
    assert_eq!(
        fs::read(store.app_proxy_binary_path(&id, record.instance)).unwrap(),
        PROXY
    );
    assert_eq!(
        fs::read(store.index_binary_path(&id, record.instance)).unwrap(),
        INDEX
    );
    assert!(
        store
            .instance_source_dir(&id, record.instance)
            .join("Cargo.toml")
            .is_file()
    );
    let files = instance_files(&outcome.home);
    assert!(!files.contains(FOREIGN.0), "{files:?}");
    assert!(files.contains(DISTRIBUTION_MANIFEST_FILENAME), "{files:?}");
    assert_eq!(
        instance_files(&outcome.home.join("bin")),
        [BINARY_NAME, APP_PROXY_BINARY_NAME, INDEX_BINARY_NAME]
            .into_iter()
            .map(str::to_owned)
            .collect()
    );
    assert_eq!(
        fs::read(payload.with_file_name(FOREIGN.0)).unwrap(),
        FOREIGN.1,
        "the foreign tenant is left byte-identical"
    );
    assert!(
        crate::commands::vvm::bundle::installed_bundle_intact(&store, record),
        "the instance verifies as a release-bundle instance"
    );

    // The same release again is the same instance.
    let again = perform_import(&quiet(), &store, &release_request(&payload)).unwrap();
    assert!(again.reused);
    assert_eq!(store.instances_of(&id).unwrap().len(), 1);
}

/// A member whose bytes differ from the manifest refuses the import by
/// name, before an instance is published.
#[test]
fn a_tampered_member_refuses_the_import_and_names_the_file() {
    let tmp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(tmp.path().join("opt"));
    let payload = unpack_release(&tmp.path().join("unpacked"), VIBE);
    let proxy = payload.with_file_name(APP_PROXY_BINARY_NAME);
    write_payload(&proxy, b"tampered-proxy");

    let error = refused(perform_import(&quiet(), &store, &release_request(&payload)));
    assert!(error.contains(&proxy.display().to_string()), "{error}");
    assert!(error.contains("PROP-019"), "{error}");
    assert!(
        store
            .instances_of(&VersionId::new(Kind::Tag, "1.2.3"))
            .unwrap()
            .is_empty()
    );

    fs::remove_file(&proxy).unwrap();
    let error = refused(perform_import(&quiet(), &store, &release_request(&payload)));
    assert!(error.contains(&proxy.display().to_string()), "{error}");
}

/// A `DISTRIBUTION.json` describing other `vibe` bytes, another version or
/// another commit is not this payload's release: the import refuses
/// rather than guessing which of the two is stale.
#[test]
fn a_manifest_that_does_not_describe_the_payload_refuses() {
    let tmp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(tmp.path().join("opt"));
    let payload = unpack_release(&tmp.path().join("unpacked"), b"a-different-release");
    let error = refused(perform_import(&quiet(), &store, &release_request(&payload)));
    assert!(error.contains(&payload.display().to_string()), "{error}");

    let payload = unpack_release(&tmp.path().join("second"), VIBE);
    let mut wrong_tag = release_request(&payload);
    wrong_tag.tag = "1.2.4";
    let error = refused(perform_import(&quiet(), &store, &wrong_tag));
    assert!(error.contains("version 1.2.3"), "{error}");
    let wrong_commit = request(&payload, false);
    let error = refused(perform_import(&quiet(), &store, &wrong_commit));
    assert!(error.contains("abc1234"), "{error}");
}

/// Without a manifest the payload brings the companions found beside it
/// under their canonical names, reports the ones missing, and never copies
/// a foreign file.
#[test]
fn a_bare_payload_brings_the_companions_beside_it() {
    let tmp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(tmp.path().join("opt"));
    let dir = tmp.path().join("bin");
    fs::create_dir_all(&dir).unwrap();
    let payload = dir.join("ready-vibe");
    write_payload(&payload, b"bare");
    write_payload(&dir.join(FOREIGN.0), FOREIGN.1);

    let (_, missing) = loose_distribution(&payload);
    let missing: Vec<_> = missing.iter().map(|companion| companion.name).collect();
    assert_eq!(missing, [APP_PROXY_BINARY_NAME, INDEX_BINARY_NAME]);
    let alone = perform_import(&quiet(), &store, &request(&payload, false)).unwrap();
    let id = alone.record.version_id();
    assert!(
        !store
            .app_proxy_binary_path(&id, alone.record.instance)
            .exists()
    );

    write_payload(&dir.join(APP_PROXY_BINARY_NAME), PROXY);
    let (_, missing) = loose_distribution(&payload);
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].name, INDEX_BINARY_NAME);
    let with_proxy = perform_import(&quiet(), &store, &request(&payload, false)).unwrap();
    assert!(
        !with_proxy.reused,
        "the same `vibe` with a new companion is a new distribution"
    );
    let instance = with_proxy.record.instance;
    assert_eq!(
        fs::read(store.app_proxy_binary_path(&id, instance)).unwrap(),
        PROXY
    );
    assert_eq!(fs::read(store.binary_path(&id, instance)).unwrap(), b"bare");
    assert_eq!(
        instance_files(&with_proxy.home.join("bin")),
        [BINARY_NAME, APP_PROXY_BINARY_NAME]
            .into_iter()
            .map(str::to_owned)
            .collect()
    );
    assert!(placer::installed_files_match(&store, &with_proxy.record));
    assert!(crate::commands::vvm::ensure_activatable(&store, &with_proxy.record).is_ok());
    assert!(!instance_files(&with_proxy.home).contains(FOREIGN.0));
    assert_eq!(fs::read(dir.join(FOREIGN.0)).unwrap(), FOREIGN.1);

    let again = perform_import(&quiet(), &store, &request(&payload, false)).unwrap();
    assert!(again.reused);
}

/// Nothing beside the payload, nothing to read: the record still says
/// `unknown` rather than inventing a provenance.
#[test]
fn a_bare_payload_with_no_manifest_beside_it_stays_unknown() {
    let tmp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(tmp.path().join("opt"));
    let payload = tmp.path().join("vibe");
    write_payload(&payload, b"bare");

    let mut request = request(&payload, false);
    request.commit = None;
    let outcome = perform_import(&quiet(), &store, &request).unwrap();
    assert_eq!(outcome.record.commit, UNKNOWN_COMMIT);
}

#[test]
fn tampered_or_missing_legacy_payload_is_never_reused() {
    let temp = tempfile::tempdir().unwrap();
    let store = VersionStore::new(temp.path().join("opt"));
    let payload = temp.path().join("ready-vibe");
    write_payload(&payload, b"candidate");
    perform_import(&quiet(), &store, &request(&payload, false)).unwrap();
    let id = VersionId::new(Kind::Tag, "1.2.3");

    fs::write(store.binary_path(&id, 1), b"tampered").unwrap();
    perform_import(&quiet(), &store, &request(&payload, false)).unwrap();
    assert_eq!(store.instances_of(&id).unwrap().len(), 2);

    fs::remove_file(store.binary_path(&id, 2)).unwrap();
    perform_import(&quiet(), &store, &request(&payload, false)).unwrap();
    assert_eq!(store.instances_of(&id).unwrap().len(), 3);
    assert_eq!(fs::read(store.binary_path(&id, 3)).unwrap(), b"candidate");
}
