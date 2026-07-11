//! `fractality` — the CLI (plan D13, ergonomics law D17).
//!
//! UNIX grammar: quiet plain-text default, one record per line, stable
//! order (newest last — `tail -1` is the latest run), `--json` on every
//! read verb, semantic exit codes documented per verb:
//!
//! - `0` — success (for `mc status`: running).
//! - `1` — a truthful negative: no such run, ambiguous prefix, daemon
//!   not running (`mc status`).
//! - `2` — infrastructure error: no daemon and it could not be started,
//!   transport failure, unusable home.

mod boss;
mod broker;
mod fetch;
mod gate_cmd;
mod harness;
mod hook;
mod mc_cmd;
mod out;
mod route_cmd;
mod scoreboard;
mod session;
mod statusline;
mod swarm;

use harness::HarnessCmd;
use mc_cmd::McCmd;
use session::SessionCmd;

use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use fractality_core::run::{RunRecord, RunState};
use fractality_mc_client::{ClientError, McClient, connect_or_start};

specmark::scope!("spec://fractality/PROP-001#architecture");

const EXIT_OK: u8 = 0;
const EXIT_NEGATIVE: u8 = 1;
const EXIT_INFRA: u8 = 2;

#[derive(Parser)]
#[command(
    name = "fractality",
    version,
    about = "Delegate tasks to isolated worker agents under mission-control"
)]
struct Cli {
    /// Fractality home (default: FRACTALITY_HOME or ~/.fractality).
    #[arg(long, global = true, value_name = "DIR")]
    home: Option<Utf8PathBuf>,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Manage the mission-control daemon.
    Mc {
        #[command(subcommand)]
        cmd: McCmd,
    },
    /// List runs (newest last; header + one line per run).
    Ps {
        /// Filter by state (queued, starting, running, waiting_on_boss,
        /// completed, failed, killed).
        #[arg(long)]
        state: Option<String>,
        /// Show only the newest N runs.
        #[arg(long, value_name = "N")]
        limit: Option<usize>,
        /// Print run ids only, no header (compose with xargs).
        #[arg(short, long)]
        quiet: bool,
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },
    /// Show one run in full (accepts a unique id prefix).
    Show {
        id: String,
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },
    /// Run a packet synchronously: register, spawn, wait for a terminal
    /// state, print the one-screen summary. Exit: 0 completed, 1 failed
    /// (or invalid packet), 3 killed, 2 infrastructure.
    Run {
        /// Task packet (TOML, plan D7).
        #[arg(long, value_name = "FILE")]
        packet: Utf8PathBuf,
        /// Machine-readable output (the final run record).
        #[arg(long)]
        json: bool,
    },
    /// Register + launch a packet asynchronously; prints the run id and
    /// returns immediately (compose with `wait`). Inside a worker the
    /// parent is taken from FRACTALITY_RUN_ID — the call tree builds
    /// itself. Exit: 0 spawned/queued, 1 invalid packet, 2 infra.
    Spawn {
        /// Task packet (TOML, plan D7).
        #[arg(long, value_name = "FILE")]
        packet: Utf8PathBuf,
        /// Explicit parent run (defaults to $FRACTALITY_RUN_ID).
        #[arg(long, value_name = "RUN_ID")]
        parent: Option<String>,
        /// Machine-readable output (the registered run record).
        #[arg(long)]
        json: bool,
    },
    /// Wait for runs to reach a terminal state (shell `wait` semantics:
    /// blocks on all, exit code mirrors the LAST id's outcome — 0
    /// completed, 1 failed, 3 killed, 2 pod lost/infra). Prints one line
    /// per run as it settles.
    Wait {
        /// Run ids (or unique prefixes).
        #[arg(required = true)]
        ids: Vec<String>,
        /// Give up after this many seconds (0 = wait forever).
        #[arg(long, default_value_t = 0)]
        timeout: u64,
    },
    /// Show the call tree rooted at a run, or the whole forest.
    Tree {
        /// Root run id (or unique prefix); omit for all roots.
        id: Option<String>,
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },
    /// Kill a run (optionally its whole subtree). Exit: 0 killed, 1 the
    /// run was already terminal, 2 infra.
    Kill {
        /// Run id (or unique prefix).
        id: String,
        /// Kill the whole call tree rooted here.
        #[arg(long)]
        tree: bool,
    },
    /// List runs parked on a question (D18). Exit: 0 (even when empty).
    Questions {
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },
    /// Answer a parked run; it resumes with the text as its tool result.
    /// Exit: 0 answered, 1 the run is not waiting, 2 infra.
    Answer {
        /// Run id (or unique prefix).
        id: String,
        /// The answer text (or use --file).
        #[arg(conflicts_with = "file")]
        text: Option<String>,
        /// Read the answer from a file.
        #[arg(long, value_name = "FILE")]
        file: Option<Utf8PathBuf>,
    },
    /// The mission-control scoreboard (D16): outcomes, tokens, cost,
    /// wall time — totals and by profile/model/day.
    Stats {
        /// Machine-readable output (the raw metrics document).
        #[arg(long)]
        json: bool,
    },
    /// Boss sessions (Campaign 2): register, close, and inspect the
    /// attribution unit the scoreboard aggregates by.
    Session {
        #[command(subcommand)]
        cmd: SessionCmd,
    },
    /// The initiative scoreboard (Campaign 2): session facts, parked
    /// questions, today/month lines — strictly measured, never
    /// invented. Session pick: --session, else FRACTALITY_BOSS_SESSION,
    /// else global only.
    Scoreboard {
        /// Session id (or unique prefix) to show.
        #[arg(long, value_name = "ID")]
        session: Option<String>,
        /// One-line strip (statusline shape).
        #[arg(long, conflicts_with = "json")]
        line: bool,
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },
    /// Run the need-gate decision procedure (D-C3-8) on a task's signals
    /// and a candidate worker's capability class; print the verdict —
    /// inline | route | fold-local | spawn | escalate — and its reason.
    /// Pure calculus, no daemon. Exit: 0 decided, 2 bad class.
    Gate {
        /// Candidate worker capability class: weak | medium | strong.
        #[arg(long, value_name = "CLASS")]
        class: String,
        /// The caller's current nesting depth.
        #[arg(long, default_value_t = 0)]
        depth: u32,
        /// The task is an O(1) lookup / single fact.
        #[arg(long)]
        o1_lookup: bool,
        /// The (O(1)) task needs a tool the boss lacks.
        #[arg(long)]
        needs_absent_tool: bool,
        /// Task + context fit the worker window with >=30% margin.
        #[arg(long)]
        fits_window: bool,
        /// The task draws on a single skill.
        #[arg(long)]
        single_skill: bool,
        /// Cross-chunk dependence dominates (Silo regime).
        #[arg(long)]
        cross_chunk_dominant: bool,
        /// A largest-window profile is available for a Silo route.
        #[arg(long)]
        large_window_available: bool,
        /// Decomposable into composable sub-results.
        #[arg(long)]
        decomposable: bool,
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },
    /// Score a task on the delegation matrix's four axes and print the
    /// verdict (the §verdict procedure as data). Exit: 0 delegate,
    /// 1 keep, 2 bad axes.
    Route {
        /// reversible | irreversible
        #[arg(long, value_name = "V")]
        error_cost: String,
        /// compilable | boot-loadable | untransferable
        #[arg(long, value_name = "V")]
        context: String,
        /// mechanical | judgment
        #[arg(long, value_name = "V")]
        verify: String,
        /// S | M | L
        #[arg(long, value_name = "V")]
        size: String,
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },
    /// Download a document once, locally (D12 tariff hygiene: workers
    /// have web tools denied; the boss fetches, the corpus is shared).
    Fetch {
        /// http(s) URL to download.
        url: String,
        /// Destination file path.
        #[arg(long, value_name = "FILE")]
        out: Utf8PathBuf,
        /// Overwrite an existing file.
        #[arg(long)]
        force: bool,
    },
    /// Install, inspect, or remove the harness integration (hooks +
    /// statusline entries owned by their command string).
    Harness {
        #[command(subcommand)]
        cmd: HarnessCmd,
    },
    /// A harness hook target (plumbing: Claude Code invokes this with
    /// the event's JSON on stdin; never errors, per the availability
    /// law).
    #[command(hide = true)]
    Hook {
        /// Event verb: session-start | user-prompt-submit |
        /// post-tool-use | stop | session-end.
        event: String,
    },
    /// The statusline command (plumbing: Claude Code pipes session
    /// JSON on stdin; prints one line).
    #[command(hide = true)]
    Statusline,
    /// The ask_boss MCP stdio server (plumbing: Claude Code launches
    /// this inside workers; not for human use).
    #[command(hide = true)]
    McpBroker,
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    let home = match fractality_mc_client::home::resolve(cli.home.as_deref()) {
        Ok(h) => h,
        Err(e) => return fail(EXIT_INFRA, &e),
    };
    let code = match cli.cmd {
        Cmd::Mc { cmd } => match cmd {
            McCmd::Start => mc_cmd::mc_start(&home).await,
            McCmd::Stop => mc_cmd::mc_stop(&home).await,
            McCmd::Status { json } => mc_cmd::mc_status(&home, json).await,
        },
        Cmd::Ps {
            state,
            limit,
            quiet,
            json,
        } => ps(&home, state, limit, quiet, json).await,
        Cmd::Show { id, json } => show(&home, &id, json).await,
        Cmd::Run { packet, json } => run_packet(&home, &packet, json).await,
        Cmd::Spawn {
            packet,
            parent,
            json,
        } => swarm::spawn(&home, &packet, parent.as_deref(), json).await,
        Cmd::Wait { ids, timeout } => swarm::wait(&home, &ids, timeout).await,
        Cmd::Tree { id, json } => swarm::tree(&home, id.as_deref(), json).await,
        Cmd::Kill { id, tree } => swarm::kill(&home, &id, tree).await,
        Cmd::Questions { json } => boss::questions(&home, json).await,
        Cmd::Answer { id, text, file } => {
            boss::answer(&home, &id, text.as_deref(), file.as_deref()).await
        }
        Cmd::Stats { json } => boss::stats(&home, json).await,
        Cmd::Scoreboard {
            session,
            line,
            json,
        } => scoreboard::scoreboard(&home, session.as_deref(), line, json).await,
        Cmd::Session { cmd } => match cmd {
            SessionCmd::Begin {
                harness,
                external_id,
                cwd,
                json,
            } => session::begin(&home, &harness, &external_id, cwd.as_deref(), json).await,
            SessionCmd::End { id } => session::end(&home, &id).await,
            SessionCmd::Show { id, json } => session::show(&home, &id, json).await,
            SessionCmd::Ls { open, json } => session::ls(&home, open, json).await,
        },
        Cmd::Gate {
            class,
            depth,
            o1_lookup,
            needs_absent_tool,
            fits_window,
            single_skill,
            cross_chunk_dominant,
            large_window_available,
            decomposable,
            json,
        } => gate_cmd::gate(
            &class,
            depth,
            o1_lookup,
            needs_absent_tool,
            fits_window,
            single_skill,
            cross_chunk_dominant,
            large_window_available,
            decomposable,
            json,
        ),
        Cmd::Route {
            error_cost,
            context,
            verify,
            size,
            json,
        } => route_cmd::route(&error_cost, &context, &verify, &size, json),
        Cmd::Fetch { url, out, force } => fetch::fetch(&url, &out, force).await,
        Cmd::Harness { cmd } => match cmd {
            HarnessCmd::Install {
                harness,
                project,
                target,
            } => harness_dispatch(&harness, || harness::install(target.as_deref(), project)),
            HarnessCmd::Status {
                harness,
                project,
                target,
            } => harness_dispatch(&harness, || harness::status(target.as_deref(), project)),
            HarnessCmd::Remove {
                harness,
                project,
                target,
            } => harness_dispatch(&harness, || harness::remove(target.as_deref(), project)),
        },
        Cmd::Hook { event } => hook::hook(&home, &event).await,
        Cmd::Statusline => statusline::statusline(&home).await,
        Cmd::McpBroker => broker::serve(&home).await,
    };
    std::process::ExitCode::from(code)
}

