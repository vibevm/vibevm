#[cfg(test)]
mod cases {
    use super::super::*;
    use crate::vibedeps;
    use std::sync::{Arc, Mutex};
    use vibe_core::{Group, manifest::SpecFormat};

    fn source() -> tempfile::TempDir {
        let source = tempfile::tempdir().unwrap();
        fs::write(source.path().join("vibe.toml"), "[package]\ngroup='org.race'\nname='sample'\nversion='1.0.0'\nkind='flow'\nsources=[]\n").unwrap();
        source
    }
    fn slot(root: &Path) -> PathBuf {
        vibedeps::slot_abs_path(
            root,
            &Group::parse("org.race").unwrap(),
            "sample",
            &"1.0.0".parse().unwrap(),
        )
    }
    fn install(root: &Path, source: &Path, mode: CopyMode) -> Result<Vec<PathBuf>, WorkspaceError> {
        vibedeps::materialise_with_spec_format(
            root,
            &Group::parse("org.race").unwrap(),
            "sample",
            &"1.0.0".parse().unwrap(),
            source,
            mode,
            SpecFormat::Mixed,
            &ContentHash::parse(
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            )
            .unwrap(),
        )
    }

    #[test]
    fn m_06_a_3_new_copy_paths_never_overwrite_racing_occupants() {
        racing_occupants(CopyMode::Copy);
    }

    #[test]
    fn m_06_a_3_new_hardlink_paths_never_overwrite_racing_occupants() {
        racing_occupants(CopyMode::Hardlink);
    }

    fn racing_occupants(mode: CopyMode) {
        for recorded in [false, true] {
            for foreign in [b"source".as_slice(), b"foreign".as_slice()] {
                let source = source();
                let root = tempfile::tempdir().unwrap();
                if recorded {
                    fs::write(source.path().join("new.payload"), b"source").unwrap();
                }
                install(root.path(), source.path(), mode).unwrap();
                fs::write(source.path().join("new.payload"), b"source").unwrap();
                let destination = slot(root.path()).join("new.payload");
                if recorded {
                    fs::remove_file(&destination).unwrap();
                }
                let old_record = fs::read(slot(root.path()).join(SLOT_RECORD_FILENAME)).unwrap();
                let foreign = foreign.to_vec();
                let expected = foreign.clone();
                let injected_identity = Arc::new(Mutex::new(None));
                let witness = injected_identity.clone();
                placement::inject::arm(Box::new(move |path| {
                    assert!(path.ends_with("new.payload"));
                    fs::write(path, &foreign).unwrap();
                    let project = vibe_safefs::Project::open(path.parent().unwrap()).unwrap();
                    *witness.lock().unwrap() = Some(
                        project
                            .read_source_observation("new.payload")
                            .unwrap()
                            .unwrap()
                            .identity(),
                    );
                }));
                assert!(
                    install(root.path(), source.path(), mode).is_err(),
                    "{mode:?} recorded={recorded}"
                );
                assert_eq!(fs::read(&destination).unwrap(), expected);
                let project = vibe_safefs::Project::open(destination.parent().unwrap()).unwrap();
                assert_eq!(
                    project
                        .read_source_observation("new.payload")
                        .unwrap()
                        .unwrap()
                        .identity(),
                    injected_identity.lock().unwrap().unwrap()
                );
                assert_eq!(
                    fs::read(slot(root.path()).join(SLOT_RECORD_FILENAME)).unwrap(),
                    old_record
                );
                assert_eq!(
                    fs::read(source.path().join("new.payload")).unwrap(),
                    b"source"
                );
            }
        }
    }

    #[test]
    fn m_06_a_3_new_record_refuses_concurrent_metadata_and_reports_landed_payload() {
        let source = source();
        let root = tempfile::tempdir().unwrap();
        fs::write(source.path().join("data"), b"landed payload").unwrap();
        placement::inject::arm(Box::new(|path| {
            fs::write(
                path.parent().unwrap().join(SLOT_RECORD_FILENAME),
                b"foreign record",
            )
            .unwrap();
        }));
        let error = install(root.path(), source.path(), CopyMode::Copy).unwrap_err();
        assert!(error.to_string().contains("already published"), "{error}");
        assert_eq!(
            fs::read(slot(root.path()).join(SLOT_RECORD_FILENAME)).unwrap(),
            b"foreign record"
        );
        assert_eq!(
            fs::read(slot(root.path()).join("data")).unwrap(),
            b"landed payload"
        );
        assert!(slot(root.path()).join("vibe.toml").is_file());
    }

    #[test]
    fn m_06_a_3_unrecorded_slot_keeps_matching_foreign_payload_and_unrelated_files() {
        let source = source();
        let root = tempfile::tempdir().unwrap();
        let destination = slot(root.path());
        fs::write(source.path().join("data"), b"same").unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(destination.join("data"), b"same").unwrap();
        fs::write(destination.join("operator"), b"unrelated").unwrap();
        let project = vibe_safefs::Project::open(&destination).unwrap();
        let before = project.read_source_observation("data").unwrap().unwrap();
        install(root.path(), source.path(), CopyMode::Copy).unwrap();
        let after = project.read_source_observation("data").unwrap().unwrap();
        assert_eq!(before.identity(), after.identity());
        assert_eq!(before.permissions(), after.permissions());
        assert_eq!(
            fs::read(destination.join("operator")).unwrap(),
            b"unrelated"
        );
    }
}
