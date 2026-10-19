use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use vibe_wire::generated::{
    boot_debugger::e1::{request as q, result as r},
    machine::e1::operation_envelope as env,
};
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../vibe-workspace/src/boot_debugger/m_06_f_2_fixture.rs"
));
fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("vibe.toml"),
        "[project]\nname='scale'\nversion='1.0.0'\n",
    )
    .unwrap();
    std::fs::write(root.path().join("STATIC.md"), scale_static()).unwrap();
    std::fs::write(root.path().join("INDEX.md"), SCALE_INDEX).unwrap();
    root
}
fn read_result(reader: &mut impl BufRead) -> r::Result {
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    let envelope: env::OperationEnvelope = serde_json::from_str(&line).unwrap();
    let env::Body::Result(body) = envelope.body else {
        panic!("{envelope:?}")
    };
    assert_eq!(body.effect_state, env::BodyResultEffectState::NoEffects);
    serde_json::from_str(body.payload_json.as_deref().unwrap()).unwrap()
}
#[test]
fn m_06_f_2_cases_interactive_line_ui_keeps_large_capture_and_canonical_bytes() {
    let root = fixture();
    let before = std::fs::read(root.path().join("STATIC.md")).unwrap();
    let output = vibe_test_support::vibe()
        .args(["boot", "--root"])
        .arg(root.path())
        .arg("--view")
        .write_stdin("select 127\nsource 0\ncost\nquit\n")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    assert!(text.contains("128 occurrences"));
    assert!(text.contains("127: 127:0"));
    assert!(text.contains("Captured bytes:"));
    assert!(text.contains("unavailable"));
    assert_eq!(
        std::fs::read(root.path().join("STATIC.md")).unwrap(),
        before
    );
    assert!(!root.path().join(".vibe/trace").exists());
    eprintln!(
        "m06f2 real vibe boot --view stdin: 128 rows, selection/source/cost, bytes preserved, units=1048576"
    );
}
#[test]
fn m_06_f_2_cases_structured_process_reads_frozen_chunks_after_live_change() {
    let root = fixture();
    // Link the incumbent test settings isolation before starting the real binary.
    let _isolated = vibe_test_support::vibe();
    let mut child = Command::new(env!("CARGO_BIN_EXE_vibe"))
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .args(["boot", "--root"])
        .arg(root.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let captured = read_result(&mut output);
    let id = captured.snapshot_id.unwrap();
    let send = |command, input: &mut std::process::ChildStdin| {
        let request = vibe_orchestrator::machine_operation::boot_debugger::input(&q::Request {
            schema: 1,
            context_id: captured.context_id.clone(),
            command,
        });
        serde_json::to_writer(&mut *input, &request).unwrap();
        writeln!(input).unwrap();
        input.flush().unwrap();
    };
    send(
        q::Command::Inspect(Box::new(q::CommandInspect {
            snapshot_id: id.clone(),
        })),
        &mut input,
    );
    let view = read_result(&mut output);
    let r::Body::Inspect(view) = view.body else {
        panic!()
    };
    assert_eq!(view.nodes.len(), 128);
    let resource = view
        .resources
        .iter()
        .find(|r| r.path == "STATIC.md")
        .unwrap();
    std::fs::write(root.path().join("STATIC.md"), "new live source").unwrap();
    let baseline = scale_static();
    let mut joined = Vec::new();
    let mut offset = "0".to_string();
    let mut chunks = 0;
    loop {
        send(
            q::Command::Chunk(Box::new(q::CommandChunk {
                snapshot_id: id.clone(),
                resource_id: resource.resource_id.clone(),
                filter: q::CommandChunkFilter::Raw,
                offset,
                limit: 2048,
            })),
            &mut input,
        );
        let result = read_result(&mut output);
        let r::Body::Chunk(chunk) = result.body else {
            panic!()
        };
        assert!(chunk.hex_bytes.len() <= 4096);
        for pair in chunk.hex_bytes.as_bytes().chunks_exact(2) {
            joined.push(u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap());
        }
        chunks += 1;
        if let Some(next) = chunk.next_offset {
            offset = next;
        } else {
            break;
        }
    }
    assert_eq!(joined, baseline.as_bytes());
    drop(input);
    assert!(child.wait().unwrap().success());
    eprintln!(
        "m06f2 real structured CLI: result_bytes={} chunks={} chunk_limit=2048 old capture unchanged after live edit",
        baseline.len(),
        chunks
    );
}
#[test]
fn m_06_f_2_cases_large_capture_must_not_be_refused_by_diagnostic_size_gate() {
    let root = fixture();
    // Sparse UTF-8 resource exceeds the standard constructor's 64MiB read gate.
    // This is a regression obligation, never an expected-success refusal test.
    let file = std::fs::File::create(root.path().join("STATIC.md")).unwrap();
    file.set_len(64 * 1024 * 1024 + 1).unwrap();
    let output = vibe_test_support::vibe()
        .args(["boot", "--root"])
        .arg(root.path())
        .write_stdin("")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let captured = read_result(&mut output.as_slice());
    assert!(captured.snapshot_id.is_some());
    let r::Body::Capture(capture) = captured.body else {
        panic!("expected complete capture")
    };
    assert!(capture.resources.iter().any(|r| r.path == "STATIC.md"));
    assert_eq!(
        std::fs::metadata(root.path().join("STATIC.md"))
            .unwrap()
            .len(),
        64 * 1024 * 1024 + 1
    );
}

#[test]
fn m_06_f_2_cases_capture_policy_separates_editor_limits_and_aggregate_capture() {
    use vibe_orchestrator::document_analysis::{
        AnalysisLimits, DocumentAnalysisHost, ResourceDeclaration, ResourceRole,
    };
    let root = fixture();
    let declarations = || {
        vec![
            ResourceDeclaration {
                path: "vibe.toml".into(),
                role: ResourceRole::Manifest,
            },
            ResourceDeclaration {
                path: "STATIC.md".into(),
                role: ResourceRole::Opaque,
            },
        ]
    };
    let limits = AnalysisLimits {
        resource_bytes: 4,
        capture_bytes: 8,
        open_buffers: 1,
    };
    let editor =
        DocumentAnalysisHost::new(root.path(), "vibe.toml", declarations(), limits).unwrap();
    assert!(editor.client().unwrap().freeze().is_err());
    let boot =
        DocumentAnalysisHost::for_boot_capture(root.path(), "vibe.toml", declarations(), limits)
            .unwrap();
    let client = boot.client().unwrap();
    let capture = client.freeze().unwrap();
    let id = boot.resource("STATIC.md").unwrap();
    use vibe_orchestrator::document_analysis::{BufferIntent, ByteEncoding, DocumentAnalysisError};
    assert_eq!(
        client.open(
            1,
            id,
            1,
            ByteEncoding::Utf8,
            std::sync::Arc::from(&b"large buffer"[..]),
            BufferIntent::Replace
        ),
        Err(DocumentAnalysisError::ResourceLimit)
    );
    assert_eq!(
        capture.resource(id).unwrap().raw().unwrap(),
        scale_static().as_bytes()
    );
    std::fs::write(root.path().join("STATIC.md"), b"new").unwrap();
    assert_eq!(
        capture.resource(id).unwrap().raw().unwrap(),
        scale_static().as_bytes()
    );
    let next = boot.client().unwrap().freeze().unwrap();
    assert_ne!(next.id(), capture.id());
    assert_eq!(next.resource(id).unwrap().raw().unwrap(), b"new");
}
