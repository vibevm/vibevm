use super::*;
use vibe_core::manifest::SpecFormat;

fn record(files: &[PreparedSlotFile]) -> SlotRecord {
    let source_hash = ContentHash::parse(&format!("sha256:{}", "1".repeat(64))).unwrap();
    let mut rows = files
        .iter()
        .map(|file| super::super::SlotFile {
            path: file.path_wire(),
            sha256: file.sha256().to_owned(),
            source: None,
            disposition: None,
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.path.cmp(&right.path));
    SlotRecord {
        schema: super::super::SLOT_RECORD_SCHEMA,
        source_hash,
        spec_format: SpecFormat::Mixed,
        converter_recipe: None,
        derived_hash: None,
        overlay_hash: None,
        files: rows,
    }
}

#[test]
fn strict_mode_refuses_existing_unrecorded_slot_without_deleting_it() {
    let root = tempfile::tempdir().unwrap();
    let slot = root.path().join("slot");
    fs::create_dir(&slot).unwrap();
    fs::write(slot.join("sentinel.txt"), b"keep").unwrap();
    let incoming = vec![PreparedSlotFile::from_bytes(
        "owned.txt".into(),
        b"new".to_vec(),
    )];
    let error = reconcile_slot_descriptor(&slot, &incoming, &record(&incoming)).unwrap_err();
    assert!(
        error.to_string().contains("unrecorded existing slot"),
        "{error}"
    );
    assert_eq!(fs::read(slot.join("sentinel.txt")).unwrap(), b"keep");
    assert!(!slot.join("owned.txt").exists());
}

#[test]
fn strict_mode_never_adopts_same_content_unrecorded_path() {
    let root = tempfile::tempdir().unwrap();
    let slot = root.path().join("slot");
    let first = vec![PreparedSlotFile::from_bytes(
        "owned.txt".into(),
        b"old".to_vec(),
    )];
    reconcile_slot_descriptor(&slot, &first, &record(&first)).unwrap();
    fs::write(slot.join("new.txt"), b"same").unwrap();
    let incoming = vec![
        PreparedSlotFile::from_bytes("new.txt".into(), b"same".to_vec()),
        PreparedSlotFile::from_bytes("owned.txt".into(), b"updated".to_vec()),
    ];
    let error = reconcile_slot_descriptor(&slot, &incoming, &record(&incoming)).unwrap_err();
    assert!(error.to_string().contains("unrecorded occupied"), "{error}");
    assert_eq!(fs::read(slot.join("new.txt")).unwrap(), b"same");
    assert_eq!(fs::read(slot.join("owned.txt")).unwrap(), b"old");
}

#[test]
fn strict_new_path_publication_never_overwrites_for_bytes_copy_or_hardlink() {
    let root = tempfile::tempdir().unwrap();
    let slot = root.path().join("slot");
    fs::create_dir(&slot).unwrap();
    let source = root.path().join("source.txt");
    fs::write(&source, b"incoming").unwrap();
    for (name, prepared) in [
        (
            "bytes.txt",
            PreparedSlotFile::from_bytes("bytes.txt".into(), b"incoming".to_vec()),
        ),
        (
            "copy.txt",
            PreparedSlotFile::from_source("copy.txt".into(), source.clone(), CopyMode::Copy)
                .unwrap(),
        ),
        (
            "hardlink.txt",
            PreparedSlotFile::from_source(
                "hardlink.txt".into(),
                source.clone(),
                CopyMode::Hardlink,
            )
            .unwrap(),
        ),
    ] {
        fs::write(slot.join(name), b"occupant").unwrap();
        let error = place_atomically(&slot, &prepared, false).unwrap_err();
        assert!(
            error.to_string().contains("occupied") || error.to_string().contains("exist"),
            "{error}"
        );
        assert_eq!(fs::read(slot.join(name)).unwrap(), b"occupant");
    }
    let fresh =
        PreparedSlotFile::from_source("fresh-hardlink.txt".into(), source, CopyMode::Hardlink)
            .unwrap();
    place_atomically(&slot, &fresh, false).unwrap();
    assert_eq!(
        fs::read(slot.join("fresh-hardlink.txt")).unwrap(),
        b"incoming"
    );
}

#[test]
fn strict_mode_updates_exact_recorded_file_and_preserves_unrecorded_build_output() {
    let root = tempfile::tempdir().unwrap();
    let slot = root.path().join("slot");
    let first = vec![PreparedSlotFile::from_bytes(
        "owned.txt".into(),
        b"old".to_vec(),
    )];
    reconcile_slot_descriptor(&slot, &first, &record(&first)).unwrap();
    fs::write(slot.join("build-output.txt"), b"keep").unwrap();
    let second = vec![PreparedSlotFile::from_bytes(
        "owned.txt".into(),
        b"new".to_vec(),
    )];
    reconcile_slot_descriptor(&slot, &second, &record(&second)).unwrap();
    assert_eq!(fs::read(slot.join("owned.txt")).unwrap(), b"new");
    assert_eq!(fs::read(slot.join("build-output.txt")).unwrap(), b"keep");
}

#[test]
fn strict_mode_refuses_to_replace_a_changed_recorded_file() {
    let root = tempfile::tempdir().unwrap();
    let slot = root.path().join("slot");
    let first = vec![PreparedSlotFile::from_bytes(
        "owned.txt".into(),
        b"recorded".to_vec(),
    )];
    reconcile_slot_descriptor(&slot, &first, &record(&first)).unwrap();
    fs::write(slot.join("owned.txt"), b"unrecorded edit").unwrap();
    let second = vec![PreparedSlotFile::from_bytes(
        "owned.txt".into(),
        b"incoming".to_vec(),
    )];
    let error = reconcile_slot_descriptor(&slot, &second, &record(&second)).unwrap_err();
    assert!(error.to_string().contains("changed before"), "{error}");
    assert_eq!(
        fs::read(slot.join("owned.txt")).unwrap(),
        b"unrecorded edit"
    );
}

#[test]
fn strict_recorded_hardlink_update_keeps_old_file_when_source_changes() {
    let root = tempfile::tempdir().unwrap();
    let slot = root.path().join("slot");
    let source = root.path().join("source.txt");
    fs::write(&source, b"prepared").unwrap();
    let first = vec![PreparedSlotFile::from_bytes(
        "owned.txt".into(),
        b"old owned".to_vec(),
    )];
    reconcile_slot_descriptor(&slot, &first, &record(&first)).unwrap();
    let prepared =
        PreparedSlotFile::from_source("owned.txt".into(), source.clone(), CopyMode::Hardlink)
            .unwrap();
    fs::write(&source, b"changed after preparation").unwrap();
    let error = place_strict_recorded(&slot, &prepared).unwrap_err();
    assert!(
        error.to_string().contains("incoming payload changed"),
        "{error}"
    );
    assert_eq!(fs::read(slot.join("owned.txt")).unwrap(), b"old owned");

    let current = vec![
        PreparedSlotFile::from_source("owned.txt".into(), source, CopyMode::Hardlink).unwrap(),
    ];
    reconcile_slot_descriptor(&slot, &current, &record(&current)).unwrap();
    assert_eq!(
        fs::read(slot.join("owned.txt")).unwrap(),
        b"changed after preparation"
    );
}

#[test]
fn legacy_reconcile_retains_existing_unrecorded_slot_migration() {
    let root = tempfile::tempdir().unwrap();
    let slot = root.path().join("slot");
    fs::create_dir(&slot).unwrap();
    fs::write(slot.join("legacy.txt"), b"old").unwrap();
    let incoming = vec![PreparedSlotFile::from_bytes(
        "owned.txt".into(),
        b"new".to_vec(),
    )];
    reconcile_slot(&slot, &incoming, &record(&incoming)).unwrap();
    assert!(!slot.join("legacy.txt").exists());
    assert_eq!(fs::read(slot.join("owned.txt")).unwrap(), b"new");
}
