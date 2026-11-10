use super::*;
use std::os::unix::fs::PermissionsExt;

fn apply(
    source: &Source,
    planned: Box<vibe_install::PlannedInstall>,
) -> Result<(), vibe_install::Error> {
    vibe_install::apply(source, *planned, SlotIntegrity::Verify, &hook_policy()).map(|_| ())
}

#[test]
fn initial_unsafe_metadata_refuses_before_any_publication_or_hook() {
    for name in ["vibe.toml", "vibe.lock", "vibe.lock.provenance"] {
        for kind in ["symlink", "hardlink", "directory"] {
            let (temp, source, planned) = prepared(Case::Complete);
            let path = temp.path().join(name);
            let sentinel = temp.path().join("sentinel");
            let before = fs::read(&path).unwrap_or_else(|_| b"SYNTHETIC_SENTINEL".to_vec());
            fs::write(&sentinel, &before).unwrap();
            if path.exists() {
                fs::remove_file(&path).unwrap();
            }
            match kind {
                "symlink" => symlink(&sentinel, &path).unwrap(),
                "hardlink" => fs::hard_link(&sentinel, &path).unwrap(),
                _ => fs::create_dir(&path).unwrap(),
            }
            assert!(apply(&source, planned).is_err(), "{name}/{kind}");
            assert_eq!(source.calls(), 0);
            assert_eq!(fs::read(&sentinel).unwrap(), before);
            if name != "vibe.toml" {
                assert_eq!(
                    fs::read_to_string(temp.path().join("vibe.toml")).unwrap(),
                    BEFORE
                );
            }
        }
    }
}

#[test]
fn changed_recognized_manifest_input_refuses_before_effects() {
    let (temp, source, planned) = prepared(Case::Complete);
    let changed = BEFORE.replace("rollback-probe", "another-project");
    let path = temp.path().join("vibe.toml");
    fs::write(&path, &changed).unwrap();
    let before_identity = identity(&path);
    let error = apply(&source, planned).unwrap_err();
    assert!(
        matches!(&error,vibe_install::Error::Workspace(vibe_workspace::WorkspaceError::SpecMaterialization{reason,..})
        if reason.contains("PROP-072#SOURCE-HOST-BOOT-LAW") && reason.contains("authored node manifest differs from admitted workspace")),
        "{error}"
    );
    assert_eq!(source.calls(), 0);
    assert_eq!(identity(&path), before_identity);
    assert_eq!(
        fs::read_to_string(temp.path().join("vibe.toml")).unwrap(),
        changed
    );
}

#[test]
fn definitely_before_publication_keeps_original_object_and_bytes() {
    let (temp, source, planned) = prepared(Case::Complete);
    let path = temp.path().join("vibe.toml");
    let before_id = identity(&path);
    vibe_safefs::fail_before_publish(Some("vibe.toml"));
    let result = apply(&source, planned);
    vibe_safefs::fail_before_publish(None);
    let error = result.unwrap_err();
    assert!(!matches!(error, vibe_install::Error::RollbackFailed { .. }));
    assert_eq!(source.calls(), 0);
    assert_eq!(
        identity(&path),
        before_id,
        "never-published guard must not rewrite A"
    );
    assert_eq!(fs::read_to_string(path).unwrap(), BEFORE);
}

#[test]
fn same_byte_replacement_during_verification_is_a_conflict_not_our_receipt() {
    let (temp, source, planned) = prepared(Case::Complete);
    let kept = std::sync::Arc::new(std::sync::Mutex::new(None));
    let record = kept.clone();
    vibe_safefs::arm_before_publish_verify(Some(Box::new(move |parent, name| {
        assert_eq!(name, "vibe.toml");
        let path = parent.path().join(name);
        let bytes = fs::read(&path).unwrap();
        let replacement = parent.path().join("other-object");
        fs::write(&replacement, &bytes).unwrap();
        fs::rename(replacement, &path).unwrap();
        *record.lock().unwrap() = Some((bytes, identity(&path)));
    })));
    let error = apply(&source, planned).unwrap_err();
    vibe_safefs::arm_before_publish_verify(None);
    assert!(matches!(error, vibe_install::Error::RollbackFailed { .. }));
    assert_eq!(source.calls(), 0);
    let (bytes, id) = kept.lock().unwrap().take().unwrap();
    let path = temp.path().join("vibe.toml");
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(identity(&path), id);
}

