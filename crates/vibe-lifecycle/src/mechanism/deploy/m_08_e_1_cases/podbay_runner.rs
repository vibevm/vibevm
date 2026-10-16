//! Test-only adaptation of the Root-owned physical port. No native launcher.
use crate::process::{ProcessError, ProcessOutput, ProcessRunner, ProcessSpec, StreamMode};
use sha2::{Digest, Sha256};
use std::{
    ffi::OsStr,
    fs::OpenOptions,
    io::{BufRead, BufReader, Read, Write},
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};

/// Fixture DTOs matching Root's pinned protocol/1. Keeping this wire vocabulary
/// local avoids a machine-absolute source dependency in the library test target.
/// The physical broker remains the sole process owner.
pub mod protocol {
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;
    pub const PROTOCOL: &str = "m08e1.podbay-process-runner/1";
    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum StreamMode {
        Capture,
    }
    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct ProcessSpec {
        pub program: String,
        pub args: Vec<String>,
        pub cwd: String,
        pub env: BTreeMap<String, String>,
        pub stdin: Option<Vec<u8>>,
        pub stdout: StreamMode,
        pub stderr: StreamMode,
        pub scratch: String,
    }
    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Request {
        pub protocol: String,
        pub case_id: String,
        pub step_id: String,
        pub spec: ProcessSpec,
    }
    #[derive(Clone, Debug, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct ProcessResult {
        pub status: String,
        pub case_id: String,
        pub step_id: String,
        pub spec_sha256: String,
        pub code: Option<i32>,
        pub stdout: Vec<u8>,
        pub stderr: Vec<u8>,
        pub stdout_truncated: bool,
        pub stderr_truncated: bool,
        pub stdin_delivered: bool,
        pub exited: bool,
        pub drain_complete: bool,
        pub stdout_eof: bool,
        pub stderr_eof: bool,
        pub timed_out: bool,
        pub signal: Option<i32>,
        pub physical_receipt: String,
    }
}

pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn failure(message: impl Into<String>) -> ProcessError {
    ProcessError::Wait(message.into())
}
fn utf8(value: &OsStr) -> Result<String, ProcessError> {
    value
        .to_str()
        .map(str::to_owned)
        .ok_or_else(|| failure("non-UTF8 process value"))
}
pub fn wire(spec: &ProcessSpec) -> Result<protocol::ProcessSpec, ProcessError> {
    if spec.stdout != StreamMode::Capture || spec.stderr != StreamMode::Capture {
        return Err(failure("physical port supports capture only"));
    }
    if spec
        .stdin
        .as_ref()
        .is_some_and(|s| std::str::from_utf8(s).is_err())
    {
        return Err(failure("physical port requires UTF8 stdin"));
    }
    Ok(protocol::ProcessSpec {
        program: utf8(spec.program.as_os_str())?,
        args: spec
            .args
            .iter()
            .map(|s| utf8(s))
            .collect::<Result<_, _>>()?,
        cwd: utf8(spec.cwd.as_os_str())?,
        env: spec
            .env
            .iter()
            .map(|(k, v)| Ok((utf8(k)?, utf8(v)?)))
            .collect::<Result<_, ProcessError>>()?,
        stdin: spec.stdin.clone(),
        stdout: protocol::StreamMode::Capture,
        stderr: protocol::StreamMode::Capture,
        scratch: utf8(spec.scratch.as_os_str())?,
    })
}
pub fn native(spec: &protocol::ProcessSpec) -> ProcessSpec {
    ProcessSpec {
        program: spec.program.clone().into(),
        args: spec.args.iter().map(Into::into).collect(),
        cwd: spec.cwd.clone().into(),
        env: spec.env.iter().map(|(k, v)| (k.into(), v.into())).collect(),
        stdin: spec.stdin.clone(),
        stdout: StreamMode::Capture,
        stderr: StreamMode::Capture,
        scratch: spec.scratch.clone().into(),
    }
}
fn identifier(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}
pub fn decode(request: &protocol::Request, bytes: &[u8]) -> Result<ProcessOutput, ProcessError> {
    if bytes.len() > 196608 || !bytes.ends_with(b"\n") {
        return Err(failure(
            "incomplete/oversized physical response; original step remains uncertain",
        ));
    }
    let r: protocol::ProcessResult =
        serde_json::from_slice(bytes).map_err(|_| failure("unresolved physical response"))?;
    let expected = hash(&serde_json::to_vec(&request.spec).map_err(|e| failure(e.to_string()))?);
    if r.status != "complete"
        || r.case_id != request.case_id
        || r.step_id != request.step_id
        || r.spec_sha256 != expected
        || r.physical_receipt.is_empty()
    {
        return Err(failure("physical response/spec/receipt binding differs"));
    }
    if !r.stdin_delivered
        || !r.exited
        || !r.drain_complete
        || !r.stdout_eof
        || !r.stderr_eof
        || r.stdout.len() > 16384
        || r.stderr.len() > 16384
    {
        return Err(failure(
            "incomplete physical process I/O; retain original step",
        ));
    }
    // ProcessOutput cannot represent these fields. Preserve the raw result and
    // fail rather than erase them or manufacture an exit code.
    if r.timed_out || r.signal.is_some() {
        return Err(failure(format!(
            "physical process timed_out={} signal={:?} code={:?}",
            r.timed_out, r.signal, r.code
        )));
    }
    Ok(ProcessOutput {
        code: r.code,
        stdout: r.stdout,
        stderr: r.stderr,
        stdout_truncated: r.stdout_truncated,
        stderr_truncated: r.stderr_truncated,
    })
}

