//! Local import of a ready-built `vibe` executable into the immutable VVM
//! inventory. The path is local-only: no resolver, network, or signature
//! machinery participates.

use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_core::progress::{Progress, ProgressTask};
use vibe_publish::release_manifest::{
    BundleDistributionManifest, DISTRIBUTION_MANIFEST_FILENAME, DISTRIBUTION_MANIFEST_MAX_BYTES,
    DistributionComponentName,
};

use super::model::{InstallRecord, Kind, Origin, Profile, VersionId};
use super::placer;
use super::store::{BINARY_NAME, VersionStore, open_regular_no_follow};
use crate::output;

/// The provenance of a payload with no explicit `--commit`, when nothing
/// beside it says which commit built it.
const UNKNOWN_COMMIT: &str = "unknown";

pub(crate) struct ImportRequest<'a> {
    pub executable: &'a Path,
    pub tag: &'a str,
    pub commit: Option<&'a str>,
    pub profile: Profile,
    pub replace_candidate: bool,
    pub now: &'a str,
}

pub(crate) struct ImportOutcome {
    pub record: InstallRecord,
    pub home: std::path::PathBuf,
    pub reused: bool,
}

/// What the payload-verification stage observed, including the commit the
/// record will carry.
struct ObservedPayload {
    id: VersionId,
    dist: Vec<(PathBuf, String)>,
    manifest: placer::Manifest,
    digest: String,
    commit: String,
    existing: Vec<InstallRecord>,
    reused: Option<InstallRecord>,
}

pub(crate) fn perform_import(
    ctx: &output::Context,
    store: &VersionStore,
    req: &ImportRequest<'_>,
) -> Result<ImportOutcome> {
    let progress = ctx.progress();
    let observed = observed_stage(&progress, "Verifying local import payload", |task| {
        task.detail(format!("payload: {}", req.executable.display()));
        let tag = parse_tag(req.tag)?;
        ensure_payload_file(req.executable)?;
        let id = VersionId::new(Kind::Tag, tag);
        let dist = vec![(req.executable.to_path_buf(), BINARY_NAME.to_string())];
        let manifest = placer::manifest_for(&dist)?;
        let digest = manifest
            .content_hash_for(BINARY_NAME)
            .context("import payload manifest omitted its required SHA-256")?
            .to_string();
        let commit = match req.commit {
            Some(commit) => commit.to_owned(),
            None => {
                commit_beside(req.executable, &digest).unwrap_or_else(|| UNKNOWN_COMMIT.to_owned())
            }
        };
        task.detail(format!("commit: {commit}"));
        let mut existing = store.instances_of(&id)?;
        existing.sort_by_key(|record| record.instance);
        let reused = existing
            .iter()
            .rev()
            .find(|record| {
                record.payload_sha256.as_deref() == Some(digest.as_str())
                    && record.commit == commit
                    && record.profile == req.profile
                    && placer::installed_files_match(store, record)
            })
            .cloned();
        task.detail(format!(
            "version: {id}; existing instances: {}",
            existing.len()
        ));
        Ok(ObservedPayload {
            id,
            dist,
            manifest,
            digest,
            commit,
            existing,
            reused,
        })
    })?;
    let ObservedPayload {
        id,
        dist,
        manifest,
        digest,
        commit,
        existing,
        reused,
    } = observed;
    // Kept as a CLI compatibility flag. Remote version labels are mutable;
    // every distinct payload is now admitted as a fresh immutable local #N.
    let _ = req.replace_candidate;

    if let Some(record) = reused {
        let instance_dir = store.instance_dir(&id, record.instance);
        let copying = progress.task("Copying import payload");
        copying.detail(format!("reusing {}", record.selector()));
        copying.skip("matching verified payload already installed");
        let recording = progress.task("Recording imported instance");
        recording.skip("inventory unchanged");
        ctx.summary(&format!(
            "{} payload already imported — reused",
            record.selector()
        ));
        return Ok(ImportOutcome {
            record,
            home: instance_dir,
            reused: true,
        });
    }

    let instance = observed_stage(&progress, "Copying import payload", |task| {
        task.detail(format!("destination version: {id}"));
        let instance = store.alloc_instance()?;
        let previous = existing.last().and_then(|record| {
            let dir = store.instance_dir(&id, record.instance);
            placer::read_manifest(&dir).map(|manifest| (dir, manifest))
        });
        let previous_ref = previous
            .as_ref()
            .map(|(dir, manifest)| (dir.as_path(), manifest));
        placer::place(store, &id, instance, &dist, &manifest, previous_ref)?;
        task.detail(format!("instance: {id}#{instance}"));
        Ok(instance)
    })?;

    let record = InstallRecord {
        kind: Kind::Tag,
        id: id.id.clone(),
        instance,
        commit,
        toolchain: "prebuilt".to_string(),
        profile: req.profile,
        installed_at: req.now.to_string(),
        origin: Origin::Binary,
        source_path: None,
        payload_sha256: Some(digest),
        distribution_manifest_sha256: None,
    };
    observed_stage(&progress, "Recording imported instance", |task| {
        task.detail(format!("selector: {}", record.selector()));
        store.record_install(record.clone())?;
        Ok(())
    })?;

    let instance_dir = store.instance_dir(&id, instance);
    ctx.created(&instance_dir.display().to_string());
    ctx.summary(&format!("imported {id}#{instance}"));
    Ok(ImportOutcome {
        record,
        home: instance_dir,
        reused: false,
    })
}

