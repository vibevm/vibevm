//! The correlated LSP client cell (ORACLE-GO §2, §6): one pump loop
//! that answers server→client requests, tracks notifications
//! ($/progress, publishDiagnostics, showMessage), and matches
//! responses by id under a deadline. Generic over a [`Transport`] seam
//! so the whole layer replays without gopls.

specmark::scope!("spec://go-ai-native-lang/go/mechanisms/TCG-ORACLE-GO-v0.1#session");

use std::collections::HashMap;
use std::io::BufReader;
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::{Duration, Instant};

use specmark::spec;

use crate::position::PositionEncoding;
use crate::{TcgBridgeError, frame, gopls_config};

/// The wire seam: send one frame, receive one frame (or EOF) under a
/// deadline. Production is a child's stdio; tests inject scripts.
pub trait Transport: Send {
    fn send(&mut self, value: &serde_json::Value) -> Result<(), TcgBridgeError>;
    /// `Ok(None)` is EOF — the child ended the stream.
    fn recv(&mut self, deadline: Instant) -> Result<Option<serde_json::Value>, TcgBridgeError>;
}

/// What the server granted at initialize (ORACLE-GO §2) — every
/// downstream feature keys off this record, never off assumptions.
/// gopls has no serverStatus channel; readiness rides `$/progress`
/// end events, bounded by the caller's deadline.
#[derive(Debug, Clone)]
pub struct Capabilities {
    pub position_encoding: PositionEncoding,
    pub pull_diagnostics: bool,
    pub server_version: String,
}

/// The client: correlation, auto-answers, notification state.
#[spec(implements = "spec://go-ai-native-lang/go/mechanisms/TCG-ORACLE-GO-v0.1#session")]
pub struct LspClient<T: Transport> {
    transport: T,
    next_id: u64,
    /// Response frames (result or error) that arrived while pumping
    /// for something else, whole, keyed by request id.
    parked: HashMap<u64, serde_json::Value>,
    /// A `$/progress` end event was seen — the initial-load readiness
    /// signal (ORACLE-GO §6).
    progress_ended: bool,
    /// uri → diagnostics from the push channel — THE diagnostics
    /// source when the server did not grant pull (ORACLE-GO §2).
    pub published: HashMap<String, serde_json::Value>,
}

