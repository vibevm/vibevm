//! The Claude Code hook adapter (Campaign 2 D4/D5): `fractality hook
//! <event>` reads the hook's stdin JSON, talks to mission-control, and
//! emits the event's hook JSON on stdout.
//!
//! **The availability law (D4, constant):** a broken initiative system
//! must never break a boss session. Every failure — unparseable stdin,
//! MC down, a refused call — exits 0 with empty output. The engine
//! stays pure; this cell is the one place that knows Claude Code's
//! wire shapes (I4: harness specifics live at the edge).
//!
//! This file is a recorded env-composition root (conform env_roots):
//! `CLAUDE_ENV_FILE` is the harness's own seam for persisting the
//! session export into later Bash calls — the environment IS the
//! protocol here, exactly like `FRACTALITY_RUN_ID` in swarm.rs.

use fractality_core::api::SessionBeginRequest;
use fractality_core::session::{BOSS_SESSION_ENV, SessionNote};
use fractality_mc_client::McClient;
use serde::Deserialize;

use crate::EXIT_OK;

specmark::scope!("spec://fractality/PROP-001#sessions");

/// The work-shaped tool set (barkain study BD1: reads never count).
/// The installed matcher already filters to these; the guard here is
/// defense against a hand-edited matcher going wider.
const WORK_TOOLS: &[&str] = &["Bash", "Edit", "Write", "MultiEdit", "NotebookEdit"];

/// The fields this adapter reads from any hook event's stdin (probed
/// live on CC 2.1.202, plan Ф0.s1; unknown fields are ignored by
/// design — R1 tolerance).
#[derive(Debug, Deserialize)]
struct HookInput {
    session_id: String,
    cwd: String,
    #[serde(default)]
    tool_name: Option<String>,
    /// PostToolUse serves the tool's wall time (finding F21).
    #[serde(default)]
    duration_ms: Option<u64>,
    /// True when this Stop already continues from a prior stop hook —
    /// the loop guard the harness documents; we never alert into it.
    #[serde(default)]
    stop_hook_active: bool,
}

/// Entry point for every `fractality hook <event>` verb. Never fails:
/// see the availability law above.
pub(crate) async fn hook(home: &camino::Utf8Path, event: &str) -> u8 {
    // The kill switch (D6): one env var silences the whole initiative
    // layer without touching any settings file.
    if matches!(
        std::env::var("FRACTALITY_INITIATIVE").as_deref(),
        Ok("off") | Ok("0") | Ok("false")
    ) {
        return EXIT_OK;
    }
    let mut raw = String::new();
    if std::io::Read::read_to_string(&mut std::io::stdin(), &mut raw).is_err() {
        return EXIT_OK;
    }
    let Ok(input) = serde_json::from_str::<HookInput>(&raw) else {
        return EXIT_OK;
    };
    // Connect only — the adapter must never boot a daemon under a
    // session that did not opt in by starting one (and must add no
    // spawn latency to hooks). No daemon → silent no-op.
    let Ok(Some(client)) = McClient::connect(home).await else {
        return EXIT_OK;
    };
    // `session begin` doubles as the lookup: idempotent per
    // (harness, external_id), so every event resolves the same record
    // (hook processes do not see the Bash-side env export).
    let Ok(begun) = client
        .session_begin(&SessionBeginRequest {
            harness: "claude-code".to_owned(),
            external_id: input.session_id.clone(),
            cwd: input.cwd.clone().into(),
        })
        .await
    else {
        return EXIT_OK;
    };
    let sid = begun.session.session_id;

    match event {
        "session-start" => {
            export_session_env(&sid.to_string());
            let board = match (client.metrics().await, client.session_metrics(sid).await) {
                (Ok(global), Ok(session)) => {
                    let now = fractality_core::time::now_ms();
                    let today = fractality_core::time::utc_date_string(now);
                    let month = today[..7].to_owned();
                    // PP-002: dated proof workers self-verify here, best-effort
                    // (a fetch miss just omits the line — the hook never errors).
                    let credibility = client
                        .runs(None, None)
                        .await
                        .ok()
                        .and_then(|runs| fractality_core::worker_credibility(&runs));
                    fractality_initiative::render_board(
                        &global,
                        Some(&session),
                        credibility.as_ref(),
                        now,
                        &today,
                        &month,
                    )
                }
                _ => return EXIT_OK,
            };
            let out = serde_json::json!({
                "hookSpecificOutput": {
                    "hookEventName": "SessionStart",
                    "additionalContext": format!(
                        "fractality scoreboard (live facts from mission-control):\n{board}"
                    ),
                }
            });
            println!("{out}");
        }
        "post-tool-use" => {
            if let Some(tool) = &input.tool_name
                && WORK_TOOLS.contains(&tool.as_str())
            {
                let _ = client
                    .session_note(
                        sid,
                        SessionNote::WorkTool {
                            tool: tool.clone(),
                            duration_ms: input.duration_ms.unwrap_or(0),
                        },
                    )
                    .await;
                // The mid-work channel (DEF-C2-1, F23): single-prompt
                // sessions never re-enter UserPromptSubmit, so the
                // threshold nudge rides PostToolUse additionalContext.
                // The WorkTool note above is folded before this read,
                // so the slate the decision sees is current. The
                // config pre-check keeps the off-path at one MC call.
                let cfg = load_config(home);
                if cfg.midwork_nudges
                    && let Ok(session) = client.session_metrics(sid).await
                {
                    let now = fractality_core::time::now_ms();
                    if let Some(nudge) =
                        fractality_initiative::decide_midwork_nudge(&cfg, &session, now)
                    {
                        let out = serde_json::json!({
                            "hookSpecificOutput": {
                                "hookEventName": "PostToolUse",
                                "additionalContext": nudge.text,
                            }
                        });
                        println!("{out}");
                        let _ = client
                            .session_note(
                                sid,
                                SessionNote::NudgeSent {
                                    reason: nudge.reason,
                                },
                            )
                            .await;
                    }
                }
            }
        }
        "session-end" => {
            let _ = client.session_end(sid).await;
        }
        "user-prompt-submit" => {
            let cfg = load_config(home);
            let Ok(session) = client.session_metrics(sid).await else {
                return EXIT_OK;
            };
            let now = fractality_core::time::now_ms();
            if let Some(nudge) = fractality_initiative::decide_prompt_nudge(&cfg, &session, now) {
                let out = serde_json::json!({
                    "hookSpecificOutput": {
                        "hookEventName": "UserPromptSubmit",
                        "additionalContext": nudge.text,
                    }
                });
                println!("{out}");
                let _ = client
                    .session_note(
                        sid,
                        SessionNote::NudgeSent {
                            reason: nudge.reason,
                        },
                    )
                    .await;
            }
        }
        "stop" => {
            if input.stop_hook_active {
                return EXIT_OK;
            }
            let cfg = load_config(home);
            let Ok(session) = client.session_metrics(sid).await else {
                return EXIT_OK;
            };
            if let Some((run_id, text)) = fractality_initiative::decide_stop_alert(&cfg, &session) {
                // Stop-time additionalContext is continue-feedback by
                // the harness's contract: the turn keeps going exactly
                // once per unanswered question (the ack below dedupes).
                let out = serde_json::json!({
                    "hookSpecificOutput": {
                        "hookEventName": "Stop",
                        "additionalContext": text,
                    }
                });
                println!("{out}");
                let _ = client
                    .session_note(sid, SessionNote::QuestionAlert { run_id })
                    .await;
            }
        }
        _ => {}
    }
    EXIT_OK
}

