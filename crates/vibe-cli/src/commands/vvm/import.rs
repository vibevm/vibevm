//! Local import of a ready-built `vibe` executable into the immutable VVM
//! inventory. The path is local-only: no resolver, network, or signature
//! machinery participates.

use std::path::Path;

use anyhow::{Context, Result, bail};
use vibe_core::progress::{Progress, ProgressTask};

use super::model::{InstallRecord, Kind, Origin, Profile, VersionId};
use super::placer;
use super::store::{BINARY_NAME, VersionStore, open_regular_no_follow};
use crate::output;

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

pub(crate) fn perform_import(
    ctx: &output::Context,
    store: &VersionStore,
    req: &ImportRequest<'_>,
) -> Result<ImportOutcome> {
    let progress = ctx.progress();
    let (id, dist, manifest, digest, existing, reused) =
        observed_stage(&progress, "Verifying local import payload", |task| {
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
            let mut existing = store.instances_of(&id)?;
            existing.sort_by_key(|record| record.instance);
            let reused = existing
                .iter()
                .rev()
                .find(|record| {
                    record.payload_sha256.as_deref() == Some(digest.as_str())
                        && record.commit == req.commit.unwrap_or("unknown")
                        && record.profile == req.profile
                        && placer::installed_files_match(store, record)
                })
                .cloned();
            task.detail(format!(
                "version: {id}; existing instances: {}",
                existing.len()
            ));
            Ok((id, dist, manifest, digest, existing, reused))
        })?;
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
        commit: req.commit.unwrap_or("unknown").to_string(),
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