impl<T: Transport> LspClient<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            next_id: 0,
            parked: HashMap::new(),
            progress_ended: false,
            published: HashMap::new(),
        }
    }

    pub fn notify(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<(), TcgBridgeError> {
        self.transport.send(&serde_json::json!({
            "jsonrpc": "2.0", "method": method, "params": params,
        }))
    }

    /// One request, answered or refused within `budget`. A response
    /// carrying LSP ServerCancelled (-32802) with `retriggerRequest:
    /// true` is re-sent under the SAME deadline — specified client
    /// behaviour, not a failure. The deadline stays the cap: a cancel
    /// storm ends as the op's timeout, never a spin.
    pub fn request(
        &mut self,
        method: &str,
        params: serde_json::Value,
        budget: Duration,
    ) -> Result<serde_json::Value, TcgBridgeError> {
        let deadline = Instant::now() + budget;
        loop {
            self.next_id += 1;
            let id = self.next_id;
            self.transport.send(&serde_json::json!({
                "jsonrpc": "2.0", "id": id, "method": method, "params": params.clone(),
            }))?;
            let msg = loop {
                if let Some(msg) = self.parked.remove(&id) {
                    break msg;
                }
                let Some(msg) = self.pump_one(deadline, method)? else {
                    return Err(TcgBridgeError::OracleCrashed {
                        detail: format!("stream ended while `{method}` was in flight"),
                    });
                };
                self.dispatch(msg)?;
            };
            let Some(err) = msg.get("error") else {
                return Ok(msg.get("result").cloned().unwrap_or_default());
            };
            let cancelled = err.get("code").and_then(serde_json::Value::as_i64) == Some(-32802);
            let retrigger = err
                .pointer("/data/retriggerRequest")
                .and_then(serde_json::Value::as_bool)
                == Some(true);
            if cancelled && retrigger {
                continue;
            }
            return Err(TcgBridgeError::Protocol {
                detail: format!("server error for request {id}: {err}"),
            });
        }
    }

    /// Wait until the server looks ready (ORACLE-GO §6): gopls has no
    /// serverStatus channel, so readiness is the first `$/progress`
    /// end event — the initial workspace-load progress closing —
    /// bounded by the deadline. `false` = the deadline passed; the
    /// caller degrades, never crashes. (The Rust campaign's falsified
    /// progress-drain heuristic waited for token DRAIN; this waits for
    /// an explicit `kind: "end"`, and the live chain pins whether the
    /// shipped gopls emits it — a miss just means degraded answers.)
    pub fn wait_ready(&mut self, budget: Duration) -> bool {
        let deadline = Instant::now() + budget;
        while !self.progress_ended {
            match self.pump_one(deadline, "readiness") {
                Ok(Some(msg)) => {
                    if self.dispatch(msg).is_err() {
                        return false;
                    }
                }
                Ok(None) | Err(_) => return false,
            }
        }
        true
    }

    /// Pump until the push channel carries diagnostics for `uri` (the
    /// pull-less fallback, ORACLE-GO §2), bounded by `budget`. Returns
    /// the published set if it arrived.
    pub fn wait_published(&mut self, uri: &str, budget: Duration) -> Option<serde_json::Value> {
        let deadline = Instant::now() + budget;
        loop {
            if let Some(d) = self.published.get(uri) {
                return Some(d.clone());
            }
            match self.pump_one(deadline, "publishDiagnostics") {
                Ok(Some(msg)) => {
                    if self.dispatch(msg).is_err() {
                        return None;
                    }
                }
                Ok(None) | Err(_) => return self.published.get(uri).cloned(),
            }
        }
    }

    /// Receive one frame under the deadline; timeouts surface as the
    /// caller's op timeout.
    fn pump_one(
        &mut self,
        deadline: Instant,
        op: &str,
    ) -> Result<Option<serde_json::Value>, TcgBridgeError> {
        if Instant::now() >= deadline {
            return Err(TcgBridgeError::Timeout {
                op: op.to_string(),
                budget_ms: 0,
            });
        }
        self.transport.recv(deadline).map_err(|e| match e {
            TcgBridgeError::Timeout { .. } => TcgBridgeError::Timeout {
                op: op.to_string(),
                budget_ms: 0,
            },
            other => other,
        })
    }

    /// Route one inbound frame: park responses, answer server
    /// requests, absorb notifications.
    fn dispatch(&mut self, msg: serde_json::Value) -> Result<(), TcgBridgeError> {
        let has_id = msg.get("id").is_some();
        let method = msg.get("method").and_then(|m| m.as_str());
        match (has_id, method) {
            // Server → client REQUEST: answer, never stall the server.
            (true, Some(m)) => {
                let id = msg["id"].clone();
                let result = match m {
                    "workspace/configuration" => {
                        let n = msg
                            .pointer("/params/items")
                            .and_then(|i| i.as_array())
                            .map_or(1, Vec::len);
                        serde_json::Value::Array(vec![gopls_config(); n])
                    }
                    _ => serde_json::Value::Null,
                };
                self.transport.send(&serde_json::json!({
                    "jsonrpc": "2.0", "id": id, "result": result,
                }))
            }
            // Response: park the WHOLE frame — result or error — for
            // the requester, which owns the retrigger/refuse decision.
            (true, None) => {
                if let Some(id) = msg["id"].as_u64() {
                    self.parked.insert(id, msg);
                }
                Ok(())
            }
            // Notification: absorb into state.
            (false, Some("$/progress")) => {
                if msg.pointer("/params/value/kind").and_then(|k| k.as_str()) == Some("end") {
                    self.progress_ended = true;
                }
                Ok(())
            }
            (false, Some("textDocument/publishDiagnostics")) => {
                if let Some(uri) = msg.pointer("/params/uri").and_then(|u| u.as_str()) {
                    let diags = msg
                        .pointer("/params/diagnostics")
                        .cloned()
                        .unwrap_or_default();
                    self.published.insert(uri.to_string(), diags);
                }
                Ok(())
            }
            _ => Ok(()), // unknown notifications are legal noise
        }
    }

    /// The ORACLE-GO §2 handshake: initialize (utf-8 requested, pull
    /// diagnostics declared, workDoneProgress), read the granted set,
    /// initialized.
    #[spec(implements = "spec://go-ai-native-lang/go/mechanisms/TCG-ORACLE-GO-v0.1#session")]
    pub fn initialize(
        &mut self,
        root_uri: &str,
        budget: Duration,
    ) -> Result<Capabilities, TcgBridgeError> {
        let result = self
            .request(
                "initialize",
                serde_json::json!({
                    "processId": std::process::id(),
                    "rootUri": root_uri,
                    "initializationOptions": gopls_config(),
                    "capabilities": {
                        "general": { "positionEncodings": ["utf-8", "utf-16"] },
                        "textDocument": {
                            "synchronization": { "didSave": true },
                            "publishDiagnostics": { "relatedInformation": true },
                            "diagnostic": { "dynamicRegistration": false },
                            "hover": { "contentFormat": ["plaintext", "markdown"] },
                            "completion": {
                                "completionItem": { "snippetSupport": false },
                            },
                        },
                        "window": { "workDoneProgress": true },
                        "workspace": { "configuration": true },
                    },
                    "workspaceFolders": [{ "uri": root_uri, "name": "root" }],
                }),
                budget,
            )
            .map_err(|e| match e {
                TcgBridgeError::OracleCrashed { detail } => {
                    TcgBridgeError::WorkspaceUnloadable { detail }
                }
                other => other,
            })?;
        let caps = Capabilities {
            position_encoding: PositionEncoding::from_wire(
                result
                    .pointer("/capabilities/positionEncoding")
                    .and_then(|p| p.as_str()),
            ),
            pull_diagnostics: result.pointer("/capabilities/diagnosticProvider").is_some(),
            server_version: result
                .pointer("/serverInfo/version")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
        };
        self.notify("initialized", serde_json::json!({}))?;
        Ok(caps)
    }
}

