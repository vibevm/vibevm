use super::*;
use std::ffi::OsString;

#[test]
fn selected_manifest_lock_witness_reads_exact_bytes_without_writing() {
    let dir = tempfile::tempdir().unwrap();
    let manifest = b"[package]\nname = 'sample'\n";
    let lock = b"[meta]\nversion = 1\n";
    std::fs::write(dir.path().join("vibe.toml"), manifest).unwrap();
    std::fs::write(dir.path().join("vibe.lock"), lock).unwrap();
    let before = std::fs::read_dir(dir.path()).unwrap().count();
    let witness = capture_selected_manifest_lock(dir.path()).unwrap();
    assert_eq!(witness.manifest.bytes, manifest);
    assert_eq!(witness.manifest.size, manifest.len() as u64);
    assert_eq!(witness.manifest.sha256.len(), "sha256:".len() + 64);
    assert_eq!(witness.lock.as_ref().unwrap().bytes, lock);
    witness.validate().unwrap();
    recheck_selected_manifest_lock(dir.path(), &witness).unwrap();
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), before);
    assert!(!dir.path().join(".vibe").exists());
}

#[test]
fn selected_witness_validation_refuses_same_shape_wrong_digest_and_size() {
    let mut witness = SelectedManifestLockWitness {
        manifest: SelectedFileWitness::from_bytes(b"manifest".to_vec()),
        lock: Some(SelectedFileWitness::from_bytes(b"lock".to_vec())),
    };
    witness.validate().unwrap();
    witness.manifest.sha256 = format!("sha256:{}", "a".repeat(64));
    assert!(matches!(
        witness.validate(),
        Err(SourceSnapshotError::InvalidWitness {
            name: "vibe.toml",
            ..
        })
    ));
    witness.manifest = SelectedFileWitness::from_bytes(b"manifest".to_vec());
    witness.lock.as_mut().unwrap().size += 1;
    assert!(matches!(
        witness.validate(),
        Err(SourceSnapshotError::InvalidWitness {
            name: "vibe.lock",
            ..
        })
    ));
}

#[test]
fn selected_manifest_lock_witness_refuses_manifest_drift() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("vibe.toml"), b"one").unwrap();
    let witness = capture_selected_manifest_lock(dir.path()).unwrap();
    std::fs::write(dir.path().join("vibe.toml"), b"two").unwrap();
    assert!(matches!(
        recheck_selected_manifest_lock(dir.path(), &witness),
        Err(SourceSnapshotError::Drift { name: "vibe.toml" })
    ));
}

#[test]
fn selected_manifest_lock_witness_refuses_lock_disappearance_and_reappearance() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("vibe.toml"), b"manifest").unwrap();
    let absent = capture_selected_manifest_lock(dir.path()).unwrap();
    assert!(absent.lock.is_none());
    std::fs::write(dir.path().join("vibe.lock"), b"lock").unwrap();
    assert!(matches!(
        recheck_selected_manifest_lock(dir.path(), &absent),
        Err(SourceSnapshotError::Drift { name: "vibe.lock" })
    ));
    let present = capture_selected_manifest_lock(dir.path()).unwrap();
    std::fs::remove_file(dir.path().join("vibe.lock")).unwrap();
    assert!(matches!(
        recheck_selected_manifest_lock(dir.path(), &present),
        Err(SourceSnapshotError::Drift { name: "vibe.lock" })
    ));
}

#[test]
fn selected_manifest_lock_witness_refuses_oversize_file() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("vibe.toml"),
        vec![b'x'; 8 * 1024 * 1024 + 1],
    )
    .unwrap();
    assert!(matches!(
        capture_selected_manifest_lock(dir.path()),
        Err(SourceSnapshotError::Read {
            name: "vibe.toml",
            ..
        })
    ));
}

#[cfg(unix)]
#[test]
fn selected_manifest_lock_witness_refuses_symlink() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("real.toml"), b"manifest").unwrap();
    std::os::unix::fs::symlink("real.toml", dir.path().join("vibe.toml")).unwrap();
    assert!(matches!(
        capture_selected_manifest_lock(dir.path()),
        Err(SourceSnapshotError::Read {
            name: "vibe.toml",
            ..
        })
    ));
}