fn observed_stage<T>(
    progress: &Progress,
    label: &str,
    run: impl FnOnce(&ProgressTask) -> Result<T>,
) -> Result<T> {
    let task = progress.task(label);
    match run(&task) {
        Ok(value) => {
            task.finish();
            Ok(value)
        }
        Err(error) => {
            task.fail(error.to_string());
            Err(error)
        }
    }
}

fn parse_tag(raw: &str) -> Result<String> {
    let tag = raw.trim();
    if tag.starts_with('v') || semver::Version::parse(tag).is_err() {
        bail!("invalid import tag `{raw}`; expected a semantic version such as `1.2.3`");
    }
    Ok(tag.to_string())
}

/// The commit that built a hand-downloaded payload, read from the
/// `DISTRIBUTION.json` the same release wrote beside its executables.
///
/// vibe embeds no build commit in the executable — `vibe --version` prints
/// `CARGO_PKG_VERSION` and nothing else — so a bundle extracted by hand
/// carries its provenance only in that manifest. It is read under the
/// independent parser limit and believed only when it declares a `vibe`
/// component whose digest is exactly these bytes: a manifest left behind by
/// another release in the same directory then cannot relabel this payload.
/// `payload_digest` is lower-case hex; the manifest spells the same value
/// `sha256:<hex>`.
fn commit_beside(executable: &Path, payload_digest: &str) -> Option<String> {
    let manifest = executable.parent()?.join(DISTRIBUTION_MANIFEST_FILENAME);
    let bytes = read_bounded(&manifest, DISTRIBUTION_MANIFEST_MAX_BYTES)?;
    let manifest = BundleDistributionManifest::from_json_slice(&bytes).ok()?;
    let describes_payload = manifest.components.iter().any(|component| {
        component.name == DistributionComponentName::Vibe
            && component.digest.strip_prefix("sha256:") == Some(payload_digest)
    });
    describes_payload.then_some(manifest.source_commit)
}