pub struct PodBayRunner {
    socket: PathBuf,
    evidence: PathBuf,
    steps: Mutex<std::collections::VecDeque<protocol::Request>>,
}
impl PodBayRunner {
    pub fn new(
        socket: &Path,
        evidence: &Path,
        steps: Vec<protocol::Request>,
    ) -> Result<Self, ProcessError> {
        let mut ids = std::collections::BTreeSet::new();
        for s in &steps {
            if s.protocol != protocol::PROTOCOL
                || !identifier(&s.case_id)
                || !identifier(&s.step_id)
                || !ids.insert((&s.case_id, &s.step_id))
            {
                return Err(failure("invalid/duplicate physical step"));
            }
        }
        Ok(Self {
            socket: socket.into(),
            evidence: evidence.into(),
            steps: Mutex::new(steps.into()),
        })
    }
    pub fn remaining(&self) -> usize {
        self.steps.lock().unwrap().len()
    }
}
fn retain(path: &Path, bytes: &[u8]) -> Result<(), ProcessError> {
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| failure(e.to_string()))?;
    f.write_all(bytes)
        .and_then(|()| f.sync_all())
        .map_err(|e| failure(e.to_string()))
}
impl ProcessRunner for PodBayRunner {
    fn run(&self, spec: &ProcessSpec) -> Result<ProcessOutput, ProcessError> {
        let converted = wire(spec)?;
        let mut steps = self
            .steps
            .lock()
            .map_err(|_| failure("poisoned physical admission"))?;
        let request = steps
            .front()
            .ok_or_else(|| failure("no sealed physical step"))?;
        if converted != request.spec {
            return Err(failure("ProcessSpec differs from exact next step"));
        }
        let mut encoded = serde_json::to_vec(request).map_err(|e| failure(e.to_string()))?;
        encoded.push(b'\n');
        if encoded.len() > 65536 {
            return Err(failure("physical request exceeds 64 KiB"));
        }
        // Reserve durably before any connection. On uncertainty neither this
        // runner nor a restarted driver may reissue the operation.
        let prefix = format!("{}--{}", request.case_id, request.step_id);
        retain(
            &self.evidence.join(format!("{prefix}.request.json")),
            &encoded,
        )?;
        let request = steps.pop_front().unwrap();
        let outcome = (|| {
            let mut stream = UnixStream::connect(&self.socket)?;
            stream.set_read_timeout(Some(Duration::from_secs(120)))?;
            stream.set_write_timeout(Some(Duration::from_secs(5)))?;
            stream.write_all(&encoded)?;
            stream.shutdown(std::net::Shutdown::Write)?;
            let mut bytes = Vec::new();
            let read = BufReader::new(stream.take(196609)).read_until(b'\n', &mut bytes);
            // Partial bytes are evidence even when the transport fails.
            retain(
                &self.evidence.join(format!("{prefix}.response.json")),
                &bytes,
            )
            .map_err(std::io::Error::other)?;
            read?;
            Ok::<_, std::io::Error>(bytes)
        })();
        match outcome {
            Ok(bytes) => {
                let decoded = decode(&request, &bytes);
                if decoded.is_err() {
                    steps.clear();
                }
                decoded
            }
            Err(e) => {
                retain(
                    &self.evidence.join(format!("{prefix}.uncertain.txt")),
                    e.to_string().as_bytes(),
                )?;
                // Disallow advancing into later effects after an uncertain step.
                steps.clear();
                Err(failure(format!(
                    "{} / {} uncertain; no retry: {e}",
                    request.case_id, request.step_id
                )))
            }
        }
    }
}

