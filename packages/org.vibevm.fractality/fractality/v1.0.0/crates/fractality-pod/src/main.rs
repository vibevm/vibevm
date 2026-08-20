//! fractality-pod — the per-run worker supervisor (plan D3).
//!
//! One pod per run. The pod spawns the worker from a [`WorkerSpec`],
//! owns what only a local parent can own (stdio → run-dir files, the
//! Job Object / process group, the exit report), and speaks to
//! mission-control over the bus. Discovery is lockfile-first and
//! re-read on every reconnect — a daemon restart strands nobody: the
//! pod keeps supervising, re-registers when the new daemon appears, and
//! delivers its exit report to whichever daemon generation is alive.

mod collect;
mod pump;
mod resolve;
mod supervise;
mod worker_env;

use collect::Collection;
use pump::{feed_stdin, pump, pump_transcript};
use worker_env::{augment_worker_env, sweep_for_orphan};

use camino::Utf8PathBuf;
use clap::Parser;
use fractality_backend_claude_code::ClaudeCodeBackend;
use fractality_core::Packet;
use fractality_core::api::{
    PodCommand, PodEvent, PodEventRequest, PodHeartbeat, PodRegisterRequest,
};
use fractality_core::ids::{PodId, RunId};
use fractality_core::profile::ProfilesFile;
use fractality_core::run::{KillReason, RunState, UsageTotals};
use fractality_core::worker::{BackendSecrets, RunContext, RunSpec, WorkerBackend, WorkerSpec};
use fractality_mc_client::McClient;
use tracing_subscriber::EnvFilter;

specmark::scope!("spec://fractality/PROP-001#architecture");

const STDOUT_FILE: &str = "worker-stdout.jsonl";
const STDERR_FILE: &str = "worker-stderr.log";
/// How long the pod keeps trying to deliver the exit report.
const EXIT_DELIVERY_BUDGET: std::time::Duration = std::time::Duration::from_secs(30);

#[derive(Parser)]
#[command(name = "fractality-pod", version)]
struct Args {
    /// Fractality home (lockfile discovery root).
    #[arg(long)]
    home: Utf8PathBuf,
    /// Product path: a run spec written by mission-control
    /// (fractality-core::worker::RunSpec). The pod resolves the packet,
    /// the profile, and the token itself — secrets never transit specs.
    #[arg(long, conflicts_with_all = ["run_id", "run_dir", "spec"])]
    run_spec: Option<Utf8PathBuf>,
    /// Test/manual seam: the run this pod supervises.
    #[arg(long, requires_all = ["run_dir", "spec"])]
    run_id: Option<RunId>,
    /// Test/manual seam: the run directory.
    #[arg(long)]
    run_dir: Option<Utf8PathBuf>,
    /// Test/manual seam: a complete raw worker spec
    /// (fractality-core::worker::WorkerSpec) — no profile resolution.
    #[arg(long)]
    spec: Option<Utf8PathBuf>,
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("FRACTALITY_LOG").unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();
    let args = Args::parse();
    match run(args).await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            tracing::error!(error = %e, "pod failed");
            std::process::ExitCode::from(2)
        }
    }
}

