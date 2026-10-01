use super::*;
use crate::process::minimal_environment;
use std::io::{Read, Write};
use std::process::{Child, Command};
use std::sync::mpsc;
use std::time::Instant;

const HELPER: &str = "process::observed::tests::native_capture_helper";
const FLOOD: usize = STREAM_CAP + 64 * 1024;
const INPUT: usize = 3 * STREAM_CAP;

fn spec(dir: &std::path::Path, case: &str) -> ProcessSpec {
    ProcessSpec {
        program: std::env::current_exe().unwrap(),
        args: vec![
            "--exact".into(),
            HELPER.into(),
            "--nocapture".into(),
            "--test-threads=1".into(),
        ],
        cwd: dir.into(),
        env: minimal_environment([("VIBE_OBSERVED_HELPER".into(), case.into())]),
        stdin: None,
        stdout: StreamMode::Capture,
        stderr: StreamMode::Capture,
        scratch: dir.into(),
    }
}
fn observed(spec: ProcessSpec, options: ObservedCapture) -> ObservedOutcome {
    std::thread::spawn(move || SystemProcessRunner.run_observed(&spec, options).unwrap())
        .join()
        .unwrap()
}

/// Inert without a private environment guard. Parent commands select this full
/// name exactly, so another inert helper cannot yield a false green child exit.
#[test]
fn native_capture_helper() {
    let Ok(case) = std::env::var("VIBE_OBSERVED_HELPER") else {
        return;
    };
    // A deadlock produces a non-success child exit within a finite deadline.
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(8));
        std::process::exit(91);
    });
    match case.as_str() {
        "flood" => {
            let out =
                std::thread::spawn(|| std::io::stdout().write_all(&vec![b'o'; FLOOD]).unwrap());
            let err =
                std::thread::spawn(|| std::io::stderr().write_all(&vec![b'e'; FLOOD]).unwrap());
            let mut input = Vec::new();
            std::io::stdin().read_to_end(&mut input).unwrap();
            out.join().unwrap();
            err.join().unwrap();
            std::process::exit(if input.len() == INPUT { 23 } else { 92 });
        }
        "short" => {
            std::io::stderr().write_all(b"exact-error").unwrap();
            std::process::exit(17);
        }
        "success" => {
            std::io::stderr().write_all(b"success").unwrap();
            std::process::exit(0);
        }
        "exit-before-input" => std::process::exit(19),
        "direct" => {
            let mut command = Command::new(std::env::current_exe().unwrap());
            command
                .args(["--exact", HELPER, "--nocapture", "--test-threads=1"])
                .env("VIBE_OBSERVED_HELPER", "descendant")
                .stdin(Stdio::null());
            let child = command.spawn().unwrap();
            // Direct helper intentionally leaves inherited handles with its
            // bounded descendant; the test waits for descendant-done explicitly.
            drop(child); // std Child drop leaves this intentional descendant running.
            let deadline = Instant::now() + Duration::from_secs(2);
            while !std::path::Path::new("descendant-started").exists() && Instant::now() < deadline
            {
                std::thread::sleep(Duration::from_millis(5));
            }
            std::process::exit(if std::path::Path::new("descendant-started").exists() {
                29
            } else {
                93
            });
        }
        "descendant" => {
            let child = Command::new(std::env::current_exe().unwrap())
                .args(["--exact", HELPER, "--nocapture", "--test-threads=1"])
                .env("VIBE_OBSERVED_HELPER", "holder")
                .stdin(Stdio::null())
                .spawn()
                .unwrap();
            let mut holder = OwnedHelper(child);
            let status = holder.0.wait().unwrap();
            std::fs::write("descendant-done", b"done").unwrap();
            std::process::exit(if status.success() { 0 } else { 94 });
        }
        "holder" => {
            std::fs::write("descendant-started", b"ready").unwrap();
            let deadline = Instant::now() + Duration::from_secs(3);
            while !std::path::Path::new("descendant-finish").exists() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(5));
            }
            std::process::exit(0);
        }
        other => panic!("unknown helper case {other}"),
    }
}

