//! `go-ai-native-tcg serve` — the persistent enriching relay
//! (TCG-PROTOCOL-GO §1–§3): outer frames in on stdin, frames out on
//! stdout; `validate` answers are widened with the gate's findings +
//! advice + the FILLED markers stream; everything else passes through
//! the oracle. The relay OWNS session init (a host's first frame may
//! be any op); client `init` frames respawn gopls.

specmark::scope!("spec://go-ai-native-lang/go/mechanisms/TCG-PROTOCOL-GO-v0.1#parity");

use std::io::{BufRead, Write};
use std::path::Path;

use anyhow::Result;
use go_ai_native_tcg_bridge::client::ChildTransport;
use go_ai_native_tcg_bridge::{GoOracle, TcgBridgeError};
use serde::Deserialize;

use crate::{
    Policy, READINESS_BUDGET, ScopeAnswer, brands_of, cell_of, enrich_validate,
    finalise_completions, parse_position, seam_file_for,
};

/// The wire protocol both hops share (TCG-PROTOCOL-GO §1).
pub const ORACLE_PROTOCOL: u64 = 1;

#[derive(Deserialize)]
struct InboundFrame {
    proto: Option<u64>,
    id: Option<u64>,
    op: Option<String>,
    #[serde(default)]
    params: serde_json::Value,
}

fn write_line(out: &mut impl Write, value: &serde_json::Value) {
    // A dead stdout means the host is gone — exiting quietly is the
    // right shutdown, not a panic.
    if writeln!(out, "{value}").is_err() {
        std::process::exit(0);
    }
    let _ = out.flush();
}

fn ok_frame(id: Option<u64>, result: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "proto": ORACLE_PROTOCOL, "id": id, "ok": true, "result": result,
    })
}

fn error_frame(id: Option<u64>, kind: &str, detail: String) -> serde_json::Value {
    serde_json::json!({
        "proto": ORACLE_PROTOCOL, "id": id, "ok": false,
        "error": { "kind": kind, "detail": detail },
    })
}

fn str_param(params: &serde_json::Value, key: &str) -> Option<String> {
    params.get(key).and_then(|v| v.as_str()).map(str::to_string)
}

fn position_param(
    params: &serde_json::Value,
) -> Option<go_ai_native_tcg_bridge::position::OuterPosition> {
    let p = params.get("position")?;
    if let Some(s) = p.as_str() {
        return parse_position(s).ok();
    }
    Some(go_ai_native_tcg_bridge::position::OuterPosition {
        line: p.get("line")?.as_u64()? as u32,
        character: p.get("character")?.as_u64()? as u32,
    })
}

fn init_result(oracle: &GoOracle<ChildTransport>) -> serde_json::Value {
    serde_json::json!({
        "gopls_version": oracle.capabilities().server_version,
        "position_encoding": match oracle.capabilities().position_encoding {
            go_ai_native_tcg_bridge::position::PositionEncoding::Utf8 => "utf-8",
            go_ai_native_tcg_bridge::position::PositionEncoding::Utf16 => "utf-16",
        },
        "pull_diagnostics": oracle.capabilities().pull_diagnostics,
        "ready": oracle.ready(),
    })
}

