//! The ask_boss MCP broker (D18 layer 3): a stdio JSON-RPC server the
//! worker's Claude Code launches as an MCP server (`fractality
//! mcp-broker`, plumbing — hidden from help).
//!
//! One tool, `ask_boss(question)`: the call parks the run on the bus
//! (`running -> waiting_on_boss`, question.md on the plane), then polls
//! the run record until the boss's answer lands (`fractality answer`)
//! and returns it as the tool result — the worker stays alive, blocked
//! on exactly this one tool call. Timeouts are the packet's wall budget:
//! if mission-control kills the parked run, the poll sees a terminal
//! state and reports the death instead of an answer.
//!
//! The broker discovers its run and home from the worker environment
//! (`FRACTALITY_RUN_ID`, `FRACTALITY_HOME` — pod-injected, D5): the
//! environment IS the protocol on this seam, same as `swarm.rs`.

use fractality_core::ids::RunId;
use fractality_mc_client::McClient;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

specmark::scope!("spec://fractality/PROP-001#architecture");

/// MCP protocol revision this broker answers with when the client does
/// not name one.
const PROTOCOL_VERSION: &str = "2024-11-05";

/// What one inbound JSON-RPC message asks of the broker.
#[derive(Debug, PartialEq)]
pub(crate) enum Route {
    /// Answer immediately with this JSON-RPC response.
    Reply(Value),
    /// Run the ask_boss bus dance, then reply (needs the bus, async).
    AskBoss { id: Value, question: String },
    /// Record an escalation on the bus, then reply. Terminal: the run's
    /// task is handed UP the tree (D-C3-6) — nothing to wait for.
    Escalate {
        id: Value,
        reason: String,
        needs: String,
    },
    /// A notification or unparseable line — nothing to write back.
    Silent,
}

/// Pure protocol routing: one inbound message → what to do. Kept free
/// of I/O so the wire behavior is a plain unit test.
pub(crate) fn route(line: &str) -> Route {
    let Ok(msg) = serde_json::from_str::<Value>(line) else {
        // A malformed line on a framed stdio channel: nothing sane to
        // address a reply to (no id) — stay silent, keep serving.
        return Route::Silent;
    };
    let id = msg.get("id").cloned();
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    match (method, id) {
        ("initialize", Some(id)) => {
            let requested = msg
                .pointer("/params/protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or(PROTOCOL_VERSION);
            Route::Reply(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": requested,
                    "capabilities": { "tools": {} },
                    "serverInfo": {
                        "name": "fractality-broker",
                        "version": env!("CARGO_PKG_VERSION"),
                    },
                },
            }))
        }
        ("tools/list", Some(id)) => Route::Reply(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": { "tools": [
                {
                    "name": "ask_boss",
                    "description": "Ask the supervising boss a question and wait for the \
                         answer. Use when genuinely stuck or before anything destructive: \
                         ask one precise, answerable question instead of guessing.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "question": {
                                "type": "string",
                                "description": "One precise question for the boss.",
                            },
                        },
                        "required": ["question"],
                    },
                },
                {
                    "name": "escalate",
                    "description": "Hand this whole task UP to the boss and STOP. Use when \
                         the task cannot or should not be finished here — it needs a \
                         capability, decision, or a larger context window you do not have, \
                         or splitting it would destroy the cross-cutting reasoning it \
                         needs. This ENDS your run (it is not a failure); after calling it, \
                         stop working and end your turn.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "reason": {
                                "type": "string",
                                "description": "Why the task cannot be finished here.",
                            },
                            "needs": {
                                "type": "string",
                                "description": "What would unblock it above (capability, \
                                     decision, larger window, budget).",
                            },
                        },
                        "required": ["reason"],
                    },
                },
            ]},
        })),
        ("tools/call", Some(id)) => {
            let name = msg.pointer("/params/name").and_then(Value::as_str);
            match name {
                Some("ask_boss") => match msg
                    .pointer("/params/arguments/question")
                    .and_then(Value::as_str)
                {
                    Some(q) if !q.trim().is_empty() => Route::AskBoss {
                        id,
                        question: q.to_owned(),
                    },
                    _ => Route::Reply(arg_error(
                        id,
                        "ask_boss requires a non-empty `question` string",
                    )),
                },
                Some("escalate") => {
                    let reason = msg
                        .pointer("/params/arguments/reason")
                        .and_then(Value::as_str);
                    let needs = msg
                        .pointer("/params/arguments/needs")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    match reason {
                        Some(r) if !r.trim().is_empty() => Route::Escalate {
                            id,
                            reason: r.to_owned(),
                            needs: needs.to_owned(),
                        },
                        _ => Route::Reply(arg_error(
                            id,
                            "escalate requires a non-empty `reason` string",
                        )),
                    }
                }
                other => Route::Reply(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": { "code": -32602, "message": format!(
                        "unknown tool `{}` (this broker serves ask_boss and escalate)",
                        other.unwrap_or("?")
                    )},
                })),
            }
        }
        ("ping", Some(id)) => Route::Reply(json!({
            "jsonrpc": "2.0", "id": id, "result": {},
        })),
        // Notifications (no id) are acknowledged by silence.
        (_, None) => Route::Silent,
        (other, Some(id)) => Route::Reply(json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32601, "message": format!("method `{other}` not found") },
        })),
    }
}

