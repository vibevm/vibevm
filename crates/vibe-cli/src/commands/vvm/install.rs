//! The install pipeline orchestration (PROP-019 §2.7): lock, build, place
//! the distribution into a new instance by diff-copy (skipping when nothing
//! changed), record provenance, and flip `current`.
//!
//! The terminal apps (vibeterm, vibeframe) and the GUI launchers
//! (vibe-launcher) used to be packaged into the instance alongside the
//! `vibe` binary; they have moved to a separate products repo
//! (`vibevm-term`) and now publish themselves to `PATH` through their own
//! version-manager. The install pipeline here handles the essential `vibe`
//! and `vibe-index` binaries — terminal apps resolve separately through
//! `$VIBEVM_<APP>` → the active instance's packaged `<app>/` (back-compat)
//! → `PATH` lookup, with an in-place fallback for `vibe tree`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#build");

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use specmark::spec;
use vibe_core::progress::Progress;

use super::builder::{Builder, ResolvedVersion};
use super::model::{InstallRecord, Origin, Profile, VersionId};
use super::placer::{self, Manifest};
use super::store::{BINARY_NAME, INDEX_BINARY_NAME, VersionStore};
use crate::output;

/// A best-effort store lock so source installs and local imports do not race.
pub(crate) struct InstallLock {
    _file: fs::File,
}

