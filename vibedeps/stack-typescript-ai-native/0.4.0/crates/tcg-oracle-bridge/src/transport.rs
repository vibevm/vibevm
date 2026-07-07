//! The persistent oracle transport: spawn `node <materialised oracle>`,
//! correlate NDJSON requests/responses by id with a per-request timeout,
//! shut down gracefully, kill on drop (TCG-ORACLE §6).
//!
//! Reading happens on a dedicated thread feeding an mpsc channel —
//! blocking `read_line` with a timeout is not a thing Windows pipes
//! offer, a reader thread + `recv_timeout` is. The trait seam exists so
//! consumers (the CLI's serve/enrich layer, vibe-tcg's registry) test
//! against a double with no node anywhere near the unit suite (the
//! hooks-runner seam lesson).

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::time::Duration;

use crate::{
    CompleteResult, InitResult, ORACLE_PROTOCOL, Position, ResponseFrame, ScopeResult,
    TcgBridgeError, TypeResult, ValidateResult, error_from_wire, materialise_oracle,
    parse_response_line,
};

/// The op-level transport seam: everything above it (enrichment, the
/// CLI, vibe-tcg) speaks typed requests; everything below it is
/// process mechanics. Implement it with a double to test the layers
/// above without node.
pub trait OracleTransport {
    fn request(
        &mut self,
        op: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, TcgBridgeError>;

    fn init(
        &mut self,
        root: &Path,
        cells_dir: Option<&str>,
        seam: &str,
    ) -> Result<InitResult, TcgBridgeError> {
        let mut params = serde_json::json!({
            "root": root.to_string_lossy(),
            "seam": seam,
        });
        if let Some(cd) = cells_dir {
            params["cells_dir"] = serde_json::Value::String(cd.to_string());
        }
        let v = self.request("init", params)?;
        serde_json::from_value(v).map_err(|e| TcgBridgeError::Protocol {
            detail: format!("init result shape: {e}"),
        })
    }

    fn validate(
        &mut self,
        file: &str,
        content: Option<&str>,
    ) -> Result<ValidateResult, TcgBridgeError> {
        let mut params = serde_json::json!({ "file": file });
        if let Some(c) = content {
            params["content"] = serde_json::Value::String(c.to_string());
        }
        let v = self.request("validate", params)?;
        serde_json::from_value(v).map_err(|e| TcgBridgeError::Protocol {
            detail: format!("validate result shape: {e}"),
        })
    }

    fn scope(
        &mut self,
        file: &str,
        position: Option<Position>,
    ) -> Result<ScopeResult, TcgBridgeError> {
        let mut params = serde_json::json!({ "file": file });
        if let Some(p) = position {
            params["position"] = serde_json::json!({ "line": p.line, "character": p.character });
        }
        let v = self.request("scope", params)?;
        serde_json::from_value(v).map_err(|e| TcgBridgeError::Protocol {
            detail: format!("scope result shape: {e}"),
        })
    }

    fn complete(
        &mut self,
        file: &str,
        position: Position,
        content: Option<&str>,
        prefix: Option<&str>,
        max: u64,
    ) -> Result<CompleteResult, TcgBridgeError> {
        let mut params = serde_json::json!({
            "file": file,
            "position": { "line": position.line, "character": position.character },
            "max": max,
        });
        if let Some(c) = content {
            params["content"] = serde_json::Value::String(c.to_string());
        }
        if let Some(p) = prefix {
            params["prefix"] = serde_json::Value::String(p.to_string());
        }
        let v = self.request("complete", params)?;
        serde_json::from_value(v).map_err(|e| TcgBridgeError::Protocol {
            detail: format!("complete result shape: {e}"),
        })
    }

    fn quick_info(
        &mut self,
        file: &str,
        position: Position,
        content: Option<&str>,
    ) -> Result<TypeResult, TcgBridgeError> {
        let mut params = serde_json::json!({
            "file": file,
            "position": { "line": position.line, "character": position.character },
        });
        if let Some(c) = content {
            params["content"] = serde_json::Value::String(c.to_string());
        }
        let v = self.request("type", params)?;
        serde_json::from_value(v).map_err(|e| TcgBridgeError::Protocol {
            detail: format!("type result shape: {e}"),
        })
    }
}

/// The production transport over a spawned node child.
pub struct SystemOracle {
    child: Child,
    stdin: ChildStdin,
    lines: Receiver<Result<String, std::io::Error>>,
    next_id: u64,
    timeout: Duration,
}

impl SystemOracle {
    /// Materialise the embedded oracle under `project_root` and spawn
    /// it. The child starts idle (no `--root`): drive it with
    /// [`OracleTransport::init`], whose response carries the
    /// typescript-resolution outcome as a typed error instead of an
    /// exit code race.
    pub fn spawn(project_root: &Path, timeout: Duration) -> Result<Self, TcgBridgeError> {
        let script = materialise_oracle(project_root)?;
        let mut child = Command::new("node")
            .arg(&script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| TcgBridgeError::NodeMissing { source: e })?;
        let stdin = child.stdin.take().ok_or_else(|| TcgBridgeError::OracleCrashed {
            detail: "child stdin not piped".to_string(),
        })?;
        let stdout = child.stdout.take().ok_or_else(|| TcgBridgeError::OracleCrashed {
            detail: "child stdout not piped".to_string(),
        })?;
        let (tx, rx): (Sender<Result<String, std::io::Error>>, _) = channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => break, // EOF — the child is gone
                    Ok(_) => {
                        if tx.send(Ok(line)).is_err() {
                            break;
                        }
                    }
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
            lines: rx,
            next_id: 1,
            timeout,
        })
    }

    /// Graceful shutdown: the `shutdown` op, then wait (bounded), then
    /// kill if the child lingers.
    pub fn shutdown(mut self) -> Result<(), TcgBridgeError> {
        let id = self.next_id;
        let line = serde_json::json!({
            "proto": ORACLE_PROTOCOL, "id": id, "op": "shutdown", "params": {},
        });
        let _ = writeln!(self.stdin, "{line}");
        let _ = self.stdin.flush();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) => return Ok(()),
                Ok(None) if std::time::Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(50));
                }
                _ => {
                    let _ = self.child.kill();
                    let _ = self.child.wait();
                    return Ok(());
                }
            }
        }
    }
}

