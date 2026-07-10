//! Admission control (Phase 4): per-profile `max_concurrent` slots with
//! FIFO queueing.
//!
//! Registration always succeeds and lands the run in `queued`; this cell
//! decides when a queued, spawn-requested run actually launches. A slot
//! frees on any terminal event, and the sweeper ticks admission as a
//! self-heal (a launch that failed mid-flight or a daemon restart never
//! strands the queue). Launch marks `starting` in the journal FIRST —
//! after a restart only `queued` runs are launch candidates, so a pod
//! can never be double-spawned for one run.

use std::sync::Arc;

use camino::Utf8Path;
use fractality_core::Packet;
use fractality_core::journal::Event;
use fractality_core::profile::ProfilesFile;
use fractality_core::run::RunRecord;

use crate::state::AppState;

specmark::scope!("spec://fractality/PROP-001#architecture");

/// Registration-time validation (the D14/F16 contract: a bad packet is
/// refused at the door with the exact fix surface, before anything is
/// journaled). Admission re-checks cheaply at launch; this is the copy
/// that turns into a 400.
pub fn preflight(state: &AppState, packet: &Packet) -> Result<(), String> {
    let profiles = ProfilesFile::load(&state.cfg.home).map_err(|e| e.to_string())?;
    let profile = profiles
        .get(&packet.routing.profile)
        .map_err(|e| e.to_string())?;
    profile
        .resolve_model(&packet.routing.model)
        .map_err(|e| e.to_string())?;
    if profile.backend != "claude-code" {
        return Err(format!(
            "backend `{}` is not available in this build (only `claude-code`); \
             fix profiles.toml",
            profile.backend
        ));
    }
    let user_home = fractality_mc_client::home::user_home()?;
    let token_path = fractality_mc_client::home::expand_user(&profile.token_file, &user_home);
    if !token_path.as_std_path().is_file() {
        return Err(format!(
            "token file `{token_path}` does not exist; create it or fix \
             profiles.toml (existence is checked here; only the pod reads it)"
        ));
    }
    Ok(())
}

/// One admission pass: launch queued runs while their profiles have free
/// slots. Called after registration, after every terminal event, and by
/// the sweeper. Never holds the registry lock across a launch.
pub fn tick(state: &Arc<AppState>) {
    let profiles = match ProfilesFile::load(&state.cfg.home) {
        Ok(p) => p,
        Err(e) => {
            // Queued runs stay queued — visible in `ps`, never silently
            // dropped; the profiles file is the fix surface.
            tracing::warn!(error = %e, "admission paused: profiles unreadable");
            return;
        }
    };

    // A run is attempted at most once per tick: a claim that fails on a
    // journal fault would otherwise re-surface the same candidate forever.
    let mut attempted = std::collections::HashSet::new();
    loop {
        // Pick ONE candidate per iteration; capacity is re-derived after
        // every launch so a failing launch cannot over-admit.
        let candidate = state.queued_candidates().into_iter().find(|r| {
            if attempted.contains(&r.run_id) {
                return false;
            }
            let cap = profiles
                .get(&r.profile)
                .map(|p| p.limits.max_concurrent as usize)
                .unwrap_or(0);
            state.active_count(&r.profile) < cap
        });
        let Some(run) = candidate else { break };
        attempted.insert(run.run_id);

        // `queued -> starting` journals the launch claim before any side
        // effect — atomically, so a concurrent tick cannot double-spawn
        // (restart-safe too: after a crash only `queued` runs are
        // candidates again).
        if !state.claim_queued(run.run_id) {
            // Claimed or killed by someone else between pick and claim.
            continue;
        }
        if let Err(message) = launch(state, &run) {
            tracing::error!(run_id = %run.run_id, "launch failed: {message}");
            let _ = state.record(Event::Error {
                run_id: run.run_id,
                message,
                terminal: true,
            });
        }
    }
}

/// Everything between "admitted" and "a pod is supervising": re-read the
/// packet from the run dir (D4: the run dir is the source), provision
/// the workspace (D8), write the run spec, launch the pod detached (D3).
fn launch(state: &Arc<AppState>, record: &RunRecord) -> Result<(), String> {
    let packet_path = record.run_dir.join("packet.toml");
    let packet_text = std::fs::read_to_string(packet_path.as_std_path())
        .map_err(|e| format!("reading `{packet_path}`: {e}"))?;
    let packet = Packet::from_toml_str(&packet_text).map_err(|e| e.to_string())?;

    // Launch-time re-validation: the profile may have changed since
    // registration; failing here is journaled terminal by the caller.
    preflight(state, &packet)?;

    let workspace_dir =
        crate::workspace::provision(&packet.workspace, record.run_id, &record.run_dir)?;

    let run_spec = fractality_core::worker::RunSpec {
        schema: 1,
        run_id: record.run_id,
        run_dir: record.run_dir.clone(),
        workspace_dir,
        depth: record.depth,
        node_id: state.node.node_id.clone(),
    };
    let spec_path = record
        .run_dir
        .join(fractality_core::worker::RunSpec::FILE_NAME);
    std::fs::write(
        spec_path.as_std_path(),
        run_spec.to_toml_string().map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("writing `{spec_path}`: {e}"))?;

    launch_pod(&state.cfg.home, &record.run_dir, &spec_path)?;
    Ok(())
}

/// Spawns the pod binary detached (pods outlive the daemon by design).
fn launch_pod(home: &Utf8Path, run_dir: &Utf8Path, spec_path: &Utf8Path) -> Result<(), String> {
    let pod_bin = fractality_mc_client::resolve_pod_binary().map_err(|e| e.to_string())?;
    let pod_log = std::fs::File::create(run_dir.join("pod.log").as_std_path())
        .map_err(|e| format!("creating pod.log: {e}"))?;
    let pod_log_err = pod_log
        .try_clone()
        .map_err(|e| format!("cloning pod.log handle: {e}"))?;
    let mut cmd = std::process::Command::new(&pod_bin);
    cmd.arg("--home")
        .arg(home.as_str())
        .arg("--run-spec")
        .arg(spec_path.as_str())
        .stdin(std::process::Stdio::null())
        .stdout(pod_log)
        .stderr(pod_log_err);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Detached, no console: the pod must survive a daemon exit (D3).
        cmd.creation_flags(0x0000_0200 | 0x0800_0000);
    }
    let child = cmd
        .spawn()
        .map_err(|e| format!("spawning pod `{}`: {e}", pod_bin.display()))?;
    tracing::info!(pod_launcher_pid = child.id(), "pod launched");
    Ok(())
}
