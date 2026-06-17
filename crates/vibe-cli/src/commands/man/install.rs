//! The install pipeline orchestration (PROP-019 §2.7): lock, build, place
//! the distribution into a new instance by diff-copy (skipping when nothing
//! changed), record provenance, and flip `current`.

specmark::scope!("spec://vibevm/common/PROP-019#build");

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use specmark::spec;

use super::builder::{Builder, ResolvedVersion};
use super::model::{InstallRecord, Origin, Profile, VersionId};
use super::placer::{self, Manifest};
use super::store::{BINARY_NAME, VersionStore};
use crate::output;

/// A best-effort install lock so two installs do not race (PROP-019 §2.7).
struct InstallLock {
    path: PathBuf,
}

impl InstallLock {
    fn acquire(store: &VersionStore) -> Result<InstallLock> {
        let dir = store.data_dir();
        fs::create_dir_all(&dir).with_context(|| format!("creating `{}`", dir.display()))?;
        let path = dir.join(".install.lock");
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(_) => Ok(InstallLock { path }),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => bail!(
                "another `vibe man install` is in progress (remove `{}` if it is stale)",
                path.display()
            ),
            Err(e) => Err(e).with_context(|| format!("creating lock `{}`", path.display())),
        }
    }
}

impl Drop for InstallLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Parameters for [`perform_install`].
pub(crate) struct InstallRequest<'a> {
    pub resolved: &'a ResolvedVersion,
    pub profile: Profile,
    pub force: bool,
    /// RFC3339 timestamp, stamped at the composition layer.
    pub now: &'a str,
    pub origin: Origin,
    pub source_path: Option<String>,
}

/// Build and publish a resolved version into a fresh instance (PROP-019
/// §2.7, §2.15). If the build is byte-identical to the latest instance and
/// `--force` is absent, no new instance is made — `current` just points at
/// it (the dedup-skip).
#[spec(implements = "spec://vibevm/common/PROP-019#build")]
pub(crate) fn perform_install(
    ctx: &output::Context,
    store: &VersionStore,
    source_root: &Path,
    req: &InstallRequest,
    builder: &dyn Builder,
) -> Result<()> {
    let _lock = InstallLock::acquire(store)?;
    let id = &req.resolved.id;

    ctx.step(&format!(
        "building {id} ({}) from {}",
        req.profile.as_str(),
        source_root.display()
    ));
    let out = builder.build(source_root, &store.build_dir(), req.profile)?;
    let dist = vec![(out.binary.clone(), BINARY_NAME.to_string())];
    let manifest = placer::manifest_for(&dist)?;

    let prev = latest_instance(store, id)?;
    if let Some((prev_dir, prev_man, prev_rec)) = &prev
        && !req.force
        && placer::matches(&manifest, prev_man)
    {
        store.write_current(prev_dir)?;
        ctx.summary(&format!(
            "{id} already up to date (instance {}) — active",
            prev_rec.instance
        ));
        return Ok(());
    }

    let instance = store.alloc_instance()?;
    let prev_ref = prev.as_ref().map(|(dir, man, _)| (dir.as_path(), man));
    placer::place(store, id, instance, &dist, &manifest, prev_ref)?;

    store.record_install(InstallRecord {
        kind: id.kind,
        id: id.id.clone(),
        instance,
        commit: req.resolved.commit.clone(),
        toolchain: out.toolchain,
        profile: req.profile.as_str().to_string(),
        installed_at: req.now.to_string(),
        origin: req.origin,
        source_path: req.source_path.clone(),
    })?;

    let inst_dir = store.instance_dir(id, instance);
    store.write_current(&inst_dir)?;
    ctx.created(&inst_dir.display().to_string());
    ctx.summary(&format!("installed {id} (instance {instance}) — active"));
    Ok(())
}

/// The newest existing instance of `id` plus its manifest, for diff-copy.
fn latest_instance(
    store: &VersionStore,
    id: &VersionId,
) -> Result<Option<(PathBuf, Manifest, InstallRecord)>> {
    let mut insts = store.instances_of(id)?;
    insts.sort_by_key(|r| r.instance);
    let Some(rec) = insts.pop() else {
        return Ok(None);
    };
    let dir = store.instance_dir(id, rec.instance);
    let manifest = placer::read_manifest(&dir).unwrap_or_default();
    Ok(Some((dir, manifest, rec)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::man::builder::BuildOutput;
    use crate::commands::man::model::Kind;
    use specmark::verifies;

    /// A builder that writes a chosen byte string into the managed target dir
    /// (where `out.binary` resolves) instead of compiling.
    struct FakeBuilder {
        content: Vec<u8>,
    }

    impl Builder for FakeBuilder {
        fn build(&self, _root: &Path, target_dir: &Path, profile: Profile) -> Result<BuildOutput> {
            let dir = target_dir.join(profile.target_subdir());
            fs::create_dir_all(&dir).unwrap();
            let binary = dir.join(BINARY_NAME);
            fs::write(&binary, &self.content).unwrap();
            Ok(BuildOutput {
                binary,
                toolchain: "rustc 0.0.0-fake".into(),
            })
        }
    }

    fn quiet() -> output::Context {
        output::Context::from_flags(true, false, None, true)
    }

    fn req<'a>(resolved: &'a ResolvedVersion, force: bool, now: &'a str) -> InstallRequest<'a> {
        InstallRequest {
            resolved,
            profile: Profile::Debug,
            force,
            now,
            origin: Origin::Managed,
            source_path: None,
        }
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-019#instances", r = 1)]
    fn install_makes_instance_skips_unchanged_and_forces_rebuild() {
        let tmp = tempfile::tempdir().unwrap();
        let store = VersionStore::new(tmp.path());
        let src = tempfile::tempdir().unwrap();
        let resolved = ResolvedVersion {
            id: VersionId::new(Kind::Branch, "main"),
            commit: "deadbeefcafe".into(),
        };

        perform_install(
            &quiet(),
            &store,
            src.path(),
            &req(&resolved, false, "t1"),
            &FakeBuilder {
                content: b"v1".to_vec(),
            },
        )
        .unwrap();
        let inst1 = store.instance_dir(&resolved.id, 1);
        assert!(inst1.join(BINARY_NAME).is_file());
        assert_eq!(store.read_current().unwrap(), inst1);
        assert_eq!(store.instances_of(&resolved.id).unwrap().len(), 1);

        // Identical bytes → no new instance (dedup-skip).
        perform_install(
            &quiet(),
            &store,
            src.path(),
            &req(&resolved, false, "t2"),
            &FakeBuilder {
                content: b"v1".to_vec(),
            },
        )
        .unwrap();
        assert_eq!(store.instances_of(&resolved.id).unwrap().len(), 1);

        // Changed bytes → a new instance, current advances.
        perform_install(
            &quiet(),
            &store,
            src.path(),
            &req(&resolved, false, "t3"),
            &FakeBuilder {
                content: b"v2".to_vec(),
            },
        )
        .unwrap();
        assert_eq!(store.instances_of(&resolved.id).unwrap().len(), 2);
        assert_eq!(
            store.read_current().unwrap(),
            store.instance_dir(&resolved.id, 2)
        );

        // --force on identical bytes → still a new instance.
        perform_install(
            &quiet(),
            &store,
            src.path(),
            &req(&resolved, true, "t4"),
            &FakeBuilder {
                content: b"v2".to_vec(),
            },
        )
        .unwrap();
        assert_eq!(store.instances_of(&resolved.id).unwrap().len(), 3);
    }
}