/// Resolves the two input modes into (run_id, run_dir, worker spec,
/// collection plan).
///
/// Product mode (`--run-spec`) is where invariant I1's composition root
/// lives: the ambient env snapshot, the token-file read (at spawn time,
/// content never logged), and the config-dir provisioning all happen
/// here, feeding the pure backend constructor.
fn resolve_inputs(
    args: &Args,
) -> Result<(RunId, Utf8PathBuf, WorkerSpec, Option<Collection>), String> {
    if let Some(run_spec_path) = &args.run_spec {
        let text = std::fs::read_to_string(run_spec_path.as_std_path())
            .map_err(|e| format!("reading run spec `{run_spec_path}`: {e}"))?;
        let run_spec = RunSpec::from_toml_str(&text).map_err(|e| e.to_string())?;

        let packet_path = run_spec.run_dir.join("packet.toml");
        let packet_text = std::fs::read_to_string(packet_path.as_std_path())
            .map_err(|e| format!("reading packet `{packet_path}`: {e}"))?;
        let packet = Packet::from_toml_str(&packet_text).map_err(|e| e.to_string())?;

        let profiles = ProfilesFile::load(&args.home).map_err(|e| e.to_string())?;
        let profile = profiles
            .get(&packet.routing.profile)
            .map_err(|e| e.to_string())?;
        if profile.backend != ClaudeCodeBackend::ID {
            return Err(format!(
                "backend `{}` is not available in this build (only `{}`); \
                 fix profiles.toml or route the packet elsewhere",
                profile.backend,
                ClaudeCodeBackend::ID
            ));
        }

        let user_home = fractality_mc_client::home::user_home()?;
        let token_path = fractality_mc_client::home::expand_user(&profile.token_file, &user_home);
        let token = std::fs::read_to_string(token_path.as_std_path())
            .map_err(|e| format!("reading token file `{token_path}`: {e}"))?
            .trim()
            .to_owned();
        if token.is_empty() {
            return Err(format!("token file `{token_path}` is empty"));
        }

        let ctx = RunContext {
            run_id: run_spec.run_id,
            run_dir: run_spec.run_dir.clone(),
            workspace_dir: run_spec.workspace_dir.clone(),
            depth: run_spec.depth,
            node_id: run_spec.node_id.clone(),
        };
        // The composition-root snapshot: the one ambient read, handed to
        // the pure constructor (and to its poisoned-parent test).
        let os_env: std::collections::BTreeMap<String, String> = std::env::vars().collect();
        let mut spec = ClaudeCodeBackend
            .build_spec(&packet, profile, &BackendSecrets::new(token), &os_env, &ctx)
            .map_err(|e| e.to_string())?;

        // F14: resolve the program against the PATH the worker will see —
        // a bare `claude` must find npm's `.cmd` shim, which CreateProcess
        // alone never does. Product mode only: the raw `--spec` seam
        // states explicit paths and stays untouched.
        spec.argv[0] =
            resolve::resolve_program(&spec.argv[0], spec.env.get("PATH").map(String::as_str))?;

        // Generic provisioning: whatever config dir the backend chose
        // must exist before spawn (F4: a fresh dir onboards headless).
        if let Some(config_dir) = spec.env.get("CLAUDE_CONFIG_DIR") {
            std::fs::create_dir_all(config_dir)
                .map_err(|e| format!("creating config dir `{config_dir}`: {e}"))?;
        }
        std::fs::create_dir_all(run_spec.workspace_dir.as_std_path())
            .map_err(|e| format!("creating workspace `{}`: {e}", run_spec.workspace_dir))?;

        // Phase 4 nesting seam: give the worker the means to delegate
        // further (I1 stays intact — these are deliberate injections at
        // the composition root, not inheritance).
        let pod_exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(std::path::Path::to_path_buf));
        augment_worker_env(&mut spec.env, &args.home, pod_exe_dir.as_deref());

        // D18 layer 3: materialize the ask_boss MCP config the invocation
        // already names. The broker is this build's own CLI binary.
        if profile.permissions.ask_boss {
            worker_env::write_mcp_config(
                &ClaudeCodeBackend::mcp_config_path(&ctx),
                pod_exe_dir.as_deref(),
            )?;
        }

        let collection = Collection {
            workspace_dir: run_spec.workspace_dir.clone(),
            result_file: packet.output.result.clone(),
            acceptance: packet.task.acceptance.clone(),
            output_schema: packet.output.output_schema.clone(),
        };
        return Ok((run_spec.run_id, run_spec.run_dir, spec, Some(collection)));
    }

    match (&args.run_id, &args.run_dir, &args.spec) {
        (Some(run_id), Some(run_dir), Some(spec_path)) => {
            let spec_text = std::fs::read_to_string(spec_path.as_std_path())
                .map_err(|e| format!("reading spec `{spec_path}`: {e}"))?;
            let spec = WorkerSpec::from_toml_str(&spec_text).map_err(|e| e.to_string())?;
            Ok((*run_id, run_dir.clone(), spec, None))
        }
        _ => Err("pass either --run-spec <file> (product path) or all of \
             --run-id/--run-dir/--spec (raw seam)"
            .to_owned()),
    }
}