fn flood_spec(dir: &std::path::Path) -> ProcessSpec {
    let mut spec = spec(dir, "flood");
    spec.stdin = Some(vec![b'i'; INPUT]);
    spec
}
fn assert_flood(outcome: &ObservedOutcome) {
    assert!(outcome.exit_observed);
    assert_eq!(outcome.output.code, Some(23));
    assert!(outcome.faults.is_empty(), "{:?}", outcome.faults);
    for (bytes, evidence) in [
        (&outcome.output.stdout, &outcome.stdout),
        (&outcome.output.stderr, &outcome.stderr),
    ] {
        assert_eq!(bytes.len(), STREAM_CAP);
        assert!(evidence.drained_bytes >= FLOOD as u64); // stdout includes test-harness preamble.
        assert_eq!(evidence.retained_bytes, STREAM_CAP as u64);
        assert_eq!(evidence.spooled_bytes, STREAM_CAP as u64);
        assert!(evidence.eof && evidence.truncated && evidence.counters_complete);
        assert!(evidence.faults.is_empty(), "{:?}", evidence.faults);
        let path = evidence.spool_path.as_ref().unwrap();
        assert_eq!(std::fs::metadata(path).unwrap().len(), STREAM_CAP as u64);
        assert_eq!(std::fs::read(path).unwrap(), *bytes);
    }
    assert_eq!(outcome.stderr.drained_bytes, FLOOD as u64);
    assert_eq!(outcome.output.stderr, vec![b'e'; STREAM_CAP]);
}

#[test]
fn flood_both_streams_with_large_stdin_preserves_bounded_prefix_and_offsets() {
    let dir = tempfile::tempdir().unwrap();
    // Finite enough for every <=2048-byte record in this finite helper oracle.
    let (sender, receiver) = mpsc::sync_channel(8192);
    let start = Instant::now();
    let outcome = observed(
        flood_spec(dir.path()),
        ObservedCapture {
            records: Some(sender),
            ..Default::default()
        },
    );
    assert!(start.elapsed() < Duration::from_secs(8));
    assert_flood(&outcome);
    let mut offsets = [0u64; 2];
    for record in receiver.try_iter() {
        let index = if record.origin == CaptureOrigin::Stdout {
            0
        } else {
            1
        };
        assert_eq!(record.byte_offset, offsets[index]);
        assert!(!record.bytes.is_empty() && record.bytes.len() <= CAPTURE_CHUNK_CAP);
        if index == 1 {
            assert!(record.bytes.iter().all(|byte| *byte == b'e'));
        }
        offsets[index] += record.bytes.len() as u64;
    }
    assert_eq!(
        offsets,
        [outcome.stdout.drained_bytes, outcome.stderr.drained_bytes]
    );
    assert_eq!(outcome.stdout.delivered_bytes, offsets[0]);
    assert_eq!(outcome.stderr.delivered_bytes, offsets[1]);
}

#[test]
fn full_and_disconnected_consumers_do_not_hold_or_cancel_child() {
    for disconnected in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let (sender, receiver) = mpsc::sync_channel(1);
        let receiver = if disconnected {
            drop(receiver);
            None
        } else {
            Some(receiver)
        };
        let start = Instant::now();
        let outcome = observed(
            flood_spec(dir.path()),
            ObservedCapture {
                records: Some(sender),
                ..Default::default()
            },
        );
        assert!(start.elapsed() < Duration::from_secs(8));
        assert_flood(&outcome);
        for evidence in [&outcome.stdout, &outcome.stderr] {
            assert!(evidence.dropped_delivery_bytes > 0);
            assert_eq!(
                evidence.delivered_bytes + evidence.dropped_delivery_bytes,
                evidence.drained_bytes
            );
            assert!(evidence.delivery_loss_ranges.len() <= LOSS_RANGE_CAP);
            assert!(!evidence.delivery_loss_ranges_truncated);
            assert_eq!(
                evidence
                    .delivery_loss_ranges
                    .iter()
                    .map(|range| range.end - range.start)
                    .sum::<u64>(),
                evidence.dropped_delivery_bytes
            );
            if disconnected {
                assert_eq!(evidence.delivered_bytes, 0);
            }
        }
        drop(receiver);
    }
}

