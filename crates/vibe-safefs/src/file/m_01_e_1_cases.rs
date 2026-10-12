//! Owned E1 filesystem fixtures; no customer or ambient state.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-070#SOURCE-OBSERVATION-FIXTURES");
use crate::{Project, StreamOutcome};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
fn state(root: &std::path::Path) -> Vec<(std::path::PathBuf, String, Vec<u8>)> {
    fn walk(
        root: &std::path::Path,
        path: &std::path::Path,
        out: &mut Vec<(std::path::PathBuf, String, Vec<u8>)>,
    ) {
        let m = fs::symlink_metadata(path).unwrap();
        let mut identity = format!(
            "{:?}:{}:{}",
            m.file_type(),
            m.len(),
            m.permissions().readonly()
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            identity.push_str(&format!(
                ":{}:{}:{}:{}",
                m.dev(),
                m.ino(),
                m.nlink(),
                m.mode()
            ));
        }
        let bytes = if m.is_symlink() {
            fs::read_link(path)
                .unwrap()
                .as_os_str()
                .as_encoded_bytes()
                .to_vec()
        } else if m.is_file() {
            fs::read(path).unwrap()
        } else {
            Vec::new()
        };
        out.push((
            path.strip_prefix(root).unwrap().to_path_buf(),
            identity,
            bytes,
        ));
        if m.is_dir() {
            let mut children: Vec<_> = fs::read_dir(path)
                .unwrap()
                .map(|e| e.unwrap().path())
                .collect();
            children.sort();
            for child in children {
                walk(root, &child, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out
}

#[test]
fn large_verified_stream_can_be_cancelled_without_writes() {
    let root = tempfile::tempdir().unwrap();
    let bytes = vec![0x5a; 4 * 1024 * 1024];
    fs::write(root.path().join("asset"), &bytes).unwrap();
    let before = state(root.path());
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let project = Project::open(root.path()).unwrap();
    let (mut reader, completion) = project
        .open_verified_stream_bounded("asset", &digest, bytes.len() as u64, bytes.len() as u64)
        .unwrap();
    let mut first = [0; 4096];
    assert_eq!(reader.read(&mut first).unwrap(), first.len());
    assert_eq!(first, [0x5a; 4096]);
    assert_eq!(completion.outcome(), StreamOutcome::Incomplete);
    drop(reader);
    assert_eq!(
        completion.outcome(),
        StreamOutcome::Uncertain("held stream dropped before completion".into())
    );
    let (mut control, done) = project
        .open_verified_stream_bounded("asset", &digest, bytes.len() as u64, bytes.len() as u64)
        .unwrap();
    assert_eq!(
        std::io::copy(&mut control, &mut std::io::sink()).unwrap(),
        bytes.len() as u64
    );
    assert_eq!(
        done.outcome(),
        StreamOutcome::Complete {
            bytes: bytes.len() as u64,
            sha256: digest
        }
    );
    assert_eq!(state(root.path()), before);
}

#[cfg(unix)]
#[test]
fn native_links_refuse_without_touching_targets() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("target"), b"OUTSIDE").unwrap();
    symlink(outside.path().join("target"), root.path().join("link")).unwrap();
    fs::write(root.path().join("source"), b"SHARED").unwrap();
    fs::hard_link(root.path().join("source"), root.path().join("alias")).unwrap();
    let before = (state(root.path()), state(outside.path()));
    let project = Project::open(root.path()).unwrap();
    let link = project
        .read_file_snapshot_bounded("link", 1024)
        .unwrap_err();
    assert!(format!("{link:#}").contains("opening"), "{link:#}");
    let shared = project
        .read_file_snapshot_bounded("source", 1024)
        .unwrap_err();
    assert!(format!("{shared:#}").contains("hard link"), "{shared:#}");
    assert_eq!((state(root.path()), state(outside.path())), before);
}

#[cfg(unix)]
#[test]
fn held_replacement_refuses_equal_bytes_and_preserves_injected_state() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("source"), b"SAME").unwrap();
    let project = Project::open(root.path()).unwrap();
    let path = root.path().to_path_buf();
    let injected = std::rc::Rc::new(std::cell::RefCell::new(None));
    let capture = injected.clone();
    crate::arm_before_bounded_read(Some(Box::new(move |_, _| {
        fs::rename(path.join("source"), path.join("old")).unwrap();
        fs::write(path.join("source"), b"SAME").unwrap();
        *capture.borrow_mut() = Some(state(&path));
    })));
    let result = project.read_file_snapshot_bounded("source", 1024);
    crate::arm_before_bounded_read(None);
    let error = result.unwrap_err();
    assert!(
        format!("{error:#}").contains("final-name race"),
        "{error:#}"
    );
    assert_eq!(state(root.path()), *injected.borrow().as_ref().unwrap());
    let old = project
        .read_file_snapshot_bounded("old", 1024)
        .unwrap()
        .unwrap();
    let new = project
        .read_file_snapshot_bounded("source", 1024)
        .unwrap()
        .unwrap();
    assert_eq!(old.bytes, new.bytes);
    assert_ne!(old.identity, new.identity);
}

#[test]
fn portable_identity_mismatch_refuses_without_namespace_change() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("source"), b"UNCHANGED").unwrap();
    let before = state(root.path());
    let project = Project::open(root.path()).unwrap();
    crate::arm_bounded_read_identity_check(Some(Box::new(|_| false)));
    let result = project.read_file_snapshot_bounded("source", 1024);
    crate::arm_bounded_read_identity_check(None);
    let error = result.unwrap_err();
    assert!(
        format!("{error:#}").contains("final-name race"),
        "{error:#}"
    );
    assert_eq!(
        project
            .read_file_snapshot_bounded("source", 1024)
            .unwrap()
            .unwrap()
            .bytes,
        b"UNCHANGED"
    );
    assert_eq!(state(root.path()), before);
}

#[test]
fn growth_after_open_refuses_a_truncated_prefix() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("source"), b"SMALL").unwrap();
    let project = Project::open(root.path()).unwrap();
    let path = root.path().to_path_buf();
    let injected = std::rc::Rc::new(std::cell::RefCell::new(None));
    let capture = injected.clone();
    crate::arm_before_bounded_read(Some(Box::new(move |_, _| {
        fs::write(path.join("source"), vec![b'X'; 2048]).unwrap();
        *capture.borrow_mut() = Some(state(&path));
    })));
    let result = project.read_file_snapshot_bounded("source", 1024);
    crate::arm_before_bounded_read(None);
    let error = result.unwrap_err();
    assert!(format!("{error:#}").contains("cap"), "{error:#}");
    assert_eq!(state(root.path()), *injected.borrow().as_ref().unwrap());
}
