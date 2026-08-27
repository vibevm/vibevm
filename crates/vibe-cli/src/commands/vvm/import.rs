//! Local import of a ready-built `vibe` executable into the immutable VVM
//! inventory. The path is local-only: no resolver, network, or signature
//! machinery participates.

use std::fs;
use std::io::{BufReader, Read};
use std::path::Path;

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};

use super::env;
use super::install::InstallLock;
use super::model::{InstallRecord, Kind, Origin, Profile, VersionId};
use super::placer;
use super::store::{BINARY_NAME, VersionStore};
use crate::output;

pub(crate) struct ImportRequest<'a> {
    pub executable: &'a Path,
    pub tag: &'a str,
    pub commit: Option<&'a str>,
    pub profile: Profile,
    pub activate: bool,
    pub replace_candidate: bool,
    pub now: &'a str,
}

pub(crate) fn perform_import(
    ctx: &output::Context,
    store: &VersionStore,
    req: &ImportRequest<'_>,
) -> Result<()> {
    let _lock = InstallLock::acquire(store)?;
    let tag = parse_tag(req.tag)?;
    ensure_payload_file(req.executable)?;
    let digest = sha256_file(req.executable)?;
    let id = VersionId::new(Kind::Tag, tag);

    let mut existing = store.instances_of(&id)?;
    existing.sort_by_key(|record| record.instance);
    if let Some(record) = existing
        .iter()
        .rev()
        .find(|record| record.payload_sha256.as_deref() == Some(digest.as_str()))
    {
        let instance_dir = store.instance_dir(&id, record.instance);
        activate_if_requested(store, &instance_dir, req.activate)?;
        ctx.summary(&format!(
            "{id} payload already imported (instance {}) — reused{}",
            record.instance,
            if req.activate { " and activated" } else { "" }
        ));
        return Ok(());
    }

    if !existing.is_empty() && !req.replace_candidate {
        bail!(
            "refusing to replace immutable release `{id}` with a different SHA256 payload; \
             pass `--replace-candidate` to preserve the old instance and add a new inspection candidate"
        );
    }

    let instance = store.alloc_instance()?;
    let dist = vec![(req.executable.to_path_buf(), BINARY_NAME.to_string())];
    let manifest = placer::manifest_for(&dist)?;
    let previous = existing.last().and_then(|record| {
        let dir = store.instance_dir(&id, record.instance);
        placer::read_manifest(&dir).map(|manifest| (dir, manifest))
    });
    let previous_ref = previous
        .as_ref()
        .map(|(dir, manifest)| (dir.as_path(), manifest));
    placer::place(store, &id, instance, &dist, &manifest, previous_ref)?;

    store.record_install(InstallRecord {
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
    })?;

    let instance_dir = store.instance_dir(&id, instance);
    activate_if_requested(store, &instance_dir, req.activate)?;
    ctx.created(&instance_dir.display().to_string());
    ctx.summary(&format!(
        "imported {id} (instance {instance}){}",
        if req.activate { " — active" } else { "" }
    ));
    Ok(())
}

fn parse_tag(raw: &str) -> Result<String> {
    let tag = raw.trim();
    if tag.starts_with('v') || semver::Version::parse(tag).is_err() {
        bail!("invalid import tag `{raw}`; expected a semantic version such as `1.2.3`");
    }
    Ok(tag.to_string())
}

fn ensure_payload_file(path: &Path) -> Result<()> {
    let metadata = fs::metadata(path)
        .with_context(|| format!("reading import payload `{}`", path.display()))?;
    if !metadata.is_file() {
        bail!("import payload `{}` is not a file", path.display());
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String> {
    let file = fs::File::open(path)
        .with_context(|| format!("opening import payload `{}`", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .with_context(|| format!("hashing import payload `{}`", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn activate_if_requested(store: &VersionStore, instance_dir: &Path, activate: bool) -> Result<()> {
    if activate {
        store.write_current(instance_dir)?;
        env::write_shims(&store.shim_dir())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quiet() -> output::Context {
        output::Context::from_flags(true, false, None, true, crate::cli::AgentModeArg::Auto)
    }

    fn request<'a>(
        executable: &'a Path,
        replace_candidate: bool,
        activate: bool,
    ) -> ImportRequest<'a> {
        ImportRequest {
            executable,
            tag: "1.2.3",
            commit: Some("abc1234"),
            profile: Profile::Release,
            activate,
            replace_candidate,
            now: "2026-08-20T00:00:00Z",
        }
    }

    #[test]
    fn import_reuses_same_hash_and_refuses_a_different_immutable_payload() {
        let tmp = tempfile::tempdir().unwrap();
        let store = VersionStore::new(tmp.path().join("opt"));
        let payload = tmp.path().join("ready-vibe.exe");
        fs::write(&payload, b"candidate-one").unwrap();

        perform_import(&quiet(), &store, &request(&payload, false, false)).unwrap();
        assert!(
            store.read_current().is_none(),
            "import is inactive by default"
        );

        perform_import(&quiet(), &store, &request(&payload, false, false)).unwrap();
        let id = VersionId::new(Kind::Tag, "1.2.3");
        assert_eq!(store.instances_of(&id).unwrap().len(), 1);
        assert_eq!(store.load_state().unwrap().next_instance, 2);

        fs::write(&payload, b"candidate-two").unwrap();
        let error = perform_import(&quiet(), &store, &request(&payload, false, false))
            .unwrap_err()
            .to_string();
        assert!(error.contains("refusing to replace immutable release"));
        assert_eq!(store.instances_of(&id).unwrap().len(), 1);
    }

    #[test]
    fn replace_candidate_preserves_old_instance_and_use_writes_current_and_shims() {
        let tmp = tempfile::tempdir().unwrap();
        let store = VersionStore::new(tmp.path().join("opt"));
        let payload = tmp.path().join("ready-vibe.exe");
        fs::write(&payload, b"candidate-one").unwrap();
        perform_import(&quiet(), &store, &request(&payload, false, false)).unwrap();

        fs::write(&payload, b"candidate-two").unwrap();
        perform_import(&quiet(), &store, &request(&payload, true, true)).unwrap();

        let id = VersionId::new(Kind::Tag, "1.2.3");
        assert_eq!(store.instances_of(&id).unwrap().len(), 2);
        assert_eq!(store.read_current().unwrap(), store.instance_dir(&id, 2));
        assert!(store.shim_dir().join("vibe").is_file());
        let shim = fs::read_to_string(store.shim_dir().join("vibe")).unwrap();
        assert!(shim.contains("vibe self use"));
        assert!(!shim.contains("vibe man use"));
        if cfg!(windows) {
            assert!(store.shim_dir().join("vibe.cmd").is_file());
        }
    }
}
