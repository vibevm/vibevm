//! Incoming observed mode versus recorded and foreign destination ownership.
#[cfg(all(test, unix))]
mod cases {
    use crate::vibedeps;
    use std::fs;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use std::path::{Path, PathBuf};
    use vibe_core::{ContentHash, Group};

    fn manifest(declared_file: bool) -> String {
        let package = "[package]\ngroup='org.mode'\nname='runner'\nversion='1.0.0'\nkind='tool'\n";
        if declared_file {
            format!(
                "{package}[[package.sources]]\nfile='run.payload'\nmedia='application/octet-stream'\ninterpretation='opaque'\nrole='resource'\nloading='explicit'\n"
            )
        } else {
            format!("{package}sources=[]\n")
        }
    }
    fn slot(root: &Path) -> PathBuf {
        vibedeps::slot_abs_path(
            root,
            &Group::parse("org.mode").unwrap(),
            "runner",
            &"1.0.0".parse().unwrap(),
        )
    }
    fn materialise(root: &Path, source: &Path) -> Result<Vec<PathBuf>, crate::WorkspaceError> {
        vibedeps::materialise(
            root,
            &Group::parse("org.mode").unwrap(),
            "runner",
            &"1.0.0".parse().unwrap(),
            source,
            &ContentHash::parse(
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            )
            .unwrap(),
        )
    }

    #[test]
    fn recorded_declared_copy_reconciles_mode_without_chmodding_the_old_inode() {
        let source = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        fs::write(source.path().join("vibe.toml"), manifest(true)).unwrap();
        let input = source.path().join("run.payload");
        fs::write(&input, b"same raw payload").unwrap();
        fs::set_permissions(&input, fs::Permissions::from_mode(0o644)).unwrap();
        materialise(root.path(), source.path()).unwrap();
        let output = slot(root.path()).join("run.payload");
        let record = vibedeps::read_slot_record(&slot(root.path())).unwrap();
        let old = fs::metadata(&output).unwrap();
        let outside = tempfile::tempdir().unwrap();
        let old_alias = outside.path().join("old-alias");
        fs::hard_link(&output, &old_alias).unwrap();
        fs::set_permissions(&input, fs::Permissions::from_mode(0o755)).unwrap();
        materialise(root.path(), source.path()).unwrap();
        let current = fs::metadata(&output).unwrap();
        assert_eq!(
            current.permissions().mode() & 0o777,
            0o755,
            "same bytes do not imply same observed mode"
        );
        assert_ne!(
            (old.dev(), old.ino()),
            (current.dev(), current.ino()),
            "replace the recorded file, never chmod a possibly shared old inode"
        );
        assert_eq!(
            fs::metadata(&old_alias).unwrap().permissions().mode() & 0o777,
            0o644
        );
        assert_eq!(fs::read(&output).unwrap(), b"same raw payload");
        assert_eq!(
            vibedeps::read_slot_record(&slot(root.path())).unwrap(),
            record,
            "byte record format/identity remains unchanged"
        );
        materialise(root.path(), source.path()).unwrap();
        let repeated = fs::metadata(&output).unwrap();
        assert_eq!(
            (current.dev(), current.ino(), current.modified().unwrap()),
            (repeated.dev(), repeated.ino(), repeated.modified().unwrap()),
            "unchanged mode+bytes remains a true keep"
        );
    }

    #[test]
    fn foreign_same_byte_file_keeps_its_mode_and_different_bytes_still_refuse() {
        for same_bytes in [true, false] {
            let source = tempfile::tempdir().unwrap();
            let root = tempfile::tempdir().unwrap();
            fs::write(source.path().join("vibe.toml"), manifest(false)).unwrap();
            materialise(root.path(), source.path()).unwrap();
            let output = slot(root.path()).join("run.payload");
            let foreign = if same_bytes {
                b"incoming".as_slice()
            } else {
                b"foreign".as_slice()
            };
            fs::write(&output, foreign).unwrap();
            fs::set_permissions(&output, fs::Permissions::from_mode(0o644)).unwrap();
            let old = fs::metadata(&output).unwrap();
            let record = fs::read(slot(root.path()).join(vibedeps::SLOT_RECORD_FILENAME)).unwrap();
            fs::write(source.path().join("vibe.toml"), manifest(true)).unwrap();
            fs::write(source.path().join("run.payload"), b"incoming").unwrap();
            fs::set_permissions(
                source.path().join("run.payload"),
                fs::Permissions::from_mode(0o755),
            )
            .unwrap();
            let result = materialise(root.path(), source.path());
            if same_bytes {
                result.unwrap();
            } else {
                assert!(result.is_err());
                assert_eq!(
                    fs::read(slot(root.path()).join(vibedeps::SLOT_RECORD_FILENAME)).unwrap(),
                    record
                );
            }
            let current = fs::metadata(&output).unwrap();
            assert_eq!(
                (old.dev(), old.ino(), old.permissions()),
                (current.dev(), current.ino(), current.permissions())
            );
            assert_eq!(fs::read(&output).unwrap(), foreign);
        }
    }
}