impl Drop for SystemOracle {
    fn drop(&mut self) {
        // Kill-on-drop: no zombie node children (TCG-ORACLE §6). A child
        // that already exited makes both calls no-ops.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl OracleTransport for SystemOracle {
    fn request(
        &mut self,
        op: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, TcgBridgeError> {
        let id = self.next_id;
        self.next_id += 1;
        let line = serde_json::json!({
            "proto": ORACLE_PROTOCOL, "id": id, "op": op, "params": params,
        });
        writeln!(self.stdin, "{line}").map_err(|e| TcgBridgeError::OracleCrashed {
            detail: format!("writing request: {e}"),
        })?;
        self.stdin.flush().map_err(|e| TcgBridgeError::OracleCrashed {
            detail: format!("flushing request: {e}"),
        })?;

        let deadline = std::time::Instant::now() + self.timeout;
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return Err(TcgBridgeError::Timeout {
                    op: op.to_string(),
                    budget_ms: self.timeout.as_millis() as u64,
                });
            }
            let raw = match self.lines.recv_timeout(remaining) {
                Ok(Ok(l)) => l,
                Ok(Err(e)) => {
                    return Err(TcgBridgeError::OracleCrashed {
                        detail: format!("reading response: {e}"),
                    });
                }
                Err(RecvTimeoutError::Timeout) => {
                    return Err(TcgBridgeError::Timeout {
                        op: op.to_string(),
                        budget_ms: self.timeout.as_millis() as u64,
                    });
                }
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(TcgBridgeError::OracleCrashed {
                        detail: "oracle stream closed mid-session".to_string(),
                    });
                }
            };
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                continue;
            }
            let frame: ResponseFrame = parse_response_line(trimmed)?;
            // FIFO in practice (one in-flight request via &mut self), but
            // match by id anyway; foreign ids are protocol noise, logged
            // and skipped rather than mis-delivered.
            if frame.id != Some(id) {
                eprintln!(
                    "tcg-oracle-bridge: skipping response for id {:?} while waiting for {id}",
                    frame.id
                );
                continue;
            }
            return if frame.ok {
                Ok(frame.result.unwrap_or(serde_json::Value::Null))
            } else {
                Err(match frame.error {
                    Some(e) => error_from_wire(e),
                    None => TcgBridgeError::Protocol {
                        detail: "ok:false frame with no error body".to_string(),
                    },
                })
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    /// A no-node double: scripted responses per op.
    struct DoubleTransport {
        script: VecDeque<Result<serde_json::Value, TcgBridgeError>>,
        pub seen: Vec<(String, serde_json::Value)>,
    }

    impl OracleTransport for DoubleTransport {
        fn request(
            &mut self,
            op: &str,
            params: serde_json::Value,
        ) -> Result<serde_json::Value, TcgBridgeError> {
            self.seen.push((op.to_string(), params));
            self.script.pop_front().expect("scripted response")
        }
    }

    #[test]
    fn typed_ops_shape_their_requests_and_parse_their_results() {
        let mut t = DoubleTransport {
            script: VecDeque::from([
                Ok(serde_json::json!({
                    "ts_version": "6.0.3", "config_file": "tsconfig.json", "root_files": 2,
                })),
                Ok(serde_json::json!({
                    "diagnostics": [], "facts": [], "markers": [], "degraded": false,
                })),
                Ok(serde_json::json!({
                    "entries": [
                        {"name": "greet", "kind": "function", "type_text": "…", "unsafe": false},
                    ],
                })),
            ]),
            seen: Vec::new(),
        };
        let init = t
            .init(Path::new("/proj"), Some("src/cells"), "index")
            .expect("init");
        assert_eq!(init.ts_version, "6.0.3");
        let v = t.validate("src/a.ts", Some("const x = 1;")).expect("validate");
        assert!(v.diagnostics.is_empty());
        let c = t
            .complete(
                "src/a.ts",
                Position { line: 3, character: 8 },
                None,
                Some("gre"),
                10,
            )
            .expect("complete");
        assert_eq!(c.entries[0].name, "greet");
        assert!(!c.entries[0].unsafe_);

        // request shapes: init carried cells_dir + seam; complete carried
        // prefix + max + position in the wire convention.
        assert_eq!(t.seen[0].0, "init");
        assert_eq!(t.seen[0].1["cells_dir"], "src/cells");
        assert_eq!(t.seen[2].1["prefix"], "gre");
        assert_eq!(t.seen[2].1["max"], 10);
        assert_eq!(t.seen[2].1["position"]["line"], 3);
    }
}