/// Read a small regular file whole, or nothing at all: a missing, oversized,
/// linked or unreadable file is simply an absent observation here.
fn read_bounded(path: &Path, maximum: u64) -> Option<Vec<u8>> {
    let (mut file, metadata) = open_regular_no_follow(path).ok()?;
    if metadata.len() > maximum {
        return None;
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.read_to_end(&mut bytes).ok()?;
    Some(bytes)
}

fn ensure_payload_file(path: &Path) -> Result<()> {
    let (_, metadata) = open_regular_no_follow(path)
        .with_context(|| format!("reading import payload `{}`", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            bail!("import payload `{}` is not executable", path.display());
        }
    }
    #[cfg(not(unix))]
    let _ = metadata;
    Ok(())
}

#[cfg(test)]
mod tests {
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

    /// A `DISTRIBUTION.json` that declares `payload` as its `vibe`
    /// component, as a bundle extracted by hand leaves beside the binary.
    fn write_manifest_beside(payload: &Path, commit: &str, bytes: &[u8]) {
        use vibe_publish::release_manifest::{
            DistributionComponent, DistributionComponentName, DistributionSourceArchive,
        };
        let digest = |bytes: &[u8]| format!("sha256:{:x}", sha2::Sha256::digest(bytes));
        let source = b"source-archive";
        let manifest = BundleDistributionManifest::new(
            semver::Version::new(1, 2, 3),
            commit,
            crate::commands::vvm::bundle::current_target().unwrap(),
            vec![
                DistributionComponent {
                    name: DistributionComponentName::Vibe,
                    path: BINARY_NAME.to_string(),
                    size: bytes.len() as u64,
                    digest: digest(bytes),
                },
                DistributionComponent {
                    name: DistributionComponentName::VibeAppProxy,
                    path: crate::commands::vvm::store::APP_PROXY_BINARY_NAME.to_string(),
                    size: 1,
                    digest: digest(b"p"),
                },
                DistributionComponent {
                    name: DistributionComponentName::VibeIndex,
                    path: crate::commands::vvm::store::INDEX_BINARY_NAME.to_string(),
                    size: 1,
                    digest: digest(b"i"),
                },
            ],
            DistributionSourceArchive {
                path: vibe_publish::release_manifest::DISTRIBUTION_SOURCE_ARCHIVE_FILENAME
                    .to_string(),
                size: source.len() as u64,
                digest: digest(source),
                tree_oid: "a".repeat(40),
            },
        )
        .unwrap();
        fs::write(
            payload
                .parent()
                .unwrap()
                .join(DISTRIBUTION_MANIFEST_FILENAME),
            manifest.to_json_bytes().unwrap(),
        )
        .unwrap();
    }

    const RELEASE_COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

    /// A hand-downloaded payload imported without `--commit` takes its
    /// commit from the `DISTRIBUTION.json` the same release wrote beside
    /// it — the self-import of `vibe self apply` passes no commit at all.
    #[test]
    fn a_hand_downloaded_payload_takes_its_commit_from_the_manifest_beside_it() {
        let tmp = tempfile::tempdir().unwrap();
        let store = VersionStore::new(tmp.path().join("opt"));
        let payload = tmp.path().join("vibe");
        let bytes = b"hand-downloaded";
        write_payload(&payload, bytes);
        write_manifest_beside(&payload, RELEASE_COMMIT, bytes);

        let mut request = request(&payload, false);
        request.commit = None;
        let outcome = perform_import(&quiet(), &store, &request).unwrap();
        assert_eq!(outcome.record.commit, RELEASE_COMMIT);

        // The same payload and the same manifest are one instance, not two.
        let again = perform_import(&quiet(), &store, &request).unwrap();
        assert!(again.reused);
        assert_eq!(
            store
                .instances_of(&outcome.record.version_id())
                .unwrap()
                .len(),
            1
        );
    }

    /// An explicit `--commit` still wins, and a manifest that describes
    /// other bytes is not this payload's provenance.
    #[test]
    fn an_explicit_commit_wins_and_a_foreign_manifest_is_not_believed() {
        let tmp = tempfile::tempdir().unwrap();
        let store = VersionStore::new(tmp.path().join("opt"));
        let payload = tmp.path().join("vibe");
        write_payload(&payload, b"hand-downloaded");
        write_manifest_beside(&payload, RELEASE_COMMIT, b"a-different-release");

        let explicit = request(&payload, false);
        let mut anonymous = request(&payload, false);
        anonymous.commit = None;
        let unknown = perform_import(&quiet(), &store, &anonymous).unwrap();
        assert_eq!(
            unknown.record.commit, UNKNOWN_COMMIT,
            "a manifest whose `vibe` digest is other bytes says nothing about this payload"
        );

        let explicit = perform_import(&quiet(), &store, &explicit).unwrap();
        assert_eq!(explicit.record.commit, "abc1234");
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
}
