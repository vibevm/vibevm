use super::*;
use std::ffi::OsString;

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
    // Unix distributions expose Python 3 under its versioned executable name.
    let program = if cfg!(windows) { "python" } else { "python3" }.into();
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

#[cfg(unix)]
#[test]
fn a_busy_executable_is_retried_until_it_spawns_and_the_last_failure_is_kept() {
    use std::cell::Cell;
    use std::time::Duration;
    let busy = || std::io::Error::from_raw_os_error(26);
    let calls = Cell::new(0);
    let spawned = spawn_retrying_busy(20, Duration::ZERO, || {
        calls.set(calls.get() + 1);
        if calls.get() < 3 {
            Err(busy())
        } else {
            Ok("child")
        }
    });
    assert_eq!(spawned.unwrap(), "child");
    assert_eq!(calls.get(), 3, "two busy failures, then the spawn");

    calls.set(0);
    let exhausted = spawn_retrying_busy(4, Duration::ZERO, || -> std::io::Result<()> {
        calls.set(calls.get() + 1);
        Err(busy())
    });
    assert_eq!(calls.get(), 4, "bounded by the attempt count");
    let error = exhausted.unwrap_err();
    assert_eq!(error.raw_os_error(), Some(26));
    assert_eq!(
        error.to_string(),
        busy().to_string(),
        "the text is unchanged"
    );

    calls.set(0);
    let missing = spawn_retrying_busy(20, Duration::ZERO, || -> std::io::Result<()> {
        calls.set(calls.get() + 1);
        Err(std::io::Error::from(std::io::ErrorKind::NotFound))
    });
    assert_eq!(missing.unwrap_err().kind(), std::io::ErrorKind::NotFound);
    assert_eq!(calls.get(), 1, "any other failure is reported at once");
}

#[test]
fn discarding_a_run_removes_only_its_scratch_and_keeps_a_delegated_one() {
    let dir = tempfile::tempdir().unwrap();
    let kept = allocate_run_id(dir.path()).unwrap();
    let stopped = allocate_run_id(dir.path()).unwrap();
    let scratch = execution_scratch(dir.path(), &stopped, "slot").unwrap();
    write_atomic_json(&scratch, "context.json", &serde_json::json!({})).unwrap();
    discard_run_scratch(dir.path(), &stopped).unwrap();
    let base = dir.path().join(".vibe/lifecycle");
    assert!(!base.join(&stopped).exists());
    assert!(base.join(&kept).is_dir(), "another run's scratch stays");

    let parked = allocate_run_id(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join(".vibe/agentic/outbox").join(&parked)).unwrap();
    discard_run_scratch(dir.path(), &parked).unwrap();
    assert!(
        base.join(&parked).is_dir(),
        "a delegated run keeps its scratch"
    );

    std::fs::remove_dir(base.join(&kept)).unwrap();
    std::fs::remove_dir(base.join(&parked)).unwrap();
    let last = allocate_run_id(dir.path()).unwrap();
    discard_run_scratch(dir.path(), &last).unwrap();
    assert!(!base.exists(), "an emptied lifecycle directory goes too");
    assert!(discard_run_scratch(dir.path(), "NOT-HEX").is_err());
}
