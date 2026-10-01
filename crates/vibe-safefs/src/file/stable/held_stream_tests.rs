use std::io::{Read, Write};

use sha2::{Digest, Sha256};

use super::*;

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
fn exact_small_large_and_empty_streams_report_completion() {
    for bytes in [b"small".to_vec(), vec![0x5a; 2 * WINDOW + 17], Vec::new()] {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("asset"), &bytes).unwrap();
        let project = Project::open(root.path()).unwrap();
        let (mut reader, completion) = project
            .open_verified_stream_bounded(
                "asset",
                &digest(&bytes),
                bytes.len() as u64,
                2 * WINDOW as u64 + 17,
            )
            .unwrap();
        if bytes.is_empty() {
            assert_eq!(
                completion.outcome(),
                StreamOutcome::Complete {
                    bytes: 0,
                    sha256: digest(&bytes)
                }
            );
        } else {
            assert_eq!(completion.outcome(), StreamOutcome::Incomplete);
        }
        let mut actual = Vec::new();
        let mut large = vec![0u8; 2 * WINDOW];
        let first = reader.read(&mut large).unwrap();
        assert!(first <= WINDOW);
        actual.extend_from_slice(&large[..first]);
        reader.read_to_end(&mut actual).unwrap();
        assert_eq!(actual, bytes);
        assert_eq!(
            completion.outcome(),
            StreamOutcome::Complete {
                bytes: bytes.len() as u64,
                sha256: digest(&bytes),
            }
        );
        drop(reader);
        assert!(matches!(
            completion.outcome(),
            StreamOutcome::Complete { .. }
        ));
    }
}

#[test]
fn oversize_and_wrong_expected_bytes_refuse_before_reader() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("asset"), b"payload").unwrap();
    let project = Project::open(root.path()).unwrap();
    assert!(
        project
            .open_verified_stream_bounded("asset", &digest(b"payload"), 7, 6)
            .is_err()
    );
    assert!(
        project
            .open_verified_stream_bounded("asset", &digest(b"wrong"), 7, 7)
            .is_err()
    );
    assert!(
        project
            .open_verified_stream_bounded("asset", &digest(b"payload"), 6, 7)
            .is_err()
    );
}

#[cfg(unix)]
#[test]
fn source_symlink_hardlink_and_preflight_drift_refuse() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("asset"), b"payload").unwrap();
    std::os::unix::fs::symlink("asset", root.path().join("link")).unwrap();
    std::fs::hard_link(root.path().join("asset"), root.path().join("alias")).unwrap();
    let project = Project::open(root.path()).unwrap();
    for name in ["link", "alias"] {
        assert!(
            project
                .open_verified_stream_bounded(name, &digest(b"payload"), 7, 7)
                .is_err()
        );
    }
    std::fs::remove_file(root.path().join("alias")).unwrap();
    let path = root.path().join("asset");
    crate::race_hook::arm_between_stream_passes(Some(Box::new(move |_, _| {
        std::fs::write(&path, b"changed").unwrap();
    })));
    assert!(
        project
            .open_verified_stream_bounded("asset", &digest(b"payload"), 7, 7)
            .is_err()
    );
}

#[test]
fn dropping_reader_before_full_body_never_completes() {
    let root = tempfile::tempdir().unwrap();
    let bytes = vec![0x5a; 2 * WINDOW];
    std::fs::write(root.path().join("asset"), &bytes).unwrap();
    let project = Project::open(root.path()).unwrap();
    let (mut reader, completion) = project
        .open_verified_stream_bounded(
            "asset",
            &digest(&bytes),
            bytes.len() as u64,
            bytes.len() as u64,
        )
        .unwrap();
    let mut window = [0_u8; 5];
    assert_eq!(reader.read(&mut window).unwrap(), 5);
    drop(reader);
    assert!(matches!(completion.outcome(), StreamOutcome::Uncertain(_)));
}

#[test]
fn reader_satisfies_streaming_body_thread_and_lifetime_bounds() {
    fn require_body_reader<R: Read + Send + 'static>(_: R) {}
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("asset"), b"payload").unwrap();
    let project = Project::open(root.path()).unwrap();
    let (reader, completion) = project
        .open_verified_stream_bounded("asset", &digest(b"payload"), 7, 7)
        .unwrap();
    require_body_reader(reader);
    assert!(matches!(completion.outcome(), StreamOutcome::Uncertain(_)));
}

#[cfg(unix)]
#[test]
fn in_place_mutation_during_stream_is_uncertain() {
    let root = tempfile::tempdir().unwrap();
    let bytes = vec![0x5a; 2 * WINDOW];
    let path = root.path().join("asset");
    std::fs::write(&path, &bytes).unwrap();
    let project = Project::open(root.path()).unwrap();
    let (mut reader, completion) = project
        .open_verified_stream_bounded(
            "asset",
            &digest(&bytes),
            bytes.len() as u64,
            bytes.len() as u64,
        )
        .unwrap();
    let mut first = [0u8; WINDOW];
    assert_eq!(reader.read(&mut first).unwrap(), WINDOW);
    std::fs::write(path, vec![0x41; bytes.len()]).unwrap();
    assert!(reader.read_to_end(&mut Vec::new()).is_err());
    assert!(matches!(completion.outcome(), StreamOutcome::Uncertain(_)));
}

#[cfg(unix)]
#[test]
fn short_growth_and_final_name_swap_are_uncertain() {
    for change in 0..3 {
        let root = tempfile::tempdir().unwrap();
        let bytes = vec![0x5a; 2 * WINDOW];
        let path = root.path().join("asset");
        std::fs::write(&path, &bytes).unwrap();
        let project = Project::open(root.path()).unwrap();
        let (mut reader, completion) = project
            .open_verified_stream_bounded(
                "asset",
                &digest(&bytes),
                bytes.len() as u64,
                bytes.len() as u64,
            )
            .unwrap();
        let mut first = [0u8; WINDOW];
        assert_eq!(reader.read(&mut first).unwrap(), WINDOW);
        match change {
            0 => std::fs::OpenOptions::new()
                .write(true)
                .open(&path)
                .unwrap()
                .set_len((WINDOW - 1) as u64)
                .unwrap(),
            1 => std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .unwrap()
                .write_all(b"extra")
                .unwrap(),
            _ => {
                std::fs::rename(&path, root.path().join("old")).unwrap();
                std::fs::write(&path, &bytes).unwrap();
            }
        }
        assert!(reader.read_to_end(&mut Vec::new()).is_err());
        assert!(matches!(completion.outcome(), StreamOutcome::Uncertain(_)));
    }
}