/// Only one harness exists today; naming it keeps the CLI grammar
/// stable for the day a second adapter lands (I4).
fn harness_dispatch(name: &str, run: impl FnOnce() -> u8) -> u8 {
    if name == "claude-code" {
        run()
    } else {
        fail_code(
            EXIT_NEGATIVE,
            &format!("unknown harness `{name}` (only `claude-code` is supported today)"),
        )
    }
}

/// `fractality run --packet <file>`: the sync delegation loop (D13).
async fn run_packet(home: &camino::Utf8Path, packet_path: &camino::Utf8Path, json: bool) -> u8 {
    let text = match std::fs::read_to_string(packet_path.as_std_path()) {
        Ok(t) => t,
        Err(e) => return fail_code(EXIT_NEGATIVE, &format!("reading `{packet_path}`: {e}")),
    };
    let packet = match fractality_core::Packet::from_toml_str(&text) {
        Ok(p) => p,
        Err(e) => return fail_code(EXIT_NEGATIVE, &e.to_string()),
    };
    // Client-side wait cap: the packet's wall budget plus grace. Budget
    // enforcement (the kill) is Phase 4; until then an overrun stops the
    // WAIT loudly, never silently.
    let wait_cap = std::time::Duration::from_secs(packet.budget.wall_secs + 60);

    let client = match connect_or_start(home).await {
        Ok(c) => c,
        Err(e) => return fail_code(err_code(&e), &e.to_string()),
    };
    let parent = match swarm::resolve_parent(&client, None).await {
        Ok(p) => p,
        Err((code, message)) => return fail_code(code, &message),
    };
    let started = std::time::Instant::now();
    let run = match client
        .register_run(&fractality_core::api::RegisterRunRequest {
            packet,
            parent,
            spawn: true,
            origin_session: swarm::origin_session_from_env(),
        })
        .await
    {
        Ok(r) => r,
        Err(e) => return fail_code(err_code(&e), &e.to_string()),
    };
    eprintln!("run {} spawned (dir {})", run.run_id, run.run_dir);

    let mut parked_notice = false;
    let final_run = loop {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        match client.run(run.run_id).await {
            Ok(r) if r.state.is_terminal() => break r,
            Ok(r) if r.state == RunState::WaitingOnBoss => {
                if !parked_notice {
                    parked_notice = true;
                    eprintln!(
                        "run {} PARKED on a question: {}\n  answer with: fractality answer {} \"<text>\"",
                        r.run_id,
                        r.question.as_deref().unwrap_or("-"),
                        r.run_id,
                    );
                }
                if started.elapsed() > wait_cap {
                    // D17 exit family 4: parked past its wait — the run
                    // stays alive for a later answer; this loop stops.
                    return fail_code(
                        4,
                        &format!(
                            "run {} is still parked on its question past the wall budget; \
                             it keeps waiting — `fractality questions` to triage",
                            run.run_id
                        ),
                    );
                }
            }
            Ok(_) if started.elapsed() > wait_cap => {
                return fail_code(
                    EXIT_INFRA,
                    &format!(
                        "run {} outlived its wall budget plus grace and is still not \
                         terminal — the mission-control watchdog should have killed it; \
                         inspect with `fractality show {}` and `mc.log`",
                        run.run_id, run.run_id
                    ),
                );
            }
            Ok(_) => continue,
            Err(e) => return fail_code(err_code(&e), &e.to_string()),
        }
    };
    if parked_notice {
        eprintln!("run {} resumed and finished", run.run_id);
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&final_run)
                .unwrap_or_else(|e| format!("{{\"error\":\"json: {e}\"}}"))
        );
    } else {
        out::print_run_summary(&final_run, started.elapsed());
    }
    swarm::state_code(&final_run)
}