#[test]
fn spool_fault_preserves_actual_exit_and_retained_capture() {
    let dir = tempfile::tempdir().unwrap();
    let mut spec = spec(dir.path(), "short");
    let blocked = dir.path().join("file-not-directory");
    std::fs::write(&blocked, b"block").unwrap();
    spec.scratch = blocked;
    let outcome = observed(spec, ObservedCapture::default());
    assert!(outcome.exit_observed);
    assert_eq!(outcome.output.code, Some(17));
    assert_eq!(outcome.output.stderr, b"exact-error");
    assert!(outcome.stderr.eof);
    assert_eq!(outcome.stderr.spooled_bytes, 0);
    assert_eq!(
        outcome.stderr.faults[0].stage,
        CaptureFaultStage::SpoolCreate
    );
}

#[test]
fn success_removes_spools_and_non_capture_modes_fabricate_no_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let outcome = observed(spec(dir.path(), "success"), ObservedCapture::default());
    assert_eq!(outcome.output.code, Some(0));
    for evidence in [&outcome.stdout, &outcome.stderr] {
        assert!(evidence.spool_removed);
        assert!(!evidence.spool_path.as_ref().unwrap().exists());
    }
    let mut spec = spec(dir.path(), "short");
    spec.stdout = StreamMode::Inherit;
    spec.stderr = StreamMode::Null;
    let outcome = observed(spec, ObservedCapture::default());
    assert_eq!(outcome.output.code, Some(17));
    for evidence in [&outcome.stdout, &outcome.stderr] {
        assert!(!evidence.captured && !evidence.eof && !evidence.truncated);
        assert_eq!(evidence.drained_bytes, 0);
        assert!(evidence.spool_path.is_none());
    }
    assert!(outcome.output.stdout.is_empty() && outcome.output.stderr.is_empty());
}

#[test]
fn bounded_pre_spawn_refusals_are_distinct_from_effectful_outcomes() {
    let dir = tempfile::tempdir().unwrap();
    let spec = spec(dir.path(), "short");
    assert!(matches!(
        SystemProcessRunner.run_observed(
            &spec,
            ObservedCapture {
                post_exit_grace: Duration::ZERO,
                ..Default::default()
            }
        ),
        Err(ObservedRefusal::InvalidGrace)
    ));
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        assert!(matches!(
            SystemProcessRunner.run_observed(&spec, ObservedCapture::default()),
            Err(ObservedRefusal::NestedRuntime)
        ));
    });
    let mut missing = spec;
    missing.program = dir.path().join("missing-executable");
    assert!(matches!(
        SystemProcessRunner.run_observed(&missing, ObservedCapture::default()),
        Err(ObservedRefusal::Spawn(_))
    ));
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

// RAII completion handshake keeps the bounded descendant alive until the
// deadline assertion and explicitly waits for its final marker before teardown.
struct DescendantFinish(PathBuf);
impl Drop for DescendantFinish {
    fn drop(&mut self) {
        let _ = std::fs::write(self.0.join("descendant-finish"), b"finish");
        let deadline = Instant::now() + Duration::from_secs(4);
        while !self.0.join("descendant-done").exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}
#[test]
fn descendant_held_pipes_return_after_grace_with_incomplete_eof() {
    let dir = tempfile::tempdir().unwrap();
    let finish = DescendantFinish(dir.path().into());
    let start = Instant::now();
    let outcome = observed(
        spec(dir.path(), "direct"),
        ObservedCapture {
            post_exit_grace: Duration::from_millis(100),
            ..Default::default()
        },
    );
    assert_eq!(outcome.output.code, Some(29));
    assert!(outcome.exit_observed);
    assert!(start.elapsed() < Duration::from_secs(2));
    assert!(!outcome.stdout.eof && !outcome.stderr.eof);
    assert!(outcome.stdout.truncated && outcome.stderr.truncated);
    drop(finish);
    assert!(dir.path().join("descendant-done").exists());
}

#[test]
fn child_exit_closes_blocked_stdin_writer_and_preserves_status() {
    let dir = tempfile::tempdir().unwrap();
    let mut spec = spec(dir.path(), "exit-before-input");
    spec.stdin = Some(vec![b'i'; INPUT]);
    let outcome = observed(spec, ObservedCapture::default());
    assert!(outcome.exit_observed);
    assert_eq!(outcome.output.code, Some(19));
    assert!(
        outcome
            .faults
            .iter()
            .any(|fault| fault.stage == CaptureFaultStage::Stdin)
    );
}

// The descendant guardian owns and reaps its holder, even on unwind.
struct OwnedHelper(Child);
impl Drop for OwnedHelper {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
