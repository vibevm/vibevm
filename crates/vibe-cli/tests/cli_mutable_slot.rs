//! Production-grain mutable `file://` slot freshness (PROP-054 §9.3).

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use common::{UserScratch, make_wal_dir_registry};
use specmark::verifies;

fn set_old_mtime(path: &Path, seconds: u64) -> SystemTime {
    fs::OpenOptions::new()
        .write(true)
        .open(path)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(UNIX_EPOCH + Duration::from_secs(seconds)))
        .unwrap();
    fs::metadata(path).unwrap().modified().unwrap()
}

fn recorded_mtimes(slot: &Path) -> BTreeMap<String, SystemTime> {
    vibe_workspace::vibedeps::read_slot_record(slot)
        .unwrap()
        .files
        .into_iter()
        .map(|file| {
            let modified = fs::metadata(slot.join(&file.path))
                .unwrap()
                .modified()
                .unwrap();
            (file.path, modified)
        })
        .collect()
}

fn age_recorded_payloads(slot: &Path) -> BTreeMap<String, SystemTime> {
    let record = vibe_workspace::vibedeps::read_slot_record(slot).unwrap();
    for (index, file) in record.files.iter().enumerate() {
        set_old_mtime(
            &slot.join(&file.path),
            1_000_000 + u64::try_from(index).unwrap(),
        );
    }
    recorded_mtimes(slot)
}

fn install_json(
    user: &UserScratch,
    project: &Path,
    registry: &Path,
    named: bool,
) -> serde_json::Value {
    let mut command = user.vibe();
    command.arg("--json").arg("install");
    if named {
        command.arg("org.vibevm.world/wal");
    }
    let output = command
        .arg("--path")
        .arg(project)
        .arg("--registry")
        .arg(registry)
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "install failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::Deserializer::from_slice(&output.stdout)
        .into_iter::<serde_json::Value>()
        .collect::<Result<Vec<_>, _>>()
        .expect("stdout is a stream of JSON documents")
        .pop()
        .expect("install emits its outcome")
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#MUTABLE-GETS-A-GATE")]
fn in_project_file_registry_skips_unchanged_then_reconciles_one_changed_file() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let registry = make_wal_dir_registry(project.path());

    let first = install_json(&user, project.path(), &registry, true);
    assert_eq!(first["materialised"].as_array().unwrap().len(), 1);

    let slot = project
        .path()
        .join(common::slot_dir("org.vibevm.world.wal", "0.2.0"));
    let changed_payload = slot.join("README.md");
    let record = slot.join(vibe_workspace::vibedeps::SLOT_RECORD_FILENAME);
    let payload_mtimes = age_recorded_payloads(&slot);
    let record_mtime = set_old_mtime(&record, 3_000_000);

    // Bare installs exercise freshness::Stale for the in-workspace file://
    // source, then the fresh LocalRegistry hash earns the later slot skip.
    let second = install_json(&user, project.path(), &registry, false);
    assert!(second["materialised"].as_array().unwrap().is_empty());
    assert_eq!(
        second["skipped"].as_array().unwrap(),
        &[serde_json::Value::String(common::slot_dir(
            "org.vibevm.world.wal",
            "0.2.0"
        ))]
    );
    assert_eq!(recorded_mtimes(&slot), payload_mtimes);
    assert_eq!(
        fs::metadata(&record).unwrap().modified().unwrap(),
        record_mtime
    );

    let registry_changed = registry.join("org.vibevm.world/wal/v0.2.0/README.md");
    let edited = format!(
        "{}\nmutable registry edit\n",
        fs::read_to_string(&registry_changed).unwrap()
    );
    fs::write(&registry_changed, &edited).unwrap();

    let third = install_json(&user, project.path(), &registry, false);
    assert_eq!(third["materialised"].as_array().unwrap().len(), 1);
    assert!(third["skipped"].as_array().unwrap().is_empty());
    let after = recorded_mtimes(&slot);
    let changed_paths = payload_mtimes
        .iter()
        .filter(|(path, before)| after.get(*path) != Some(*before))
        .map(|(path, _)| path.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        changed_paths,
        vec!["README.md"],
        "exactly the edited registry payload is rewritten"
    );
    assert_ne!(
        fs::metadata(&record).unwrap().modified().unwrap(),
        record_mtime,
        "the slot record advances to the fresh source hash"
    );
    assert_eq!(fs::read_to_string(changed_payload).unwrap(), edited);
}