fn fail(code: u8, message: &str) -> std::process::ExitCode {
    eprintln!("fractality: {message}");
    std::process::ExitCode::from(code)
}

fn err_code(e: &ClientError) -> u8 {
    match e {
        ClientError::NotRunning { .. } => EXIT_NEGATIVE,
        _ => EXIT_INFRA,
    }
}

async fn ps(
    home: &camino::Utf8Path,
    state: Option<String>,
    limit: Option<usize>,
    quiet: bool,
    json: bool,
) -> u8 {
    let state_filter = match state.as_deref().map(str::parse::<RunState>).transpose() {
        Ok(s) => s,
        Err(e) => return fail_code(EXIT_NEGATIVE, &e),
    };
    let client = match connect_or_start(home).await {
        Ok(c) => c,
        Err(e) => return fail_code(err_code(&e), &e.to_string()),
    };
    match client.runs(state_filter, limit).await {
        Ok(runs) => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&runs).expect("runs serialize")
                );
            } else {
                out::print_runs(&runs, quiet);
            }
            EXIT_OK
        }
        Err(e) => fail_code(err_code(&e), &e.to_string()),
    }
}

async fn show(home: &camino::Utf8Path, raw_id: &str, json: bool) -> u8 {
    let client = match connect_or_start(home).await {
        Ok(c) => c,
        Err(e) => return fail_code(err_code(&e), &e.to_string()),
    };
    let record = match resolve_run(&client, raw_id).await {
        Ok(r) => r,
        Err((code, message)) => return fail_code(code, &message),
    };
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&record).expect("run serializes")
        );
    } else {
        out::print_run_detail(&record);
    }
    EXIT_OK
}

/// Exact ULID or unique prefix (git muscle memory).
async fn resolve_run(client: &McClient, raw: &str) -> Result<RunRecord, (u8, String)> {
    if let Ok(id) = raw.parse() {
        return match client.run(id).await {
            Ok(r) => Ok(r),
            Err(ClientError::Api { status: 404, .. }) => {
                Err((EXIT_NEGATIVE, format!("no run {id}")))
            }
            Err(e) => Err((err_code(&e), e.to_string())),
        };
    }
    let runs = client
        .runs(None, None)
        .await
        .map_err(|e| (err_code(&e), e.to_string()))?;
    let matches: Vec<&RunRecord> = runs
        .iter()
        .filter(|r| r.run_id.matches_prefix(raw))
        .collect();
    match matches.as_slice() {
        [one] => Ok((*one).clone()),
        [] => Err((EXIT_NEGATIVE, format!("no run matches `{raw}`"))),
        many => Err((
            EXIT_NEGATIVE,
            format!(
                "`{raw}` is ambiguous ({} runs): {}",
                many.len(),
                many.iter()
                    .map(|r| r.run_id.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )),
    }
}

fn fail_code(code: u8, message: &str) -> u8 {
    eprintln!("fractality: {message}");
    code
}
