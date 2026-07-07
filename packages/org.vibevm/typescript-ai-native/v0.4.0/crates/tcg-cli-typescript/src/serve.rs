//! `tcg-typescript serve` — the persistent enriching relay
//! (TCG-PROTOCOL-v0.1 §3): protocol frames in on stdin, frames out on
//! stdout; `validate` responses are widened with `conform_findings` +
//! `advice`, `init` is completed with the policy's `cells_dir`/`seam`
//! when the caller did not set them, everything else passes through
//! verbatim. The middle layer ADDS fields, never reshapes.

use std::io::{BufRead, Write};
use std::path::Path;

use anyhow::Result;
use serde::Deserialize;
use tcg_oracle_bridge::{
    ORACLE_PROTOCOL, OracleTransport, SystemOracle, TcgBridgeError, ValidateResult,
};

use crate::{ORACLE_TIMEOUT, Policy, enrich_validate};

#[derive(Deserialize)]
struct InboundFrame {
    proto: Option<u64>,
    id: Option<u64>,
    op: Option<String>,
    #[serde(default)]
    params: serde_json::Value,
}

fn wire_kind(e: &TcgBridgeError) -> &'static str {
    match e {
        TcgBridgeError::NodeMissing { .. } => "node-missing",
        TcgBridgeError::TypescriptUnresolvable { .. } => "typescript-unresolvable",
        TcgBridgeError::OracleCrashed { .. } | TcgBridgeError::Io { .. } => "oracle-crashed",
        TcgBridgeError::Protocol { .. } => "protocol",
        TcgBridgeError::Timeout { .. } => "timeout",
    }
}

fn write_line(out: &mut impl Write, value: &serde_json::Value) {
    // A dead stdout means the host is gone — exiting quietly is the
    // right shutdown, not a panic.
    if writeln!(out, "{value}").is_err() {
        std::process::exit(0);
    }
    let _ = out.flush();
}

fn error_frame(id: Option<u64>, kind: &str, detail: String) -> serde_json::Value {
    serde_json::json!({
        "proto": ORACLE_PROTOCOL,
        "id": id,
        "ok": false,
        "error": { "kind": kind, "detail": detail },
    })
}

/// The relay loop. Returns the process exit code.
pub fn run_serve(root: &Path) -> Result<i32> {
    let root = root
        .canonicalize()
        .unwrap_or_else(|_| root.to_path_buf());
    let policy = Policy::load(&root)?;
    let mut oracle = SystemOracle::spawn(&root, ORACLE_TIMEOUT)?;

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let frame: InboundFrame = match serde_json::from_str(trimmed) {
            Ok(f) => f,
            Err(e) => {
                write_line(
                    &mut stdout,
                    &error_frame(None, "protocol", format!("unparseable request: {e}")),
                );
                continue;
            }
        };
        let id = frame.id;
        if frame.proto != Some(ORACLE_PROTOCOL) {
            write_line(
                &mut stdout,
                &error_frame(
                    id,
                    "protocol",
                    format!("proto {:?} != {ORACLE_PROTOCOL}", frame.proto),
                ),
            );
            continue;
        }
        let op = frame.op.unwrap_or_default();
        let mut params = frame.params;

        if op == "shutdown" {
            write_line(
                &mut stdout,
                &serde_json::json!({
                    "proto": ORACLE_PROTOCOL, "id": id, "ok": true, "result": {},
                }),
            );
            break;
        }

        // init: complete with the policy's topology when absent, and
        // default the root to ours (the relay serves ONE project).
        if op == "init" {
            if params.get("root").is_none() {
                params["root"] = serde_json::Value::String(root.to_string_lossy().into_owned());
            }
            if params.get("cells_dir").is_none() {
                if let Some(cd) = policy.config.typescript.cells_dir.as_deref() {
                    params["cells_dir"] = serde_json::Value::String(cd.to_string());
                }
            }
            if params.get("seam").is_none() {
                params["seam"] =
                    serde_json::Value::String(policy.config.typescript.seam.clone());
            }
        }

        // The oracle does not echo `file` back; the relay remembers the
        // request's own so enrichment sees the real path (cell rules and
        // the in_test convention are path-driven).
        let validate_file: Option<String> = if op == "validate" {
            params
                .get("file")
                .and_then(|f| f.as_str())
                .map(str::to_string)
        } else {
            None
        };

        match oracle.request(&op, params) {
            Ok(result) => {
                let final_result = match (&op[..], &validate_file) {
                    ("validate", Some(file)) => {
                        match serde_json::from_value::<ValidateResult>(result.clone()) {
                            Ok(raw) => {
                                let enriched = enrich_validate(&policy, file, raw);
                                serde_json::to_value(enriched).unwrap_or(result)
                            }
                            Err(_) => result,
                        }
                    }
                    _ => result,
                };
                write_line(
                    &mut stdout,
                    &serde_json::json!({
                        "proto": ORACLE_PROTOCOL, "id": id, "ok": true,
                        "result": final_result,
                    }),
                );
            }
            Err(e) => {
                write_line(&mut stdout, &error_frame(id, wire_kind(&e), e.to_string()));
                // A crashed child cannot answer anything further —
                // surface it per-op and end the session; the host's
                // registry owns the respawn policy (PROP-026 §4).
                if matches!(e, TcgBridgeError::OracleCrashed { .. }) {
                    break;
                }
            }
        }
    }
    let _ = oracle.shutdown();
    Ok(0)
}