/// A JSON-RPC invalid-params (-32602) error reply for a bad tool argument.
fn arg_error(id: Value, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": -32602, "message": message },
    })
}

/// A tool result envelope (success or in-band error, MCP shape).
fn tool_reply(id: Value, text: String, is_error: bool) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "content": [{ "type": "text", "text": text }],
            "isError": is_error,
        },
    })
}

/// The bus dance: park on the question, poll for the boss's answer.
async fn ask_boss(home: &camino::Utf8Path, run_id: RunId, question: &str) -> (String, bool) {
    // No autostart from the worker side: a worker must never birth
    // daemons. A missing daemon is an in-band tool error.
    let client = match McClient::connect(home).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return (
                "mission-control is not running; the question cannot be delivered".into(),
                true,
            );
        }
        Err(e) => return (format!("bus error: {e}"), true),
    };
    if let Err(e) = client.question(run_id, question).await {
        return (format!("question refused: {e}"), true);
    }
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        match client.run(run_id).await {
            Ok(r) => {
                if let Some(answer) = r.answer {
                    return (answer, false);
                }
                if r.state.is_terminal() {
                    return (
                        format!("run went {} while waiting for the answer", r.state),
                        true,
                    );
                }
            }
            Err(e) => {
                // Transient bus trouble (daemon restart): keep polling —
                // the run record survives generations. stderr reaches
                // Claude Code's MCP server log.
                eprintln!("fractality mcp-broker: answer poll failed, retrying: {e}");
            }
        }
    }
}

/// Records the escalation on the bus (D-C3-6): the run ends `escalated`,
/// terminal. Unlike ask_boss there is nothing to wait for — the task is
/// handed up. No autostart from the worker side (a worker never births
/// daemons); a missing daemon is an in-band tool error.
async fn escalate(
    home: &camino::Utf8Path,
    run_id: RunId,
    reason: &str,
    needs: &str,
) -> (String, bool) {
    let client = match McClient::connect(home).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return (
                "mission-control is not running; the escalation cannot be delivered".into(),
                true,
            );
        }
        Err(e) => return (format!("bus error: {e}"), true),
    };
    match client.escalate(run_id, reason, needs).await {
        Ok(_) => (
            "Escalated: your task has been handed UP the tree to the boss. This run is now \
             finished — stop working and end your turn."
                .into(),
            false,
        ),
        Err(e) => (format!("escalation refused: {e}"), true),
    }
}

