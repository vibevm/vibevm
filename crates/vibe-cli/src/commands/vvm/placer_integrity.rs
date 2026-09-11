use super::*;

pub(super) fn actual_matches(entry: &FileEntry, path: &Path) -> bool {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return false;
    };
    if !meta.file_type().is_file() || meta.len() != entry.size {
        return false;
    }
    #[cfg(unix)]
    if is_essential_binary(&entry.rel) {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o111 == 0 {
            return false;
        }
    }
    match &entry.hash {
        Some(expected) => content_hash(path).is_ok_and(|actual| &actual == expected),
        None => mtime_nanos(&meta) == entry.mtime_nanos,
    }
}

pub(super) fn safe_reuse(new: &FileEntry, previous: &FileEntry, path: &Path) -> bool {
    unchanged(new, previous)
        && actual_matches(previous, path)
        && (!is_essential_binary(&new.rel) || actual_matches(new, path))
}

pub(crate) fn matches_on_disk(
    store: &VersionStore,
    new: &Manifest,
    previous: &Manifest,
    previous_dir: &Path,
) -> bool {
    store.guard_mutation_tree(previous_dir).is_ok()
        && matches(new, previous)
        && new.files.iter().all(|entry| {
            previous
                .get(&entry.rel)
                .is_some_and(|old| safe_reuse(entry, old, &previous_dir.join(&entry.rel)))
        })
}

pub(crate) fn installed_files_match(
    store: &VersionStore,
    record: &super::super::model::InstallRecord,
) -> bool {
    let home = store.instance_dir(&record.version_id(), record.instance);
    if store.guard_mutation_tree(&home).is_err() {
        return false;
    }
    let Some(manifest) = read_manifest(&home) else {
        return false;
    };
    if !manifest_shape_valid(record.origin, &manifest) {
        return false;
    }
    manifest
        .files
        .iter()
        .all(|entry| actual_matches(entry, &home.join(&entry.rel)))
}

pub(super) fn manifest_shape_valid(
    origin: super::super::model::Origin,
    manifest: &Manifest,
) -> bool {
    let expected = if origin == super::super::model::Origin::Binary {
        vec![BINARY_NAME.to_string()]
    } else {
        vec![
            format!("bin/{BINARY_NAME}"),
            format!("bin/{INDEX_BINARY_NAME}"),
        ]
    };
    if manifest.files.len() != expected.len() {
        return false;
    }
    let mut actual = std::collections::BTreeSet::new();
    for entry in &manifest.files {
        let valid_hash = entry.hash.as_deref().is_some_and(|hash| {
            hash.len() == 64
                && hash
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        });
        if !valid_hash || !expected.contains(&entry.rel) || !actual.insert(entry.rel.as_str()) {
            return false;
        }
    }
    actual.len() == expected.len()
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::{PermissionsExt, symlink};

    #[test]
    fn source_instance_integrity_rejects_linked_bin_ancestor() {
        let temp = tempfile::tempdir().unwrap();
        let store = VersionStore::new(temp.path().join("opt"));
        let id = super::super::super::model::VersionId::new(
            super::super::super::model::Kind::Branch,
            "main",
        );
        let mut dist = Vec::new();
        for (name, rel) in [
            (BINARY_NAME, format!("bin/{BINARY_NAME}")),
            (INDEX_BINARY_NAME, format!("bin/{INDEX_BINARY_NAME}")),
        ] {
            let path = temp.path().join(name);
            fs::write(&path, name.as_bytes()).unwrap();
            let mut permissions = fs::metadata(&path).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&path, permissions).unwrap();
            dist.push((path, rel));
        }
        let manifest = manifest_for(&dist).unwrap();
        place(&store, &id, 1, &dist, &manifest, None).unwrap();
        let home = store.instance_dir(&id, 1);
        let outside = temp.path().join("outside-bin");
        fs::rename(home.join("bin"), &outside).unwrap();
        symlink(&outside, home.join("bin")).unwrap();
        let record = super::super::super::model::InstallRecord {
            kind: id.kind,
            id: id.id.clone(),
            instance: 1,
            commit: "c".into(),
            toolchain: "rustc".into(),
            profile: super::super::super::model::Profile::Debug,
            installed_at: "now".into(),
            origin: super::super::super::model::Origin::External,
            source_path: Some("/source".into()),
            payload_sha256: None,
            distribution_manifest_sha256: None,
        };
        assert!(!installed_files_match(&store, &record));
    }
}
