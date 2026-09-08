use super::*;

#[test]
fn durable_projection_distinguishes_empty_from_malformed_or_missing_named_state() {
    let empty = TempDir::new().unwrap();
    slot(
        empty.path(),
        "m-orphan",
        "[package]\ngroup = \"org.lock\"\nname = \"m-orphan\"\nkind = \"tool\"\nversion = \"1.0.0\"\n",
    );
    assert!(read_durable_resolution(empty.path()).unwrap().is_empty());

    let malformed = TempDir::new().unwrap();
    fs::write(malformed.path().join(Lockfile::FILENAME), "not a lockfile").unwrap();
    let error = read_durable_resolution(malformed.path()).unwrap_err();
    assert!(matches!(
        error,
        WorkspaceError::ExtensionWorld { source }
            if matches!(*source, crate::extension_world::ExtensionWorldError::InvalidLock { .. })
    ));

    let nonregular = TempDir::new().unwrap();
    fs::create_dir(nonregular.path().join(Lockfile::FILENAME)).unwrap();
    let error = read_durable_resolution(nonregular.path()).unwrap_err();
    assert!(matches!(
        error,
        WorkspaceError::ExtensionWorld { source }
            if matches!(*source, crate::extension_world::ExtensionWorldError::NonRegularLock { .. })
    ));

    let missing = TempDir::new().unwrap();
    write_lock(missing.path(), vec![locked("z-tools", "sha256:aa", &[])]);
    assert!(read_durable_resolution(missing.path()).is_err());

    let mismatch = TempDir::new().unwrap();
    slot(
        mismatch.path(),
        "z-tools",
        "[package]\ngroup = \"org.lock\"\nname = \"z-tools\"\nkind = \"flow\"\nversion = \"1.0.0\"\n",
    );
    write_lock(mismatch.path(), vec![locked("z-tools", "sha256:aa", &[])]);
    assert!(read_durable_resolution(mismatch.path()).is_err());

    let materialization = TempDir::new().unwrap();
    slot(
        materialization.path(),
        "z-tools",
        "[package]\ngroup = \"org.lock\"\nname = \"z-tools\"\nkind = \"tool\"\nversion = \"1.0.0\"\nmaterialization = \"in-place\"\n",
    );
    let orphan =
        crate::vibedeps::in_place_slot_abs_path(materialization.path(), &group(), "z-tools");
    write(
        &orphan.join(Manifest::FILENAME),
        "[package]\ngroup = \"org.lock\"\nname = \"z-tools\"\nkind = \"tool\"\nversion = \"1.0.0\"\nmaterialization = \"in-place\"\n",
    );
    write_lock(
        materialization.path(),
        vec![locked("z-tools", "sha256:aa", &[])],
    );
    let error = read_durable_resolution(materialization.path()).unwrap_err();
    assert!(matches!(
        error,
        WorkspaceError::ExtensionWorld { source }
            if matches!(
                *source,
                crate::extension_world::ExtensionWorldError::SlotMaterializationMismatch {
                    declared: "in-place",
                    locked: "copy",
                    ..
                }
            )
    ));
}
