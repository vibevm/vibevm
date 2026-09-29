//! The V4 advisor-channel verb (PP-003 / D-C3-7): `fractality advise`.
//!
//! Advice moves judgment **sideways-up** without moving ownership (VISION
//! §V4): a caller consults a bigger model, reads its judgment, and keeps its
//! own task and loop. Unlike escalation (which hands the task UP the tree) an
//! advice run holds no loop, no task, no state — it returns advice and is
//! done.
//!
//! The verb is thin over the sync `run` loop: it marks the packet
//! `output.advice` (so the caller need not remember the flag), then registers
//! and waits like `run`. Mission-control's admission then applies the RD-10
//! caller-class bar ([`check_advisor_caller_class`]): a caller below `medium`
//! is refused at the door, because advice makes a weak caller worse, not
//! better. Routing the call to a rung above the caller is the caller's job
//! (the ladder is guidance — `RoutingPolicy::advisor_class_for` — not
//! enforced here).
//!
//! [`check_advisor_caller_class`]: fractality-mission-control admission

use fractality_mc_client::connect_or_start;

use crate::swarm::{resolve_parent, run_once, state_code};
use crate::{EXIT_NEGATIVE, err_code, fail_code, out};

specmark::scope!("spec://fractality/PROP-001#architecture");

/// `fractality advise --packet <file>`: consult an advisor synchronously.
///
/// The packet is marked `output.advice` before registration; the run is
/// awaited to a terminal state and its summary printed (the caller wants the
/// judgment back). No retry-on-violation — advice is judgment, not a
/// schema-gated deliverable — so this does not share `run`'s re-dispatch.
/// Exit mirrors `run`: 0 completed, 1 failed (the caller-class bar is a 400
/// the client surfaces as 1) or an invalid packet, 3 killed, 2 infrastructure.
pub(crate) async fn advise(
    home: &camino::Utf8Path,
    packet_path: &camino::Utf8Path,
    json: bool,
) -> u8 {
    let text = match std::fs::read_to_string(packet_path.as_std_path()) {
        Ok(t) => t,
        Err(e) => return fail_code(EXIT_NEGATIVE, &format!("reading `{packet_path}`: {e}")),
    };
    let mut packet = match fractality_core::Packet::from_toml_str(&text) {
        Ok(p) => p,
        Err(e) => return fail_code(EXIT_NEGATIVE, &e.to_string()),
    };
    // Asking for advice IS the marker: the caller need not remember to set
    // `output.advice`. Mission-control's caller-class bar applies at admission.
    packet.output.advice = true;

    // Client-side wait cap mirrors `run`: the packet's wall budget plus grace.
    let wait_cap = std::time::Duration::from_secs(packet.budget.wall_secs + 60);
    let client = match connect_or_start(home).await {
        Ok(c) => c,
        Err(e) => return fail_code(err_code(&e), &e.to_string()),
    };
    let parent = match resolve_parent(&client, None).await {
        Ok(p) => p,
        Err((code, message)) => return fail_code(code, &message),
    };

    let overall = std::time::Instant::now();
    match run_once(&client, packet, parent, wait_cap).await {
        Ok(run) => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&run)
                        .unwrap_or_else(|e| format!("{{\"error\":\"json: {e}\"}}"))
                );
            } else {
                out::print_run_summary(&run, overall.elapsed());
            }
            state_code(&run)
        }
        Err(code) => code,
    }
}
