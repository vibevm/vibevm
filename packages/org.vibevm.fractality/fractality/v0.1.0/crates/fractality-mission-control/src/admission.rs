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
use fractality_core::ids::RunId;
use fractality_core::journal::Event;
use fractality_core::profile::{Profile, ProfilesFile};
use fractality_core::routing::{CapabilityClass, RoutingPolicy};
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

/// The capability class the depth guard should charge a spawn against: the
/// class the parent's profile declares, or the conservative `medium`
/// default when profiles are unreadable or the profile is gone. Profile
/// *validity* is [`preflight`]'s job (it turns a broken profile into a
/// 400); the guard only needs a class to derive a depth cap, so it must
/// never fail the registration on its own — it degrades to `medium`.
pub fn parent_capability_class(state: &AppState, profile_name: &str) -> CapabilityClass {
    match ProfilesFile::load(&state.cfg.home) {
        Ok(profiles) => profiles
            .get(profile_name)
            .map(|p| p.capability_class)
            .unwrap_or(CapabilityClass::Medium),
        Err(_) => CapabilityClass::Medium,
    }
}

/// The depth guard (D-C3-3): may a spawn nesting to `child_depth` under a
/// parent of `parent_class` be admitted? The cap is the routing policy's
/// per-class `max_depth` — routing semantics, where `0` means *no
/// spawning* (a weak-class worker is never a spawning root, D-C3-10) —
/// tightened by the parent's own `budget.max_depth` axis when it declares
/// one (`0` = unlimited on that axis, the six-axis convention). Pure and
/// total; the caller maps `Err` to a door refusal.
///
/// This enforcement is deliberately independent of [`needgate::decide`]'s
/// advisory fold-at-cap: the gate *recommends* folding at the cap, this
/// *refuses* a spawn that ignored the recommendation — defense in depth
/// so a caller that bypasses the gate still cannot open an unbounded tree.
///
/// [`needgate::decide`]: fractality_core::needgate::decide
pub fn check_spawn_depth(
    parent_class: CapabilityClass,
    parent_budget_max_depth: u32,
    policy: &RoutingPolicy,
    child_depth: u32,
) -> Result<(), String> {
    let policy_cap = policy.for_class(parent_class).max_depth;
    if policy_cap == 0 {
        return Err(format!(
            "a `{}`-class worker may not spawn children (routing policy \
             max_depth = 0 for this class: route or fold the task instead)",
            parent_class.as_str()
        ));
    }
    // The effective cap is the tightest active bound: the class ceiling
    // and the parent's declared subtree budget (budget `0` = unlimited).
    let effective = match parent_budget_max_depth {
        0 => policy_cap,
        budget => policy_cap.min(budget),
    };
    if child_depth > effective {
        let budget_note = if parent_budget_max_depth != 0 && parent_budget_max_depth < policy_cap {
            format!(", parent budget.max_depth = {parent_budget_max_depth}")
        } else {
            String::new()
        };
        return Err(format!(
            "spawn would nest to depth {child_depth}, past the cap {effective} for \
             `{}`-class workers (policy max_depth = {policy_cap}{budget_note})",
            parent_class.as_str()
        ));
    }
    Ok(())
}

/// The sibling invariants a spawn must satisfy (D-C3-4/5), checked in one
/// pass over the parent's non-terminal children (read from their run dirs):
///
///   - **no near-duplicate** — its task must not match an active sibling's
///     ([`Packet::task_fingerprint`]: task + inputs, NOT execution params,
///     so a legitimate fan-out of the same-titled task over different
///     chunks still passes). Two siblings doing the same work is
///     orchestration collapse.
///   - **at most one merge node** — if the spawn sets `output.merge`, no
///     active sibling may already be the merge node: a fan-out has ONE
///     designated answer, not two.
///
/// Best-effort under concurrency: two simultaneous spawns can race past
/// this (it is not atomic with the record) — an acceptable v1. A sibling
/// whose packet cannot be read is skipped.
///
/// [`Packet::task_fingerprint`]: fractality_core::Packet::task_fingerprint
pub fn check_sibling_invariants(
    state: &AppState,
    packet: &Packet,
    parent: RunId,
) -> Result<(), String> {
    let new_fp = packet.task_fingerprint();
    let is_merge = packet.output.merge;
    for sib in state.active_children(parent) {
        let Some(sib_packet) =
            std::fs::read_to_string(sib.run_dir.join("packet.toml").as_std_path())
                .ok()
                .and_then(|t| Packet::from_toml_str(&t).ok())
        else {
            continue;
        };
        if sib_packet.task_fingerprint() == new_fp {
            return Err(format!(
                "near-duplicate of active sibling {} (same task under the same parent)",
                sib.run_id
            ));
        }
        if is_merge && sib_packet.output.merge {
            return Err(format!(
                "a merge node already exists under this parent (sibling {}); \
                 a fan-out has one designated answer",
                sib.run_id
            ));
        }
    }
    Ok(())
}

/// Cold-verifier suppression (Ф5, FD-9 / §10.2): an acceptance VERIFIER run
/// must have real work to verify. A packet with `output.verifier` is
/// refused unless its `context.context_from` names at least one run that
/// produced a result — no cold verification over an empty tree. Verifiers
/// read only named results (RD-11: clean context by design, the fold law),
/// so `context_from` is exactly the set of work under review. Pure over the
/// registry snapshot; the caller maps `Err` to a door refusal.
pub fn check_verifier_has_work(state: &AppState, packet: &Packet) -> Result<(), String> {
    if !packet.output.verifier {
        return Ok(());
    }
    let has_work = packet.context.context_from.iter().any(|id| {
        state
            .get_run(*id)
            .and_then(|r| r.collected)
            .is_some_and(|c| c.result_source != "none")
    });
    if has_work {
        Ok(())
    } else {
        Err(
            "cold verifier: an acceptance/verifier run needs real work to check — its \
             `context.context_from` names no run that produced a result (no cold \
             verification over an empty tree)"
                .to_owned(),
        )
    }
}

/// Whether a profile's bearer-token file is present — the availability
/// signal for masking (FD-8). Mirrors the existence check [`preflight`]
/// makes at the door, factored out so routing can consult it without
/// re-validating the whole profile. `~` expands against the user home.
pub fn token_present(profile: &Profile, user_home: &Utf8Path) -> bool {
    let token = fractality_mc_client::home::expand_user(&profile.token_file, user_home);
    token.as_std_path().is_file()
}

/// Availability masking (FD-8): the profiles currently usable for routing
/// — those whose token file is present. A router considers ONLY these, so
/// an absent-credential profile is excluded before scoring rather than
/// dispatched-to and failed. Names come back in profiles.toml order; an
/// unreadable profiles file or user home masks everything to empty.
pub fn usable_profiles(state: &AppState) -> Vec<String> {
    let Ok(profiles) = ProfilesFile::load(&state.cfg.home) else {
        return Vec::new();
    };
    let Ok(user_home) = fractality_mc_client::home::user_home() else {
        return Vec::new();
    };
    profiles
        .profile
        .iter()
        .filter(|(_, p)| token_present(p, &user_home))
        .map(|(name, _)| name.clone())
        .collect()
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