/// The serve loop: newline-delimited JSON-RPC over stdio.
pub(crate) async fn serve(home: &camino::Utf8Path) -> u8 {
    let run_id: RunId = match std::env::var("FRACTALITY_RUN_ID")
        .ok()
        .and_then(|v| v.parse().ok())
    {
        Some(id) => id,
        None => {
            eprintln!("fractality: mcp-broker needs FRACTALITY_RUN_ID (pod-injected)");
            return 2;
        }
    };
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    let mut stdout = tokio::io::stdout();
    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }
        let reply = match route(&line) {
            Route::Silent => continue,
            Route::Reply(v) => v,
            Route::AskBoss { id, question } => {
                let (text, is_error) = ask_boss(home, run_id, &question).await;
                tool_reply(id, text, is_error)
            }
            Route::Escalate { id, reason, needs } => {
                let (text, is_error) = escalate(home, run_id, &reason, &needs).await;
                tool_reply(id, text, is_error)
            }
        };
        let mut payload = reply.to_string();
        payload.push('\n');
        if stdout.write_all(payload.as_bytes()).await.is_err() {
            break; // The worker is gone; so is our reason to exist.
        }
        let _ = stdout.flush().await;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// initialize echoes the client's protocol version and names the
    /// one capability that exists (tools).
    #[test]
    fn initialize_reply_carries_version_and_tools() {
        let r = route(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-01-01"}}"#,
        );
        let Route::Reply(v) = r else {
            panic!("initialize must reply");
        };
        assert_eq!(
            v.pointer("/result/protocolVersion").and_then(Value::as_str),
            Some("2025-01-01")
        );
        assert!(v.pointer("/result/capabilities/tools").is_some());
    }

    /// tools/list serves ask_boss then escalate, each with its required arg.
    #[test]
    fn tools_list_serves_ask_boss_and_escalate() {
        let Route::Reply(v) = route(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#) else {
            panic!("tools/list must reply");
        };
        assert_eq!(
            v.pointer("/result/tools/0/name").and_then(Value::as_str),
            Some("ask_boss")
        );
        assert_eq!(
            v.pointer("/result/tools/0/inputSchema/required/0")
                .and_then(Value::as_str),
            Some("question")
        );
        assert_eq!(
            v.pointer("/result/tools/1/name").and_then(Value::as_str),
            Some("escalate")
        );
        assert_eq!(
            v.pointer("/result/tools/1/inputSchema/required/0")
                .and_then(Value::as_str),
            Some("reason")
        );
        assert!(v.pointer("/result/tools/2").is_none(), "exactly two tools");
    }

    /// A valid ask_boss call routes to the bus dance with its question.
    #[test]
    fn ask_boss_call_routes_with_the_question() {
        let r = route(
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"ask_boss","arguments":{"question":"which color?"}}}"#,
        );
        assert_eq!(
            r,
            Route::AskBoss {
                id: json!(3),
                question: "which color?".into()
            }
        );
    }

    /// A valid escalate call routes to the bus with reason + needs.
    #[test]
    fn escalate_call_routes_with_reason_and_needs() {
        let r = route(
            r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"escalate","arguments":{"reason":"silo task","needs":"bigger window"}}}"#,
        );
        assert_eq!(
            r,
            Route::Escalate {
                id: json!(7),
                reason: "silo task".into(),
                needs: "bigger window".into(),
            }
        );
    }

    /// escalate requires a non-empty reason; `needs` defaults to empty.
    #[test]
    fn escalate_requires_reason_and_defaults_needs() {
        // Missing reason → invalid-params error, no routing.
        let Route::Reply(v) = route(
            r#"{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"escalate","arguments":{"needs":"x"}}}"#,
        ) else {
            panic!("missing reason must reply an error");
        };
        assert_eq!(
            v.pointer("/error/code").and_then(Value::as_i64),
            Some(-32602)
        );
        // Reason present, needs absent → routes with empty needs.
        let r = route(
            r#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"escalate","arguments":{"reason":"cannot do it here"}}}"#,
        );
        assert_eq!(
            r,
            Route::Escalate {
                id: json!(9),
                reason: "cannot do it here".into(),
                needs: String::new(),
            }
        );
    }

    /// Unknown tools, empty questions, unknown methods, and notifications
    /// all get the documented wire behavior.
    #[test]
    fn protocol_edges_answer_as_specified() {
        let Route::Reply(v) = route(
            r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"other","arguments":{}}}"#,
        ) else {
            panic!("unknown tool must reply an error");
        };
        assert_eq!(
            v.pointer("/error/code").and_then(Value::as_i64),
            Some(-32602)
        );

        let Route::Reply(v) = route(
            r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"ask_boss","arguments":{"question":"  "}}}"#,
        ) else {
            panic!("blank question must reply an error");
        };
        assert_eq!(
            v.pointer("/error/code").and_then(Value::as_i64),
            Some(-32602)
        );

        let Route::Reply(v) = route(r#"{"jsonrpc":"2.0","id":6,"method":"nope"}"#) else {
            panic!("unknown method must reply an error");
        };
        assert_eq!(
            v.pointer("/error/code").and_then(Value::as_i64),
            Some(-32601)
        );

        assert_eq!(
            route(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#),
            Route::Silent,
            "notifications are silent"
        );
        assert_eq!(route("not json"), Route::Silent, "garbage is silent");
    }
}