#[test]
fn run_id_and_key_components_are_fixed_hex_lengths() {
    let dir = tempfile::tempdir().unwrap();
    let run = allocate_run_id(dir.path()).unwrap();
    assert_eq!(run.len(), 32);
    let scratch = execution_scratch(dir.path(), &run, &"x".repeat(10_000)).unwrap();
    assert_eq!(scratch.file_name().unwrap().to_string_lossy().len(), 64);
    let upper = execution_scratch(dir.path(), &"A".repeat(32), "x").unwrap_err();
    assert!(upper.to_string().contains("lowercase hex"));
}

#[test]
fn proposed_run_id_is_pure_and_reserved_scratch_is_explicit() {
    let dir = tempfile::tempdir().unwrap();
    let id = propose_run_id(dir.path()).unwrap();
    assert!(is_valid_run_id(&id));
    assert!(!dir.path().join(".vibe").exists());
    let scratch = ensure_reserved_run_scratch(dir.path(), &id).unwrap();
    assert_eq!(scratch, dir.path().join(".vibe/lifecycle").join(&id));
    assert!(scratch.is_dir());
    assert_eq!(
        ensure_reserved_run_scratch(dir.path(), &id).unwrap(),
        scratch
    );
}

#[test]
fn reserved_run_scratch_refuses_invalid_id_and_foreign_shape() {
    let dir = tempfile::tempdir().unwrap();
    assert!(ensure_reserved_run_scratch(dir.path(), &"A".repeat(32)).is_err());
    assert!(!dir.path().join(".vibe").exists());
    let id = propose_run_id(dir.path()).unwrap();
    let base = dir.path().join(".vibe/lifecycle");
    std::fs::create_dir_all(&base).unwrap();
    std::fs::write(base.join(&id), b"foreign file").unwrap();
    assert!(ensure_reserved_run_scratch(dir.path(), &id).is_err());
    std::fs::remove_file(base.join(&id)).unwrap();
    #[cfg(unix)]
    {
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), base.join(&id)).unwrap();
        assert!(ensure_reserved_run_scratch(dir.path(), &id).is_err());
    }
}

#[test]
fn captured_stream_reader_stops_at_cap_plus_one() {
    let dir = tempfile::tempdir().unwrap();
    let stream = dir.path().join("stream");
    std::fs::write(&stream, vec![b'x'; STREAM_CAP + 4096]).unwrap();
    let mut capture = CaptureFile {
        path: stream.clone(),
        file: std::fs::File::open(&stream).unwrap(),
    };
    let (bytes, truncated) = read_capped(StreamMode::Capture, Some(&mut capture), false).unwrap();
    assert_eq!(bytes.len(), STREAM_CAP);
    assert!(truncated);
}

#[test]
fn minimal_environment_excludes_token_genre() {
    let env = minimal_environment([("VIBE_CONTEXT".into(), "x".into())]);
    assert!(env.contains_key(&OsString::from("VIBE_CONTEXT")));
    assert!(
        !env.keys()
            .any(|key| key.to_string_lossy().contains("TOKEN"))
    );
}

#[test]
fn client_environment_is_clean_and_uses_only_the_injected_home() {
    let home = std::path::Path::new("/isolated/client-home");
    let claude = home.join(".claude");
    let env = client_environment(home, Some(("CLAUDE_CONFIG_DIR", &claude)));

    assert_eq!(
        env.get(&OsString::from("HOME")).map(OsString::as_os_str),
        Some(home.as_os_str())
    );
    assert_eq!(
        env.get(&OsString::from("USERPROFILE"))
            .map(OsString::as_os_str),
        Some(home.as_os_str())
    );
    assert_eq!(
        env.get(&OsString::from("CLAUDE_CONFIG_DIR"))
            .map(OsString::as_os_str),
        Some(claude.as_os_str())
    );
    for forbidden in [
        "PATH",
        "CODEX_HOME",
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
    ] {
        assert!(!env.contains_key(&OsString::from(forbidden)), "{forbidden}");
    }
    assert!(env.keys().all(|key| {
        matches!(
            key.to_string_lossy().as_ref(),
            "SystemRoot"
                | "WINDIR"
                | "TEMP"
                | "TMP"
                | "LANG"
                | "LC_ALL"
                | "HOME"
                | "USERPROFILE"
                | "CLAUDE_CONFIG_DIR"
        )
    }));
}