/// Adapter conformance only. These manufactured wire faults are never vendor
/// observations and cannot close a commissioning row.
pub fn adapter_conformance() {
    let spec = ProcessSpec {
        program: "/runtime/codex/codex".into(),
        args: vec!["--version".into()],
        cwd: "/case/project".into(),
        env: [("HOME".into(), "/home/olegchir".into())].into(),
        stdin: None,
        stdout: StreamMode::Capture,
        stderr: StreamMode::Capture,
        scratch: "/case/arena".into(),
    };
    let request = protocol::Request {
        protocol: protocol::PROTOCOL.into(),
        case_id: "adapter-only".into(),
        step_id: "faults".into(),
        spec: wire(&spec).unwrap(),
    };
    assert_eq!(wire(&native(&request.spec)).unwrap(), request.spec);
    let result = protocol::ProcessResult {
        status: "complete".into(),
        case_id: request.case_id.clone(),
        step_id: request.step_id.clone(),
        spec_sha256: hash(&serde_json::to_vec(&request.spec).unwrap()),
        code: Some(0),
        stdout: b"fixture protocol bytes, not a vendor reply".to_vec(),
        stderr: Vec::new(),
        stdout_truncated: false,
        stderr_truncated: false,
        stdin_delivered: true,
        exited: true,
        drain_complete: true,
        stdout_eof: true,
        stderr_eof: true,
        timed_out: false,
        signal: None,
        physical_receipt: "adapter-only".into(),
    };
    let encoded = |r: &protocol::ProcessResult| {
        let mut bytes = serde_json::to_vec(r).unwrap();
        bytes.push(b'\n');
        bytes
    };
    assert_eq!(decode(&request, &encoded(&result)).unwrap().code, Some(0));
    for mode in 0..13 {
        let mut bad = result.clone();
        match mode {
            0 => bad.case_id.push('x'),
            1 => bad.step_id.push('x'),
            2 => bad.spec_sha256 = "0".repeat(64),
            3 => bad.status = "unresolved".into(),
            4 => bad.stdin_delivered = false,
            5 => bad.exited = false,
            6 => bad.drain_complete = false,
            7 => bad.stdout_eof = false,
            8 => bad.stderr_eof = false,
            9 => bad.timed_out = true,
            10 => bad.signal = Some(9),
            11 => bad.physical_receipt.clear(),
            _ => bad.stdout = vec![b'x'; 16385],
        }
        assert!(
            decode(&request, &encoded(&bad)).is_err(),
            "wire fault {mode}"
        );
    }
    let mut no_code = result.clone();
    no_code.code = None;
    assert_eq!(decode(&request, &encoded(&no_code)).unwrap().code, None);
    for stream in [true, false] {
        let mut truncated = result.clone();
        truncated.stdout_truncated = stream;
        truncated.stderr_truncated = !stream;
        let output = decode(&request, &encoded(&truncated)).unwrap();
        assert_eq!(output.stdout_truncated, stream);
        assert_eq!(output.stderr_truncated, !stream);
    }
    let mut partial = encoded(&result);
    partial.pop();
    assert!(decode(&request, &partial).is_err());
    assert!(decode(&request, &vec![b' '; 196609]).is_err());
    let mut invalid = spec.clone();
    invalid.stdin = Some(vec![255]);
    assert!(wire(&invalid).is_err());
    invalid = spec.clone();
    invalid.stderr = StreamMode::Null;
    assert!(wire(&invalid).is_err());
    use std::os::unix::ffi::OsStringExt;
    invalid = spec.clone();
    invalid.args.push(std::ffi::OsString::from_vec(vec![255]));
    assert!(wire(&invalid).is_err());

    // A failed connection reserves the identity. A restarted adapter cannot
    // retry it; advancing to another effect is forbidden after uncertainty.
    let dir = tempfile::tempdir().unwrap();
    let runner = PodBayRunner::new(
        &dir.path().join("absent.sock"),
        dir.path(),
        vec![request.clone()],
    )
    .unwrap();
    assert!(runner.run(&spec).is_err());
    assert_eq!(runner.remaining(), 0);
    let restarted =
        PodBayRunner::new(&dir.path().join("absent.sock"), dir.path(), vec![request]).unwrap();
    assert!(restarted.run(&spec).is_err());
    assert!(
        dir.path()
            .join("adapter-only--faults.request.json")
            .is_file()
    );
    eprintln!(
        "adapter-only conformance: binding, partial I/O, timeout/signal, None exit, both truncation flags, UTF8 and durable no-retry checked; no vendor qualification"
    );
}