/// `<home>/initiative.toml`, absent or malformed → the default posture
/// (availability law: configuration can tune the initiative layer, it
/// can never break a session).
fn load_config(home: &camino::Utf8Path) -> fractality_initiative::NudgeConfig {
    match std::fs::read_to_string(home.join("initiative.toml").as_std_path()) {
        Ok(text) => fractality_initiative::NudgeConfig::from_toml_str(&text),
        Err(_) => fractality_initiative::NudgeConfig::default(),
    }
}

/// Persists the session export for later Bash calls via the harness's
/// own seam. SessionStart-only (CC sets the variable there); append
/// keeps other hooks' exports intact (the harness's documented
/// contract).
fn export_session_env(session_id: &str) {
    let Ok(path) = std::env::var("CLAUDE_ENV_FILE") else {
        return;
    };
    if path.is_empty() {
        return;
    }
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&path)
    {
        let _ = writeln!(f, "export {BOSS_SESSION_ENV}={session_id}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hook_input_tolerates_unknown_fields_and_missing_optionals() {
        let input: HookInput = serde_json::from_str(
            r#"{"session_id":"abc","cwd":"C:/p","hook_event_name":"SessionStart","source":"startup","transcript_path":"t"}"#,
        )
        .expect("parses");
        assert_eq!(input.session_id, "abc");
        assert_eq!(input.tool_name, None);
        assert_eq!(input.duration_ms, None);
    }

    #[test]
    fn post_tool_use_shape_carries_tool_and_duration() {
        let input: HookInput = serde_json::from_str(
            r#"{"session_id":"abc","cwd":"C:/p","tool_name":"Bash","duration_ms":417,"tool_response":{"x":1}}"#,
        )
        .expect("parses");
        assert_eq!(input.tool_name.as_deref(), Some("Bash"));
        assert_eq!(input.duration_ms, Some(417));
    }

    #[test]
    fn the_work_tool_set_excludes_reads() {
        for read_tool in ["Read", "Glob", "Grep", "WebFetch"] {
            assert!(
                !WORK_TOOLS.contains(&read_tool),
                "{read_tool} must not count"
            );
        }
    }
}