/// The production transport: a spawned gopls child on piped stdio with
/// a reader thread, kill-on-drop (ORACLE-GO §7). gopls's own stderr
/// chatter is discarded so protocol streams stay clean.
pub struct ChildTransport {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    frames: Receiver<Result<serde_json::Value, TcgBridgeError>>,
}

impl ChildTransport {
    pub fn spawn(
        program: &std::path::Path,
        root: &std::path::Path,
    ) -> Result<Self, TcgBridgeError> {
        let mut child = std::process::Command::new(program)
            .current_dir(root)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| TcgBridgeError::GoplsMissing {
                detail: format!("spawning {}: {e}", program.display()),
            })?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| TcgBridgeError::OracleCrashed {
                detail: "child stdin not piped".to_string(),
            })?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| TcgBridgeError::OracleCrashed {
                detail: "child stdout not piped".to_string(),
            })?;
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                match frame::read_frame(&mut reader) {
                    Ok(Some(v)) => {
                        if tx.send(Ok(v)).is_err() {
                            break;
                        }
                    }
                    Ok(None) => break, // clean EOF ends the thread
                    Err(e) => {
                        let _ = tx.send(Err(e));
                        break;
                    }
                }
            }
        });
        Ok(Self {
            child,
            stdin,
            frames: rx,
        })
    }
}

impl Drop for ChildTransport {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Transport for ChildTransport {
    fn send(&mut self, value: &serde_json::Value) -> Result<(), TcgBridgeError> {
        frame::write_frame(&mut self.stdin, value)
    }

    fn recv(&mut self, deadline: Instant) -> Result<Option<serde_json::Value>, TcgBridgeError> {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match self.frames.recv_timeout(remaining) {
            Ok(Ok(v)) => Ok(Some(v)),
            Ok(Err(e)) => Err(e),
            Err(RecvTimeoutError::Timeout) => Err(TcgBridgeError::Timeout {
                op: "recv".to_string(),
                budget_ms: 0,
            }),
            Err(RecvTimeoutError::Disconnected) => Ok(None),
        }
    }
}

#[cfg(test)]
#[path = "client/tests.rs"]
pub(crate) mod tests;
