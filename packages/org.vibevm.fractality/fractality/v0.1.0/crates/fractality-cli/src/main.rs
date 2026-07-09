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

mod out;

use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use fractality_core::run::{RunRecord, RunState};
use fractality_mc_client::lock::Lockfile;
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
}

#[derive(Subcommand)]
enum McCmd {
    /// Start the daemon (idempotent; exit 0 when already running).
    Start,
    /// Stop the daemon gracefully (idempotent; exit 0 when not running).
    Stop,
    /// Report daemon state. Exit 0 running, 1 stopped, 2 error.
    Status {
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },
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
            McCmd::Start => mc_start(&home).await,
            McCmd::Stop => mc_stop(&home).await,
            McCmd::Status { json } => mc_status(&home, json).await,
        },
        Cmd::Ps {
            state,
            limit,
            quiet,
            json,
        } => ps(&home, state, limit, quiet, json).await,
        Cmd::Show { id, json } => show(&home, &id, json).await,
    };
    std::process::ExitCode::from(code)
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

async fn mc_start(home: &camino::Utf8Path) -> u8 {
    match connect_or_start(home).await {
        Ok(client) => match (client.health().await, Lockfile::read(home)) {
            (Ok(health), Ok(Some(lock))) => {
                println!(
                    "mc running pid={} port={} uptime={} runs_open={}",
                    health.pid,
                    lock.port,
                    fractality_core::time::format_duration_ms(
                        fractality_core::time::now_ms().saturating_sub(health.started_ts_ms)
                    ),
                    health.runs_open,
                );
                EXIT_OK
            }
            (Err(e), _) => {
                eprintln!("fractality: daemon started but health failed: {e}");
                EXIT_INFRA
            }
            (_, lock) => {
                eprintln!("fractality: daemon healthy but lockfile unreadable: {lock:?}");
                EXIT_INFRA
            }
        },
        Err(e) => {
            eprintln!("fractality: {e}");
            EXIT_INFRA
        }
    }
}

async fn mc_stop(home: &camino::Utf8Path) -> u8 {
    match McClient::connect(home).await {
        Ok(None) => {
            println!("mc is not running");
            EXIT_OK
        }
        Ok(Some(client)) => {
            if let Err(e) = client.shutdown().await {
                eprintln!("fractality: shutdown call failed: {e}");
                return EXIT_INFRA;
            }
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                match McClient::connect(home).await {
                    Ok(None) => {
                        println!("mc stopped");
                        return EXIT_OK;
                    }
                    Ok(Some(_)) if std::time::Instant::now() < deadline => continue,
                    Ok(Some(_)) => {
                        eprintln!("fractality: daemon still answering after 10s");
                        return EXIT_INFRA;
                    }
                    Err(e) => {
                        eprintln!("fractality: {e}");
                        return EXIT_INFRA;
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("fractality: {e}");
            EXIT_INFRA
        }
    }
}

async fn mc_status(home: &camino::Utf8Path, json: bool) -> u8 {
    match McClient::connect(home).await {
        Ok(Some(client)) => match client.health().await {
            Ok(health) => {
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&health).expect("health serializes")
                    );
                } else {
                    let port = Lockfile::read(home)
                        .ok()
                        .flatten()
                        .map(|l| l.port.to_string())
                        .unwrap_or_else(|| "?".to_owned());
                    println!(
                        "running pid={} port={} uptime={} runs={}/{}",
                        health.pid,
                        port,
                        fractality_core::time::format_duration_ms(
                            fractality_core::time::now_ms().saturating_sub(health.started_ts_ms)
                        ),
                        health.runs_open,
                        health.runs_total,
                    );
                }
                EXIT_OK
            }
            Err(e) => {
                eprintln!("fractality: {e}");
                EXIT_INFRA
            }
        },
        Ok(None) => {
            if json {
                println!("{{\"status\":\"stopped\"}}");
            } else {
                println!("stopped");
            }
            EXIT_NEGATIVE
        }
        Err(e) => {
            eprintln!("fractality: {e}");
            EXIT_INFRA
        }
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