#[test]
fn atomic_json_uses_a_unique_create_new_file_and_ignores_a_planted_name() {
    let dir = tempfile::tempdir().unwrap();
    let run = allocate_run_id(dir.path()).unwrap();
    let scratch = execution_scratch(dir.path(), &run, "link-test").unwrap();
    let outside = dir.path().join("outside.json");
    std::fs::write(&outside, b"sentinel").unwrap();
    let planted = scratch.join("reply.json");
    std::fs::hard_link(&outside, &planted).unwrap();
    let published =
        write_atomic_json(&scratch, "reply.json", &serde_json::json!({"good":true})).unwrap();
    assert_ne!(published, planted);
    assert!(published.starts_with(&scratch));
    assert!(std::fs::symlink_metadata(&published).unwrap().is_file());
    assert_eq!(std::fs::read(&outside).unwrap(), b"sentinel");
}

#[test]
fn unique_scratch_genre_cannot_escape_verified_scratch() {
    let dir = tempfile::tempdir().unwrap();
    let run = allocate_run_id(dir.path()).unwrap();
    let scratch = execution_scratch(dir.path(), &run, "genre").unwrap();
    assert!(super::scratch::create_unique_file(&scratch, "../escape").is_err());
    assert!(!scratch.parent().unwrap().join("escape").exists());
}

#[test]
#[cfg(unix)]
fn pending_reply_reads_the_create_new_handle_not_a_replaced_link() {
    let dir = tempfile::tempdir().unwrap();
    let run = allocate_run_id(dir.path()).unwrap();
    let scratch = execution_scratch(dir.path(), &run, "pending-link-test").unwrap();
    let outside = dir.path().join("outside-reply.json");
    std::fs::write(&outside, br#"{"envelope":1}"#).unwrap();
    let mut pending = allocate_pending_reply(&scratch).unwrap();
    let path = pending.path().to_path_buf();
    std::fs::remove_file(&path).unwrap();
    std::os::unix::fs::symlink(&outside, &path).unwrap();
    assert!(pending.read_capped(1024).is_err());
    assert_eq!(std::fs::read(&outside).unwrap(), br#"{"envelope":1}"#);
}

#[test]
#[cfg(windows)]
#[ignore = "requires Windows symlink/reparse privilege (worker host returned Win32 1314)"]
fn pending_reply_refuses_a_replaced_windows_reparse_file() {
    let dir = tempfile::tempdir().unwrap();
    let run = allocate_run_id(dir.path()).unwrap();
    let scratch = execution_scratch(dir.path(), &run, "pending-reparse-test").unwrap();
    let outside = dir.path().join("outside-reply.json");
    std::fs::write(&outside, br#"{"envelope":1}"#).unwrap();
    let mut pending = allocate_pending_reply(&scratch).unwrap();
    pending.publish();
    let path = pending.path().to_path_buf();
    std::fs::remove_file(&path).unwrap();
    std::os::windows::fs::symlink_file(&outside, &path)
        .expect("Windows reparse oracle requires Developer Mode symlink privilege");
    assert!(pending.read_capped(1024).is_err());
    assert_eq!(std::fs::read(&outside).unwrap(), br#"{"envelope":1}"#);
}

#[test]
fn stdin_write_failure_kills_and_reaps_before_returning() {
    let dir = tempfile::tempdir().unwrap();
    let run = allocate_run_id(dir.path()).unwrap();
    let scratch = execution_scratch(dir.path(), &run, "stdin-reap").unwrap();
    let marker = dir.path().join("late-side-effect");
    let program = "python".into();
    let args = vec![
        "-c".into(),
        format!(
            "import os,time,pathlib; os.close(0); time.sleep(.5); pathlib.Path(r'{}').write_text('late'); time.sleep(3)",
            marker.display()
        )
        .into(),
    ];
    let error = SystemProcessRunner
        .run(&ProcessSpec {
            program,
            args,
            cwd: dir.path().into(),
            env: minimal_environment(Vec::<(String, String)>::new()),
            stdin: Some(vec![b'x'; 8 * 1024 * 1024]),
            stdout: StreamMode::Null,
            stderr: StreamMode::Null,
            scratch,
        })
        .unwrap_err();
    assert!(matches!(error, ProcessError::Stdin(_)), "{error}");
    std::thread::sleep(std::time::Duration::from_millis(1000));
    assert!(
        !marker.exists(),
        "child continued after stdin transport failure"
    );
}