#[test]
fn rollback_preserves_mode_and_captured_comment_decoration() {
    let (temp, source, planned) = prepared(Case::Unchanged);
    let path = temp.path().join("vibe.toml");
    let decorated = format!("# operator header\n{BEFORE}# operator footer\n");
    fs::write(&path, &decorated).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    let error = apply(&source, planned).unwrap_err();
    assert!(!matches!(error, vibe_install::Error::RollbackFailed { .. }));
    let observation = source.observation();
    let published = observation["published"].as_str().unwrap();
    assert!(published.contains("# operator header") && published.contains("# operator footer"));
    assert_eq!(fs::read_to_string(&path).unwrap(), decorated);
    assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o640
    );
}

#[test]
fn partial_lock_publication_rolls_back_prior_absence_and_manifest() {
    let (temp, source, planned) = prepared(Case::Complete);
    let lock = temp.path().join("vibe.lock");
    fs::remove_file(&lock).unwrap();
    vibe_safefs::fail_after_publish(Some("vibe.lock"));
    let result = apply(&source, planned);
    vibe_safefs::fail_after_publish(None);
    let error = result.unwrap_err();
    assert!(
        !matches!(error, vibe_install::Error::RollbackFailed { .. }),
        "{error}"
    );
    assert_eq!(source.calls(), 1);
    assert!(
        !lock.exists(),
        "owned after-image must restore prior absence"
    );
    assert_eq!(
        fs::read_to_string(temp.path().join("vibe.toml")).unwrap(),
        BEFORE
    );
}

#[test]
fn empty_provenance_is_removed_on_success_without_changing_lock_mode() {
    let (temp, source, planned) = prepared(Case::Complete);
    let lock = temp.path().join("vibe.lock");
    fs::set_permissions(&lock, fs::Permissions::from_mode(0o640)).unwrap();
    let sidecar = temp.path().join("vibe.lock.provenance");
    fs::write(&sidecar, "# older provenance\n").unwrap();
    apply(&source, planned).unwrap();
    assert!(!sidecar.exists());
    assert_eq!(
        fs::metadata(lock).unwrap().permissions().mode() & 0o777,
        0o640
    );
}

#[test]
fn unchanged_shared_readonly_manifest_remains_a_supported_input() {
    let (temp, source, _) = prepared(Case::Complete);
    let path = temp.path().join("vibe.toml");
    let content = format!("{BEFORE}\n[requires.packages]\n'org.vibevm/giant'='^1.0.0'\n");
    fs::write(&path, &content).unwrap();
    let sibling = temp.path().join("shared-manifest");
    fs::hard_link(&path, &sibling).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o444)).unwrap();
    let before = identity(&path);
    let Plan::Ready(planned) = vibe_install::plan(
        &source,
        temp.path(),
        InstallRequest {
            roots: vec![PackageRef::parse("org.vibevm/giant").unwrap()],
            features: FeatureRequest::default(),
            language: None,
            exact: false,
            generated_by: "probe".into(),
            offline: true,
            no_recommends: false,
        },
        &NullObserver,
    )
    .unwrap() else {
        panic!("explicit root must plan");
    };
    apply(&source, planned).unwrap();
    assert_eq!(identity(&path), before);
    assert_eq!(fs::read_to_string(&path).unwrap(), content);
    assert_eq!(fs::read_to_string(sibling).unwrap(), content);
    assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o444
    );
}

#[test]
fn invalid_utf8_lock_or_provenance_refuses_before_manifest_effects() {
    for name in ["vibe.lock", "vibe.lock.provenance"] {
        let (temp, source, planned) = prepared(Case::Complete);
        fs::write(temp.path().join(name), [0xff, 0xfe]).unwrap();
        let error = apply(&source, planned).unwrap_err();
        assert!(error.to_string().contains("not UTF-8"));
        assert!(error.to_string().contains(name));
        assert_eq!(source.calls(), 0);
        assert_eq!(
            fs::read_to_string(temp.path().join("vibe.toml")).unwrap(),
            BEFORE
        );
        assert_eq!(fs::read(temp.path().join(name)).unwrap(), [0xff, 0xfe]);
    }
}