impl InstallLock {
    pub(crate) fn acquire(store: &VersionStore) -> Result<InstallLock> {
        let dir = store.data_dir();
        store.guard_mutation_path(&dir)?;
        fs::create_dir_all(&dir).with_context(|| format!("creating `{}`", dir.display()))?;
        let path = dir.join(".install.lock");
        store.guard_mutation_path(&path)?;
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .with_context(|| format!("opening mutation lock `{}`", path.display()))?;
        match file.try_lock() {
            Ok(()) => {
                let lock = InstallLock { _file: file };
                store.recover_activation_locked()?;
                Ok(lock)
            }
            Err(fs::TryLockError::WouldBlock) => bail!(
                "another mutating `vibe self` command is in progress \
                 (the OS lock releases automatically when that process exits)"
            ),
            Err(fs::TryLockError::Error(e)) => {
                Err(e).with_context(|| format!("locking `{}`", path.display()))
            }
        }
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

pub(crate) struct InstallOutcome {
    pub record: InstallRecord,
    pub home: PathBuf,
    pub reused: bool,
}

/// Build and publish a resolved version into a fresh instance (PROP-019
/// §2.7, §2.15). If the build is byte-identical to the latest instance and
/// `--force` is absent, no new instance is made — `current` just points at
/// it (the dedup-skip).
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-019#build")]
pub(crate) fn perform_install(
    ctx: &output::Context,
    store: &VersionStore,
    source_root: &std::path::Path,
    req: &InstallRequest,
    builder: &dyn Builder,
    progress: &Progress,
) -> Result<InstallOutcome> {
    let id = &req.resolved.id;

    ctx.step(&format!(
        "building {id} ({}) from {}",
        req.profile.as_str(),
        source_root.display()
    ));
    store.guard_mutation_tree(&store.build_dir())?;
    let out = builder.build(source_root, &store.build_dir(), req.profile)?;
    let dist = vec![
        (out.binary.clone(), format!("bin/{BINARY_NAME}")),
        (out.index_binary.clone(), format!("bin/{INDEX_BINARY_NAME}")),
    ];
    let placement = progress.task("Placing essential binaries");
    placement.set_progress(0, Some(dist.len() as u64), "files");
    let manifest = match placer::manifest_for(&dist) {
        Ok(manifest) => manifest,
        Err(error) => {
            placement.fail("distribution inspection failed");
            return Err(error.into());
        }
    };

    let prev = latest_instance(store, id)?;
    if let Some((prev_dir, prev_man, prev_rec)) = &prev
        && !req.force
        && placer::matches_on_disk(store, &manifest, prev_man, prev_dir)
        && same_provenance(prev_rec, req, &out.toolchain)
    {
        ctx.summary(&format!(
            "{id}#{} already up to date — reused",
            prev_rec.instance,
        ));
        placement.skip("installed files are unchanged");
        Ok(InstallOutcome {
            record: prev_rec.clone(),
            home: prev_dir.clone(),
            reused: true,
        })
    } else {
        let instance = store.alloc_instance()?;
        let prev_ref = prev.as_ref().map(|(dir, man, _)| (dir.as_path(), man));
        if let Err(error) = placer::place(store, id, instance, &dist, &manifest, prev_ref) {
            placement.fail("placement failed");
            return Err(error.into());
        }
        placement.set_progress(dist.len() as u64, Some(dist.len() as u64), "files");

        let record = InstallRecord {
            kind: id.kind,
            id: id.id.clone(),
            instance,
            commit: req.resolved.commit.clone(),
            toolchain: out.toolchain,
            profile: req.profile,
            installed_at: req.now.to_string(),
            origin: req.origin,
            source_path: req.source_path.clone(),
            payload_sha256: None,
            distribution_manifest_sha256: None,
        };
        if let Err(error) = store.record_install(record.clone()) {
            placement.fail("recording the installed version failed");
            return Err(error.into());
        }

        let inst_dir = store.instance_dir(id, instance);
        ctx.created(&inst_dir.display().to_string());
        ctx.summary(&format!("installed {id}#{instance}"));
        placement.finish();
        Ok(InstallOutcome {
            record,
            home: inst_dir,
            reused: false,
        })
    }
}

fn same_provenance(record: &InstallRecord, req: &InstallRequest<'_>, toolchain: &str) -> bool {
    record.commit == req.resolved.commit
        && record.toolchain == toolchain
        && record.profile == req.profile
        && record.origin == req.origin
        && record.source_path == req.source_path
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
    use crate::commands::vvm::builder::BuildOutput;
    use crate::commands::vvm::env::{self, EnvPersister, Persisted};
    use crate::commands::vvm::model::Kind;
    use specmark::verifies;

    /// A builder that writes a chosen byte string into the managed target dir
    /// (where `out.binary` resolves) instead of compiling.
    struct FakeBuilder {
        content: Vec<u8>,
    }

    impl Builder for FakeBuilder {
        fn build(
            &self,
            _root: &std::path::Path,
            target_dir: &std::path::Path,
            profile: Profile,
        ) -> Result<BuildOutput> {
            let dir = target_dir.join(profile.target_subdir());
            fs::create_dir_all(&dir).unwrap();
            let binary = dir.join(BINARY_NAME);
            let index_binary = dir.join(INDEX_BINARY_NAME);
            fs::write(&binary, &self.content).unwrap();
            fs::write(&index_binary, &self.content).unwrap();
            #[cfg(unix)]
            for path in [&binary, &index_binary] {
                use std::os::unix::fs::PermissionsExt;
                let mut permissions = fs::metadata(path).unwrap().permissions();
                permissions.set_mode(0o755);
                fs::set_permissions(path, permissions).unwrap();
            }
            Ok(BuildOutput {
                binary,
                index_binary,
                toolchain: "rustc 0.0.0-fake".into(),
            })
        }
    }

    fn quiet() -> output::Context {
        output::Context::from_flags(true, false, None, true, crate::cli::AgentModeArg::Auto)
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

    struct FakePersister;

    impl EnvPersister for FakePersister {
        fn set_vibevm_home(&self, _home: &std::path::Path) -> Result<Persisted> {
            Ok(Persisted::Changed)
        }

        fn ensure_on_path(&self, _dir: &std::path::Path) -> Result<Persisted> {
            Ok(Persisted::Changed)
        }

        fn activation_hint(&self) -> String {
            "test".into()
        }
    }

    fn perform_install_and_activate(
        ctx: &output::Context,
        store: &VersionStore,
        source_root: &std::path::Path,
        req: &InstallRequest<'_>,
        builder: &dyn Builder,
    ) -> Result<InstallOutcome> {
        let outcome = perform_install(ctx, store, source_root, req, builder, &Progress::default())?;
        env::activate_instance(store, &outcome.home, &FakePersister)?;
        Ok(outcome)
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#instances", r = 2)]
    fn install_makes_instance_skips_unchanged_and_forces_rebuild() {
        let tmp = tempfile::tempdir().unwrap();
        let store = VersionStore::new(tmp.path());
        let src = tempfile::tempdir().unwrap();
        let resolved = ResolvedVersion {
            id: VersionId::new(Kind::Branch, "main"),
            commit: "deadbeefcafe".into(),
        };

        perform_install_and_activate(
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
        assert!(inst1.join("bin").join(BINARY_NAME).is_file());
        assert!(inst1.join("bin").join(INDEX_BINARY_NAME).is_file());
        assert_eq!(store.read_current().unwrap().unwrap(), inst1);
        assert_eq!(store.instances_of(&resolved.id).unwrap().len(), 1);

        // Identical bytes → no new instance (dedup-skip).
        perform_install_and_activate(
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

        // A modified immutable instance is never reactivated as a dedup hit.
        fs::write(inst1.join("bin").join(BINARY_NAME), b"tampered").unwrap();
        perform_install_and_activate(
            &quiet(),
            &store,
            src.path(),
            &req(&resolved, false, "t3"),
            &FakeBuilder {
                content: b"v1".to_vec(),
            },
        )
        .unwrap();
        assert_eq!(store.instances_of(&resolved.id).unwrap().len(), 2);
        assert_eq!(fs::read(store.binary_path(&resolved.id, 2)).unwrap(), b"v1");
        assert_eq!(
            fs::read(inst1.join("bin").join(BINARY_NAME)).unwrap(),
            b"tampered"
        );

        // A mutable remote label at a new commit is a distinct local instance
        // even when its payload bytes happen to be identical.
        let moved = ResolvedVersion {
            id: resolved.id.clone(),
            commit: "feedbeefcafe".into(),
        };
        perform_install_and_activate(
            &quiet(),
            &store,
            src.path(),
            &req(&moved, false, "t4"),
            &FakeBuilder {
                content: b"v1".to_vec(),
            },
        )
        .unwrap();
        assert_eq!(store.instances_of(&resolved.id).unwrap().len(), 3);

        // Changed bytes → a new instance, current advances.
        perform_install_and_activate(
            &quiet(),
            &store,
            src.path(),
            &req(&resolved, false, "t5"),
            &FakeBuilder {
                content: b"v2".to_vec(),
            },
        )
        .unwrap();
        assert_eq!(store.instances_of(&resolved.id).unwrap().len(), 4);
        assert_eq!(
            store.read_current().unwrap().unwrap(),
            store.instance_dir(&resolved.id, 4)
        );

        // --force on identical bytes → still a new instance.
        perform_install_and_activate(
            &quiet(),
            &store,
            src.path(),
            &req(&resolved, true, "t6"),
            &FakeBuilder {
                content: b"v2".to_vec(),
            },
        )
        .unwrap();
        assert_eq!(store.instances_of(&resolved.id).unwrap().len(), 5);
    }

    #[test]
    fn mutation_lock_refuses_parallel_writer_and_recovers_after_release() {
        let tmp = tempfile::tempdir().unwrap();
        let store = VersionStore::new(tmp.path());
        let first = InstallLock::acquire(&store).unwrap();
        assert!(InstallLock::acquire(&store).is_err());
        drop(first);
        InstallLock::acquire(&store).unwrap();
    }

    #[test]
    fn linked_build_cache_is_rejected_before_builder_runs() {
        use std::cell::Cell;

        struct NeverBuilder(Cell<bool>);
        impl Builder for NeverBuilder {
            fn build(
                &self,
                _root: &std::path::Path,
                _target: &std::path::Path,
                _profile: Profile,
            ) -> Result<BuildOutput> {
                self.0.set(true);
                anyhow::bail!("builder should not run")
            }
        }

        let temp = tempfile::tempdir().unwrap();
        let store = VersionStore::new(temp.path().join("opt"));
        fs::create_dir_all(store.data_dir()).unwrap();
        let outside = temp.path().join("outside");
        fs::create_dir_all(&outside).unwrap();
        fs::create_dir_all(store.build_dir().join("debug")).unwrap();
        make_dir_redirect(&outside, &store.build_dir().join("debug/deps"));
        let resolved = ResolvedVersion {
            id: VersionId::new(Kind::Branch, "main"),
            commit: "c".repeat(40),
        };
        let builder = NeverBuilder(Cell::new(false));
        assert!(
            perform_install(
                &quiet(),
                &store,
                temp.path(),
                &req(&resolved, false, "now"),
                &builder,
                &Progress::default(),
            )
            .is_err()
        );
        assert!(!builder.0.get());
        assert!(fs::read_dir(outside).unwrap().next().is_none());
    }

    #[cfg(unix)]
    fn make_dir_redirect(target: &std::path::Path, link: &std::path::Path) {
        std::os::unix::fs::symlink(target, link).unwrap();
    }

    #[cfg(windows)]
    fn make_dir_redirect(target: &std::path::Path, link: &std::path::Path) {
        use std::os::windows::process::CommandExt;
        let command = format!("mklink /J \"{}\" \"{}\"", link.display(), target.display());
        let status = std::process::Command::new("cmd")
            .args(["/d", "/c"])
            .raw_arg(command)
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(status.success(), "creating test junction failed");
    }
}