/// One op against the live session. Returns the response frame and
/// whether the session died (`oracle-crashed` ends the loop; the host
/// registry owns respawn).
fn handle_op(
    policy: &Policy,
    oracle: &mut GoOracle<ChildTransport>,
    id: Option<u64>,
    op: &str,
    params: &serde_json::Value,
) -> (serde_json::Value, bool) {
    let file = str_param(params, "file");
    let content = str_param(params, "content");
    let outcome: Result<serde_json::Value, TcgBridgeError> = match op {
        "validate" => match &file {
            Some(f) => {
                let text = match content.clone() {
                    Some(t) => Ok(t),
                    None => std::fs::read_to_string(policy.root.join(f)).map_err(|e| {
                        TcgBridgeError::Protocol {
                            detail: format!("`{f}`: no content and no disk state: {e}"),
                        }
                    }),
                };
                match text {
                    Ok(text) => oracle.validate(f, Some(text.clone())).map(|raw| {
                        let enriched = enrich_validate(policy, f, &text, raw);
                        serde_json::to_value(enriched).unwrap_or_default()
                    }),
                    Err(e) => Err(e),
                }
            }
            None => Err(TcgBridgeError::Protocol {
                detail: "`validate` needs `file`".to_string(),
            }),
        },
        "scope" => match &file {
            Some(f) => {
                let pos = position_param(params).unwrap_or(
                    go_ai_native_tcg_bridge::position::OuterPosition {
                        line: 1,
                        character: 0,
                    },
                );
                oracle.complete(f, pos, content.clone()).map(|entries| {
                    let symbols =
                        finalise_completions(&policy.config, entries, f, None, 200);
                    let cell = cell_of(&policy.config, f);
                    let seam = seam_file_for(&policy.config, f);
                    // Brands come from the seams package's files (and
                    // the target file itself), through the extractor.
                    let mut branded = Vec::new();
                    let mut sources: Vec<String> = vec![f.clone()];
                    if let Ok(entries) = std::fs::read_dir(policy.root.join(&seam)) {
                        for entry in entries.filter_map(std::result::Result::ok) {
                            let p = entry.path();
                            if p.extension().and_then(|e| e.to_str()) == Some("go") {
                                let rel = format!(
                                    "{seam}/{}",
                                    entry.file_name().to_string_lossy()
                                );
                                sources.push(rel);
                            }
                        }
                    }
                    sources.sort();
                    sources.dedup();
                    if let Ok(records) = go_ai_native_extract_bridge::extract_tree(
                        &policy.root,
                        &policy.extractor,
                        Some(&sources),
                    ) {
                        for record in &records {
                            branded.extend(brands_of(record));
                        }
                    }
                    serde_json::to_value(ScopeAnswer {
                        symbols,
                        cell,
                        seam_file: seam,
                        branded,
                    })
                    .unwrap_or_default()
                })
            }
            None => Err(TcgBridgeError::Protocol {
                detail: "`scope` needs `file`".to_string(),
            }),
        },
        "complete" => match (&file, position_param(params)) {
            (Some(f), Some(pos)) => oracle.complete(f, pos, content.clone()).map(|entries| {
                let prefix = str_param(params, "prefix");
                let max = params
                    .get("max")
                    .and_then(|m| m.as_u64())
                    .unwrap_or(50)
                    .max(1) as usize;
                serde_json::json!({
                    "entries": finalise_completions(
                        &policy.config, entries, f, prefix.as_deref(), max,
                    ),
                })
            }),
            _ => Err(TcgBridgeError::Protocol {
                detail: "`complete` needs `file` and `position`".to_string(),
            }),
        },
        "type" => match (&file, position_param(params)) {
            (Some(f), Some(pos)) => {
                oracle
                    .hover(f, pos, content.clone())
                    .map(|(display, documentation)| {
                        serde_json::json!({
                            "display": display, "documentation": documentation,
                        })
                    })
            }
            _ => Err(TcgBridgeError::Protocol {
                detail: "`type` needs `file` and `position`".to_string(),
            }),
        },
        "update" => match &file {
            Some(f) => oracle
                .update(f, content.clone())
                .map(|version| serde_json::json!({ "version": version })),
            None => Err(TcgBridgeError::Protocol {
                detail: "`update` needs `file`".to_string(),
            }),
        },
        unknown => Err(TcgBridgeError::Protocol {
            detail: format!(
                "unknown op `{unknown}` — known: init, update, validate, scope, \
                 complete, type, shutdown"
            ),
        }),
    };
    match outcome {
        Ok(result) => (ok_frame(id, result), false),
        Err(e) => {
            let crashed = matches!(e, TcgBridgeError::OracleCrashed { .. });
            (error_frame(id, e.wire_kind(), e.to_string()), crashed)
        }
    }
}

/// The relay loop. Returns the process exit code.
pub fn run_serve(root: &Path) -> Result<i32> {
    let root = go_ai_native_tcg_bridge::verbatim_free(
        &root.canonicalize().unwrap_or_else(|_| root.to_path_buf()),
    );
    let policy = Policy::load(&root)?;
    // The relay owns the session: boot gopls up front so the host's
    // FIRST frame can be validate/scope/… (client init frames remain
    // re-init).
    let mut oracle = match GoOracle::spawn(&root, READINESS_BUDGET) {
        Ok(o) => o,
        Err(e) => {
            // A boot refusal is one well-formed frame, then exit — the
            // host registry surfaces the recipe.
            let mut stdout = std::io::stdout();
            write_line(
                &mut stdout,
                &error_frame(None, e.wire_kind(), e.to_string()),
            );
            return Ok(1);
        }
    };
    eprintln!(
        "go-ai-native-tcg serve: oracle up — gopls {}, ready={}, {}",
        oracle.capabilities().server_version,
        oracle.ready(),
        root.display()
    );

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

        if op == "shutdown" {
            write_line(&mut stdout, &ok_frame(id, serde_json::json!({})));
            break;
        }
        if op == "init" {
            // Re-init: a fresh gopls session (overlays cleared).
            match GoOracle::spawn(&root, READINESS_BUDGET) {
                Ok(fresh) => {
                    let old = std::mem::replace(&mut oracle, fresh);
                    let _ = old.shutdown();
                    write_line(&mut stdout, &ok_frame(id, init_result(&oracle)));
                }
                Err(e) => {
                    write_line(&mut stdout, &error_frame(id, e.wire_kind(), e.to_string()));
                }
            }
            continue;
        }

        let (response, crashed) = handle_op(&policy, &mut oracle, id, &op, &frame.params);
        write_line(&mut stdout, &response);
        if crashed {
            // The session cannot answer further — end it; the host's
            // registry owns the respawn policy (PROP-026 §4).
            break;
        }
    }
    let _ = oracle.shutdown();
    Ok(0)
}
