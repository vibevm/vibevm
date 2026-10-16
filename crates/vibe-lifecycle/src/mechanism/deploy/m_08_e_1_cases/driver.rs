//! Namespace-contained driver. Root provides the sealed physical manifest.
//! Discovery is explicitly not a completed matrix or a vendor-state witness.
use super::podbay_runner::{self, PodBayRunner, protocol};
use crate::{ClientExecutable, client_commands, process::ProcessRunner};
use serde::Deserialize;
use std::{path::Path, sync::Arc};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Seal {
    protocol: String,
    driver_sha256: String,
    home: String,
    stage: String,
    steps: Vec<protocol::Request>,
    #[serde(default)]
    row_id: Option<String>,
    #[serde(default)]
    peer_address: Option<String>,
    #[serde(default)]
    capture_limits: Option<CaptureLimits>,
    #[serde(default)]
    session_start: Option<podbay_runner::session_protocol::SessionRequest>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CaptureLimits {
    stdout: usize,
    stderr: usize,
}
fn namespace() -> Result<Seal, String> {
    // Read the namespace-only input before touching HOME or a vendor path.
    let bytes = std::fs::read("/fixture-input/driver-seal.json")
        .map_err(|_| "pending: Root physical seal/namespace not supplied")?;
    let seal: Seal = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if seal.protocol != protocol::PROTOCOL
        || seal.home != "/home/olegchir"
        || std::env::current_dir().map_err(|e| e.to_string())? != Path::new("/case/project")
        || std::env::var_os("HOME").as_deref() != Some(std::ffi::OsStr::new("/home/olegchir"))
    {
        return Err("Root namespace/identity differs".into());
    }
    let elf = std::fs::read(std::env::current_exe().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if podbay_runner::hash(&elf) != seal.driver_sha256 {
        return Err("sealed driver digest differs".into());
    }
    Ok(seal)
}
struct Authority(client_commands::Context);
impl client_commands::Authority for Authority {
    fn current(
        &self,
        _: &vibe_wire::generated::client_command::e1::request::Request,
    ) -> Result<client_commands::Context, &'static str> {
        Ok(self.0.clone())
    }
}
pub fn commission(client: &str) -> Result<(), String> {
    let seal = namespace()?;
    if seal.stage == "codex-negative" && client == "codex" {
        actual_negative_for(seal, true)?;
        return Err("pending: actual Codex wrong-version/revocation/truncation observed; full matrix remains separate".into());
    }
    if seal.stage == "claude-resource-lifecycle" && client == "claude_code" {
        return claude_resource_lifecycle(seal);
    }
    if seal.stage == "claude-resource-probe" && client == "claude_code" {
        return claude_resource_probe(seal);
    }
    if matches!(
        seal.stage.as_str(),
        "codex-session"
            | "codex-resource-lifecycle"
            | "codex-project-trust"
            | "codex-plugin-probe"
            | "codex-plugin-install"
            | "codex-plugin-remove"
            | "codex-plugin-update"
            | "codex-plugin-recovery"
            | "codex-mcp-lifecycle"
    ) && client == "codex"
    {
        return codex_session(seal);
    }
    if seal.stage == "claude-mcp-lifecycle" && client == "claude_code" {
        return claude_mcp_lifecycle(seal);
    }
    if !matches!(
        seal.stage.as_str(),
        "scoped-help" | "command-help" | "codex-schema" | "codex-observation"
    ) {
        return Err("pending: release-qualified matrix manifest required".into());
    }
    // Retired discovery keys must never be replayed, including after a
    // truncated response. These are new, bounded, read-only grammar probes.
    let (case, expected): (&str, &[(&str, &[&str])]) = match (client, seal.stage.as_str()) {
        ("claude_code", "scoped-help") => (
            "claude_code.scoped-help-002",
            &[
                ("prepare-version", &["--version"]),
                ("execute-version", &["--version"]),
                ("plugin-help", &["plugin", "--help"]),
                ("mcp-help", &["mcp", "--help"]),
            ],
        ),
        ("codex", "scoped-help") => (
            "codex.scoped-help-001",
            &[
                ("version", &["--version"]),
                ("plugin-help", &["plugin", "--help"]),
                ("mcp-help", &["mcp", "--help"]),
                ("app-server-help", &["app-server", "--help"]),
            ],
        ),
        ("claude_code", "command-help") => (
            "claude_code.command-help-003",
            &[
                ("plugin-list-help", &["plugin", "list", "--help"]),
                ("plugin-install-help", &["plugin", "install", "--help"]),
                ("plugin-uninstall-help", &["plugin", "uninstall", "--help"]),
                ("plugin-update-help", &["plugin", "update", "--help"]),
                ("plugin-details-help", &["plugin", "details", "--help"]),
                (
                    "marketplace-add-help",
                    &["plugin", "marketplace", "add", "--help"],
                ),
                ("mcp-add-json-help", &["mcp", "add-json", "--help"]),
                ("mcp-list-help", &["mcp", "list", "--help"]),
            ],
        ),
        ("codex", "command-help") => (
            "codex.command-help-002",
            &[
                ("plugin-list-help", &["plugin", "list", "--help"]),
                ("plugin-add-help", &["plugin", "add", "--help"]),
                ("plugin-remove-help", &["plugin", "remove", "--help"]),
                (
                    "marketplace-add-help",
                    &["plugin", "marketplace", "add", "--help"],
                ),
                ("mcp-list-help", &["mcp", "list", "--help"]),
                (
                    "schema-help",
                    &["app-server", "generate-json-schema", "--help"],
                ),
            ],
        ),
        ("codex", "codex-schema") => (
            "codex.schema-003",
            &[(
                "schema",
                &[
                    "app-server",
                    "generate-json-schema",
                    "--out",
                    "/case/evidence/codex-schema",
                    "--experimental",
                ],
            )],
        ),
        ("codex", "codex-observation") => (
            "codex.observation-004",
            &[
                ("skills", &["app-server", "--stdio"]),
                ("config", &["app-server", "--stdio"]),
            ],
        ),
        _ => return Err("unbound commissioning client/stage".into()),
    };
    let steps: Vec<_> = seal
        .steps
        .into_iter()
        .filter(|s| s.case_id == case)
        .collect();
    // Validate the whole batch before the first process effect. In particular,
    // a short Claude batch cannot panic at the post-Host slice below.
    if steps.len() != expected.len()
        || steps.iter().zip(expected).any(|(step, (id, args))| {
            step.step_id != *id
                || step.spec.args != *args
                || step.spec.program
                    != if client == "claude_code" {
                        "/runtime/claude/claude"
                    } else {
                        "/runtime/codex/codex"
                    }
        })
    {
        return Err(format!(
            "pending: exact scoped-help batch differs for {case}"
        ));
    }
    if seal.stage == "codex-observation" {
        for step in &steps {
            let body = std::str::from_utf8(
                step.spec
                    .stdin
                    .as_deref()
                    .ok_or("missing exact app-server input")?,
            )
            .map_err(|e| e.to_string())?;
            let actual: Vec<serde_json::Value> = body
                .lines()
                .map(serde_json::from_str)
                .collect::<Result<_, _>>()
                .map_err(|e| e.to_string())?;
            let query = if step.step_id == "skills" {
                serde_json::json!({"id":2,"method":"skills/list","params":{"cwds":["/case/project"],"forceReload":true}})
            } else {
                serde_json::json!({"id":2,"method":"config/read","params":{"cwd":"/case/project","includeLayers":false}})
            };
            let expected = vec![
                serde_json::json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":"m08e1-local-observer","version":"1.0.0"},"capabilities":{"experimentalApi":true,"explicitGatewayOauth":true}}}),
                serde_json::json!({"method":"initialized"}),
                query,
            ];
            if actual != expected {
                return Err("app-server input differs from qualified finite query".into());
            }
        }
    }
    let runner = Arc::new(
        PodBayRunner::new(
            Path::new("/fixture-ipc/process-runner.sock"),
            Path::new("/case/evidence"),
            steps.clone(),
        )
        .map_err(|e| e.to_string())?,
    );
    // Claude's already admitted version recipe exercises the actual opaque
    // Host/Prepared/ProcessGrant path. Codex discovery does not broaden 0.148.
    let mut first = 0;
    if client == "claude_code" && seal.stage == "scoped-help" {
        let authority = Arc::new(Authority(client_commands::Context {
            executable: ClientExecutable::Resolved {
                command: "claude".into(),
                path: "/runtime/claude/claude".into(),
            },
            cwd: "/case/project".into(),
            user_home: "/home/olegchir".into(),
            roots: vec!["/case".into(), "/home/olegchir".into()],
            environment_ref: "root-sealed-discovery".into(),
            principal: "m08e1".into(),
            policy: "credential-free-local-discovery".into(),
            evidence: "claude-2.1.299".into(),
            revoked: false,
        }));
        let host = client_commands::Host::new(authority, runner.clone());
        let request = vibe_wire::behaviour::client_command::read_request(br#"{"schema":1,"client":"claude","version":"2.1.299","contract_version":1,"operation":"version"}"#).map_err(str::to_owned)?;
        let prepared = host.prepare(request).map_err(str::to_owned)?;
        let plan = serde_json::to_vec_pretty(prepared.inspect()).map_err(|e| e.to_string())?;
        std::fs::write("/case/evidence/claude-command-plan.json", plan)
            .map_err(|e| e.to_string())?;
        let grant = host.approve(&prepared).map_err(str::to_owned)?;
        let output = host
            .execute(&prepared, Some(&grant))
            .map_err(str::to_owned)?;
        complete(&output).map_err(|e| format!("{case}/execute-version: {e}"))?;
        first = 2;
    }
    for step in &steps[first..] {
        let output = runner
            .run(&podbay_runner::native(&step.spec))
            .map_err(|e| format!("{case}/{}: {e}", step.step_id))?;
        complete(&output).map_err(|e| format!("{case}/{}: {e}", step.step_id))?;
        if seal.stage == "codex-observation" {
            let text = std::str::from_utf8(&output.stdout).map_err(|e| e.to_string())?;
            let replies: Vec<serde_json::Value> = text
                .lines()
                .map(serde_json::from_str)
                .collect::<Result<_, _>>()
                .map_err(|e| e.to_string())?;
            for id in [1, 2] {
                let matches: Vec<_> = replies
                    .iter()
                    .filter(|r| r.get("id") == Some(&serde_json::json!(id)))
                    .collect();
                if matches.len() != 1
                    || matches[0].get("result").is_none()
                    || matches[0].get("error").is_some()
                {
                    return Err(format!(
                        "{case}/{}: missing/duplicate/error app-server reply {id}; retain exact step",
                        step.step_id
                    ));
                }
            }
        }
        if client == "codex" && step.step_id == "version" {
            let version = std::str::from_utf8(&output.stdout)
                .map_err(|_| format!("{case}/version: non-UTF8 version"))?;
            if version.trim() != "codex-cli 0.159.3" {
                return Err(format!("{case}/version: exact release mismatch"));
            }
        }
    }
    if runner.remaining() != 0 {
        return Err("unconsumed sealed discovery steps".into());
    }
    Err(format!(
        "pending: {client} discovery retained; actual absence/v1/v2/removal, resource loading and local peer not yet qualified"
    ))
}
fn complete(output: &crate::process::ProcessOutput) -> Result<(), String> {
    if output.code != Some(0) || output.stdout_truncated || output.stderr_truncated {
        return Err(format!(
            "discovery incomplete: code={:?}, stdout_truncated={}, stderr_truncated={}",
            output.code, output.stdout_truncated, output.stderr_truncated
        ));
    }
    Ok(())
}

/// An application peer inside the PodBay-owned driver, never a process launcher.
/// Root must supply and seal a reachable loopback address for both processes.
/// Help-only fixtures cannot create one. Requests and responses are retained
/// separately from vendor stdout; a peer response alone proves no client load.
#[allow(dead_code)]
struct LocalPeer {
    stop: Arc<std::sync::atomic::AtomicBool>,
    thread: Option<std::thread::JoinHandle<Result<(), String>>>,
}
#[allow(dead_code)]
impl LocalPeer {
    fn start(address: &str, evidence: &Path, revision: &str) -> Result<Self, String> {
        use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
        let address: SocketAddr = address.parse().map_err(|_| "invalid sealed peer address")?;
        if address.ip() != IpAddr::V4(Ipv4Addr::LOCALHOST) || address.port() == 0 {
            return Err("peer requires an exact Root-sealed IPv4 loopback port".into());
        }
        if !matches!(revision, "v1" | "v2") {
            return Err("unbound peer resource revision".into());
        }
        let listener = TcpListener::bind(address).map_err(|e| e.to_string())?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let cancelled = stop.clone();
        let evidence = evidence.to_path_buf();
        let revision = revision.to_owned();
        let thread = std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(90);
            let mut count = 0;
            while !cancelled.load(std::sync::atomic::Ordering::SeqCst) {
                if std::time::Instant::now() >= deadline {
                    return Err("local peer deadline reached".into());
                }
                match listener.accept() {
                    Ok((stream, remote)) => {
                        if !remote.ip().is_loopback() || count == 64 {
                            return Err("local peer connection bound exceeded".into());
                        }
                        count += 1;
                        Self::exchange(stream, &evidence, &revision, count)?;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(e) => return Err(e.to_string()),
                }
            }
            Ok(())
        });
        Ok(Self {
            stop,
            thread: Some(thread),
        })
    }
    fn finish(mut self) -> Result<(), String> {
        self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
        self.thread
            .take()
            .unwrap()
            .join()
            .map_err(|_| "local peer panicked")?
    }
    fn reply(
        request: &serde_json::Value,
        revision: &str,
    ) -> Result<Option<serde_json::Value>, String> {
        if request["jsonrpc"] != "2.0" {
            return Err("invalid peer JSON-RPC version".into());
        }
        let method = request["method"].as_str().ok_or("missing peer method")?;
        let mut unsupported = false;
        let result = match method {
            "initialize" => {
                let version = request["params"]["protocolVersion"]
                    .as_str()
                    .ok_or("missing MCP version")?;
                if version.is_empty() {
                    return Err("empty MCP version".into());
                }
                // This finite fixture implements exactly this revision. MCP
                // negotiation selects a supported version, never arbitrary echo.
                let version = "2025-06-18";
                serde_json::json!({"protocolVersion":version,"capabilities":{"resources":{},"tools":{}},"serverInfo":{"name":"m08e1-local-peer","version":revision}})
            }
            "notifications/initialized" | "notifications/cancelled" => serde_json::Value::Null,
            "ping" => serde_json::json!({}),
            "tools/list" => {
                serde_json::json!({"tools":[{"name":"fixture_resource","description":format!("M08E1 {revision} resource"),"inputSchema":{"type":"object","properties":{},"additionalProperties":false}}]})
            }
            "resources/list" => {
                serde_json::json!({"resources":[{"uri":"m08e1://resource","name":format!("M08E1 {revision}"),"mimeType":"text/plain"}]})
            }
            "resources/templates/list" => serde_json::json!({"resourceTemplates":[]}),
            "resources/read" if request["params"]["uri"] == "m08e1://resource" => {
                serde_json::json!({"contents":[{"uri":"m08e1://resource","mimeType":"text/plain","text":format!("M08E1 {revision} exact local resource\n")}]})
            }
            "tools/call" if request["params"]["name"] == "fixture_resource" => {
                serde_json::json!({"content":[{"type":"text","text":format!("M08E1 {revision} exact local resource\n")}],"isError":false})
            }
            _ => {
                unsupported = true;
                serde_json::Value::Null
            }
        };
        Ok(request
            .get("id")
            .map(|id| if unsupported {
                serde_json::json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"Method not found"}})
            } else { serde_json::json!({"jsonrpc":"2.0","id":id,"result":result}) }))
    }
    fn exchange(
        mut stream: std::net::TcpStream,
        evidence: &Path,
        revision: &str,
        sequence: usize,
    ) -> Result<(), String> {
        use std::io::{BufRead, Read, Write};
        let timeout = Some(std::time::Duration::from_secs(2));
        stream
            .set_read_timeout(timeout)
            .map_err(|e| e.to_string())?;
        stream
            .set_write_timeout(timeout)
            .map_err(|e| e.to_string())?;
        let retain = |suffix: &str, value: &serde_json::Value| -> Result<(), String> {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(evidence.join(format!("peer-{revision}-{sequence:03}{suffix}.json")))
                .map_err(|e| e.to_string())?;
            file.write_all(&serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?)
                .and_then(|()| file.sync_all())
                .map_err(|e| e.to_string())
        };
        let mut reader = std::io::BufReader::new((&mut stream).take(73728));
        let mut line = String::new();
        reader.read_line(&mut line).map_err(|e| e.to_string())?;
        retain(".route", &serde_json::json!({"request_line":line}))?;
        let route = line.clone();
        let mut header_bytes = line.len();
        let mut length = None;
        loop {
            line.clear();
            if reader.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
                return Err("incomplete peer headers".into());
            }
            header_bytes += line.len();
            if header_bytes > 8192 {
                return Err("peer headers exceed bound".into());
            }
            if line == "\r\n" {
                break;
            }
            let (key, value) = line.split_once(':').ok_or("invalid peer header")?;
            if key.eq_ignore_ascii_case("content-length") {
                if length.is_some() {
                    return Err("duplicate content length".into());
                }
                length = Some(
                    value
                        .trim()
                        .parse::<usize>()
                        .map_err(|_| "invalid content length")?,
                );
            }
            if key.eq_ignore_ascii_case("transfer-encoding") {
                return Err("chunked peer requests are not qualified".into());
            }
        }
        if route != format!("POST /mcp/{revision} HTTP/1.1\r\n") {
            // Drain the bounded declared body before close: unread request bytes
            // can otherwise reset the connection instead of delivering status.
            let length = length.unwrap_or(0);
            if length > 65536 {
                return Err("oversized peer HTTP probe body".into());
            }
            let mut discard = Vec::new();
            reader
                .by_ref()
                .take(length as u64)
                .read_to_end(&mut discard)
                .map_err(|e| e.to_string())?;
            if discard.len() != length {
                return Err("incomplete peer HTTP probe body".into());
            }
            drop(reader);
            let response = if route == format!("GET /mcp/{revision} HTTP/1.1\r\n") {
                "HTTP/1.1 405 Method Not Allowed\r\nAllow: POST\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            } else {
                // This peer is unauthenticated and publishes no OAuth metadata.
                // Unknown local routes remain retained, explicit 404 evidence.
                "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            };
            retain(
                ".http-response",
                &serde_json::json!({"status_line":response.lines().next(),"request_line":route}),
            )?;
            return stream
                .write_all(response.as_bytes())
                .map_err(|e| e.to_string());
        }
        let length = length
            .filter(|v| *v <= 65536)
            .ok_or("missing/oversized peer body")?;
        let mut body = Vec::new();
        let read = reader.take(length as u64).read_to_end(&mut body);
        // Preserve the exact bounded body before parsing/dispatch, even when
        // transport or an unqualified protocol extension causes refusal.
        retain(
            ".request",
            &serde_json::json!({"expected_bytes":length,"body":body}),
        )?;
        read.map_err(|e| e.to_string())?;
        if body.len() != length {
            return Err("incomplete local peer body".into());
        }
        let request: serde_json::Value =
            serde_json::from_slice(&body).map_err(|e| e.to_string())?;
        let reply = Self::reply(&request, revision)?;
        // Retain only the bounded MCP body, never HTTP auth/cookie headers.
        let record = serde_json::to_vec_pretty(
            &serde_json::json!({"revision":revision,"request":request,"response":reply}),
        )
        .map_err(|e| e.to_string())?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(evidence.join(format!("peer-{revision}-{sequence:03}.json")))
            .map_err(|e| e.to_string())?;
        file.write_all(&record)
            .and_then(|()| file.sync_all())
            .map_err(|e| e.to_string())?;
        let (status, body) = match reply {
            Some(value) => (
                "200 OK",
                serde_json::to_vec(&value).map_err(|e| e.to_string())?,
            ),
            None => ("202 Accepted", Vec::new()),
        };
        write!(stream, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).map_err(|e| e.to_string())?;
        stream.write_all(&body).map_err(|e| e.to_string())
    }
}
impl Drop for LocalPeer {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
pub fn version_contract() -> Result<(), String> {
    messages_contract_checks()?;
    peer_trace_replay()?;
    podbay_runner::adapter_conformance();
    use vibe_agent_projection::client_commands::{Client, supported_version};
    assert!(supported_version(Client::Codex, "0.148.9"));
    assert!(!supported_version(Client::Codex, "0.159.4"));
    assert!(!supported_version(Client::Claude, "2.2.0"));
    let seal = namespace()?;
    if seal.stage == "actual-executable-drift" {
        actual_executable_drift(seal)?;
        return Err(
            "pending: actual executable drift refused; remaining exact profile/matrix obligations"
                .into(),
        );
    }
    if seal.stage == "actual-negative" {
        actual_negative(seal)?;
    }
    Err(
        "pending: exact 0.159.3 qualification and actual wrong-version/truncation observations"
            .into(),
    )
}

fn actual_negative(seal: Seal) -> Result<(), String> {
    actual_negative_for(seal, false)
}
fn actual_negative_for(seal: Seal, codex: bool) -> Result<(), String> {
    struct MutableAuthority(std::sync::Mutex<client_commands::Context>);
    impl client_commands::Authority for MutableAuthority {
        fn current(
            &self,
            _: &vibe_wire::generated::client_command::e1::request::Request,
        ) -> Result<client_commands::Context, &'static str> {
            self.0
                .lock()
                .map(|value| value.clone())
                .map_err(|_| "poisoned test authority")
        }
    }
    let case = if codex {
        "codex.negative-030"
    } else {
        "claude_code.negative-009"
    };
    let program = if codex {
        "/runtime/codex/codex"
    } else {
        "/runtime/claude/claude"
    };
    let ids = ["wrong-version", "prepare-revocation", "stderr-truncated"];
    let invalid_argument = format!("--m08e1-invalid-{}", "x".repeat(512));
    if !seal
        .capture_limits
        .as_ref()
        .is_some_and(|limits| limits.stdout == 16384 && limits.stderr == 128)
    {
        return Err("Root negative-fixture capture limits not sealed".into());
    }
    if seal.steps.len() != 3
        || seal.steps.iter().zip(ids).any(|(step, id)| {
            step.case_id != case
                || step.step_id != id
                || step.spec.program != program
                || step.spec.args
                    != if id == "stderr-truncated" {
                        vec!["plugin".into(), "list".into(), invalid_argument.clone()]
                    } else {
                        vec!["--version".into()]
                    }
        })
    {
        return Err("actual negative batch differs before effects".into());
    }
    let runner = Arc::new(
        PodBayRunner::new(
            Path::new("/fixture-ipc/process-runner.sock"),
            Path::new("/case/evidence"),
            seal.steps.clone(),
        )
        .map_err(|e| e.to_string())?,
    );
    let authority = Arc::new(MutableAuthority(std::sync::Mutex::new(
        client_commands::Context {
            executable: ClientExecutable::Resolved {
                command: if codex { "codex" } else { "claude" }.into(),
                path: program.into(),
            },
            cwd: "/case/project".into(),
            user_home: "/home/olegchir".into(),
            roots: vec!["/case".into(), "/home/olegchir".into()],
            environment_ref: "root-sealed-actual-negative".into(),
            principal: "m08e1".into(),
            policy: if codex {
                "negative-030"
            } else {
                "negative-006"
            }
            .into(),
            evidence: if codex {
                "codex-0.159.3"
            } else {
                "claude-2.1.299"
            }
            .into(),
            revoked: false,
        },
    )));
    let host = client_commands::Host::new(authority.clone(), runner.clone());
    let request = |version: &str| {
        let value = serde_json::json!({"schema":1,"client":if codex {"codex"} else {"claude"},"version":version,"contract_version":1,"operation":"version"});
        vibe_wire::behaviour::client_command::read_request(&serde_json::to_vec(&value).unwrap())
            .map_err(str::to_owned)
    };
    let wrong = request(if codex { "0.148.9" } else { "2.1.298" })?;
    match host.prepare(wrong) {
        Err("version evidence mismatch") => (),
        Err(other) => {
            return Err(format!(
                "wrong-version refused for a different reason: {other}"
            ));
        }
        Ok(_) => return Err("wrong actual version admitted".into()),
    }
    let right = request(if codex { "0.159.3" } else { "2.1.299" })?;
    let prepared = host.prepare(right).map_err(str::to_owned)?;
    let grant = host.approve(&prepared).map_err(str::to_owned)?;
    authority
        .0
        .lock()
        .map_err(|_| "poisoned authority")?
        .revoked = true;
    if host.execute(&prepared, Some(&grant)).is_ok() || runner.remaining() != 1 {
        return Err("revocation did not refuse before dispatch".into());
    }
    // The vendor itself must emit truncated stderr. Manufactured DTO faults
    // remain adapter-only conformance and cannot satisfy this obligation.
    let output = runner
        .run(&podbay_runner::native(&seal.steps[2].spec))
        .map_err(|e| e.to_string())?;
    if !output.stderr_truncated || output.code == Some(0) || output.code.is_none() {
        return Err("actual stderr truncation refusal not observed; retain exact step".into());
    }
    if complete(&output).is_ok() || runner.remaining() != 0 {
        return Err("truncated actual negative accepted".into());
    }
    std::fs::write("/case/evidence/actual-negative-result.json",serde_json::to_vec_pretty(&serde_json::json!({"wrong_version":"actual version mismatch refused","revocation":"refused without dispatch","stderr_truncated":output.stderr_truncated,"exit":output.code,"executable_drift":"pending; no fabricated identity mutation"})).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    Ok(())
}
pub fn receipt_contract() -> Result<(), String> {
    file_owner_contract()?;
    namespace()?;
    Err("pending: qualified scoped observer and lifecycle effects for all matrix rows".into())
}

// File-owner verification is independent of vendor commissioning. It exercises
// the real C3 + D2 composition using external data and temporary capabilities.
// It makes no claim about vendor discovery or client-owned plugin caches.
struct FileAuthority {
    request_path: std::path::PathBuf,
    policy: std::sync::Mutex<String>,
}
impl crate::generic::GenericAuthority for FileAuthority {
    fn selected_current(
        &self,
        client: &str,
    ) -> Result<crate::generic::SelectedObservation, String> {
        let bytes = std::fs::read(&self.request_path).map_err(|e| e.to_string())?;
        let request: vibe_core::manifest::ClientRequest =
            serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if request.client_id != client {
            return Err("foreign capture client".into());
        }
        let digest = podbay_runner::hash(&bytes);
        Ok(crate::generic::SelectedObservation {
            identity: crate::generic::SourceIdentity {
                provider: "org.vibevm.fixture/external#client".into(),
                provider_version: "1.0.0".into(),
                provider_digest: digest.clone(),
                declaration: "client".into(),
                declaration_digest: digest.clone(),
                descriptor: self.request_path.display().to_string(),
                descriptor_digest: digest.clone(),
                source: self.request_path.display().to_string(),
                source_digest: digest,
            },
            request,
            selected: true,
            enabled: true,
        })
    }
    fn current_control(&self) -> Result<crate::generic::ControlObservation, String> {
        Ok(crate::generic::ControlObservation {
            principal: "m08e1-file-owner".into(),
            policy_generation: self.policy.lock().unwrap().clone(),
        })
    }
}
struct FileObserver {
    roots: Vec<std::path::PathBuf>,
    subject: String,
}

/// First actual MCP row. GenericHost owns the JSON member and D2 owns its
/// operation receipts; all vendor observations still pass the sealed runner.
/// This does not qualify a plugin ClientStateOwner or close the other rows.
fn claude_mcp_lifecycle(seal: Seal) -> Result<(), String> {
    use crate::{
        client_deploy::{self, Cancellation, Files},
        generic::*,
    };
    use vibe_agent_projection::client_capabilities::{ClientCapabilities, ClientCapability};
    use vibe_core::manifest::{ClientRequest, ClientScope, ClientShape};
    use vibe_wire::generated::client_deploy::e1::reply::{ReplyAction, ReplyStatus};
    // The prior override-context experiment is immutable failed evidence.
    // Root006 permits a new standard-HOME case, never replay of its old keys.
    let (case, scope, destination, scope_name) = match seal.row_id.as_deref() {
        Some("claude_code.mcp.user") => (
            "claude_code.mcp.user.lifecycle-010",
            ClientScope::User,
            ".claude.json",
            "user",
        ),
        Some("claude_code.mcp.project") => (
            "claude_code.mcp.project.lifecycle-011",
            ClientScope::Project,
            ".mcp.json",
            "project",
        ),
        _ => return Err("unbound actual MCP row".into()),
    };
    let consent_path = "/fixture-input/claude-project-mcp-consent.json";
    let consent = if scope == ClientScope::Project {
        let bytes = std::fs::read(consent_path).map_err(|e| e.to_string())?;
        let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if value != serde_json::json!({"enabledMcpjsonServers":["m08a2-demo"]}) {
            return Err("named-server companion differs".into());
        }
        Some(bytes)
    } else {
        None
    };
    let address = seal
        .peer_address
        .as_deref()
        .ok_or("Root peer address missing")?;
    let stages = ["version", "absence", "v1", "v2", "removed"];
    if seal.steps.len() != stages.len()
        || seal.steps.iter().zip(stages).any(|(s, id)| {
            s.case_id != case
                || s.step_id != id
                || s.spec.program != "/runtime/claude/claude"
                || s.spec.env.contains_key("CLAUDE_CONFIG_DIR")
                || s.spec.args
                    != if id == "version" {
                        vec!["--version"]
                    } else if scope == ClientScope::Project {
                        vec!["--settings", consent_path, "mcp", "list"]
                    } else {
                        vec!["mcp", "list"]
                    }
        })
    {
        return Err("actual MCP exact process batch differs".into());
    }
    let runner = Arc::new(
        PodBayRunner::new(
            Path::new("/fixture-ipc/process-runner.sock"),
            Path::new("/case/evidence"),
            seal.steps.clone(),
        )
        .map_err(|e| e.to_string())?,
    );
    let package = Path::new("/fixture-input/mcp-package");
    let request: ClientRequest = serde_json::from_slice(
        &std::fs::read(package.join("request.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    request.validate()?;
    if request.version != "=2.1.299"
        || request.bindings.len() != 1
        || request.bindings[0].scope != scope
        || request.bindings[0].shape != ClientShape::JsonMember
        || request.bindings[0].destination != destination
        || request.bindings[0].key != ["mcpServers", "m08a2-demo"]
    {
        return Err("actual MCP descriptor differs from sealed row".into());
    }
    // The exact external payload, not a guessed vendor cache layout, supplies
    // the endpoint. Prevalidate both revisions before any process/file effect.
    for phase in ["v1", "v2"] {
        let data =
            std::fs::read(package.join(phase).join("value.json")).map_err(|e| e.to_string())?;
        let value: serde_json::Value = serde_json::from_slice(&data).map_err(|e| e.to_string())?;
        if value
            != serde_json::json!({"mcpServers":{"m08a2-demo":{"type":"http","url":format!("http://{address}/mcp/{phase}")}}})
        {
            return Err("external MCP payload/Root endpoint differs".into());
        }
    }
    let project = std::path::PathBuf::from("/case/project");
    let user = std::path::PathBuf::from("/home/olegchir");
    let open = |p: &Path| vibe_safefs::Project::open(p).map_err(|e| e.to_string());
    let authority = Arc::new(FileAuthority {
        request_path: package.join("request.json"),
        policy: std::sync::Mutex::new(case.into()),
    });
    let category = |name: &str| {
        [ClientScope::Project, ClientScope::User]
            .into_iter()
            .map(|scope| ProtectedResource {
                scope,
                relative: format!("controls/{name}"),
            })
            .collect()
    };
    let files = Arc::new(
        GenericHost::new(
            open(&project)?,
            open(&user)?,
            open(Path::new("/case/settings"))?,
            authority.clone(),
            ProtectedControls {
                grants: category("grants"),
                policy: category("policy"),
                producer_trust: category("trust"),
                publication_mode: category("publication"),
            },
        )
        .map_err(|e| e.to_string())?
        .with_coordination(open(Path::new("/case/arena"))?)
        .map_err(|e| e.to_string())?,
    );
    let selected = authority.selected_current(&request.client_id)?;
    let capabilities = ClientCapabilities::admit([ClientCapability {
        provider: selected.identity.provider.clone(),
        descriptor_digest: selected.identity.descriptor_digest.clone(),
        request: request.clone(),
        disabled: false,
    }])?;
    let commands = Arc::new(client_commands::Host::new(
        Arc::new(Authority(client_commands::Context {
            executable: ClientExecutable::Missing {
                command: "file-only-command-owner".into(),
            },
            cwd: project.clone(),
            user_home: user.clone(),
            roots: vec![project.clone(), user.clone()],
            environment_ref: "no-D1-vendor-write".into(),
            principal: "root-sealed".into(),
            policy: "file-only".into(),
            evidence: case.into(),
            revoked: true,
        })),
        runner.clone(),
    ));
    let host = client_deploy::Host::new(
        open(&project)?,
        open(Path::new("/case/settings"))?,
        open(Path::new("/case/arena"))?,
        format!("actual-claude-mcp-{scope_name}"),
        scope_name.into(),
        commands,
        Arc::new(FileObserver {
            roots: vec![project.clone(), user.clone()],
            subject: selected.identity.descriptor_digest.clone(),
        }),
    )?;
    host.prepare_storage()?;
    let save = |name: &str, bytes: &[u8]| -> Result<(), String> {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(Path::new("/case/evidence").join(name))
            .map_err(|e| e.to_string())?;
        f.write_all(bytes)
            .and_then(|()| f.sync_all())
            .map_err(|e| e.to_string())
    };
    save(
        "actual-mcp-capture.json",
        &serde_json::to_vec_pretty(&selected.identity).map_err(|e| e.to_string())?,
    )?;
    let core = podbay_runner::hash(
        &std::fs::read(std::env::current_exe().map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?,
    );
    save("actual-mcp-core-before.txt", core.as_bytes())?;
    let snapshot = |phase: &str| -> Result<(), String> {
        use std::os::unix::fs::MetadataExt;
        if let Some(expected) = &consent {
            if std::fs::read(consent_path).map_err(|e| e.to_string())? != *expected {
                return Err("immutable named-server companion changed".into());
            }
        }
        let mut values = std::collections::BTreeMap::new();
        // Finite explicit files relevant to this candidate MCP binding. This
        // snapshot is not asserted to qualify every vendor discovery source.
        for path in [
            "/home/olegchir/.claude.json",
            "/home/olegchir/.claude/.claude.json",
            "/home/olegchir/.claude/settings.json",
            "/case/project/.mcp.json",
            "/case/project/.claude/settings.json",
            "/case/project/m08e1-foreign.txt",
            "/fixture-input/claude-project-mcp-consent.json",
        ] {
            let value = match std::fs::read(path) {
                Ok(bytes) => {
                    let m = std::fs::metadata(path).map_err(|e| e.to_string())?;
                    serde_json::json!({"sha256":podbay_runner::hash(&bytes),"bytes":bytes,"ino":m.ino(),"dev":m.dev(),"mtime":m.mtime(),"mtime_nsec":m.mtime_nsec()})
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => serde_json::Value::Null,
                Err(e) => return Err(e.to_string()),
            };
            values.insert(path, value);
        }
        save(
            &format!("actual-mcp-{phase}.json"),
            &serde_json::to_vec_pretty(&values).map_err(|e| e.to_string())?,
        )
    };
    let foreign = project.join("m08e1-foreign.txt");
    {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&foreign)
            .map_err(|e| e.to_string())?;
        f.write_all(b"keep\n").map_err(|e| e.to_string())?;
    }
    let foreign_mtime = std::fs::metadata(&foreign)
        .map_err(|e| e.to_string())?
        .modified()
        .map_err(|e| e.to_string())?;
    let version = runner
        .run(&podbay_runner::native(&seal.steps[0].spec))
        .map_err(|e| e.to_string())?;
    complete(&version)?;
    if version.stdout != b"2.1.299 (Claude Code)\n" {
        return Err("actual MCP exact client version differs".into());
    }
    snapshot("absence-before")?;
    let absent = runner
        .run(&podbay_runner::native(&seal.steps[1].spec))
        .map_err(|e| e.to_string())?;
    complete(&absent)?;
    if String::from_utf8_lossy(&absent.stdout).contains("m08a2-demo") {
        return Err("foreign MCP occupancy".into());
    }
    snapshot("absence-after")?;
    let mut target = String::new();
    for (i, (phase, action)) in [
        ("v1", ReplyAction::Apply),
        ("v2", ReplyAction::Update),
        ("removed", ReplyAction::Remove),
    ]
    .into_iter()
    .enumerate()
    {
        snapshot(&format!("{phase}-before"))?;
        let file_plan = if phase == "removed" {
            Files::Retained {
                host: files.clone(),
                plan: files
                    .prepare_retained(&target, RetainedOperation::Cleanup)
                    .map_err(|e| e.to_string())?,
            }
        } else {
            let admitted = files
                .admit_payload(
                    files
                        .capture(&capabilities, &request.client_id)
                        .map_err(|e| e.to_string())?,
                    payload(package, phase, &request)?,
                )
                .map_err(|e| e.to_string())?;
            let plan = files.plan(&admitted).map_err(|e| e.to_string())?;
            target = plan.target().into();
            Files::Apply {
                host: files.clone(),
                plan,
            }
        };
        let prepared = host.prepare(action, Vec::new(), Some(file_plan))?;
        if host.plan(&prepared).status != ReplyStatus::Planned {
            return Err(format!("{case}/{phase}: file plan not current"));
        }
        let grant = host.approve(&prepared)?;
        let run = host.proposed_run(phase);
        host.initialize(&prepared, &run)?;
        let reply = host.execute(
            &prepared,
            Some(&grant),
            &run,
            false,
            &Cancellation::default(),
            "2026-10-16T00:00:00Z",
        )?;
        save(
            &format!("actual-mcp-{phase}-receipt-reply.json"),
            &serde_json::to_vec_pretty(&reply).map_err(|e| e.to_string())?,
        )?;
        if reply.status != ReplyStatus::Completed
            || host.verify(&run)?.status != ReplyStatus::Completed
        {
            return Err(format!("{case}/{phase}: file effect incomplete"));
        }
        let peer = if phase != "removed" {
            Some(LocalPeer::start(
                address,
                Path::new("/case/evidence"),
                phase,
            )?)
        } else {
            None
        };
        let observed = runner.run(&podbay_runner::native(&seal.steps[i + 2].spec));
        let peer_result = peer.map(LocalPeer::finish).transpose();
        snapshot(&format!("{phase}-after"))?;
        peer_result?;
        let observed = observed.map_err(|e| format!("{case}/{phase}: {e}"))?;
        complete(&observed)?;
        let text = String::from_utf8(observed.stdout).map_err(|e| e.to_string())?;
        if phase == "removed" {
            if text.contains("m08a2-demo") {
                return Err("removed MCP still visible".into());
            }
        } else if !text.lines().any(|line| {
            line.contains("m08a2-demo:")
                && line.contains(&format!("http://{address}/mcp/{phase}"))
                && line.contains("✔ Connected")
        }) {
            return Err(format!(
                "{case}/{phase}: actual client did not report connected resource; retain effect/step, no retry"
            ));
        }
        if phase != "removed" {
            let mut initialized = false;
            for sequence in 1..=64 {
                let path =
                    Path::new("/case/evidence").join(format!("peer-{phase}-{sequence:03}.json"));
                match std::fs::read(path) {
                    Ok(bytes) => {
                        let exchange: serde_json::Value =
                            serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                        initialized |= exchange["request"]["method"] == "initialize"
                            && exchange["response"]["result"]["serverInfo"]["version"] == phase;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                    Err(e) => return Err(e.to_string()),
                }
            }
            if !initialized {
                return Err(format!("{case}/{phase}: peer resource response differs"));
            }
        }
        if std::fs::read(&foreign).map_err(|e| e.to_string())? != b"keep\n"
            || std::fs::metadata(&foreign)
                .map_err(|e| e.to_string())?
                .modified()
                .map_err(|e| e.to_string())?
                != foreign_mtime
        {
            return Err("foreign sibling changed".into());
        }
    }
    let after = podbay_runner::hash(
        &std::fs::read(std::env::current_exe().map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?,
    );
    save("actual-mcp-core-after.txt", after.as_bytes())?;
    if core != after || runner.remaining() != 0 {
        return Err("core changed or unconsumed actual MCP steps".into());
    }
    Err("pending: one actual MCP lifecycle retained; interruption/inverse/negative branches, full scoped closure and remaining matrix rows require qualification".into())
}
impl crate::client_deploy::Observer for FileObserver {
    fn subject(&self) -> Result<String, String> {
        Ok(self.subject.clone())
    }
    fn observe(
        &self,
        resources: &[String],
    ) -> Result<std::collections::BTreeMap<String, Option<String>>, String> {
        resources
            .iter()
            .map(|resource| {
                let encoded = resource
                    .strip_prefix("generic:")
                    .ok_or("only explicit generic file resources admitted")?;
                let (root, path, keys): (String, String, Vec<String>) =
                    serde_json::from_str(encoded).map_err(|e| e.to_string())?;
                if !self.roots.iter().any(|p| p == Path::new(&root)) {
                    return Err("observer root outside exact capabilities".into());
                }
                let project =
                    vibe_safefs::Project::open(Path::new(&root)).map_err(|e| e.to_string())?;
                let bytes = project
                    .read_file_bounded(&path, 1024 * 1024)
                    .map_err(|e| e.to_string())?;
                let value = match bytes {
                    None => None,
                    Some(bytes) if keys.is_empty() => Some(podbay_runner::hash(&bytes)),
                    Some(bytes) if path.ends_with(".toml") => {
                        let t: toml::Value =
                            toml::from_str(std::str::from_utf8(&bytes).map_err(|e| e.to_string())?)
                                .map_err(|e| e.to_string())?;
                        let mut member = Some(&t);
                        for key in &keys {
                            member = member.and_then(|v| v.get(key));
                        }
                        member.map(|v| {
                            let framed = toml::Value::Table(toml::map::Map::from_iter([(
                                "value".into(),
                                v.clone(),
                            )]));
                            podbay_runner::hash(toml::to_string(&framed).unwrap().as_bytes())
                        })
                    }
                    Some(bytes) => {
                        let parsed: serde_json::Value =
                            serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                        let mut member = Some(&parsed);
                        for key in &keys {
                            member = member.and_then(|v| v.get(key));
                        }
                        member.map(|v| podbay_runner::hash(&serde_json::to_vec(v).unwrap()))
                    }
                };
                Ok((resource.clone(), value))
            })
            .collect()
    }
}
fn payload(
    package: &Path,
    phase: &str,
    request: &vibe_core::manifest::ClientRequest,
) -> Result<Vec<crate::generic::GenericPayload>, String> {
    use crate::generic::GenericPayload;
    use vibe_core::manifest::ClientShape;
    let root = package.join(phase);
    request
        .bindings
        .iter()
        .map(|b| match b.shape {
            ClientShape::Tree => {
                let mut files = std::collections::BTreeMap::new();
                for name in ["SKILL.md", "reference.txt"] {
                    match std::fs::read(root.join(name)) {
                        Ok(bytes) => {
                            files.insert(name.into(), bytes);
                        }
                        Err(e)
                            if e.kind() == std::io::ErrorKind::NotFound
                                && name == "reference.txt"
                                && phase == "v2" =>
                        {
                            ()
                        }
                        Err(e) => return Err(e.to_string()),
                    }
                }
                Ok(GenericPayload::Tree(files))
            }
            ClientShape::JsonMember => {
                let mut value: serde_json::Value = serde_json::from_slice(
                    &std::fs::read(root.join("value.json")).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                for key in &b.key {
                    value = value
                        .get(key)
                        .ok_or("missing declared JSON member")?
                        .clone();
                }
                Ok(GenericPayload::JsonMember(value))
            }
            ClientShape::TomlMember => {
                let mut value: toml::Value = toml::from_str(
                    &std::fs::read_to_string(root.join("value.toml")).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                for key in &b.key {
                    value = value
                        .get(key)
                        .ok_or("missing declared TOML member")?
                        .clone();
                }
                Ok(GenericPayload::TomlMember(value))
            }
            _ => Err("unbound file payload shape".into()),
        })
        .collect()
}
fn file_owner_contract() -> Result<(), String> {
    file_owner_contract_at(None, 12, None)
}
pub(crate) fn codex_file_owner_contract() -> Result<(), String> {
    file_owner_contract_at(None, 6, Some("codex"))
}
fn file_owner_contract_at(
    external: Option<&Path>,
    expected: usize,
    selected_client: Option<&str>,
) -> Result<(), String> {
    use crate::{
        client_deploy::{self, Cancellation, Files},
        generic::*,
    };
    use vibe_agent_projection::client_capabilities::{ClientCapabilities, ClientCapability};
    use vibe_core::manifest::{ClientRequest, ClientScope};
    use vibe_wire::generated::client_deploy::e1::reply::{ReplyAction, ReplyStatus};
    let packages = if let Some(external) = external {
        external.to_path_buf()
    } else if Path::new("/fixture-input/driver-seal.json").exists() {
        Path::new("/fixture-input/packages").to_path_buf()
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/next/adapters/m_08_e_1")
    };
    let index: serde_json::Value = serde_json::from_slice(
        &std::fs::read(packages.join("PACKAGES.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let rows = index["packages"]
        .as_array()
        .ok_or("missing external packages")?;
    let rows: Vec<_> = rows
        .iter()
        .filter(|r| selected_client.is_none_or(|c| r["client"] == c))
        .collect();
    if rows.len() != expected {
        return Err("file matrix denominator differs".into());
    }
    for row in rows {
        let id = row["id"].as_str().ok_or("package id")?;
        if !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
            || id.starts_with('.')
        {
            return Err("invalid external package entry".into());
        }
        let package = packages.join(id);
        let request: ClientRequest = serde_json::from_slice(
            &std::fs::read(package.join("request.json")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        request.validate()?;
        let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
        for name in ["project", "user", "settings", "arena"] {
            std::fs::create_dir(dir.path().join(name)).map_err(|e| e.to_string())?;
        }
        std::fs::create_dir(dir.path().join("evidence")).map_err(|e| e.to_string())?;
        let project = dir.path().join("project");
        let user = dir.path().join("user");
        let open = |name: &str| {
            vibe_safefs::Project::open(&dir.path().join(name)).map_err(|e| e.to_string())
        };
        let authority = Arc::new(FileAuthority {
            request_path: package.join("request.json"),
            policy: std::sync::Mutex::new("generation-1".into()),
        });
        let category = |name: &str| {
            [ClientScope::Project, ClientScope::User]
                .into_iter()
                .map(|scope| ProtectedResource {
                    scope,
                    relative: format!("controls/{name}"),
                })
                .collect()
        };
        let files = Arc::new(
            GenericHost::new(
                open("project")?,
                open("user")?,
                open("settings")?,
                authority.clone(),
                ProtectedControls {
                    grants: category("grants"),
                    policy: category("policy"),
                    producer_trust: category("trust"),
                    publication_mode: category("publication"),
                },
            )
            .map_err(|e| e.to_string())?
            .with_coordination(open("arena")?)
            .map_err(|e| e.to_string())?,
        );
        let observed = authority.selected_current(&request.client_id)?;
        std::fs::write(
            dir.path().join("evidence/capture.json"),
            serde_json::to_vec_pretty(&observed.identity).unwrap(),
        )
        .map_err(|e| e.to_string())?;
        std::fs::write(
            dir.path().join("evidence/request.json"),
            std::fs::read(package.join("request.json")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let map = ClientCapabilities::admit([ClientCapability {
            provider: observed.identity.provider,
            descriptor_digest: observed.identity.descriptor_digest.clone(),
            request: request.clone(),
            disabled: false,
        }])?;
        let runner = Arc::new(
            PodBayRunner::new(
                &dir.path().join("never-connected.sock"),
                dir.path(),
                Vec::new(),
            )
            .map_err(|e| e.to_string())?,
        );
        let commands = Arc::new(client_commands::Host::new(
            Arc::new(Authority(client_commands::Context {
                executable: ClientExecutable::Missing {
                    command: "no-client-authority".into(),
                },
                cwd: project.clone(),
                user_home: user.clone(),
                roots: vec![project.clone(), user.clone()],
                environment_ref: "file-only".into(),
                principal: "file-only".into(),
                policy: "file-only".into(),
                evidence: "no-vendor-observation".into(),
                revoked: true,
            })),
            runner.clone(),
        ));
        let host = client_deploy::Host::new(
            open("project")?,
            open("settings")?,
            open("arena")?,
            id.replace(['_', '.'], "-"),
            row["scope"].as_str().ok_or("scope")?.into(),
            commands,
            Arc::new(FileObserver {
                roots: vec![project.clone(), user.clone()],
                subject: observed.identity.descriptor_digest,
            }),
        )?;
        // Establish D2 storage before capturing C3 state-home identity.
        host.prepare_storage()?;
        let binding = &request.bindings[0];
        let destination_root = match binding.scope {
            ClientScope::Project => &project,
            ClientScope::User => &user,
        };
        let destination = destination_root.join(&binding.destination);
        let structured = binding.shape != vibe_core::manifest::ClientShape::Tree;
        if structured {
            std::fs::create_dir_all(destination.parent().unwrap()).map_err(|e| e.to_string())?;
            let seed: &[u8] = if binding.shape == vibe_core::manifest::ClientShape::TomlMember {
                b"m08e1_foreign = 'keep'\n"
            } else {
                br#"{"m08e1_foreign":"keep"}"#
            };
            std::fs::write(&destination, seed).map_err(|e| e.to_string())?;
        }
        // Foreign siblings are outside the owned set, with exact inode mtime.
        let foreign = project.join("foreign.txt");
        std::fs::write(&foreign, b"preserve\n").map_err(|e| e.to_string())?;
        let foreign_before = std::fs::metadata(&foreign)
            .map_err(|e| e.to_string())?
            .modified()
            .map_err(|e| e.to_string())?;
        let mut target = String::new();
        for (phase, action) in [
            ("v1", ReplyAction::Apply),
            ("v2", ReplyAction::Update),
            ("removed", ReplyAction::Remove),
        ] {
            retain_file_snapshot(dir.path(), &format!("{phase}-before"))?;
            let file_plan = if phase == "removed" {
                Files::Retained {
                    host: files.clone(),
                    plan: files
                        .prepare_retained(&target, RetainedOperation::Cleanup)
                        .map_err(|e| e.to_string())?,
                }
            } else {
                let admitted_payload = payload(&package, phase, &request)?;
                std::fs::write(
                    dir.path()
                        .join("evidence")
                        .join(format!("{phase}-payload.json")),
                    serde_json::to_vec_pretty(&admitted_payload).unwrap(),
                )
                .map_err(|e| e.to_string())?;
                let admitted = files
                    .admit_payload(
                        files
                            .capture(&map, &request.client_id)
                            .map_err(|e| e.to_string())?,
                        admitted_payload,
                    )
                    .map_err(|e| e.to_string())?;
                let plan = files.plan(&admitted).map_err(|e| e.to_string())?;
                target = plan.target().into();
                Files::Apply {
                    host: files.clone(),
                    plan,
                }
            };
            let plan = host.prepare(action, Vec::new(), Some(file_plan))?;
            if host.plan(&plan).status != ReplyStatus::Planned {
                return Err(format!("{id}/{phase}: prepare failed"));
            }
            let grant = host.approve(&plan)?;
            let run = host.proposed_run(phase);
            host.initialize(&plan, &run)?;
            let refused = host.execute(
                &plan,
                None,
                &run,
                false,
                &Cancellation::default(),
                "2026-10-16T00:00:00Z",
            )?;
            assert_eq!(refused.status, ReplyStatus::NeedsAuthority);
            if phase == "v1" {
                crate::mechanism::deploy::state::generic::FAIL_PHASE.with(|failure| {
                    *failure.borrow_mut() =
                        Some(vibe_wire::generated::deploy_generic_state::BindingPhase::Checkpoint);
                });
            }
            let mut reply = host.execute(
                &plan,
                Some(&grant),
                &run,
                false,
                &Cancellation::default(),
                "2026-10-16T00:00:00Z",
            )?;
            if phase == "v1" {
                crate::mechanism::deploy::state::generic::FAIL_PHASE.with(|failure| {
                    failure.borrow_mut().take();
                });
                if reply.status != ReplyStatus::Partial {
                    return Err(format!(
                        "{id}: intended C3 checkpoint interruption not observed"
                    ));
                }
                std::fs::write(
                    dir.path().join("evidence/v1-interrupted-reply.json"),
                    serde_json::to_vec_pretty(&reply).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                retain_file_snapshot(dir.path(), "v1-interrupted")?;
                let admitted = files
                    .admit_payload(
                        files
                            .capture(&map, &request.client_id)
                            .map_err(|e| e.to_string())?,
                        payload(&package, phase, &request)?,
                    )
                    .map_err(|e| e.to_string())?;
                let retained = files
                    .prepare_recovery(&target, Some(&admitted))
                    .map_err(|e| e.to_string())?;
                let recovery = host.prepare_recovery_with_files(
                    &run,
                    Vec::new(),
                    Some(Files::Retained {
                        host: files.clone(),
                        plan: retained,
                    }),
                )?;
                let fresh = host.approve(&recovery)?;
                reply = host.execute(
                    &recovery,
                    Some(&fresh),
                    &run,
                    true,
                    &Cancellation::default(),
                    "2026-10-16T00:00:00Z",
                )?;
                if reply.run_id != run {
                    return Err("recovery changed original run identity".into());
                }
            }
            if reply.status != ReplyStatus::Completed
                || host.verify(&run)?.status != ReplyStatus::Completed
            {
                return Err(format!("{id}/{phase}: {reply:?}"));
            }
            std::fs::write(
                dir.path()
                    .join("evidence")
                    .join(format!("{phase}-reply.json")),
                serde_json::to_vec_pretty(&reply).unwrap(),
            )
            .map_err(|e| e.to_string())?;
            if phase != "removed" {
                let admitted = files
                    .admit_payload(
                        files
                            .capture(&map, &request.client_id)
                            .map_err(|e| e.to_string())?,
                        payload(&package, phase, &request)?,
                    )
                    .map_err(|e| e.to_string())?;
                let retained = files
                    .prepare_recovery(&target, Some(&admitted))
                    .map_err(|e| e.to_string())?;
                let recovery = host.prepare_recovery_with_files(
                    &run,
                    Vec::new(),
                    Some(Files::Retained {
                        host: files.clone(),
                        plan: retained,
                    }),
                )?;
                let fresh = host.approve(&recovery)?;
                let recovered = host.execute(
                    &recovery,
                    Some(&fresh),
                    &run,
                    true,
                    &Cancellation::default(),
                    "2026-10-16T00:00:00Z",
                )?;
                assert_eq!(recovered.status, ReplyStatus::Completed);
                assert_eq!(recovered.run_id, run);
            }
            if phase == "v2" && (selected_client == Some("codex") || expected == 1) {
                // The incumbent C3 retained before-image owner performs inverse.
                // D2 independently checks that the original v1 desired set was
                // restored; these are file-owner facts, not vendor observations.
                let inverse = files
                    .prepare_retained(&target, RetainedOperation::Inverse)
                    .map_err(|e| e.to_string())?;
                if files
                    .execute_retained(&inverse, None, "2026-10-16T00:00:00Z")
                    .is_ok()
                {
                    return Err("ungranted inverse admitted".into());
                }
                let inverse_grant = files
                    .approve_retained(&inverse)
                    .map_err(|e| e.to_string())?;
                files
                    .execute_retained(&inverse, Some(&inverse_grant), "2026-10-16T00:00:00Z")
                    .map_err(|e| e.to_string())?;
                let restored = host.verify(&host.proposed_run("v1"))?;
                if restored.status != ReplyStatus::Completed {
                    return Err(format!(
                        "{id}: inverse did not restore independently observed v1"
                    ));
                }
                std::fs::write(
                    dir.path().join("evidence/v2-inverse-verify.json"),
                    serde_json::to_vec_pretty(&restored).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                retain_file_snapshot(dir.path(), "v2-inverse")?;
                let admitted = files
                    .admit_payload(
                        files
                            .capture(&map, &request.client_id)
                            .map_err(|e| e.to_string())?,
                        payload(&package, "v2", &request)?,
                    )
                    .map_err(|e| e.to_string())?;
                let reapplied = files.plan(&admitted).map_err(|e| e.to_string())?;
                let fresh = files.approve(&reapplied).map_err(|e| e.to_string())?;
                files
                    .apply(&reapplied, Some(&fresh), "2026-10-16T00:00:00Z")
                    .map_err(|e| e.to_string())?;
                if host.verify(&run)?.status != ReplyStatus::Completed {
                    return Err("fresh v2 reapply after inverse not independently verified".into());
                }
            }
            if structured {
                let bytes = std::fs::read_to_string(&destination).map_err(|e| e.to_string())?;
                if binding.shape == vibe_core::manifest::ClientShape::TomlMember {
                    assert_eq!(
                        toml::from_str::<toml::Value>(&bytes).unwrap()["m08e1_foreign"].as_str(),
                        Some("keep")
                    );
                } else {
                    assert_eq!(
                        serde_json::from_str::<serde_json::Value>(&bytes).unwrap()["m08e1_foreign"],
                        "keep"
                    );
                }
            } else {
                assert_eq!(destination.join("reference.txt").exists(), phase == "v1");
                assert_eq!(destination.join("SKILL.md").exists(), phase != "removed");
            }
            assert_eq!(std::fs::read(&foreign).unwrap(), b"preserve\n");
            assert_eq!(
                std::fs::metadata(&foreign).unwrap().modified().unwrap(),
                foreign_before
            );
            assert_eq!(runner.remaining(), 0);
            retain_file_snapshot(dir.path(), &format!("{phase}-after"))?;
        }
        let retained = dir.keep();
        eprintln!(
            "file-owner-only {id}: D2 checkpoint interruption/fresh same-run recovery, apply/update/remove and independent scoped verify completed; retained {}; vendor resource loading remains pending",
            retained.display()
        );
    }
    Ok(())
}
fn retain_file_snapshot(root: &Path, label: &str) -> Result<(), String> {
    use std::os::unix::fs::MetadataExt;
    let mut snapshot = std::collections::BTreeMap::new();
    for scope in ["project", "user", "settings"] {
        for entry in walkdir::WalkDir::new(root.join(scope)).follow_links(false) {
            let entry = entry.map_err(|e| e.to_string())?;
            let metadata = std::fs::symlink_metadata(entry.path()).map_err(|e| e.to_string())?;
            if metadata.file_type().is_symlink() {
                return Err("unexpected linked fixture state".into());
            }
            if !metadata.is_file() {
                continue;
            }
            if metadata.len() > 1024 * 1024 {
                return Err("fixture state exceeds bounded snapshot".into());
            }
            let bytes = std::fs::read(entry.path()).map_err(|e| e.to_string())?;
            snapshot.insert(entry.path().strip_prefix(root).unwrap().display().to_string(),serde_json::json!({"bytes":bytes,"sha256":podbay_runner::hash(&bytes),"dev":metadata.dev(),"ino":metadata.ino(),"mtime":metadata.mtime(),"mtime_nsec":metadata.mtime_nsec()}));
        }
    }
    std::fs::write(
        root.join("evidence").join(format!("{label}.json")),
        serde_json::to_vec_pretty(&snapshot).unwrap(),
    )
    .map_err(|e| e.to_string())
}
pub fn extensibility_contract() -> Result<(), String> {
    external_package_data_contract()?;
    namespace()?;
    Err(
        "pending: actual vendor matrix must run against the frozen final implementation baseline"
            .into(),
    )
}
pub(crate) fn external_package_data_contract() -> Result<(), String> {
    // Capture the running core before introducing the new external descriptor
    // into this experiment's fresh input root. No rebuild occurs in between.
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let before = podbay_runner::hash(&std::fs::read(&executable).map_err(|e| e.to_string())?);
    let source = if Path::new("/fixture-input/driver-seal.json").exists() {
        Path::new("/fixture-input/packages").to_path_buf()
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/next/adapters/m_08_e_1")
    };
    let external = tempfile::tempdir().map_err(|e| e.to_string())?;
    let id = "external-addition.skill.project";
    let package = external.path().join(id);
    std::fs::create_dir(&package).map_err(|e| e.to_string())?;
    let template = source.join("codex.skill.project");
    let mut request: vibe_core::manifest::ClientRequest = serde_json::from_slice(
        &std::fs::read(template.join("request.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    request.client_id = "org.vibevm.fixture/externally-added-client".into();
    request.version = "=1.0.0".into();
    request.bindings[0].destination = "external-added/skills/demo".into();
    std::fs::write(
        package.join("request.json"),
        serde_json::to_vec_pretty(&request).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    for phase in ["v1", "v2"] {
        std::fs::create_dir(package.join(phase)).map_err(|e| e.to_string())?;
        std::fs::copy(
            template.join(phase).join("SKILL.md"),
            package.join(phase).join("SKILL.md"),
        )
        .map_err(|e| e.to_string())?;
    }
    std::fs::copy(
        template.join("v1/reference.txt"),
        package.join("v1/reference.txt"),
    )
    .map_err(|e| e.to_string())?;
    std::fs::write(
        external.path().join("PACKAGES.json"),
        serde_json::to_vec(&serde_json::json!({"packages":[{"id":id,"scope":"project"}]})).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    file_owner_contract_at(Some(external.path()), 1, None)?;
    let after = podbay_runner::hash(&std::fs::read(&executable).map_err(|e| e.to_string())?);
    assert_eq!(
        before, after,
        "core executable changed during external data introduction"
    );
    eprintln!(
        "external-data-only unchanged executable digest {before}; actual vendor extensibility remains pending"
    );
    Ok(())
}

/// Read-only installed-release observation. Physical lifecycle stays in PodBay.
fn codex_session(seal: Seal) -> Result<(), String> {
    use podbay_runner::session_protocol::*;
    use std::{
        io::Write,
        os::unix::net::UnixStream,
        time::{Duration, Instant},
    };
    use tungstenite::{Message, protocol::WebSocketConfig};
    let plugin_recovery = seal.stage == "codex-plugin-recovery";
    let plugin_update = seal.stage == "codex-plugin-update";
    let plugin_remove = seal.stage == "codex-plugin-remove" || plugin_update;
    let plugin_install = seal.stage == "codex-plugin-install" || plugin_remove || plugin_recovery;
    let plugin_probe = seal.stage == "codex-plugin-probe";
    let mcp_lifecycle = seal.stage == "codex-mcp-lifecycle";
    let trusted_project = seal.stage == "codex-project-trust";
    let resource_row = if matches!(
        seal.stage.as_str(),
        "codex-resource-lifecycle" | "codex-project-trust" | "codex-mcp-lifecycle"
    ) {
        Some(seal.row_id.clone().ok_or("missing Codex resource row")?)
    } else {
        None
    };
    if trusted_project && resource_row.as_deref() != Some("codex.config.project") {
        return Err("trust companion is scoped to project config case".into());
    }
    let case = resource_row
        .as_ref()
        .map(|row| {
            format!(
                "{row}.lifecycle-{}",
                if mcp_lifecycle {
                    "026"
                } else if trusted_project {
                    "021"
                } else {
                    "017"
                }
            )
        })
        .unwrap_or_else(|| {
            if plugin_recovery {
                "codex.plugin.user.recovery-031"
            } else if plugin_update {
                "codex.plugin.user.update-029"
            } else if plugin_remove {
                "codex.plugin.user.remove-028"
            } else if plugin_install {
                "codex.plugin.user.install-027"
            } else if plugin_probe {
                "codex.plugin.user.probe-025"
            } else {
                "codex.session-006"
            }
            .into()
        });
    if mcp_lifecycle
        && (!matches!(
            resource_row.as_deref(),
            Some("codex.mcp.project" | "codex.mcp.user")
        ) || seal.peer_address.as_deref() != Some("127.0.0.1:48591"))
    {
        return Err("unbound Codex MCP row/peer".into());
    }
    let start = seal.session_start.ok_or("missing exact session start")?;
    let SessionAction::Start { spec } = &start.action else {
        return Err("expected session start spec".into());
    };
    let expected_env = std::collections::BTreeMap::from([
        ("HOME".into(), "/home/olegchir".into()),
        ("USERPROFILE".into(), "/home/olegchir".into()),
        ("CODEX_HOME".into(), "/home/olegchir/.codex".into()),
        ("LANG".into(), "C.UTF-8".into()),
        ("LC_ALL".into(), "C.UTF-8".into()),
        ("TEMP".into(), "/tmp".into()),
        ("TMP".into(), "/tmp".into()),
    ]);
    if start.protocol != SESSION_PROTOCOL
        || start.case_id != case
        || start.step_id != "start"
        || if plugin_install {
            seal.steps.len()
                != if plugin_recovery {
                    4
                } else if plugin_update {
                    6
                } else if plugin_remove {
                    4
                } else {
                    2
                }
        } else {
            !seal.steps.is_empty()
        }
        || spec.program != "/runtime/codex/codex"
        || spec.args != ["app-server", "--listen", "unix:///case/arena/codex.sock"]
        || spec.cwd != "/case/project"
        || spec.scratch != "/case/arena"
        || spec.env != expected_env
        || spec.stdin != Some(Vec::new())
    {
        return Err("unbound Codex session manifest".into());
    }
    let mut lifecycle = resource_row
        .as_deref()
        .map(resource_file_lifecycle)
        .transpose()?;
    let started = podbay_runner::session_exchange(&start, "start")?;
    if started.status != SessionStatus::Started
        || started
            .physical_receipt
            .as_deref()
            .is_none_or(str::is_empty)
    {
        return Err("session not started; retain binding, no redispatch".into());
    }
    let request = |stop: bool| SessionRequest {
        protocol: SESSION_PROTOCOL.into(),
        case_id: start.case_id.clone(),
        step_id: start.step_id.clone(),
        action: if stop {
            SessionAction::Stop {
                session_id: started.session_id.clone(),
            }
        } else {
            SessionAction::Inspect {
                session_id: started.session_id.clone(),
            }
        },
    };
    let observe = |step: &str| -> Result<(), String> {
        let reply = podbay_runner::session_exchange(&request(false), step)?;
        if reply.status != SessionStatus::Observed
            || reply.physical_receipt.as_deref().is_none_or(str::is_empty)
        {
            return Err("session snapshot not observed/bound".into());
        }
        let o = reply.output.ok_or("missing session output")?;
        if o.exited
            || o.timed_out
            || o.stopped
            || o.signal.is_some()
            || o.stdin_error
            || o.stdout.truncated
            || o.stderr.truncated
            || o.stdout.io_error
            || o.stderr.io_error
        {
            return Err("session exited or output incomplete before query completion".into());
        }
        Ok(())
    };
    let queries = (|| -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(90);
        let mut transcript = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open("/case/evidence/codex-session-transcript.jsonl")
            .map_err(|e| e.to_string())?;
        let mut total = 0usize;
        let mut retain = |direction: &str, value: &serde_json::Value| -> Result<(), String> {
            let mut bytes =
                serde_json::to_vec(&serde_json::json!({"direction":direction,"value":value}))
                    .map_err(|e| e.to_string())?;
            bytes.push(b'\n');
            total += bytes.len();
            if total > 262144 {
                return Err("RPC transcript bound reached; incomplete".into());
            }
            transcript
                .write_all(&bytes)
                .and_then(|_| transcript.sync_all())
                .map_err(|e| e.to_string())
        };
        let mut readiness = 0u32;
        let readiness_deadline = Instant::now() + Duration::from_secs(10);
        let mut next_observation = Instant::now();
        let socket = loop {
            if Instant::now() >= readiness_deadline {
                return Err("Unix readiness bound".into());
            }
            match UnixStream::connect("/case/arena/codex.sock") {
                Ok(s) => break s,
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused
                    ) =>
                {
                    if Instant::now() >= next_observation && readiness < 32 {
                        readiness += 1;
                        observe(&format!("readiness-{readiness:03}"))?;
                        next_observation = Instant::now() + Duration::from_millis(300);
                    }
                    std::thread::yield_now();
                }
                Err(e) => return Err(e.to_string()),
            }
        };
        socket
            .set_read_timeout(Some(Duration::from_secs(10)))
            .map_err(|e| e.to_string())?;
        socket
            .set_write_timeout(Some(Duration::from_secs(5)))
            .map_err(|e| e.to_string())?;
        let config = WebSocketConfig::default()
            .max_message_size(Some(65536))
            .max_frame_size(Some(65536));
        let (mut ws, handshake) =
            tungstenite::client::client_with_config("ws://localhost/", socket, Some(config))
                .map_err(|e| e.to_string())?;
        retain(
            "handshake",
            &serde_json::json!({"status":handshake.status().as_u16()}),
        )?;
        let mut requests = vec![
            serde_json::json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":"m08e1-local-observer","version":"1.0.0"},"capabilities":{"experimentalApi":true,"explicitGatewayOauth":true}}}),
            serde_json::json!({"id":2,"method":"skills/list","params":{"cwds":["/case/project"],"forceReload":true}}),
            serde_json::json!({"id":3,"method":"config/read","params":{"cwd":"/case/project","includeLayers":false}}),
        ];
        if let Some(row) = &resource_row {
            requests.truncate(1);
            for id in 2..=5 {
                requests.push(if row.contains(".skill.") {
                    serde_json::json!({"id":id,"method":"skills/list","params":{"cwds":["/case/project"],"forceReload":true}})
                } else {
                    serde_json::json!({"id":id,"method":"config/read","params":{"cwd":"/case/project","includeLayers":trusted_project}})
                });
            }
        }
        if mcp_lifecycle {
            requests.truncate(1);
            for (id, method) in [
                (2, "mcpServerStatus/list"),
                (3, "config/mcpServer/reload"),
                (4, "mcpServerStatus/list"),
                (5, "mcpServer/resource/read"),
                (6, "config/mcpServer/reload"),
                (7, "mcpServerStatus/list"),
                (8, "mcpServer/resource/read"),
                (9, "config/mcpServer/reload"),
                (10, "mcpServerStatus/list"),
            ] {
                let params = match method {
                    "mcpServerStatus/list" => serde_json::json!({"detail":"full","limit":16}),
                    "mcpServer/resource/read" => {
                        serde_json::json!({"server":"m08a2-demo","uri":"m08e1://resource"})
                    }
                    _ => serde_json::Value::Null,
                };
                requests.push(serde_json::json!({"id":id,"method":method,"params":params}));
            }
        }
        if plugin_probe {
            requests.truncate(1);
            requests.extend([
                serde_json::json!({"id":2,"method":"plugin/list","params":{"cwds":[],"marketplaceKinds":["local"],"forceRefetch":false}}),
                serde_json::json!({"id":3,"method":"plugin/read","params":{"marketplacePath":"/home/olegchir/market/.agents/plugins/marketplace.json","pluginName":"m08e1-demo"}}),
                serde_json::json!({"id":4,"method":"skills/list","params":{"cwds":["/case/project"],"forceReload":true}}),
            ]);
        }
        if plugin_install {
            requests.truncate(1);
            for id in [2, 3] {
                requests.push(serde_json::json!({"id":id,"method":"plugin/list","params":{"cwds":[],"marketplaceKinds":["local"],"forceRefetch":false}}));
            }
            requests.push(serde_json::json!({"id":4,"method":"skills/list","params":{"cwds":["/case/project"],"forceReload":true}}));
        }
        if plugin_remove {
            requests.push(serde_json::json!({"id":5,"method":"plugin/list","params":{"cwds":[],"marketplaceKinds":["local"],"forceRefetch":false}}));
            requests.push(serde_json::json!({"id":6,"method":"skills/list","params":{"cwds":["/case/project"],"forceReload":true}}));
        }
        if plugin_update {
            requests.push(serde_json::json!({"id":7,"method":"plugin/list","params":{"cwds":[],"marketplaceKinds":["local"],"forceRefetch":false}}));
            requests.push(serde_json::json!({"id":8,"method":"skills/list","params":{"cwds":["/case/project"],"forceReload":true}}));
        }
        let plugin_campaign = if plugin_update {
            "029"
        } else if plugin_remove {
            "028"
        } else {
            "027"
        };
        let mut mcp_peer: Option<LocalPeer> = None;
        let mut received = std::collections::BTreeSet::new();
        let mut messages = 0usize;
        for request in requests {
            if Instant::now() >= deadline {
                return Err("RPC deadline".into());
            }
            let phase = if mcp_lifecycle {
                match request["id"].as_u64() {
                    Some(2) => Some("absence"),
                    Some(3 | 4 | 5) => Some("v1"),
                    Some(6 | 7 | 8) => Some("v2"),
                    Some(9 | 10) => Some("removed"),
                    _ => None,
                }
            } else {
                request["id"]
                    .as_u64()
                    .and_then(|id| id.checked_sub(2))
                    .and_then(|n| ["absence", "v1", "v2", "removed"].get(n as usize).copied())
            };
            let transition_now =
                !mcp_lifecycle || matches!(request["id"].as_u64(), Some(2 | 3 | 6 | 9));
            if let (Some(apply), Some(phase)) = (lifecycle.as_mut(), phase) {
                if transition_now {
                    apply(phase)?;
                }
            }
            if mcp_lifecycle && matches!(request["id"].as_u64(), Some(3 | 6)) {
                mcp_peer = Some(LocalPeer::start(
                    "127.0.0.1:48591",
                    Path::new("/case/evidence"),
                    phase.unwrap(),
                )?);
            }
            if plugin_install && request["id"] == 3 {
                if plugin_recovery {
                    codex_plugin_recovery(&seal.steps)?;
                } else {
                    codex_plugin_effect(&seal.steps[..2], "install", plugin_campaign)?;
                }
            }
            if plugin_remove && request["id"] == 5 {
                if plugin_update {
                    advance_plugin_fixture()?;
                    codex_plugin_effect(&seal.steps[2..4], "update", plugin_campaign)?;
                } else {
                    codex_plugin_effect(&seal.steps[2..], "remove", plugin_campaign)?;
                }
            }
            if plugin_update && request["id"] == 7 {
                codex_plugin_effect(&seal.steps[4..], "remove", plugin_campaign)?;
            }
            let plugin_before = if plugin_probe {
                Some(codex_plugin_probe_witness()?)
            } else {
                None
            };
            retain("send", &request)?;
            ws.send(Message::Text(request.to_string().into()))
                .map_err(|e| e.to_string())?;
            let id = request["id"].as_u64().unwrap();
            loop {
                if Instant::now() >= deadline || messages >= 128 {
                    return Err("RPC observation bound".into());
                }
                messages += 1;
                let remaining = deadline
                    .saturating_duration_since(Instant::now())
                    .min(Duration::from_secs(10));
                if remaining.is_zero() {
                    return Err("RPC deadline".into());
                }
                ws.get_mut()
                    .set_read_timeout(Some(remaining))
                    .map_err(|e| e.to_string())?;
                match ws.read().map_err(|e| e.to_string())? {
                    Message::Text(text) => {
                        let value: serde_json::Value = match serde_json::from_str(&text) {
                            Ok(v) => v,
                            Err(e) => {
                                retain("malformed", &serde_json::json!({"text":text.as_str()}))?;
                                return Err(e.to_string());
                            }
                        };
                        retain("receive", &value)?;
                        if let Some(reply_id) = value.get("id") {
                            if reply_id.as_u64() != Some(id)
                                || !received.insert(id)
                                || value.get("error").is_some()
                                || value.get("result").is_none()
                            {
                                return Err("unexpected/duplicate/error RPC response".into());
                            }
                            if plugin_install && id > 1 {
                                if plugin_update {
                                    codex_plugin_observed(
                                        id,
                                        &value["result"],
                                        if id >= 5 { "2.0.0" } else { "1.0.0" },
                                        matches!(id, 3..=6),
                                    )?;
                                } else {
                                    codex_plugin_install_observed(id, &value["result"])?;
                                }
                            }
                            if let Some(before) = &plugin_before {
                                let after = codex_plugin_probe_witness()?;
                                private_record(&Path::new("/case/evidence").join(format!("plugin-query-{id}-closure.json")), &serde_json::to_vec_pretty(&serde_json::json!({"before":format!("{before:?}"),"after":format!("{after:?}"),"stable":before == &after,"claim":"bounded candidate observation; semantic qualification remains Root review"})).map_err(|e|e.to_string())?)?;
                                if before != &after {
                                    return Err("plugin query prepared/migrated state; retain observation and require new checkpoint".into());
                                }
                                if id == 2 {
                                    let markets = value["result"]["marketplaces"]
                                        .as_array()
                                        .ok_or("missing actual marketplace array")?;
                                    let selected: Vec<_> = markets
                                        .iter()
                                        .filter(|m| m["name"] == "m08e1-local")
                                        .collect();
                                    if selected.len() != 1
                                        || selected[0]["path"]
                                            != "/home/olegchir/market/.agents/plugins/marketplace.json"
                                    {
                                        return Err("actual local marketplace differs".into());
                                    }
                                    let plugins = selected[0]["plugins"]
                                        .as_array()
                                        .ok_or("missing plugins")?;
                                    if plugins.len() != 1
                                        || plugins[0]["name"] != "m08e1-demo"
                                        || plugins[0]["installed"] != false
                                    {
                                        return Err(
                                            "actual absence plugin inventory differs".into()
                                        );
                                    }
                                }
                            }
                            if let (Some(row), Some(phase)) = (&resource_row, phase) {
                                if mcp_lifecycle {
                                    if request["method"] == "mcpServerStatus/list" {
                                        codex_mcp_observed(phase, &value["result"], false)?;
                                    } else if request["method"] == "mcpServer/resource/read" {
                                        codex_mcp_observed(phase, &value["result"], true)?;
                                        mcp_peer.take().ok_or("missing MCP peer")?.finish()?;
                                    }
                                } else {
                                    codex_resource_observed(row, phase, &value["result"])?;
                                }
                            }
                            break;
                        }
                        if !value.get("method").is_some_and(|v| v.is_string()) {
                            return Err("invalid RPC notification".into());
                        }
                    }
                    Message::Ping(bytes) => {
                        retain("ping", &serde_json::json!({"bytes":bytes.as_ref()}))?;
                        ws.flush().map_err(|e| e.to_string())?;
                    }
                    Message::Pong(bytes) => {
                        retain("pong", &serde_json::json!({"bytes":bytes.as_ref()}))?
                    }
                    other => {
                        retain(
                            "unexpected-frame",
                            &serde_json::json!({"frame":format!("{other:?}")}),
                        )?;
                        return Err("unexpected RPC frame or EOF".into());
                    }
                }
            }
            if id == 1 {
                let initialized = serde_json::json!({"method":"initialized"});
                retain("send", &initialized)?;
                ws.send(Message::Text(initialized.to_string().into()))
                    .map_err(|e| e.to_string())?;
            }
        }
        ws.close(None).map_err(|e| e.to_string())?;
        Ok(())
    })();
    // Always ask the original physical owner to retain output then stop this
    // known binding, even when RPC parsing/connection failed. No native kill.
    let observed = observe("final-output");
    let stopped = podbay_runner::session_exchange(&request(true), "stop");
    let cleanup = stopped.and_then(|r| {
        if r.status != SessionStatus::Stopped
            || r.output.is_none()
            || r.stop_receipt.as_deref().is_none_or(str::is_empty)
        {
            Err("PodBay stop/retained snapshot unconfirmed".into())
        } else {
            Ok(())
        }
    });
    let result = serde_json::json!({"query_result":queries,"output_result":observed,"cleanup_result":cleanup,
        "deliberate_stop_is_exit_zero":false,"matrix_status":"pending"});
    std::fs::write(
        "/case/evidence/codex-session-result.json",
        serde_json::to_vec_pretty(&result).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    queries?;
    observed?;
    cleanup?;
    Err("pending: actual read-only RPC transport observed; package matrix and local resource lifecycle still required".into())
}

fn peer_trace_replay() -> Result<(), String> {
    use serde_json::{Value, json};
    let discover: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/next/adapters/m_08_e_1/mcp-replay/discover.json"
    )))
    .map_err(|e| e.to_string())?;
    let initialize: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/next/adapters/m_08_e_1/mcp-replay/initialize.json"
    )))
    .map_err(|e| e.to_string())?;
    let mut records = Vec::new();
    for revision in ["v1", "v2"] {
        let discovery = LocalPeer::reply(&discover, revision)?.ok_or("discovery reply absent")?;
        if discovery
            != json!({"jsonrpc":"2.0","id":discover["id"],"error":{"code":-32601,"message":"Method not found"}})
        {
            return Err("discovery fallback changed original id/error".into());
        }
        let initialized =
            LocalPeer::reply(&initialize, revision)?.ok_or("initialize reply absent")?;
        if initialized["id"] != initialize["id"]
            || initialized["result"]["protocolVersion"] != "2025-06-18"
            || initialized["result"]["capabilities"] != json!({"resources":{},"tools":{}})
            || initialized["result"]["serverInfo"]["version"] != revision
        {
            return Err("explicit supported negotiation/capabilities differ".into());
        }
        records.push(json!({"revision":revision,"request":discover,"response":discovery}));
        records.push(json!({"revision":revision,"request":initialize,"response":initialized}));
        for method in [
            "notifications/initialized",
            "notifications/cancelled",
            "notifications/unsupported",
        ] {
            let request = json!({"jsonrpc":"2.0","method":method});
            if LocalPeer::reply(&request, revision)?.is_some() {
                return Err("notification unexpectedly replied".into());
            }
            records.push(json!({"revision":revision,"request":request,"response":null}));
        }
        for (id, method, params) in [
            (1, "ping", json!({})),
            (2, "tools/list", json!({})),
            (3, "resources/list", json!({})),
            (4, "resources/templates/list", json!({})),
            (5, "resources/read", json!({"uri":"m08e1://resource"})),
            (
                6,
                "tools/call",
                json!({"name":"fixture_resource","arguments":{}}),
            ),
        ] {
            let request = json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
            let response =
                LocalPeer::reply(&request, revision)?.ok_or("control/resource response absent")?;
            if response["id"] != id || response.get("error").is_some() {
                return Err("control/resource response differs".into());
            }
            if method == "resources/read"
                && response["result"]["contents"][0]["text"]
                    != format!("M08E1 {revision} exact local resource\n")
            {
                return Err("resource bytes differ".into());
            }
            if method == "tools/call"
                && response["result"]["content"][0]["text"]
                    != format!("M08E1 {revision} exact local resource\n")
            {
                return Err("tool resource bytes differ".into());
            }
            records.push(json!({"revision":revision,"request":request,"response":response}));
        }
    }
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    std::fs::write(
        dir.path().join("replay.json"),
        serde_json::to_vec_pretty(&records).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let retained = dir.keep();
    eprintln!(
        "peer-only retained actual discovery/initialize trace replay: {} bounded semantic checks; {}; no vendor invocation",
        records.len(),
        retained.display()
    );
    Ok(())
}

const RESOURCE_MODEL: &str = "claude-sonnet-4-5-20250929";
const RESOURCE_DESCRIPTION: &str = "M08E1_RESOURCE_DESCRIPTION_016";
const RESOURCE_LANGUAGE: &str = "Always respond in Esperanto";

fn private_record(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::{io::Write, os::unix::fs::OpenOptionsExt};
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| e.to_string())
}

// Export only exact marker fragments and hashes of their containing values.
// Searching parsed context excludes headers, model echo and arbitrary raw text.
fn resource_markers(request: &serde_json::Value) -> Vec<serde_json::Value> {
    fn visit(value: &serde_json::Value, pointer: &str, found: &mut Vec<serde_json::Value>) {
        match value {
            serde_json::Value::String(s) => {
                for marker in [
                    RESOURCE_LANGUAGE,
                    RESOURCE_DESCRIPTION,
                    "Always respond in Icelandic",
                    "M08E1_CLAUDE_DESCRIPTION_v1_022",
                    "M08E1_CLAUDE_DESCRIPTION_v2_022",
                    "M08E1_CLAUDE_BODY_v1_022",
                    "M08E1_CLAUDE_BODY_v2_022",
                ] {
                    if s.contains(marker) {
                        found.push(serde_json::json!({"pointer":pointer,"marker":marker,"value_sha256":podbay_runner::hash(s.as_bytes())}));
                    }
                }
            }
            serde_json::Value::Array(a) => {
                for (i, value) in a.iter().enumerate() {
                    visit(value, &format!("{pointer}/{i}"), found);
                }
            }
            serde_json::Value::Object(o) => {
                for (k, value) in o {
                    visit(
                        value,
                        &format!("{pointer}/{}", k.replace('~', "~0").replace('/', "~1")),
                        found,
                    );
                }
            }
            _ => (),
        }
    }
    let mut found = Vec::new();
    for key in ["system", "messages", "tools"] {
        if let Some(value) = request.get(key) {
            visit(value, &format!("/{key}"), &mut found);
        }
    }
    found
}

fn messages_reply(request: &serde_json::Value) -> Result<(&'static str, Vec<u8>), String> {
    use serde_json::json;
    if request["model"] != RESOURCE_MODEL || !request["messages"].is_array() {
        return Err("unexpected Messages model/envelope".into());
    }
    let streaming = match request.get("stream") {
        None => false,
        Some(serde_json::Value::Bool(value)) => *value,
        _ => return Err("invalid Messages stream flag".into()),
    };
    let usage = json!({"input_tokens":1,"output_tokens":1});
    let message = json!({"id":"msg_m08e1_local_016","type":"message","role":"assistant","model":RESOURCE_MODEL,"content":[{"type":"text","text":"OK"}],"stop_reason":"end_turn","stop_sequence":null,"usage":usage});
    if !streaming {
        return Ok((
            "application/json",
            serde_json::to_vec(&message).map_err(|e| e.to_string())?,
        ));
    }
    let mut initial = message.clone();
    initial["content"] = json!([]);
    initial["stop_reason"] = serde_json::Value::Null;
    initial["usage"]["output_tokens"] = json!(0);
    let events = [
        json!({"type":"message_start","message":initial}),
        json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}),
        json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"OK"}}),
        json!({"type":"content_block_stop","index":0}),
        json!({"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},"usage":{"output_tokens":1}}),
        json!({"type":"message_stop"}),
    ];
    let mut body = String::new();
    for event in events {
        body.push_str(&format!(
            "event: {}\ndata: {}\n\n",
            event["type"].as_str().unwrap(),
            event
        ));
    }
    Ok(("text/event-stream", body.into_bytes()))
}

impl LocalPeer {
    // Root013: deterministic non-forwarding HTTP fixture, not a provider client.
    fn messages(evidence: &Path, label: &str) -> Result<Self, String> {
        if label.len() > 40
            || !label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err("invalid Messages evidence label".into());
        }
        let label = label.to_owned();
        let listener = std::net::TcpListener::bind("127.0.0.1:33003").map_err(|e| e.to_string())?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let cancelled = stop.clone();
        let evidence = evidence.to_path_buf();
        let thread = std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
            let mut count = 0;
            while !cancelled.load(std::sync::atomic::Ordering::SeqCst) {
                if std::time::Instant::now() >= deadline {
                    return Err("Messages peer deadline".into());
                }
                match listener.accept() {
                    Ok((stream, remote)) => {
                        if !remote.ip().is_loopback() || count == 4 {
                            return Err("Messages connection bound".into());
                        }
                        count += 1;
                        Self::messages_exchange(stream, &evidence, &label, count)?;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(10))
                    }
                    Err(e) => return Err(e.to_string()),
                }
            }
            if count == 0 {
                return Err("no actual Messages request observed".into());
            }
            Ok(())
        });
        Ok(Self {
            stop,
            thread: Some(thread),
        })
    }

    fn messages_exchange(
        mut stream: std::net::TcpStream,
        evidence: &Path,
        label: &str,
        sequence: usize,
    ) -> Result<(), String> {
        use std::io::{BufRead, Read, Write};
        let timeout = Some(std::time::Duration::from_secs(2));
        stream
            .set_read_timeout(timeout)
            .map_err(|e| e.to_string())?;
        stream
            .set_write_timeout(timeout)
            .map_err(|e| e.to_string())?;
        // Both total bytes and wall time are bounded; slow partial senders cannot
        // extend this connection by repeatedly resetting the socket timeout.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut reader = std::io::BufReader::new((&mut stream).take(270336));
        let mut line = String::new();
        reader.read_line(&mut line).map_err(|e| e.to_string())?;
        private_record(
            &evidence.join(format!("messages-{label}{sequence:03}.route.json")),
            &serde_json::to_vec(&serde_json::json!({"request_line":line}))
                .map_err(|e| e.to_string())?,
        )?;
        let words: Vec<_> = line.split_whitespace().collect();
        if words.len() != 3
            || words[0] != "POST"
            || words[1].split('?').next() != Some("/v1/messages")
            || words[2] != "HTTP/1.1"
        {
            return Err("unexpected Messages route retained".into());
        }
        let mut header_bytes = line.len();
        let mut length = None;
        loop {
            if std::time::Instant::now() >= deadline {
                return Err("Messages headers deadline".into());
            }
            line.clear();
            if reader.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
                return Err("incomplete Messages headers".into());
            }
            header_bytes += line.len();
            if header_bytes > 8192 {
                return Err("oversized Messages headers".into());
            }
            if line == "\r\n" {
                break;
            }
            let (key, value) = line.split_once(':').ok_or("invalid Messages header")?;
            if key.eq_ignore_ascii_case("content-length") {
                if length.is_some() {
                    return Err("duplicate Messages length".into());
                }
                length = Some(
                    value
                        .trim()
                        .parse::<usize>()
                        .map_err(|_| "invalid Messages length")?,
                );
            }
            if key.eq_ignore_ascii_case("transfer-encoding")
                || key.eq_ignore_ascii_case("content-encoding")
            {
                return Err("encoded Messages body unqualified".into());
            }
        }
        let length = length
            .filter(|n| *n <= 262144)
            .ok_or("missing/oversized Messages length")?;
        let mut body = Vec::with_capacity(length);
        let read_result = (|| -> Result<(), String> {
            let mut chunk = [0; 4096];
            while body.len() < length {
                if std::time::Instant::now() >= deadline {
                    return Err("Messages body deadline".into());
                }
                let take = chunk.len().min(length - body.len());
                let n = reader.read(&mut chunk[..take]).map_err(|e| e.to_string())?;
                if n == 0 {
                    return Err("incomplete Messages body".into());
                }
                body.extend_from_slice(&chunk[..n]);
            }
            Ok(())
        })();
        drop(reader);
        private_record(
            &evidence.join(format!(
                "messages-{label}{sequence:03}.request.private.json"
            )),
            &body,
        )?;
        read_result?;
        let request: serde_json::Value =
            serde_json::from_slice(&body).map_err(|e| e.to_string())?;
        let summary = serde_json::json!({"request_sha256":podbay_runner::hash(&body),"request_bytes":body.len(),"markers":resource_markers(&request),"claim":"parsed actual request context; description inventory only, not skill body execution"});
        private_record(
            &evidence.join(format!("messages-{label}{sequence:03}.markers.json")),
            &serde_json::to_vec_pretty(&summary).map_err(|e| e.to_string())?,
        )?;
        let (mime, response) = messages_reply(&request)?;
        private_record(
            &evidence.join(format!("messages-{label}{sequence:03}.response")),
            &response,
        )?;
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", response.len()).map_err(|e|e.to_string())?;
        stream.write_all(&response).map_err(|e| e.to_string())
    }
}

fn claude_resource_probe(seal: Seal) -> Result<(), String> {
    let args = [
        "-p",
        "--model",
        RESOURCE_MODEL,
        "--max-turns",
        "1",
        "--output-format",
        "json",
        "--no-session-persistence",
        "--permission-mode",
        "dontAsk",
        "--tools",
        "Skill",
        "--strict-mcp-config",
        "--mcp-config",
        "{\"mcpServers\":{}}",
        "--",
        "Return OK.",
    ];
    if seal.peer_address.as_deref() != Some("127.0.0.1:33003") || seal.steps.len() != 1 {
        return Err("unbound Messages probe".into());
    }
    let step = &seal.steps[0];
    if step.case_id != "claude_code.resource.project.probe-019"
        || step.step_id != "observe"
        || step.spec.program != "/runtime/claude/claude"
        || step.spec.args != args
        || step.spec.cwd != "/case/project"
        || step.spec.stdin.is_some()
    {
        return Err("Messages exact invocation differs".into());
    }
    for (key, value) in [
        ("ANTHROPIC_BASE_URL", "http://127.0.0.1:33003"),
        (
            "ANTHROPIC_API_KEY",
            "m08e1-local-simulator-not-a-credential",
        ),
        ("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1"),
        ("CLAUDE_CODE_DISABLE_OFFICIAL_MARKETPLACE_AUTOINSTALL", "1"),
    ] {
        if step.spec.env.get(key).map(String::as_str) != Some(value) {
            return Err(format!("Messages sealed environment differs: {key}"));
        }
    }
    if step.spec.env.keys().any(|k| {
        k.contains("PROXY")
            || k.contains("OAUTH")
            || k == "ANTHROPIC_AUTH_TOKEN"
            || k == "CLAUDE_CONFIG_DIR"
    }) {
        return Err("unqualified provider/proxy environment".into());
    }
    // Minimal first probe inputs are Root-sealed, immutable fixture data. They
    // are not substituted for package lifecycle effects or counted as receipts.
    let settings =
        std::fs::read("/case/project/.claude/settings.json").map_err(|e| e.to_string())?;
    if serde_json::from_slice::<serde_json::Value>(&settings).map_err(|e| e.to_string())?
        != serde_json::json!({"language":"Esperanto"})
    {
        return Err("Messages probe settings differ".into());
    }
    let skill_path = "/case/project/.claude/skills/m08e1-probe/SKILL.md";
    let skill = std::fs::read(skill_path).map_err(|e| e.to_string())?;
    let expected = format!(
        "---\nname: m08e1-probe\ndescription: {RESOURCE_DESCRIPTION}\n---\nM08E1_RESOURCE_BODY_016\n"
    );
    if skill != expected.as_bytes() {
        return Err("Messages probe skill differs".into());
    }
    let runner = PodBayRunner::new(
        Path::new("/fixture-ipc/process-runner.sock"),
        Path::new("/case/evidence"),
        seal.steps.clone(),
    )
    .map_err(|e| e.to_string())?;
    let peer = LocalPeer::messages(Path::new("/case/evidence"), "")?;
    let output = runner.run(&podbay_runner::native(&step.spec));
    let peer_result = peer.finish();
    peer_result?;
    complete(&output.map_err(|e| e.to_string())?)?;
    if std::fs::read(skill_path).map_err(|e| e.to_string())? != skill
        || std::fs::read("/case/project/.claude/settings.json").map_err(|e| e.to_string())?
            != settings
    {
        return Err("Messages probe immutable inputs changed".into());
    }
    let mut found = std::collections::BTreeSet::new();
    for sequence in 1..=4 {
        let path = Path::new("/case/evidence").join(format!("messages-{sequence:03}.markers.json"));
        match std::fs::read(path) {
            Ok(bytes) => {
                let record: serde_json::Value =
                    serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                for marker in record["markers"]
                    .as_array()
                    .ok_or("invalid marker record")?
                {
                    found.insert(
                        marker["marker"]
                            .as_str()
                            .ok_or("invalid marker")?
                            .to_owned(),
                    );
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.to_string()),
        }
    }
    if !found.contains(RESOURCE_LANGUAGE)
        || !found.contains(RESOURCE_DESCRIPTION)
        || runner.remaining() != 0
    {
        return Err(
            "actual request lacks required language/description markers; retain original step"
                .into(),
        );
    }
    private_record(Path::new("/case/evidence/resource-probe-result.json"), br#"{"markers_observed":true,"matrix_status":"pending","skill_body":"not_proven","lifecycle":"not_run"}"#)?;
    Err("pending: initial Messages resource observation retained; both scopes and full package lifecycle still required".into())
}

fn messages_contract_checks() -> Result<(), String> {
    let mut request = serde_json::json!({"model":RESOURCE_MODEL,"messages":[{"role":"user","content":"Return OK."}],"system":[{"type":"text","text":RESOURCE_LANGUAGE}],"tools":[{"description":RESOURCE_DESCRIPTION}]});
    let markers = resource_markers(&request);
    assert_eq!(markers.len(), 2);
    assert_eq!(markers[0]["pointer"], "/system/0/text");
    assert_eq!(markers[1]["pointer"], "/tools/0/description");
    assert!(
        resource_markers(
            &serde_json::json!({"model":RESOURCE_DESCRIPTION,"output":RESOURCE_LANGUAGE})
        )
        .is_empty()
    );
    let (mime, body) = messages_reply(&request)?;
    assert_eq!(mime, "application/json");
    let value: serde_json::Value = serde_json::from_slice(&body).map_err(|e| e.to_string())?;
    assert_eq!(value["stop_reason"], "end_turn");
    assert_eq!(value["content"][0]["text"], "OK");
    request["stream"] = serde_json::json!(true);
    let (mime, body) = messages_reply(&request)?;
    assert_eq!(mime, "text/event-stream");
    let body = String::from_utf8(body).map_err(|e| e.to_string())?;
    let events: Vec<serde_json::Value> = body
        .lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    assert_eq!(events.len(), 6);
    assert_eq!(events[0]["message"]["content"], serde_json::json!([]));
    assert_eq!(events[2]["delta"]["text"], "OK");
    assert_eq!(events[5]["type"], "message_stop");
    request["stream"] = serde_json::json!("true");
    assert!(messages_reply(&request).is_err());
    request["model"] = serde_json::json!("unexpected");
    assert!(messages_reply(&request).is_err());
    Ok(())
}

/// Actual namespace file transitions still belong to C3/D2. The callback only
/// sequences those owners between read-only vendor observations.
fn resource_file_lifecycle(row: &str) -> Result<impl FnMut(&str) -> Result<(), String>, String> {
    use crate::{
        client_deploy::{self, Cancellation, Files},
        generic::*,
    };
    use vibe_agent_projection::client_capabilities::{ClientCapabilities, ClientCapability};
    use vibe_core::manifest::{ClientRequest, ClientScope, ClientShape};
    use vibe_wire::generated::client_deploy::e1::reply::{ReplyAction, ReplyStatus};
    let (scope, skill) = match row {
        "codex.skill.project" | "claude_code.skill.project" => (ClientScope::Project, true),
        "codex.skill.user" | "claude_code.skill.user" => (ClientScope::User, true),
        "codex.config.project" | "claude_code.config.project" | "codex.mcp.project" => {
            (ClientScope::Project, false)
        }
        "codex.config.user" | "claude_code.config.user" | "codex.mcp.user" => {
            (ClientScope::User, false)
        }
        _ => return Err("unbound Codex file row".into()),
    };
    let claude = row.starts_with("claude_code.");
    let mcp = row.contains(".mcp.");
    let companion = if matches!(row, "codex.config.project" | "codex.mcp.project") {
        let bytes =
            std::fs::read("/home/olegchir/.codex/config.toml").map_err(|e| e.to_string())?;
        if bytes != b"[projects.\"/case/project\"]\ntrust_level = \"trusted\"\n" {
            return Err("exact Root project trust companion required".into());
        }
        Some(bytes)
    } else {
        None
    };
    let package = Path::new("/fixture-input/resource-package");
    let request: ClientRequest = serde_json::from_slice(
        &std::fs::read(package.join("request.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    request.validate()?;
    if request.version != if claude { "=2.1.299" } else { "=0.159.3" }
        || request.bindings.len() != 1
    {
        return Err("unbound Codex descriptor version/shape".into());
    }
    let binding = &request.bindings[0];
    if binding.scope != scope
        || binding.destination
            != if skill {
                if claude {
                    ".claude/skills/m08a2-demo"
                } else {
                    ".agents/skills/m08a2-demo"
                }
            } else if claude {
                ".claude/settings.json"
            } else {
                ".codex/config.toml"
            }
        || binding.shape
            != if skill {
                ClientShape::Tree
            } else {
                if claude {
                    ClientShape::JsonMember
                } else {
                    ClientShape::TomlMember
                }
            }
        || binding.key
            != if skill {
                Vec::<String>::new()
            } else if mcp {
                vec!["mcp_servers".into(), "m08a2-demo".into()]
            } else {
                vec![
                    if claude {
                        "language"
                    } else {
                        "approval_policy"
                    }
                    .into(),
                ]
            }
    {
        return Err("Codex descriptor row mismatch".into());
    }
    // Validate both future payloads before admitting any deployment effect.
    for phase in ["v1", "v2"] {
        let data = payload(package, phase, &request)?;
        if mcp {
            let expected: toml::Value = toml::from_str(&format!(
                "url = \"http://127.0.0.1:48591/mcp/{phase}\"\nenabled = true\n"
            ))
            .map_err(|e| e.to_string())?;
            if !matches!(data.as_slice(), [GenericPayload::TomlMember(value)] if value == &expected)
            {
                return Err(
                    "MCP payload must contain only exact sealed local URL and enabled flag".into(),
                );
            }
        }
    }
    let project = std::path::PathBuf::from("/case/project");
    let user = std::path::PathBuf::from("/home/olegchir");
    let open = |p: &Path| vibe_safefs::Project::open(p).map_err(|e| e.to_string());
    let authority = Arc::new(FileAuthority {
        request_path: package.join("request.json"),
        policy: std::sync::Mutex::new(format!("{row}-017")),
    });
    let category = |name: &str| {
        [ClientScope::Project, ClientScope::User]
            .into_iter()
            .map(|scope| ProtectedResource {
                scope,
                relative: format!("controls/{name}"),
            })
            .collect()
    };
    let files = Arc::new(
        GenericHost::new(
            open(&project)?,
            open(&user)?,
            open(Path::new("/case/settings"))?,
            authority.clone(),
            ProtectedControls {
                grants: category("grants"),
                policy: category("policy"),
                producer_trust: category("trust"),
                publication_mode: category("publication"),
            },
        )
        .map_err(|e| e.to_string())?
        .with_coordination(open(Path::new("/case/arena"))?)
        .map_err(|e| e.to_string())?,
    );
    let selected = authority.selected_current(&request.client_id)?;
    private_record(
        Path::new("/case/evidence/resource-capture.json"),
        &serde_json::to_vec_pretty(&selected.identity).map_err(|e| e.to_string())?,
    )?;
    let capabilities = ClientCapabilities::admit([ClientCapability {
        provider: selected.identity.provider.clone(),
        descriptor_digest: selected.identity.descriptor_digest.clone(),
        request: request.clone(),
        disabled: false,
    }])?;
    let runner = Arc::new(
        PodBayRunner::new(
            Path::new("/fixture-ipc/process-runner.sock"),
            Path::new("/case/evidence"),
            Vec::new(),
        )
        .map_err(|e| e.to_string())?,
    );
    let commands = Arc::new(client_commands::Host::new(
        Arc::new(Authority(client_commands::Context {
            executable: ClientExecutable::Missing {
                command: "no-file-command".into(),
            },
            cwd: project.clone(),
            user_home: user.clone(),
            roots: vec![project.clone(), user.clone()],
            environment_ref: "file-only".into(),
            principal: "root-sealed".into(),
            policy: "file-only".into(),
            evidence: row.into(),
            revoked: true,
        })),
        runner,
    ));
    let host = client_deploy::Host::new(
        open(&project)?,
        open(Path::new("/case/settings"))?,
        open(Path::new("/case/arena"))?,
        row.replace('.', "-"),
        if scope == ClientScope::User {
            "user"
        } else {
            "project"
        }
        .into(),
        commands,
        Arc::new(FileObserver {
            roots: vec![project.clone(), user.clone()],
            subject: selected.identity.descriptor_digest,
        }),
    )?;
    host.prepare_storage()?;
    let destination = if scope == ClientScope::User {
        &user
    } else {
        &project
    }
    .join(&binding.destination);
    if destination.exists() {
        return Err("foreign occupancy before Codex lifecycle".into());
    }
    let foreign = project.join("m08e1-foreign.txt");
    private_record(&foreign, b"preserve\n")?;
    let foreign_mtime = std::fs::metadata(&foreign)
        .map_err(|e| e.to_string())?
        .modified()
        .map_err(|e| e.to_string())?;
    if !skill {
        std::fs::create_dir_all(destination.parent().unwrap()).map_err(|e| e.to_string())?;
        private_record(
            &destination,
            if claude {
                br#"{"permissions":{"allow":[]}}"#
            } else {
                b"hide_agent_reasoning = true\n"
            },
        )?;
    }
    let core = podbay_runner::hash(
        &std::fs::read(std::env::current_exe().map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?,
    );
    private_record(
        Path::new("/case/evidence/resource-core-before.txt"),
        core.as_bytes(),
    )?;
    let mut target = String::new();
    let mut next = 0usize;
    Ok(move |phase: &str| {
        if let Some(expected) = &companion {
            if std::fs::read("/home/olegchir/.codex/config.toml").map_err(|e| e.to_string())?
                != *expected
            {
                return Err("immutable Root project trust companion changed".into());
            }
            private_record(&Path::new("/case/evidence").join(format!("resource-{phase}-trust-companion.json")), &serde_json::to_vec_pretty(&serde_json::json!({"path":"/home/olegchir/.codex/config.toml","sha256":podbay_runner::hash(expected),"unchanged":true})).map_err(|e|e.to_string())?)?;
        }
        if ["absence", "v1", "v2", "removed"].get(next).copied() != Some(phase) {
            return Err("out-of-order Codex lifecycle; no replay".into());
        }
        next += 1;
        if phase != "absence" {
            let action = match phase {
                "v1" => ReplyAction::Apply,
                "v2" => ReplyAction::Update,
                _ => ReplyAction::Remove,
            };
            let plan = if phase == "removed" {
                Files::Retained {
                    host: files.clone(),
                    plan: files
                        .prepare_retained(&target, RetainedOperation::Cleanup)
                        .map_err(|e| e.to_string())?,
                }
            } else {
                let data = payload(package, phase, &request)?;
                private_record(
                    &Path::new("/case/evidence").join(format!("resource-{phase}-payload.json")),
                    &serde_json::to_vec_pretty(&data).map_err(|e| e.to_string())?,
                )?;
                let admitted = files
                    .admit_payload(
                        files
                            .capture(&capabilities, &request.client_id)
                            .map_err(|e| e.to_string())?,
                        data,
                    )
                    .map_err(|e| e.to_string())?;
                let plan = files.plan(&admitted).map_err(|e| e.to_string())?;
                target = plan.target().into();
                Files::Apply {
                    host: files.clone(),
                    plan,
                }
            };
            let prepared = host.prepare(action, Vec::new(), Some(plan))?;
            if host.plan(&prepared).status != ReplyStatus::Planned {
                return Err("Codex file plan not current".into());
            }
            let grant = host.approve(&prepared)?;
            let run = host.proposed_run(phase);
            host.initialize(&prepared, &run)?;
            let reply = host.execute(
                &prepared,
                Some(&grant),
                &run,
                false,
                &Cancellation::default(),
                "2026-10-16T00:00:00Z",
            )?;
            private_record(
                &Path::new("/case/evidence").join(format!("resource-{phase}-reply.json")),
                &serde_json::to_vec_pretty(&reply).map_err(|e| e.to_string())?,
            )?;
            if reply.status != ReplyStatus::Completed
                || host.verify(&run)?.status != ReplyStatus::Completed
            {
                return Err("Codex file effect incomplete".into());
            }
        }
        use std::os::unix::fs::MetadataExt;
        let mut snapshot = std::collections::BTreeMap::new();
        let paths = if skill {
            vec![
                destination.join("SKILL.md"),
                destination.join("reference.txt"),
                foreign.clone(),
            ]
        } else {
            vec![destination.clone(), foreign.clone()]
        };
        for path in paths {
            let value = match std::fs::read(&path) {
                Ok(bytes) => {
                    let m = std::fs::metadata(&path).map_err(|e| e.to_string())?;
                    serde_json::json!({"bytes":bytes,"sha256":podbay_runner::hash(&bytes),"ino":m.ino(),"mtime":m.mtime(),"mtime_nsec":m.mtime_nsec()})
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => serde_json::Value::Null,
                Err(e) => return Err(e.to_string()),
            };
            snapshot.insert(path.display().to_string(), value);
        }
        private_record(
            &Path::new("/case/evidence").join(format!("resource-{phase}-files.json")),
            &serde_json::to_vec_pretty(&snapshot).map_err(|e| e.to_string())?,
        )?;
        if skill {
            if destination.join("SKILL.md").exists() != matches!(phase, "v1" | "v2")
                || destination.join("reference.txt").exists() != (phase == "v1")
            {
                return Err("Codex tree closure differs".into());
            }
        } else {
            let text = std::fs::read_to_string(&destination).map_err(|e| e.to_string())?;
            if claude {
                let config: serde_json::Value =
                    serde_json::from_str(&text).map_err(|e| e.to_string())?;
                if config["permissions"] != serde_json::json!({"allow":[]}) {
                    return Err("foreign Claude config changed".into());
                }
            } else {
                let config: toml::Value = toml::from_str(&text).map_err(|e| e.to_string())?;
                if config
                    .get("hide_agent_reasoning")
                    .and_then(toml::Value::as_bool)
                    != Some(true)
                {
                    return Err("foreign Codex config changed".into());
                }
            }
        }
        if std::fs::read(&foreign).map_err(|e| e.to_string())? != b"preserve\n"
            || std::fs::metadata(&foreign)
                .map_err(|e| e.to_string())?
                .modified()
                .map_err(|e| e.to_string())?
                != foreign_mtime
        {
            return Err("foreign Codex sibling changed".into());
        }
        if phase == "removed" {
            let after = podbay_runner::hash(
                &std::fs::read(std::env::current_exe().map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?,
            );
            private_record(
                Path::new("/case/evidence/resource-core-after.txt"),
                after.as_bytes(),
            )?;
            if after != core {
                return Err("core changed during external descriptor lifecycle".into());
            }
        }
        Ok(())
    })
}

fn codex_resource_observed(
    row: &str,
    phase: &str,
    result: &serde_json::Value,
) -> Result<(), String> {
    if row.contains(".skill.") {
        let data = result["data"]
            .as_array()
            .ok_or("missing actual skills closure")?;
        if data.len() != 1
            || data[0]["cwd"] != "/case/project"
            || data[0]["errors"].as_array().is_none_or(|a| !a.is_empty())
        {
            return Err("incomplete actual skills closure".into());
        }
        let skills = data[0]["skills"]
            .as_array()
            .ok_or("missing actual skills")?;
        let found: Vec<_> = skills
            .iter()
            .filter(|s| s["name"] == "m08a2-demo")
            .collect();
        if matches!(phase, "absence" | "removed") {
            if !found.is_empty() {
                return Err("unexpected actual skill membership".into());
            }
        } else {
            let root = if row.ends_with(".user") {
                "/home/olegchir"
            } else {
                "/case/project"
            };
            if found.len() != 1
                || found[0]["path"] != format!("{root}/.agents/skills/m08a2-demo/SKILL.md")
                || found[0]["description"] != format!("M08E1_CODEX_RESOURCE_{phase}_017")
                || found[0]["enabled"] != true
            {
                return Err("actual skill revision/path/enabled differs".into());
            }
        }
    } else {
        let config = result["config"]
            .as_object()
            .ok_or("missing actual config")?;
        let expected = match phase {
            "v1" => serde_json::json!("on-request"),
            "v2" => serde_json::json!("never"),
            _ => serde_json::Value::Null,
        };
        if config.get("approval_policy") != Some(&expected)
            || config.get("hide_agent_reasoning") != Some(&serde_json::json!(true))
        {
            return Err("actual config revision/foreign value differs; project trust may require Root qualification".into());
        }
    }
    Ok(())
}

fn actual_executable_drift(seal: Seal) -> Result<(), String> {
    let case = "claude_code.executable-drift-020";
    let launcher = Path::new("/case/project/claude-launcher");
    let first = Path::new("/runtime/claude/claude");
    let alternate = Path::new("/runtime/claude-alternate/claude");
    if seal.steps.len() != 1
        || seal.steps[0].case_id != case
        || seal.steps[0].step_id != "prepare-version"
        || seal.steps[0].spec.program != launcher.to_string_lossy()
        || seal.steps[0].spec.args != ["--version"]
        || std::fs::read_link(launcher).map_err(|e| e.to_string())? != first
    {
        return Err("unbound actual executable-drift manifest".into());
    }
    // Both paths are Root-pinned views of the actual installed ELF, never a
    // manufactured executable. Only the fixture launcher name is mutable.
    let digest = podbay_runner::hash(&std::fs::read(first).map_err(|e| e.to_string())?);
    if digest != "2d894a4b1bfaaa9aab72aeff7803e3c73bc2510f279a171f6b8f59f72d17d077"
        || podbay_runner::hash(&std::fs::read(alternate).map_err(|e| e.to_string())?) != digest
    {
        return Err("actual executable-drift pin mismatch".into());
    }
    let runner = Arc::new(
        PodBayRunner::new(
            Path::new("/fixture-ipc/process-runner.sock"),
            Path::new("/case/evidence"),
            seal.steps,
        )
        .map_err(|e| e.to_string())?,
    );
    let host = client_commands::Host::new(
        Arc::new(Authority(client_commands::Context {
            executable: ClientExecutable::Resolved {
                command: "claude".into(),
                path: launcher.into(),
            },
            cwd: "/case/project".into(),
            user_home: "/home/olegchir".into(),
            roots: vec!["/case".into(), "/home/olegchir".into()],
            environment_ref: case.into(),
            principal: "root-sealed".into(),
            policy: "stale-executable-refusal".into(),
            evidence: "claude-2.1.299".into(),
            revoked: false,
        })),
        runner.clone(),
    );
    let request=vibe_wire::behaviour::client_command::read_request(br#"{"schema":1,"client":"claude","version":"2.1.299","contract_version":1,"operation":"version"}"#).map_err(str::to_owned)?;
    let prepared = host.prepare(request).map_err(str::to_owned)?;
    private_record(
        Path::new("/case/evidence/executable-drift-plan.json"),
        &serde_json::to_vec_pretty(prepared.inspect()).map_err(|e| e.to_string())?,
    )?;
    let grant = host.approve(&prepared).map_err(str::to_owned)?;
    let replacement = Path::new("/case/project/claude-launcher-next");
    std::os::unix::fs::symlink(alternate, replacement).map_err(|e| e.to_string())?;
    std::fs::rename(replacement, launcher).map_err(|e| e.to_string())?;
    let result = host.execute(&prepared, Some(&grant));
    let reason = result.err().ok_or("stale executable grant dispatched")?;
    // Name the exact semantic rejection; an exhausted-runner error cannot
    // masquerade as successful pre-dispatch identity enforcement.
    if reason != "executable drift" {
        return Err(format!("unexpected executable refusal: {reason}"));
    }
    if runner.remaining() != 0 {
        return Err("actual preparation was not consumed".into());
    }
    private_record(Path::new("/case/evidence/executable-drift-result.json"),&serde_json::to_vec_pretty(&serde_json::json!({"case_id":case,"actual_prepare_version":true,"rejection":reason,"before_target":first,"after_target":alternate,"actual_elf_sha256":digest,"execute_dispatched":false,"matrix_status":"pending"})).map_err(|e|e.to_string())?)?;
    Ok(())
}

fn claude_resource_lifecycle(seal: Seal) -> Result<(), String> {
    let row = seal
        .row_id
        .as_deref()
        .ok_or("missing Claude resource row")?;
    if !matches!(
        row,
        "claude_code.skill.project"
            | "claude_code.skill.user"
            | "claude_code.config.project"
            | "claude_code.config.user"
    ) || seal.peer_address.as_deref() != Some("127.0.0.1:33003")
    {
        return Err("unbound Claude resource lifecycle".into());
    }
    let skill = row.contains(".skill.");
    let ids: &[&str] = if skill {
        &["absence", "v1", "v1-body", "v2", "v2-body", "removed"]
    } else {
        &["absence", "v1", "v2", "removed"]
    };
    let case = format!("{row}.lifecycle-022");
    let env = std::collections::BTreeMap::from([
        ("HOME".into(), "/home/olegchir".into()),
        ("USERPROFILE".into(), "/home/olegchir".into()),
        ("LANG".into(), "C.UTF-8".into()),
        ("LC_ALL".into(), "C.UTF-8".into()),
        ("TEMP".into(), "/tmp".into()),
        ("TMP".into(), "/tmp".into()),
        (
            "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC".into(),
            "1".into(),
        ),
        ("DISABLE_TELEMETRY".into(), "1".into()),
        ("DISABLE_ERROR_REPORTING".into(), "1".into()),
        (
            "CLAUDE_CODE_DISABLE_OFFICIAL_MARKETPLACE_AUTOINSTALL".into(),
            "1".into(),
        ),
        ("ANTHROPIC_BASE_URL".into(), "http://127.0.0.1:33003".into()),
        (
            "ANTHROPIC_API_KEY".into(),
            "m08e1-local-simulator-not-a-credential".into(),
        ),
    ]);
    if seal.steps.len() != ids.len()
        || seal.steps.iter().zip(ids).any(|(step, id)| {
            step.case_id != case
                || step.step_id != *id
                || step.spec.program != "/runtime/claude/claude"
                || step.spec.cwd != "/case/project"
                || step.spec.scratch != "/case/arena"
                || step.spec.stdin.is_some()
                || step.spec.env != env
                || step.spec.args
                    != [
                        "-p",
                        "--model",
                        RESOURCE_MODEL,
                        "--max-turns",
                        "1",
                        "--output-format",
                        "json",
                        "--no-session-persistence",
                        "--permission-mode",
                        "dontAsk",
                        "--tools",
                        "Skill",
                        "--strict-mcp-config",
                        "--mcp-config",
                        "{\"mcpServers\":{}}",
                        "--",
                        if id.ends_with("-body") {
                            "/m08a2-demo"
                        } else {
                            "Return OK."
                        },
                    ]
        })
    {
        return Err("Claude resource batch differs before effects".into());
    }
    let runner = PodBayRunner::new(
        Path::new("/fixture-ipc/process-runner.sock"),
        Path::new("/case/evidence"),
        seal.steps.clone(),
    )
    .map_err(|e| e.to_string())?;
    let mut transition = resource_file_lifecycle(row)?;
    for step in &seal.steps {
        let phase = step.step_id.as_str();
        if !phase.ends_with("-body") {
            transition(phase)?;
        }
        let label = format!("{phase}-");
        let peer = LocalPeer::messages(Path::new("/case/evidence"), &label)?;
        let output = runner.run(&podbay_runner::native(&step.spec));
        let peer_result = peer.finish();
        // Both results stay available even if one fails; never retry a step.
        private_record(&Path::new("/case/evidence").join(format!("resource-{phase}-observation.json")),&serde_json::to_vec_pretty(&serde_json::json!({"case_id":case,"step_id":phase,"peer_result":peer_result,"process_result":output.as_ref().map(|o|serde_json::json!({"code":o.code,"stdout_truncated":o.stdout_truncated,"stderr_truncated":o.stderr_truncated})).map_err(|e|e.to_string())})).map_err(|e|e.to_string())?)?;
        peer_result?;
        complete(&output.map_err(|e| e.to_string())?)?;
        let mut found = std::collections::BTreeSet::new();
        for sequence in 1..=4 {
            match std::fs::read(
                Path::new("/case/evidence")
                    .join(format!("messages-{label}{sequence:03}.markers.json")),
            ) {
                Ok(bytes) => {
                    let record: serde_json::Value =
                        serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                    for marker in record["markers"]
                        .as_array()
                        .ok_or("missing parsed marker array")?
                    {
                        found.insert(
                            marker["marker"]
                                .as_str()
                                .ok_or("invalid parsed marker")?
                                .to_owned(),
                        );
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(e.to_string()),
            }
        }
        let revision = if phase.starts_with("v1") {
            Some("v1")
        } else if phase.starts_with("v2") {
            Some("v2")
        } else {
            None
        };
        let relevant: Vec<_> = found
            .iter()
            .filter(|m| {
                if skill {
                    m.starts_with("M08E1_CLAUDE_")
                } else {
                    m.starts_with("Always respond in ")
                }
            })
            .cloned()
            .collect();
        if let Some(revision) = revision {
            let expected = if skill {
                format!(
                    "M08E1_CLAUDE_{}_{revision}_022",
                    if phase.ends_with("-body") {
                        "BODY"
                    } else {
                        "DESCRIPTION"
                    }
                )
            } else {
                format!(
                    "Always respond in {}",
                    if revision == "v1" {
                        "Esperanto"
                    } else {
                        "Icelandic"
                    }
                )
            };
            let other = if revision == "v1" { "v2" } else { "v1" };
            if !relevant.contains(&expected)
                || relevant.iter().any(|m| {
                    if skill {
                        m.contains(other)
                    } else {
                        m != &expected
                    }
                })
            {
                return Err(format!(
                    "{case}/{phase}: actual parsed request revision/body marker differs"
                ));
            }
        } else if !relevant.is_empty() {
            return Err(format!(
                "{case}/{phase}: removed/absent resource still loaded"
            ));
        }
        private_record(&Path::new("/case/evidence").join(format!("resource-{phase}-markers-result.json")),&serde_json::to_vec_pretty(&serde_json::json!({"case_id":case,"step_id":phase,"observed_markers":relevant,"prompt":if phase.ends_with("-body") {"/m08a2-demo"} else {"Return OK."},"proof":"actual parsed request; fixed response ignored"})).map_err(|e|e.to_string())?)?;
    }
    if runner.remaining() != 0 {
        return Err("unconsumed Claude resource steps".into());
    }
    Err("pending: basic actual Claude resource lifecycle observed; full scenario set and complete matrix remain required".into())
}

fn codex_mcp_observed(
    phase: &str,
    result: &serde_json::Value,
    resource_read: bool,
) -> Result<(), String> {
    if resource_read {
        let contents = result["contents"]
            .as_array()
            .ok_or("missing actual MCP resource contents")?;
        if contents.len() != 1
            || contents[0]["uri"] != "m08e1://resource"
            || contents[0]["text"] != format!("M08E1 {phase} exact local resource\n")
        {
            return Err("actual MCP resource response differs".into());
        }
    } else {
        let data = result["data"]
            .as_array()
            .ok_or("missing actual MCP status closure")?;
        if result.get("nextCursor").is_some_and(|v| !v.is_null()) {
            return Err("incomplete actual MCP pagination".into());
        }
        let selected: Vec<_> = data.iter().filter(|s| s["name"] == "m08a2-demo").collect();
        if matches!(phase, "absence" | "removed") {
            if !selected.is_empty() {
                return Err("unexpected actual MCP membership".into());
            }
        } else if selected.len() != 1
            || selected[0]["toolsError"].is_string()
            || selected[0]["tools"].get("fixture_resource").is_none()
            || !selected[0]["resources"]
                .as_array()
                .is_some_and(|resources| {
                    resources.iter().any(|r| {
                        r["uri"] == "m08e1://resource" && r["name"] == format!("M08E1 {phase}")
                    })
                })
        {
            return Err("actual MCP inventory/revision differs".into());
        }
    }
    Ok(())
}

// Candidate finite closure for one Root-sealed local-only absence probe. This
// is not registered as a production ClientStateOwner or a completeness claim.
fn codex_plugin_probe_witness()
-> Result<crate::mechanism::deploy::plugin::client_state::ClientStateWitness, String> {
    use crate::mechanism::deploy::plugin::client_state::{ClientStateBinding, ClientStateWitness};
    let home = Path::new("/home/olegchir");
    let binding = ClientStateBinding {
        client: "codex-0.159.3".into(),
        coordinate: "m08e1-demo@m08e1-local".into(),
        scope: "user".into(),
        user_home: home.into(),
    };
    ClientStateWitness::observe_closure(
        "m08e1-candidate-local-absence",
        &binding,
        home,
        &[
            ".codex/config.toml",
            ".agents/plugins/marketplace.json",
            ".agents/plugins/api_marketplace.json",
            ".claude-plugin/marketplace.json",
            ".cursor-plugin/marketplace.json",
            ".codex/.tmp/plugins/.agents/plugins/marketplace.json",
            ".codex/.tmp/plugins/.agents/plugins/api_marketplace.json",
            "market/.agents/plugins/marketplace.json",
            "market/.agents/plugins/api_marketplace.json",
            "market/.claude-plugin/marketplace.json",
            "market/.cursor-plugin/marketplace.json",
            "market/plugins/m08e1-demo/plugin.json",
            "market/plugins/m08e1-demo/.codex-plugin/plugin.json",
            "market/plugins/m08e1-demo/.claude-plugin/plugin.json",
            "market/plugins/m08e1-demo/.cursor-plugin/plugin.json",
            "market/plugins/m08e1-demo/.codex-remote-plugin-install.json",
            "market/plugins/m08e1-demo/skills/m08a2-demo/SKILL.md",
            "market/plugins/m08e1-demo/skills/m08a2-demo/reference.txt",
        ],
        &[
            "market",
            "market/.agents",
            "market/.agents/plugins",
            "market/plugins",
            "market/plugins/m08e1-demo",
            "market/plugins/m08e1-demo/skills",
            "market/plugins/m08e1-demo/skills/m08a2-demo",
            ".codex/plugins/cache/m08e1-local/m08e1-demo",
        ],
    )
}

pub(crate) fn peer_http_trace_checks() -> Result<(), String> {
    use std::io::{Read, Write};
    let evidence = tempfile::tempdir().map_err(|e| e.to_string())?;
    let reservation = std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let address = reservation.local_addr().map_err(|e| e.to_string())?;
    drop(reservation);
    let peer = LocalPeer::start(&address.to_string(), evidence.path(), "v1")?;
    let post = serde_json::json!({"jsonrpc":"2.0","id":77,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"trace-replay","version":"1"}}}).to_string();
    for (route, body, status) in [
        ("GET /mcp/v1 HTTP/1.1", "", "405 Method Not Allowed"),
        (
            "GET /.well-known/oauth-protected-resource/mcp/v1 HTTP/1.1",
            "",
            "404 Not Found",
        ),
        (
            "GET /.well-known/oauth-protected-resource HTTP/1.1",
            "",
            "404 Not Found",
        ),
        ("POST /mcp/v1 HTTP/1.1", post.as_str(), "200 OK"),
    ] {
        let mut stream = std::net::TcpStream::connect(address).map_err(|e| e.to_string())?;
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(3)))
            .map_err(|e| e.to_string())?;
        write!(
            stream,
            "{route}\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        )
        .map_err(|e| e.to_string())?;
        let mut response = String::new();
        stream
            .take(65536)
            .read_to_string(&mut response)
            .map_err(|e| e.to_string())?;
        if !response.starts_with(&format!("HTTP/1.1 {status}\r\n")) {
            return Err(format!("wrong probe response: {response}"));
        }
        if status.starts_with("405") && !response.contains("Allow: POST\r\n") {
            return Err("missing allowed method".into());
        }
        if status.starts_with("200") {
            let reply: serde_json::Value = serde_json::from_str(
                response
                    .split_once("\r\n\r\n")
                    .ok_or("missing response headers")?
                    .1,
            )
            .map_err(|e| e.to_string())?;
            if reply["id"] != 77 || reply["result"]["serverInfo"]["version"] != "v1" {
                return Err("POST did not survive HTTP metadata probes".into());
            }
        }
    }
    peer.finish()?;
    for sequence in 1..=4 {
        if !evidence
            .path()
            .join(format!("peer-v1-{sequence:03}.route.json"))
            .is_file()
        {
            return Err("missing retained actual route replay".into());
        }
    }
    Ok(())
}

const PLUGIN_CACHE_RESOURCE: &str = "codex-plugin-cache:m08e1-demo@m08e1-local";
const PLUGIN_FILES: &[&str] = &[
    ".codex-plugin/plugin.json",
    "skills/m08a2-demo/SKILL.md",
    "skills/m08a2-demo/reference.txt",
];
fn plugin_finite_tree(root: &Path) -> Result<String, String> {
    use crate::mechanism::deploy::plugin::client_state::{ClientStateBinding, ClientStateWitness};
    let binding = ClientStateBinding {
        client: "codex-0.159.3".into(),
        coordinate: "m08e1-demo@m08e1-local".into(),
        scope: "user-cache-tree-only".into(),
        user_home: "/home/olegchir".into(),
    };
    let witness = || {
        ClientStateWitness::observe_closure(
            "finite-cache-effect",
            &binding,
            root,
            PLUGIN_FILES,
            &[".", ".codex-plugin", "skills", "skills/m08a2-demo"],
        )
    };
    let before = witness()?;
    let project = vibe_safefs::Project::open(root).map_err(|e| e.to_string())?;
    for (path, expected) in [
        (".", vec![".codex-plugin", "skills"]),
        (".codex-plugin", vec!["plugin.json"]),
        ("skills", vec!["m08a2-demo"]),
        ("skills/m08a2-demo", vec!["SKILL.md", "reference.txt"]),
    ] {
        let parts: Vec<_> = if path == "." {
            vec![]
        } else {
            path.split('/').collect()
        };
        let dir = project
            .dir_if_present(&parts)
            .map_err(|e| e.to_string())?
            .ok_or("missing bounded plugin directory")?;
        if project
            .child_names_bounded(&dir, 8)
            .map_err(|e| e.to_string())?
            != expected
        {
            return Err("plugin membership outside exact skill-only tree".into());
        }
    }
    let mut files = std::collections::BTreeMap::new();
    for path in PLUGIN_FILES {
        let bytes = project
            .read_file_snapshot_bounded(path, 1024 * 1024)
            .map_err(|e| e.to_string())?
            .ok_or("missing plugin member")?;
        files.insert(*path, bytes.sha256);
    }
    // Content-only effect digest; inode identities are independently bracketed
    // by the read-only witness and are not confused with desired copy bytes.
    if witness()? != before {
        return Err("plugin tree changed during bounded observation".into());
    }
    Ok(podbay_runner::hash(
        &serde_json::to_vec(&files).map_err(|e| e.to_string())?,
    ))
}
struct PluginCacheObserver;
impl crate::client_deploy::Observer for PluginCacheObserver {
    fn subject(&self) -> Result<String, String> {
        plugin_finite_tree(Path::new("/home/olegchir/market/plugins/m08e1-demo"))
    }
    fn observe(
        &self,
        resources: &[String],
    ) -> Result<std::collections::BTreeMap<String, Option<String>>, String> {
        if resources != [PLUGIN_CACHE_RESOURCE] {
            return Err("unbound plugin cache resource".into());
        }
        let home =
            vibe_safefs::Project::open(Path::new("/home/olegchir")).map_err(|e| e.to_string())?;
        let parts = [".codex", "plugins", "cache", "m08e1-local", "m08e1-demo"];
        let value = match home.dir_if_present(&parts).map_err(|e| e.to_string())? {
            None => None,
            Some(dir) => {
                let versions = home
                    .child_names_bounded(&dir, 4)
                    .map_err(|e| e.to_string())?;
                if versions.len() != 1 || !matches!(versions[0].as_str(), "1.0.0" | "2.0.0") {
                    return Err("unqualified installed plugin version membership".into());
                }
                Some(plugin_finite_tree(
                    &Path::new("/home/olegchir/.codex/plugins/cache/m08e1-local/m08e1-demo")
                        .join(&versions[0]),
                )?)
            }
        };
        Ok(std::collections::BTreeMap::from([(
            PLUGIN_CACHE_RESOURCE.into(),
            value,
        )]))
    }
}
const PLUGIN_FOOTPRINT: &str = "codex-plugin-cache-and-config:m08e1-demo@m08e1-local";

pub(super) fn plugin_footprint_checks() -> Result<(), String> {
    let absent: toml::Value = toml::from_str("hide_agent_reasoning = true").unwrap();
    let enabled: toml::Value =
        toml::from_str("[plugins.\"m08e1-demo@m08e1-local\"]\nenabled = true").unwrap();
    let disabled: toml::Value =
        toml::from_str("[plugins.\"m08e1-demo@m08e1-local\"]\nenabled = false").unwrap();
    let cache = Some("a".repeat(64));
    let desired = plugin_footprint(cache.clone(), &enabled)?;
    assert_eq!(plugin_footprint(None, &absent)?, None);
    assert_ne!(
        plugin_footprint(None, &enabled)?,
        None,
        "config-only occupant must not look absent"
    );
    assert_ne!(
        plugin_footprint(None, &enabled)?,
        desired,
        "missing cache must not certify activation"
    );
    assert_ne!(
        plugin_footprint(cache.clone(), &absent)?,
        desired,
        "cache-only landing is incomplete"
    );
    assert_ne!(
        plugin_footprint(cache.clone(), &disabled)?,
        desired,
        "disabled member is not desired"
    );
    assert_ne!(
        plugin_footprint(Some("b".repeat(64)), &enabled)?,
        desired,
        "cache drift matters"
    );
    assert!(plugin_footprint(cache, &toml::from_str("plugins = false").unwrap()).is_err());
    Ok(())
}

// A finite test witness, not a general vendor-state restoration capability.
fn plugin_footprint(cache: Option<String>, config: &toml::Value) -> Result<Option<String>, String> {
    let plugins = config
        .get("plugins")
        .map(|v| v.as_table().ok_or("invalid plugins table"))
        .transpose()?;
    let selected = plugins.and_then(|p| p.get("m08e1-demo@m08e1-local"));
    if cache.is_none() && selected.is_none() {
        return Ok(None);
    }
    Ok(Some(podbay_runner::hash(
        &serde_json::to_vec(&(cache, selected)).map_err(|e| e.to_string())?,
    )))
}

struct PluginFootprintObserver {
    foreign: toml::Value,
}
impl crate::client_deploy::Observer for PluginFootprintObserver {
    fn subject(&self) -> Result<String, String> {
        Ok(podbay_runner::hash(
            &serde_json::to_vec(&(
                PluginCacheObserver.subject()?,
                &self.foreign,
                PLUGIN_FOOTPRINT,
            ))
            .map_err(|e| e.to_string())?,
        ))
    }
    fn observe(
        &self,
        resources: &[String],
    ) -> Result<std::collections::BTreeMap<String, Option<String>>, String> {
        if resources != [PLUGIN_FOOTPRINT] {
            return Err("unbound plugin footprint".into());
        }
        let (_, config) = plugin_config_snapshot()?;
        let cache = PluginCacheObserver.observe(&[PLUGIN_CACHE_RESOURCE.into()])?
            [PLUGIN_CACHE_RESOURCE]
            .clone();
        let mut foreign = config.clone();
        if let Some(plugins) = foreign.get_mut("plugins") {
            let table = plugins.as_table_mut().ok_or("invalid plugins table")?;
            table.remove("m08e1-demo@m08e1-local");
            if table.is_empty() {
                foreign
                    .as_table_mut()
                    .ok_or("invalid config")?
                    .remove("plugins");
            }
        }
        if foreign != self.foreign {
            return Err("foreign config changed during command recovery".into());
        }
        Ok(std::collections::BTreeMap::from([(
            PLUGIN_FOOTPRINT.into(),
            plugin_footprint(cache, &config)?,
        )]))
    }
}
fn plugin_config_snapshot() -> Result<(Vec<u8>, toml::Value), String> {
    let home =
        vibe_safefs::Project::open(Path::new("/home/olegchir")).map_err(|e| e.to_string())?;
    let bytes = home
        .read_file_snapshot_bounded(".codex/config.toml", 1024 * 1024)
        .map_err(|e| e.to_string())?
        .ok_or("missing selected config")?;
    let config = toml::from_str(std::str::from_utf8(&bytes.bytes).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    Ok((bytes.bytes, config))
}

fn codex_plugin_recovery(steps: &[protocol::Request]) -> Result<(), String> {
    use crate::client_deploy::{self, Cancellation, Command, Observer};
    use vibe_wire::generated::client_deploy::e1::reply::{ReplyAction, ReplyStatus, StepStatus};
    const CASE: &str = "codex.plugin.user.recovery-031";
    let labels = [
        "prepare-install",
        "install",
        "prepare-recovery",
        "prepare-collision",
    ];
    if steps.len() != labels.len()
        || steps
            .iter()
            .zip(labels)
            .any(|(s, l)| s.case_id != CASE || s.step_id != l)
    {
        return Err("unbound recovery steps".into());
    }
    let record = |name: &str, v: serde_json::Value| {
        private_record(
            &Path::new("/case/evidence").join(format!("plugin-recovery-{name}.json")),
            &serde_json::to_vec_pretty(&v).map_err(|e| e.to_string())?,
        )
    };
    let (before_bytes, before) = plugin_config_snapshot()?;
    if before.get("plugins").is_some()
        || before.get("hide_agent_reasoning") != Some(&toml::Value::Boolean(true))
    {
        return Err("unbound initial config".into());
    }
    let observer = Arc::new(PluginFootprintObserver {
        foreign: before.clone(),
    });
    let names = [PLUGIN_FOOTPRINT.into()];
    if observer.observe(&names)?[PLUGIN_FOOTPRINT].is_some() {
        return Err("initial selected footprint occupied".into());
    }
    // Actual absence is the before-image for this install. No fabricated bytes
    // or removal command is presented as an inverse of a vendor command.
    record(
        "before",
        serde_json::json!({"config_bytes":before_bytes,"cache":null,"selected_config":null,"subject":observer.subject()?}),
    )?;
    struct StopAfterEffect {
        inner: Arc<PodBayRunner>,
        cancellation: Cancellation,
        effects: std::sync::atomic::AtomicUsize,
    }
    impl ProcessRunner for StopAfterEffect {
        fn run(
            &self,
            spec: &crate::process::ProcessSpec,
        ) -> Result<crate::process::ProcessOutput, crate::process::ProcessError> {
            let effect = spec.args.first().is_some_and(|s| s == "plugin");
            let result = self.inner.run(spec);
            if effect {
                self.effects
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                self.cancellation.request();
            }
            result
        }
    }
    let runner = Arc::new(
        PodBayRunner::new(
            Path::new("/fixture-ipc/process-runner.sock"),
            Path::new("/case/evidence"),
            steps.to_vec(),
        )
        .map_err(|e| e.to_string())?,
    );
    let cancellation = Cancellation::default();
    let stopped = Arc::new(StopAfterEffect {
        inner: runner.clone(),
        cancellation: cancellation.clone(),
        effects: 0.into(),
    });
    let context = client_commands::Context {
        executable: ClientExecutable::Resolved {
            command: "codex".into(),
            path: "/runtime/codex/codex".into(),
        },
        cwd: "/case/project".into(),
        user_home: "/home/olegchir".into(),
        roots: vec!["/case/project".into(), "/home/olegchir".into()],
        environment_ref: "Root031-exact".into(),
        principal: "m08e1".into(),
        policy: "plugin-recovery-031".into(),
        evidence: "actual-pinned-codex-0.159.3".into(),
        revoked: false,
    };
    let fresh_commands = || {
        Arc::new(client_commands::Host::new(
            Arc::new(Authority(context.clone())),
            stopped.clone(),
        ))
    };
    let host = |commands, target: &str| {
        let open = |p| vibe_safefs::Project::open(Path::new(p)).map_err(|e| e.to_string());
        client_deploy::Host::new(
            open("/case/project")?,
            open("/case/settings")?,
            open("/case/arena")?,
            target.into(),
            "user-plugin-footprint".into(),
            commands,
            observer.clone(),
        )
    };
    let mut enabled = before.clone();
    enabled.as_table_mut().ok_or("config not table")?.insert(
        "plugins".into(),
        toml::from_str::<toml::Value>("[\"m08e1-demo@m08e1-local\"]\nenabled = true\n")
            .map_err(|e| e.to_string())?,
    );
    let desired = plugin_footprint(
        Some(plugin_finite_tree(Path::new(
            "/home/olegchir/market/plugins/m08e1-demo",
        ))?),
        &enabled,
    )?;
    let command = |commands: &client_commands::Host| -> Result<Command, String> {
        let request = vibe_wire::behaviour::client_command::read_request(br#"{"schema":1,"client":"codex","version":"0.159.3","contract_version":1,"operation":"install","coordinate":"m08e1-demo@m08e1-local"}"#).map_err(str::to_owned)?;
        Ok(Command {
            prepared: commands.prepare(request).map_err(str::to_owned)?,
            resource: PLUGIN_FOOTPRINT.into(),
            desired: desired.clone(),
            retry_safe: false,
        })
    };
    let commands = fresh_commands();
    let first = host(commands.clone(), "m08e1-plugin-recovery-031")?;
    first.prepare_storage()?;
    let prepared = first.prepare(ReplyAction::Apply, vec![command(&commands)?], None)?;
    let grant = first.approve(&prepared)?;
    let run = first.proposed_run("landed-install");
    first.initialize(&prepared, &run)?;
    let reply = first.execute(
        &prepared,
        Some(&grant),
        &run,
        false,
        &cancellation,
        "2026-10-16T00:00:00Z",
    )?;
    record(
        "interrupted",
        serde_json::to_value(&reply).map_err(|e| e.to_string())?,
    )?;
    if reply.status != ReplyStatus::Cancelled
        || reply.steps[0].status != StepStatus::Completed
        || !reply.cancel_pending
    {
        return Err("expected durable observed-command cancellation boundary".into());
    }
    let landed = observer.observe(&names)?;
    let (landed_bytes, landed_config) = plugin_config_snapshot()?;
    plugin_config_effect(&before, &landed_config, false)?;
    if landed[PLUGIN_FOOTPRINT] != desired {
        return Err("cache/config did not independently land".into());
    }
    record(
        "landed",
        serde_json::json!({"observed":landed,"config_bytes":landed_bytes,"run":run,"dispatches":stopped.effects.load(std::sync::atomic::Ordering::SeqCst)}),
    )?;
    drop(grant);
    drop(prepared);
    drop(first);
    drop(commands);
    let commands = fresh_commands();
    let resumed = host(commands.clone(), "m08e1-plugin-recovery-031")?;
    let recovered = resumed.prepare_recovery(&run, vec![command(&commands)?])?;
    let grant = resumed.approve(&recovered)?;
    let reply = resumed.execute(
        &recovered,
        Some(&grant),
        &run,
        true,
        &Cancellation::default(),
        "2026-10-16T00:00:01Z",
    )?;
    record(
        "resumed",
        serde_json::to_value(&reply).map_err(|e| e.to_string())?,
    )?;
    let verified = resumed.verify(&run)?;
    record(
        "verified",
        serde_json::to_value(&verified).map_err(|e| e.to_string())?,
    )?;
    if reply.status != ReplyStatus::Completed
        || verified.status != ReplyStatus::Completed
        || reply.run_id != run
        || observer.observe(&names)? != landed
        || plugin_config_snapshot()?.0 != landed_bytes
    {
        return Err("same-run footprint recovery failed".into());
    }
    // A separate deployment must not adopt this already occupied coordinate,
    // even though its desired cache AND enabled member are byte-equivalent.
    let foreign_commands = fresh_commands();
    let collision = host(foreign_commands.clone(), "m08e1-plugin-foreign-031")?;
    let p = collision.prepare(ReplyAction::Apply, vec![command(&foreign_commands)?], None)?;
    let g = collision.approve(&p)?;
    let collision_run = collision.proposed_run("equal-foreign-footprint");
    collision.initialize(&p, &collision_run)?;
    let refused = collision.execute(
        &p,
        Some(&g),
        &collision_run,
        false,
        &Cancellation::default(),
        "2026-10-16T00:00:02Z",
    )?;
    record(
        "collision",
        serde_json::to_value(&refused).map_err(|e| e.to_string())?,
    )?;
    if refused.status != ReplyStatus::Refused
        || refused.message != "unowned or drifted command state"
        || observer.observe(&names)? != landed
        || plugin_config_snapshot()?.0 != landed_bytes
        || stopped.effects.load(std::sync::atomic::Ordering::SeqCst) != 1
        || runner.remaining() != 0
    {
        return Err("foreign occupancy refusal/no-replay evidence failed".into());
    }
    record(
        "inverse-unsupported",
        serde_json::json!({"run":run,"transition":"landed install -> actual pre-install cache absence and selected config absence after later failure","supported":false,"reason":"D2 command receipt is reversible:false, prior_state_handle:None; Command has no inverse/before-image restore API","retained_residue":landed,"config_bytes":landed_bytes,"effect_dispatches":1,"cleanup":"Root owns namespace disposal; no vendor inverse or replay attempted"}),
    )?;
    Ok(())
}

fn codex_plugin_effect(
    steps: &[protocol::Request],
    phase: &str,
    campaign: &str,
) -> Result<(), String> {
    let remove = phase == "remove";
    let update = phase == "update";
    let successor = campaign != "027";
    let case = match campaign {
        "027" => "codex.plugin.user.install-027",
        "028" => "codex.plugin.user.remove-028",
        "029" => "codex.plugin.user.update-029",
        _ => return Err("unknown plugin campaign".into()),
    };
    use crate::client_deploy::{self, Cancellation, Command};
    use vibe_wire::generated::client_deploy::e1::reply::{ReplyAction, ReplyStatus};
    struct Authority {
        campaign: String,
    }
    impl client_commands::Authority for Authority {
        fn current(
            &self,
            _: &vibe_wire::generated::client_command::e1::request::Request,
        ) -> Result<client_commands::Context, &'static str> {
            Ok(client_commands::Context {
                executable: ClientExecutable::Resolved {
                    command: "codex".into(),
                    path: "/runtime/codex/codex".into(),
                },
                cwd: "/case/project".into(),
                user_home: "/home/olegchir".into(),
                roots: vec!["/case/project".into(), "/home/olegchir".into()],
                environment_ref: format!("Root{}-exact", self.campaign),
                principal: "m08e1".into(),
                policy: format!("plugin-lifecycle-{}", self.campaign),
                evidence: "actual-pinned-codex-0.159.3".into(),
                revoked: false,
            })
        }
    }
    if steps.len() != 2
        || steps.iter().enumerate().any(|(i, s)| {
            s.case_id != case
                || s.step_id
                    != if i == 0 {
                        format!("prepare-{phase}")
                    } else {
                        phase.to_owned()
                    }
        })
    {
        return Err("unbound plugin physical steps".into());
    }
    let config_path = Path::new("/home/olegchir/.codex/config.toml");
    let config_before = std::fs::read(config_path).map_err(|e| e.to_string())?;
    let before: toml::Value =
        toml::from_str(std::str::from_utf8(&config_before).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    if before.get("hide_agent_reasoning") != Some(&toml::Value::Boolean(true))
        || (!remove && !update && before.get("plugins").is_some())
    {
        return Err("plugin fixture foreign/absence config not bound".into());
    }
    let runner = Arc::new(
        PodBayRunner::new(
            Path::new("/fixture-ipc/process-runner.sock"),
            Path::new("/case/evidence"),
            steps.to_vec(),
        )
        .map_err(|e| e.to_string())?,
    );
    let commands = Arc::new(client_commands::Host::new(
        Arc::new(Authority {
            campaign: campaign.into(),
        }),
        runner.clone(),
    ));
    let request=vibe_wire::behaviour::client_command::read_request(br#"{"schema":1,"client":"codex","version":"0.159.3","contract_version":1,"operation":"install","coordinate":"m08e1-demo@m08e1-local"}"#).map_err(str::to_owned)?;
    let request = if remove {
        vibe_wire::behaviour::client_command::read_request(br#"{"schema":1,"client":"codex","version":"0.159.3","contract_version":1,"operation":"remove","coordinate":"m08e1-demo@m08e1-local"}"#).map_err(str::to_owned)?
    } else {
        request
    };
    let command = commands.prepare(request).map_err(str::to_owned)?;
    let open = |p| vibe_safefs::Project::open(Path::new(p)).map_err(|e| e.to_string());
    let host = client_deploy::Host::new(
        open("/case/project")?,
        open("/case/settings")?,
        open("/case/arena")?,
        format!("m08e1-plugin-{campaign}"),
        "user-plugin-cache".into(),
        commands,
        Arc::new(PluginCacheObserver),
    )?;
    host.prepare_storage()?;
    let desired = if remove {
        None
    } else {
        Some(plugin_finite_tree(Path::new(
            "/home/olegchir/market/plugins/m08e1-demo",
        ))?)
    };
    let prepared = host.prepare(
        if remove {
            ReplyAction::Remove
        } else if update {
            ReplyAction::Update
        } else {
            ReplyAction::Apply
        },
        vec![Command {
            prepared: command,
            resource: PLUGIN_CACHE_RESOURCE.into(),
            desired,
            retry_safe: false,
        }],
        None,
    )?;
    let approval = host.approve(&prepared)?;
    let label = phase;
    let run = host.proposed_run(if successor { label } else { "install-027" });
    host.initialize(&prepared, &run)?;
    let result = host.execute(
        &prepared,
        Some(&approval),
        &run,
        false,
        &Cancellation::default(),
        "2026-10-16T00:00:00Z",
    )?;
    private_record(
        &Path::new("/case/evidence").join(format!("plugin-{label}-d2-reply.json")),
        &serde_json::to_vec_pretty(&result).map_err(|e| e.to_string())?,
    )?;
    if result.status != ReplyStatus::Completed || runner.remaining() != 0 {
        return Err("plugin effect not complete; retain run without automatic retry".into());
    }
    let verified = host.verify(&run)?;
    if verified.status != ReplyStatus::Completed {
        return Err("plugin D2 verification incomplete".into());
    }
    let config_after = std::fs::read(config_path).map_err(|e| e.to_string())?;
    let after: toml::Value =
        toml::from_str(std::str::from_utf8(&config_after).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let foreign_unchanged = plugin_config_effect(&before, &after, remove).is_ok();
    private_record(&Path::new("/case/evidence").join(format!("plugin-{label}-config.json")), &serde_json::to_vec_pretty(&serde_json::json!({"before":config_before,"after":config_after,"foreign_values_unchanged":foreign_unchanged})).map_err(|e|e.to_string())?)?;
    plugin_config_effect(&before, &after, remove)?;
    private_record(
        &Path::new("/case/evidence").join(format!("plugin-{label}-d2-verify.json")),
        &serde_json::to_vec_pretty(&verified).map_err(|e| e.to_string())?,
    )?;
    Ok(())
}
fn codex_plugin_install_observed(id: u64, result: &serde_json::Value) -> Result<(), String> {
    codex_plugin_observed(id, result, "1.0.0", matches!(id, 3 | 4))
}
fn codex_plugin_observed(
    id: u64,
    result: &serde_json::Value,
    version: &str,
    installed: bool,
) -> Result<(), String> {
    if matches!(id, 4 | 6 | 8) {
        let rows = result["data"].as_array().ok_or("missing skills closure")?;
        if rows.len() != 1
            || rows[0]["cwd"] != "/case/project"
            || rows[0]["errors"].as_array().is_none_or(|e| !e.is_empty())
            || !rows[0]["skills"].is_array()
        {
            return Err("incomplete or errored scoped skills observation".into());
        }
        let found: Vec<_> = rows
            .iter()
            .flat_map(|r| r["skills"].as_array().into_iter().flatten())
            .filter(|s| s["name"] == "m08e1-demo:m08a2-demo")
            .collect();
        if !installed {
            return if found.is_empty() {
                Ok(())
            } else {
                Err("removed plugin skill still active".into())
            };
        }
        if found.len() != 1
            || found[0]["description"]
                != if version == "1.0.0" {
                    "M08E1_PLUGIN_DESCRIPTION_v1_025"
                } else {
                    "M08E1_PLUGIN_DESCRIPTION_v2_029"
                }
            || found[0]["path"]
                != format!(
                    "/home/olegchir/.codex/plugins/cache/m08e1-local/m08e1-demo/{version}/skills/m08a2-demo/SKILL.md"
                )
            || found[0]["enabled"] != true
        {
            return Err("installed plugin skill activation not observed".into());
        }
    } else {
        let markets = result["marketplaces"]
            .as_array()
            .ok_or("missing plugin markets")?;
        if markets.len() != 1
            || markets[0]["path"] != "/home/olegchir/market/.agents/plugins/marketplace.json"
            || result["marketplaceLoadErrors"]
                .as_array()
                .is_none_or(|e| !e.is_empty())
        {
            return Err("incomplete or errored local marketplace observation".into());
        }
        let plugins: Vec<_> = markets
            .iter()
            .filter(|m| m["name"] == "m08e1-local")
            .flat_map(|m| m["plugins"].as_array().into_iter().flatten())
            .filter(|p| p["id"] == "m08e1-demo@m08e1-local")
            .collect();
        if plugins.len() != 1
            || plugins[0]["installed"] != installed
            || plugins[0]["enabled"] != installed
            || plugins[0]["localVersion"] != version
        {
            return Err("actual plugin installed/enabled/localVersion differs".into());
        }
    }
    Ok(())
}

/// Model-free protocol assertions; no namespace, launcher or vendor invocation.
pub(crate) fn plugin_removal_observation_checks() -> Result<(), String> {
    let absent = serde_json::json!({"marketplaces":[{"name":"m08e1-local","path":"/home/olegchir/market/.agents/plugins/marketplace.json","plugins":[{"id":"m08e1-demo@m08e1-local","installed":false,"enabled":false,"localVersion":"1.0.0"}]}],"marketplaceLoadErrors":[]});
    codex_plugin_install_observed(5, &absent)?;
    let skills = serde_json::json!({"data":[{"cwd":"/case/project","skills":[],"errors":[]}]});
    codex_plugin_install_observed(6, &skills)?;
    let mut installed = absent.clone();
    installed["marketplaces"][0]["plugins"][0]["installed"] = true.into();
    if codex_plugin_install_observed(5, &installed).is_ok() {
        return Err("installed plugin accepted as removed".into());
    }
    let mut broken = absent.clone();
    broken["marketplaceLoadErrors"] = serde_json::json!([{"message":"unavailable"}]);
    if codex_plugin_install_observed(5, &broken).is_ok() {
        return Err("errored marketplace accepted".into());
    }
    for fault in [
        "active",
        "errors",
        "missing_skills",
        "wrong_scope",
        "missing_rows",
    ] {
        let mut changed = skills.clone();
        match fault {
            "active" => {
                changed["data"][0]["skills"] = serde_json::json!([{"name":"m08e1-demo:m08a2-demo"}])
            }
            "errors" => {
                changed["data"][0]["errors"] = serde_json::json!([{"message":"unavailable"}])
            }
            "missing_skills" => {
                changed["data"][0].as_object_mut().unwrap().remove("skills");
            }
            "wrong_scope" => changed["data"][0]["cwd"] = "/another/project".into(),
            _ => changed["data"] = serde_json::json!([]),
        }
        if codex_plugin_install_observed(6, &changed).is_ok() {
            return Err(format!("{fault} accepted as skills absence"));
        }
    }
    Ok(())
}

// This changes only the sealed private fixture's source data between commands;
// it is not a client-cache writer or a production deployment path.
fn advance_plugin_fixture() -> Result<(), String> {
    let source = Path::new("/home/olegchir/market/plugins/m08e1-demo");
    let next = Path::new("/fixture-input/plugin-v2");
    let before = plugin_finite_tree(source)?;
    let desired = plugin_finite_tree(next)?;
    let root = vibe_safefs::Project::open(source).map_err(|e| e.to_string())?;
    let input = vibe_safefs::Project::open(next).map_err(|e| e.to_string())?;
    let mut values = Vec::new();
    for path in PLUGIN_FILES {
        values.push((
            *path,
            std::fs::read(next.join(path)).map_err(|e| e.to_string())?,
        ));
        input
            .read_file_snapshot_bounded(path, 1024 * 1024)
            .map_err(|e| e.to_string())?
            .ok_or("missing v2 fixture member")?;
    }
    if plugin_finite_tree(source)? != before || plugin_finite_tree(next)? != desired {
        return Err("fixture source changed before transition".into());
    }
    for (path, bytes) in values {
        root.write_atomic(path, &bytes).map_err(|e| e.to_string())?;
    }
    if plugin_finite_tree(source)? != desired {
        return Err("fixture transition differs".into());
    }
    private_record(Path::new("/case/evidence/plugin-source-update.json"), &serde_json::to_vec_pretty(&serde_json::json!({"before":before,"after":desired,"scope":"sealed private source fixture only"})).map_err(|e| e.to_string())?)
}

pub(crate) fn plugin_update_observation_checks() -> Result<(), String> {
    let list = serde_json::json!({"marketplaces":[{"name":"m08e1-local","path":"/home/olegchir/market/.agents/plugins/marketplace.json","plugins":[{"id":"m08e1-demo@m08e1-local","installed":true,"enabled":true,"localVersion":"2.0.0"}]}],"marketplaceLoadErrors":[]});
    let skills = serde_json::json!({"data":[{"cwd":"/case/project","errors":[],"skills":[{"name":"m08e1-demo:m08a2-demo","description":"M08E1_PLUGIN_DESCRIPTION_v2_029","enabled":true,"path":"/home/olegchir/.codex/plugins/cache/m08e1-local/m08e1-demo/2.0.0/skills/m08a2-demo/SKILL.md"}]}]});
    codex_plugin_observed(5, &list, "2.0.0", true)?;
    codex_plugin_observed(6, &skills, "2.0.0", true)?;
    let mut stale = list.clone();
    stale["marketplaces"][0]["plugins"][0]["localVersion"] = "1.0.0".into();
    if codex_plugin_observed(5, &stale, "2.0.0", true).is_ok()
        || codex_plugin_observed(6, &skills, "1.0.0", true).is_ok()
    {
        return Err("stale installed revision accepted".into());
    }
    let root = tempfile::tempdir().map_err(|e| e.to_string())?;
    let project = vibe_safefs::Project::open(root.path()).map_err(|e| e.to_string())?;
    for path in PLUGIN_FILES {
        project
            .write_atomic(path, b"v1")
            .map_err(|e| e.to_string())?;
    }
    let before = plugin_finite_tree(root.path())?;
    project
        .write_atomic(PLUGIN_FILES[2], b"v2")
        .map_err(|e| e.to_string())?;
    if plugin_finite_tree(root.path())? == before {
        return Err("nested resource change not observed".into());
    }
    project
        .write_atomic("foreign.txt", b"foreign")
        .map_err(|e| e.to_string())?;
    if plugin_finite_tree(root.path()).is_ok() {
        return Err("unknown cache member accepted".into());
    }
    Ok(())
}

fn plugin_config_effect(
    before: &toml::Value,
    after: &toml::Value,
    remove: bool,
) -> Result<(), String> {
    fn split(value: &toml::Value) -> Result<(toml::Value, Option<toml::Value>), String> {
        let mut foreign = value.clone();
        let table = foreign.as_table_mut().ok_or("config table")?;
        let mut selected = None;
        if let Some(plugins) = table.get_mut("plugins") {
            let plugins = plugins.as_table_mut().ok_or("plugins table")?;
            selected = plugins.remove("m08e1-demo@m08e1-local");
            if plugins.is_empty() {
                table.remove("plugins");
            }
        }
        Ok((foreign, selected))
    }
    let (before_foreign, _) = split(before)?;
    let (after_foreign, selected) = split(after)?;
    let expected: toml::Value = toml::from_str("enabled = true\n").map_err(|e| e.to_string())?;
    if before_foreign != after_foreign || selected != if remove { None } else { Some(expected) } {
        return Err(
            "plugin command changed foreign configuration or selected enablement differs".into(),
        );
    }
    Ok(())
}

pub(crate) fn plugin_foreign_config_checks() -> Result<(), String> {
    let before: toml::Value = toml::from_str(
        "hide_agent_reasoning = true\n[plugins.\"foreign@other\"]\nenabled = false\n",
    )
    .map_err(|e| e.to_string())?;
    let mut installed = before.clone();
    installed["plugins"].as_table_mut().unwrap().insert(
        "m08e1-demo@m08e1-local".into(),
        toml::from_str("enabled = true").unwrap(),
    );
    plugin_config_effect(&before, &installed, false)?;
    plugin_config_effect(&installed, &before, true)?;
    let mut lost = before.clone();
    lost["plugins"]
        .as_table_mut()
        .unwrap()
        .remove("foreign@other");
    if plugin_config_effect(&installed, &lost, true).is_ok() {
        return Err("lost foreign plugin accepted".into());
    }
    let mut changed = installed.clone();
    changed["plugins"]["foreign@other"]["enabled"] = true.into();
    if plugin_config_effect(&before, &changed, false).is_ok() {
        return Err("changed foreign plugin accepted".into());
    }
    Ok(())
}
