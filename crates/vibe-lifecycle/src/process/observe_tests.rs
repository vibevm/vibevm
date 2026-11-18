//! Real process capture: full persistent bytes remain independent of capped replies.
use super::*;
use std::sync::{Arc, Mutex};
#[derive(Default)]
struct Seen {
    streams: Mutex<[Vec<u8>; 2]>,
    complete: Mutex<Option<bool>>,
}
impl ProcessOutputObserver for Seen {
    fn bytes(&self, stream: OutputStream, bytes: &[u8]) {
        self.streams.lock().unwrap()[usize::from(stream == OutputStream::Stderr)]
            .extend_from_slice(bytes);
    }
    fn finished(&self, complete: bool) {
        *self.complete.lock().unwrap() = Some(complete);
    }
}
fn spec(root: &std::path::Path, script: &str) -> ProcessSpec {
    ProcessSpec {
        program: if cfg!(windows) { "python" } else { "python3" }.into(),
        args: vec!["-I".into(), "-c".into(), script.into()],
        cwd: root.into(),
        env: minimal_environment([]),
        stdin: None,
        stdout: StreamMode::Capture,
        stderr: StreamMode::Capture,
        scratch: root.into(),
    }
}
#[test]
fn both_runners_observe_all_bytes_beyond_preview_caps() {
    for controlled in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let seen = Arc::new(Seen::default());
        let request = spec(
            root.path(),
            "import os; os.write(1,b'O'*(2*1024*1024)+b'OUT-END'); os.write(2,b'E'*(2*1024*1024)+b'ERR-END')",
        );
        let output = if controlled {
            ControlledProcessRunner::new(vibe_core::cancellation::CancellationToken::new())
                .with_output_observer(Some(seen.clone()))
                .run(&request)
        } else {
            SystemProcessRunner.run_observed(&request, Some(seen.clone()))
        }
        .unwrap();
        assert_eq!(output.code, Some(0));
        assert_eq!(output.stdout.len(), STREAM_CAP);
        assert_eq!(output.stderr.len(), STREAM_CAP);
        assert!(output.stdout_truncated && output.stderr_truncated);
        let streams = seen.streams.lock().unwrap();
        assert_eq!(streams[0].len(), 2 * 1024 * 1024 + 7);
        assert!(streams[0].ends_with(b"OUT-END"));
        assert_eq!(streams[1].len(), 2 * 1024 * 1024 + 7);
        assert!(streams[1].ends_with(b"ERR-END"));
        assert_eq!(*seen.complete.lock().unwrap(), Some(true));
    }
}
#[test]
fn observation_is_live_while_large_stdin_is_still_being_consumed() {
    let root = tempfile::tempdir().unwrap();
    let seen = Arc::new(Seen::default());
    let inspect = seen.clone();
    let mut request = spec(
        root.path(),
        "import os,time; os.write(1,b'EARLY'); time.sleep(.5); data=b'x';\nwhile data: data=os.read(0,65536)\nos.write(2,b'FINAL')",
    );
    request.stdin = Some(vec![1; 4 * 1024 * 1024]);
    let worker = std::thread::spawn(move || SystemProcessRunner.run_observed(&request, Some(seen)));
    let until = std::time::Instant::now() + Duration::from_secs(3);
    while inspect.streams.lock().unwrap()[0].is_empty() && std::time::Instant::now() < until {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(inspect.streams.lock().unwrap()[0], b"EARLY");
    assert!(!worker.is_finished());
    assert_eq!(worker.join().unwrap().unwrap().code, Some(0));
    assert_eq!(inspect.streams.lock().unwrap()[1], b"FINAL");
}
#[test]
fn spawn_failure_never_claims_complete_observation() {
    let root = tempfile::tempdir().unwrap();
    let seen = Arc::new(Seen::default());
    let mut request = spec(root.path(), "");
    request.program = root.path().join("does-not-exist");
    assert!(
        SystemProcessRunner
            .run_observed(&request, Some(seen.clone()))
            .is_err()
    );
    assert_eq!(*seen.complete.lock().unwrap(), Some(false));
}
#[cfg(unix)]
#[test]
fn cancellation_reaps_group_and_final_drains_partial_output() {
    let root = tempfile::tempdir().unwrap();
    let seen = Arc::new(Seen::default());
    let token = vibe_core::cancellation::CancellationToken::new();
    let stop = token.clone();
    let cancel = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(250));
        stop.cancel();
    });
    let mut request = spec(
        root.path(),
        "import os,time; os.write(1,b'BEFORE-STOP'); time.sleep(30)",
    );
    request.stdin = Some(vec![1; 4 * 1024 * 1024]);
    let begin = std::time::Instant::now();
    assert!(
        ControlledProcessRunner::new(token)
            .with_output_observer(Some(seen.clone()))
            .run(&request)
            .is_err()
    );
    cancel.join().unwrap();
    assert!(begin.elapsed() < Duration::from_secs(5));
    assert_eq!(seen.streams.lock().unwrap()[0], b"BEFORE-STOP");
    assert_eq!(*seen.complete.lock().unwrap(), Some(true));
}
