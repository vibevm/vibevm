//! Deterministic rollback behavior without production failpoints.
use super::*;
use vibe_core::manifest::LockedPackage;
struct Fixture {
    dir: tempfile::TempDir,
    original_lock: Vec<u8>,
    original_manifest: Vec<u8>,
    slot: PathBuf,
    plan: Plan,
    pruned: Lockfile,
    manifest: Manifest,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let row: LockedPackage = toml::from_str(
            r#"
            kind = "flow"
            group = "org.test"
            name = "a"
            version = "1.0.0"
            source_url = "local/a"
            content_hash = "sha256:abc"
        "#,
        )
        .unwrap();
        let mut original = Lockfile::empty("test", "2026-01-01T00:00:00Z");
        original.packages.push(row.clone());
        original
            .meta
            .root_dependencies
            .push(row.as_package_ref().unwrap());
        original.write(dir.path().join("vibe.lock")).unwrap();
        let source = "[project]\nname = \"test\"\nversion = \"0.1.0\"\n[requires.packages]\n\"org.test/a\" = \"=1.0.0\"\n";
        fs::write(dir.path().join("vibe.toml"), source).unwrap();
        let slot = dir.path().join(super::super::plan::slot(&row));
        fs::create_dir_all(&slot).unwrap();
        fs::write(slot.join("payload"), "preserve me").unwrap();
        let manifest =
            Manifest::parse_str("[project]\nname = \"test\"\nversion = \"0.1.0\"\n").unwrap();
        let original_lock = fs::read(dir.path().join("vibe.lock")).unwrap();
        let original_manifest = fs::read(dir.path().join("vibe.toml")).unwrap();
        Self {
            dir,
            original_lock,
            original_manifest,
            slot,
            manifest,
            plan: Plan {
                removed: vec![row],
                removed_count: 1,
                roots: vec![],
                manifest_changed: true,
            },
            pruned: Lockfile::empty("test", "2026-01-01T00:00:00Z"),
        }
    }
}
#[test]
fn second_record_failure_restores_first_record_and_staged_slot() {
    let f = Fixture::new();
    let mut calls = 0;
    let result = apply_recording(
        f.dir.path(),
        f.dir.path(),
        &f.plan,
        &f.pruned,
        &f.manifest,
        |source, destination| {
            calls += 1;
            if calls == 2 {
                bail!("injected manifest record failure");
            }
            fs::rename(source, destination)?;
            Ok(())
        },
    );
    assert!(result.unwrap_err().to_string().contains("recording"));
    assert_eq!(
        fs::read(f.dir.path().join("vibe.lock")).unwrap(),
        f.original_lock
    );
    assert_eq!(
        fs::read(f.dir.path().join("vibe.toml")).unwrap(),
        f.original_manifest
    );
    assert_eq!(
        fs::read_to_string(f.slot.join("payload")).unwrap(),
        "preserve me"
    );
    assert!(!fs::read_dir(f.dir.path()).unwrap().any(|e| {
        e.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".vibe-uninstall-")
    }));
}
#[test]
fn incomplete_rollback_keeps_staged_payload_and_exact_recovery_location() {
    let f = Fixture::new();
    let mut calls = 0;
    let result = apply_recording(
        f.dir.path(),
        f.dir.path(),
        &f.plan,
        &f.pruned,
        &f.manifest,
        |source, destination| {
            calls += 1;
            if calls == 2 {
                fs::create_dir(&f.slot)?;
                fs::write(f.slot.join("new-owner-file"), "newer work")?;
                bail!("injected manifest record failure with occupied rollback slot");
            }
            fs::rename(source, destination)?;
            Ok(())
        },
    );
    let message = result.unwrap_err().to_string();
    assert!(message.contains("rollback incomplete"));
    let recovery = fs::read_dir(f.dir.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(".vibe-uninstall-")
        })
        .unwrap();
    assert!(message.contains(&recovery.display().to_string()));
    assert_eq!(
        fs::read_to_string(recovery.join("slot-0/payload")).unwrap(),
        "preserve me"
    );
    assert_eq!(
        fs::read_to_string(f.slot.join("new-owner-file")).unwrap(),
        "newer work"
    );
    assert_eq!(
        fs::read(f.dir.path().join("vibe.lock")).unwrap(),
        f.original_lock
    );
    assert_eq!(
        fs::read(f.dir.path().join("vibe.toml")).unwrap(),
        f.original_manifest
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-066#provenance")]
fn uninstall_prunes_project_record_and_report_conflict_restores_its_world() {
    let f = Fixture::new();
    let sidecar = f.dir.path().join("vibe.lock.provenance");
    fs::write(&sidecar,"[[package]]\npackage='org.test/a'\nversion='1.0.0'\nfetched_from='https://project.example/a'\n").unwrap();
    apply(f.dir.path(), f.dir.path(), &f.plan, &f.pruned, &f.manifest).unwrap();
    assert!(!sidecar.exists());
    let f = Fixture::new();
    let sidecar = f.dir.path().join("vibe.lock.provenance");
    fs::write(&sidecar,"[[package]]\npackage='org.test/a'\nversion='1.0.0'\nfetched_from='https://project.example/a'\n").unwrap();
    let mut records = 0;
    let result = apply_recording(
        f.dir.path(),
        f.dir.path(),
        &f.plan,
        &f.pruned,
        &f.manifest,
        |from, to| {
            fs::rename(from, to)?;
            records += 1;
            if records == 2 {
                fs::write(&sidecar, "foreign report\n")?;
            }
            Ok(())
        },
    );
    assert!(result.is_err());
    assert_eq!(
        fs::read(f.dir.path().join("vibe.lock")).unwrap(),
        f.original_lock
    );
    assert_eq!(fs::read_to_string(sidecar).unwrap(), "foreign report\n");
    assert!(f.slot.join("payload").exists());
}