async fn run(args: Args) -> Result<(), String> {
    let (run_id, run_dir, spec, collection) = resolve_inputs(&args)?;
    std::fs::create_dir_all(run_dir.as_std_path())
        .map_err(|e| format!("creating run dir `{run_dir}`: {e}"))?;

    let pod_id = PodId::generate();
    let pod_pid = std::process::id();

    // Register first: the run must exist and be adoptable before any
    // worker process appears.
    let (mut client, heartbeat_ms) = loop {
        match connect(&args.home).await {
            Some(c) => match c
                .pod_register(&PodRegisterRequest {
                    pod_id,
                    pod_pid,
                    run_id,
                })
                .await
            {
                Ok(resp) => {
                    break (c, resp.heartbeat_interval_ms.max(500));
                }
                Err(fractality_mc_client::ClientError::Api {
                    status: 404,
                    message,
                    ..
                }) => {
                    return Err(format!(
                        "run {} is unknown to mission-control ({message}); \
                         register the run first",
                        run_id
                    ));
                }
                Err(fractality_mc_client::ClientError::Api {
                    status: 409,
                    message,
                    ..
                }) => {
                    return Err(format!("run {} is not adoptable: {message}", run_id));
                }
                Err(e) => {
                    tracing::warn!(error = %e, "registration failed; retrying");
                    tokio::time::sleep(std::time::Duration::from_millis(750)).await;
                }
            },
            None => {
                tracing::warn!("mission-control not reachable; retrying registration");
                tokio::time::sleep(std::time::Duration::from_millis(750)).await;
            }
        }
    };

    // Spawn under the OS-level kill guarantee (F5).
    let mut child = supervise::spawn(&spec)?;
    let worker_pid = child.pid().unwrap_or(0);
    tracing::info!(run_id = %run_id, worker_pid, "worker spawned");

    // The transcript pump tees every line through the stream parser; the
    // watch channel is the live-metering plane the heartbeat samples
    // (level-triggered state, the F11 lesson — never a notification).
    let (usage_tx, usage_rx) = tokio::sync::watch::channel(UsageTotals::default());
    let stdout_pump = pump_transcript(child.take_stdout(), run_dir.join(STDOUT_FILE), usage_tx);
    let stderr_pump = pump(child.take_stderr(), run_dir.join(STDERR_FILE));
    let stdin_feed = feed_stdin(child.take_stdin(), spec.stdin.clone());

    report(
        &mut client,
        &args.home,
        pod_id,
        &PodEventRequest {
            run_id,
            event: PodEvent::Spawned { worker_pid },
        },
    )
    .await;

    // Supervision loop: wait for exit, heartbeat on the interval,
    // rediscover the daemon whenever the bus drops. Each tick also
    // ships the parser's usage snapshot when it moved — `show` meters
    // a run while it is still running (D16: telemetry reads MC) — and
    // executes kill commands riding the heartbeat answer (Phase 4).
    let mut last_usage_sent = UsageTotals::default();
    let mut killed_reason: Option<KillReason> = None;
    let exit_code = loop {
        let tick = tokio::time::sleep(std::time::Duration::from_millis(heartbeat_ms));
        tokio::select! {
            exit = child.wait() => break exit?,
            _ = tick => {
                let hb = PodHeartbeat {
                    run_id,
                    state: RunState::Running,
                    worker_pid: Some(worker_pid),
                };
                match client.pod_heartbeat(pod_id, &hb).await {
                    Ok(resp) => {
                        if let PodCommand::Kill { reason } = resp.command
                            && killed_reason.is_none()
                        {
                            tracing::warn!(
                                run_id = %run_id, %reason,
                                "kill command received; terminating worker tree"
                            );
                            killed_reason = Some(reason);
                            child.kill_tree();
                            // The loop keeps running: child.wait() harvests
                            // the exit on the next poll.
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "heartbeat failed; rediscovering daemon");
                        if let Some(fresh) = reconnect_and_register(
                            &args.home, pod_id, pod_pid, run_id,
                        ).await {
                            client = fresh;
                        }
                    }
                }
                let snapshot = *usage_rx.borrow();
                if snapshot != last_usage_sent && snapshot.events > 0 {
                    report(
                        &mut client,
                        &args.home,
                        pod_id,
                        &PodEventRequest { run_id, event: PodEvent::Usage { usage: snapshot } },
                    )
                    .await;
                    last_usage_sent = snapshot;
                }
            }
        }
    };

    // P5 evidence: after a kill, the worker tree must be gone — the Job
    // Object guarantee (F5) is asserted, not assumed.
    if killed_reason.is_some() {
        sweep_for_orphan(worker_pid);
    }

    // Pumps end at EOF once the child is gone.
    let _ = stdin_feed.await;
    let summary = stdout_pump.await.ok().flatten();
    let _ = stderr_pump.await;

    // Phase 3 collection: settle the run dir's derived records and ship
    // the authoritative usage totals before the exit report.
    let (result_source, result_path) =
        collect::collect_result(collection.as_ref(), summary.as_ref());
    collect::write_usage_json(
        &run_dir,
        run_id,
        summary.as_ref(),
        result_source,
        result_path.as_deref(),
    )?;
    // D-C3-2: validate the result against the packet's output_schema at
    // this seam. Ф1.2b records the verdict into status.json; the one
    // retry-on-violation is Ф1.2c. No schema declared = no gate.
    let schema_verdict = collect::validate_output_schema(
        result_path.as_deref(),
        collection.as_ref().and_then(|c| c.output_schema.as_deref()),
    );
    if schema_verdict.checked && !schema_verdict.valid {
        tracing::warn!(
            run_id = %run_id,
            violations = schema_verdict.violations.len(),
            "result violated output_schema (retry lands in Ф1.2c)"
        );
    }
    // Acceptance proves the deliverable (D7): run only over a worker
    // that exited 0 — verifying a known-failed (or just-killed) run
    // wastes its budget.
    let (acceptance, acceptance_skipped) = match collection.as_ref() {
        Some(c) if !c.acceptance.is_empty() => {
            if killed_reason.is_some() {
                (Vec::new(), Some("worker killed; acceptance not run"))
            } else if exit_code == Some(0) {
                (
                    collect::run_acceptance(
                        &run_dir,
                        &c.workspace_dir,
                        &c.acceptance,
                        collect::ACCEPTANCE_CAP,
                    )
                    .await,
                    None,
                )
            } else {
                (Vec::new(), Some("worker failed; acceptance not run"))
            }
        }
        _ => (Vec::new(), None),
    };
    if let Some(s) = &summary
        && s.totals.events > 0
    {
        report(
            &mut client,
            &args.home,
            pod_id,
            &PodEventRequest {
                run_id,
                event: PodEvent::Usage { usage: s.totals },
            },
        )
        .await;
    }

    let kill_reason_str = killed_reason.map(|r| r.to_string());
    collect::write_status(
        &run_dir,
        run_id,
        collect::StatusRecord {
            exit_code,
            worker_pid,
            result_source,
            acceptance: &acceptance,
            acceptance_skipped,
            killed_reason: kill_reason_str.as_deref(),
            schema: &schema_verdict,
        },
    )?;

    // Collection rides the bus (Phase 4): result provenance + acceptance
    // verdicts fold into the run record so remote readers never need the
    // run dir. Non-critical delivery — the plane files above are the
    // D17 fallback; mission-control mints the FileRef (D19).
    report(
        &mut client,
        &args.home,
        pod_id,
        &PodEventRequest {
            run_id,
            event: PodEvent::Collected {
                result_source: result_source.to_owned(),
                result_path: result_path.clone(),
                acceptance_passed: acceptance.iter().filter(|v| v.ok).count() as u32,
                acceptance_total: acceptance.len() as u32,
                acceptance_skipped: acceptance_skipped.map(str::to_owned),
            },
        },
    )
    .await;

    // The exit report is the one message that must not be lost quietly:
    // retry across daemon restarts within the budget.
    let deadline = std::time::Instant::now() + EXIT_DELIVERY_BUDGET;
    let event = PodEventRequest {
        run_id,
        event: PodEvent::Exit { exit_code },
    };
    loop {
        if client.pod_event(pod_id, &event).await.is_ok() {
            break;
        }
        if std::time::Instant::now() >= deadline {
            tracing::error!(
                run_id = %run_id,
                "exit report undelivered within {}s; status.json on disk is the record",
                EXIT_DELIVERY_BUDGET.as_secs()
            );
            return Err("exit report undelivered".to_owned());
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        if let Some(fresh) = reconnect_and_register(&args.home, pod_id, pod_pid, run_id).await {
            client = fresh;
        }
    }

    tracing::info!(run_id = %run_id, ?exit_code, "worker exited; pod done");
    Ok(())
}

async fn connect(home: &camino::Utf8Path) -> Option<McClient> {
    McClient::connect(home).await.ok().flatten()
}

/// Delivers a non-critical event: one direct try, one try after
/// rediscovery, then a warning — heartbeats and the exit report carry
/// the state machine even when an intermediate event is lost.
async fn report(
    client: &mut McClient,
    home: &camino::Utf8Path,
    pod_id: PodId,
    event: &PodEventRequest,
) {
    if client.pod_event(pod_id, event).await.is_ok() {
        return;
    }
    if let Some(fresh) = connect(home).await {
        *client = fresh;
        if client.pod_event(pod_id, event).await.is_ok() {
            return;
        }
    }
    tracing::warn!(run_id = %event.run_id, "event delivery failed; continuing supervision");
}

/// Fresh lockfile → fresh client → re-register (adoption after a daemon
/// restart). Returns `None` when no live daemon answers yet.
async fn reconnect_and_register(
    home: &camino::Utf8Path,
    pod_id: PodId,
    pod_pid: u32,
    run_id: RunId,
) -> Option<McClient> {
    let client = connect(home).await?;
    match client
        .pod_register(&PodRegisterRequest {
            pod_id,
            pod_pid,
            run_id,
        })
        .await
    {
        Ok(_) => {
            tracing::info!("re-registered with the current daemon generation");
            Some(client)
        }
        Err(e) => {
            tracing::warn!(error = %e, "re-registration refused");
            // Still return the client: exit reports only need a live bus.
            Some(client)
        }
    }
}