/// Root-pinned private session DTO; this is not a product launcher.
pub mod session_protocol {
    //! Private fixture lifecycle only. No vendor RPC, stdin-write, authority or success inference.
    use super::protocol::ProcessSpec;
    use serde::{Deserialize, Serialize};
    pub const SESSION_PROTOCOL: &str = "m08e1.podbay-session/1";
    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct SessionRequest {
        pub protocol: String,
        pub case_id: String,
        pub step_id: String,
        pub action: SessionAction,
    }

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
    pub enum SessionAction {
        Start { spec: ProcessSpec },
        Inspect { session_id: String },
        Stop { session_id: String },
    }
    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum SessionStatus {
        Started,
        Pending,
        Observed,
        Stopped,
        Refused,
    }
    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct SessionStream {
        pub bytes: Vec<u8>,
        pub observed_bytes: u64,
        pub truncated: bool,
        pub eof: bool,
        pub io_error: bool,
    }
    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct SessionOutput {
        pub stdin_delivered: bool,
        pub stdin_error: bool,
        pub stdout: SessionStream,
        pub stderr: SessionStream,
        pub exited: bool,
        pub exit_code: Option<i32>,
        pub signal: Option<i32>,
        pub timed_out: bool,
        pub stopped: bool,
        pub drain_complete: bool,
    }
    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct SessionResponse {
        pub protocol: String,
        pub case_id: String,
        pub step_id: String,
        pub session_id: String,
        pub status: SessionStatus,
        pub physical_receipt: Option<String>,
        pub output: Option<SessionOutput>,
        pub snapshot_is_retained: bool,
        pub stop_receipt: Option<String>,
        pub detail: Option<String>,
    }
}

/// One durable exchange. A partial reply is retained and never redispatched.
pub fn session_exchange(
    request: &session_protocol::SessionRequest,
    observation: &str,
) -> Result<session_protocol::SessionResponse, String> {
    use session_protocol::*;
    if request.protocol != SESSION_PROTOCOL
        || !identifier(&request.case_id)
        || !identifier(&request.step_id)
        || !identifier(observation)
    {
        return Err("invalid session request identity".into());
    }
    let mut bytes = serde_json::to_vec(request).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    if bytes.len() > 65536 {
        return Err("session request bound".into());
    }
    let base = format!(
        "/case/evidence/{}--{}--{}",
        request.case_id, request.step_id, observation
    );
    let persist = |suffix: &str, data: &[u8]| -> Result<(), String> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(format!("{base}.{suffix}"))
            .map_err(|e| e.to_string())?;
        file.write_all(data)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())
    };
    persist("request.json", &bytes)?;
    let mut stream =
        UnixStream::connect("/fixture-ipc/process-runner.sock").map_err(|e| e.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    stream.write_all(&bytes).map_err(|e| e.to_string())?;
    let mut reply = Vec::new();
    let read = BufReader::new(stream.take(196609)).read_until(b'\n', &mut reply);
    persist("response.raw", &reply)?;
    read.map_err(|e| format!("session response uncertain; retained original identity: {e}"))?;
    if reply.len() > 196608 || !reply.ends_with(b"\n") {
        return Err("session response incomplete/oversized; do not redispatch".into());
    }
    let response: SessionResponse = serde_json::from_slice(&reply).map_err(|e| e.to_string())?;
    if response.protocol != request.protocol
        || response.case_id != request.case_id
        || response.step_id != request.step_id
        || response.session_id.is_empty()
    {
        return Err("session response identity differs".into());
    }
    if let SessionAction::Inspect { session_id } | SessionAction::Stop { session_id } =
        &request.action
    {
        if session_id != &response.session_id {
            return Err("session token differs".into());
        }
    }
    if let Some(output) = &response.output {
        for stream in [&output.stdout, &output.stderr] {
            if stream.bytes.len() > 16384 || stream.observed_bytes < stream.bytes.len() as u64 {
                return Err("invalid bounded session output".into());
            }
        }
    }
    Ok(response)
}
