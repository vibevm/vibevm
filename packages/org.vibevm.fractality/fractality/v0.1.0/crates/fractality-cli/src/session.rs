//! The boss-session verbs (Campaign 2 D2/D3): `session begin|end|show|ls`.
//!
//! `begin` prints exactly one id on stdout so the adapter (and shell
//! muscle memory) can compose: the Claude Code SessionStart hook runs
//! it and exports the id as `FRACTALITY_BOSS_SESSION` for the rest of
//! the session. Everything else is D17 read-verb grammar.

use camino::Utf8PathBuf;
use clap::Subcommand;
use fractality_core::SessionRecord;
use fractality_core::ids::SessionId;
use fractality_mc_client::{McClient, connect_or_start};

use crate::{EXIT_NEGATIVE, EXIT_OK, err_code, fail_code};

specmark::scope!("spec://fractality/PROP-001#architecture");

/// The `fractality session <verb>` grammar (lives with its cell).
#[derive(Subcommand)]
pub(crate) enum SessionCmd {
    /// Begin (or resume) a session; prints the session id on stdout —
    /// compose: `export FRACTALITY_BOSS_SESSION=$(fractality session
    /// begin --harness claude-code --external-id <uuid>)`.
    Begin {
        /// Harness label, e.g. `claude-code`.
        #[arg(long)]
        harness: String,
        /// The harness's own session identifier.
        #[arg(long, value_name = "ID")]
        external_id: String,
        /// Session working directory (defaults to the current one).
        #[arg(long, value_name = "DIR")]
        cwd: Option<Utf8PathBuf>,
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },
    /// Mark a session ended (idempotent).
    End {
        /// Session id (or unique prefix).
        id: String,
    },
    /// Show one session: record, counters, runs bucket, parked questions.
    Show {
        /// Session id (or unique prefix).
        id: String,
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },
    /// List sessions, newest last.
    Ls {
        /// Only sessions still open.
        #[arg(long)]
        open: bool,
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },
}

/// `fractality session begin --harness <h> --external-id <id>`.
pub(crate) async fn begin(
    home: &camino::Utf8Path,
    harness: &str,
    external_id: &str,
    cwd: Option<&camino::Utf8Path>,
    json: bool,
) -> u8 {
    let cwd = match cwd {
        Some(c) => c.to_owned(),
        None => match std::env::current_dir()
            .map_err(|e| e.to_string())
            .and_then(|p| camino::Utf8PathBuf::from_path_buf(p).map_err(|p| format!("{p:?}")))
        {
            Ok(c) => c,
            Err(e) => return fail_code(EXIT_NEGATIVE, &format!("resolving cwd: {e}")),
        },
    };
    let client = match connect_or_start(home).await {
        Ok(c) => c,
        Err(e) => return fail_code(err_code(&e), &e.to_string()),
    };
    match client
        .session_begin(&fractality_core::api::SessionBeginRequest {
            harness: harness.to_owned(),
            external_id: external_id.to_owned(),
            cwd,
        })
        .await
    {
        Ok(resp) => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&resp)
                        .unwrap_or_else(|e| format!("{{\"error\":\"json: {e}\"}}"))
                );
            } else {
                // One id on stdout: `export FRACTALITY_BOSS_SESSION=$(…)`.
                println!("{}", resp.session.session_id);
                eprintln!(
                    "session {} ({})",
                    resp.session.session_id,
                    if resp.resumed { "resumed" } else { "new" }
                );
            }
            EXIT_OK
        }
        Err(e) => fail_code(err_code(&e), &e.to_string()),
    }
}

/// `fractality session end <id>` — idempotent close.
pub(crate) async fn end(home: &camino::Utf8Path, raw_id: &str) -> u8 {
    let client = match connect_or_start(home).await {
        Ok(c) => c,
        Err(e) => return fail_code(err_code(&e), &e.to_string()),
    };
    let session = match resolve_session(&client, raw_id).await {
        Ok(s) => s,
        Err((code, message)) => return fail_code(code, &message),
    };
    match client.session_end(session.session_id).await {
        Ok(s) => {
            println!("{} ended", s.session_id);
            EXIT_OK
        }
        Err(e) => fail_code(err_code(&e), &e.to_string()),
    }
}

/// `fractality session show <id>` — the record + its runs bucket +
/// parked questions (the scoreboard facts, raw form).
pub(crate) async fn show(home: &camino::Utf8Path, raw_id: &str, json: bool) -> u8 {
    let client = match connect_or_start(home).await {
        Ok(c) => c,
        Err(e) => return fail_code(err_code(&e), &e.to_string()),
    };
    let session = match resolve_session(&client, raw_id).await {
        Ok(s) => s,
        Err((code, message)) => return fail_code(code, &message),
    };
    match client.session_metrics(session.session_id).await {
        Ok(m) => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&m).expect("session metrics serialize")
                );
            } else {
                let s = &m.session;
                println!(
                    "session {} harness={} external={} state={} cwd={}",
                    s.session_id,
                    s.harness,
                    s.external_id,
                    if s.is_open() { "open" } else { "ended" },
                    s.cwd,
                );
                let c = &s.counters;
                println!(
                    "counters delegations={} work_tools={}/{} work_ms={} nudges={} alerts={}",
                    c.delegations,
                    c.work_tools_since_delegation,
                    c.work_tools_total,
                    c.work_tool_ms_total,
                    c.nudges_sent,
                    c.question_alerts,
                );
                println!(
                    "runs total={} completed={} failed={} killed={} open={} out_tokens={}",
                    m.runs.runs,
                    m.runs.completed,
                    m.runs.failed,
                    m.runs.killed,
                    m.runs.open,
                    m.runs.output_tokens,
                );
                for p in &m.parked {
                    println!(
                        "parked {} {} {}",
                        p.run_id,
                        fractality_core::time::format_duration_ms(p.waiting_ms),
                        p.question.lines().next().unwrap_or(""),
                    );
                }
            }
            EXIT_OK
        }
        Err(e) => fail_code(err_code(&e), &e.to_string()),
    }
}

/// `fractality session ls [--open]` — one line per session, newest last.
pub(crate) async fn ls(home: &camino::Utf8Path, open_only: bool, json: bool) -> u8 {
    let client = match connect_or_start(home).await {
        Ok(c) => c,
        Err(e) => return fail_code(err_code(&e), &e.to_string()),
    };
    match client.sessions(open_only).await {
        Ok(sessions) => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&sessions).expect("sessions serialize")
                );
            } else {
                for s in &sessions {
                    println!(
                        "{} {} {} deleg={} slate={} {}",
                        s.session_id,
                        if s.is_open() { "open " } else { "ended" },
                        s.harness,
                        s.counters.delegations,
                        s.counters.work_tools_since_delegation,
                        s.cwd,
                    );
                }
            }
            EXIT_OK
        }
        Err(e) => fail_code(err_code(&e), &e.to_string()),
    }
}

/// Exact ULID or unique prefix, the run-resolution muscle memory.
pub(crate) async fn resolve_session(
    client: &McClient,
    raw: &str,
) -> Result<SessionRecord, (u8, String)> {
    if let Ok(id) = raw.parse::<SessionId>() {
        return match client.session(id).await {
            Ok(s) => Ok(s),
            Err(fractality_mc_client::ClientError::Api { status: 404, .. }) => {
                Err((EXIT_NEGATIVE, format!("no session {id}")))
            }
            Err(e) => Err((err_code(&e), e.to_string())),
        };
    }
    let sessions = client
        .sessions(false)
        .await
        .map_err(|e| (err_code(&e), e.to_string()))?;
    let matches: Vec<&SessionRecord> = sessions
        .iter()
        .filter(|s| s.session_id.matches_prefix(raw))
        .collect();
    match matches.as_slice() {
        [one] => Ok((*one).clone()),
        [] => Err((EXIT_NEGATIVE, format!("no session matches `{raw}`"))),
        many => Err((
            EXIT_NEGATIVE,
            format!(
                "`{raw}` is ambiguous ({} sessions): {}",
                many.len(),
                many.iter()
                    .map(|s| s.session_id.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )),
    }
}
