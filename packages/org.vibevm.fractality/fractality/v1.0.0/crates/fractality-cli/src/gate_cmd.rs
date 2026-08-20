//! `fractality gate` (D-C3-8): the need-gate decision procedure as a verb
//! — the one auditable call that turns a task into `inline | route |
//! fold-local | spawn | escalate`, with its journaled reason. Pure
//! calculus from `fractality-core::needgate`; this cell parses the task
//! signals and the candidate worker's capability class, resolves the
//! class's routing-policy caps, and prints the verdict. It runs offline —
//! no daemon, like `fractality route`.
//!
//! The class (not a raw `--max-depth`) is the input the operator gives:
//! the routing policy owns the depth cap AND whether the class may spawn
//! at all. Deriving `can_spawn` from `cap > 0` here is where the
//! `max_depth = 0` overload is resolved (routing "no spawning" vs
//! need-gate "unlimited") — a weak class never reaches `decide`'s spawn
//! arm.

use fractality_core::DecisionRecord;
use fractality_core::needgate::{Decision, GateInputs, decide};
use fractality_core::routing::{CapabilityClass, RoutingPolicy};
use fractality_mc_client::McClient;

use crate::{EXIT_OK, fail_code};

specmark::scope!("spec://fractality/PROP-001#model");

/// Exit code for an unparseable capability class (bad axes), matching the
/// `route` verb's family: 2 = the inputs themselves are wrong.
const EXIT_BAD_AXES: u8 = 2;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn gate(
    home: &camino::Utf8Path,
    class: &str,
    depth: u32,
    o1_lookup: bool,
    needs_absent_tool: bool,
    fits_window: bool,
    single_skill: bool,
    cross_chunk_dominant: bool,
    large_window_available: bool,
    decomposable: bool,
    record: bool,
    json: bool,
) -> u8 {
    let class = match parse_class(class) {
        Ok(c) => c,
        Err(m) => return fail_code(EXIT_BAD_AXES, &m),
    };
    let inputs = inputs_for(
        class,
        depth,
        o1_lookup,
        needs_absent_tool,
        fits_window,
        single_skill,
        cross_chunk_dominant,
        large_window_available,
        decomposable,
    );
    let decision = decide(&inputs);
    if json {
        let doc = serde_json::json!({
            "verdict": decision.verdict.as_str(),
            "reason": decision.reason,
            "class": class.as_str(),
            "depth": depth,
            "max_depth": inputs.max_depth,
            "can_spawn": inputs.can_spawn,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&doc).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
        );
    } else {
        println!(
            "{} ({}): {}",
            decision.verdict.as_str(),
            class.as_str(),
            decision.reason
        );
    }
    // `--record` journals the decision to a running daemon (D-C3-8, the
    // soft-label table's producer). Best-effort telemetry: a missing
    // daemon or a failed post warns but never changes the verdict or the
    // exit code — the decision itself already stands.
    if record {
        record_to_daemon(home, &decision, class, inputs).await;
    }
    EXIT_OK
}

/// Posts the decision to the mission-control daemon if one is running.
/// Never fatal: recording is telemetry, the verdict is the product.
async fn record_to_daemon(
    home: &camino::Utf8Path,
    decision: &Decision,
    class: CapabilityClass,
    inputs: GateInputs,
) {
    let record = DecisionRecord::from_decision(decision, class, inputs);
    match McClient::connect(home).await {
        Ok(Some(client)) => {
            if let Err(e) = client.record_decision(&record).await {
                eprintln!("fractality: decision not recorded: {e}");
            }
        }
        Ok(None) => {
            eprintln!(
                "fractality: no mission-control running; decision not recorded \
                 (start one with `fractality mc start`)"
            );
        }
        Err(e) => eprintln!("fractality: decision not recorded: {e}"),
    }
}

/// Builds the gate inputs for a candidate `class`: the task signals verbatim,
/// plus the class's routing-policy caps — `max_depth` and `can_spawn`
/// (whether the class may spawn at all, `cap > 0`). The compiled-in
/// `RoutingPolicy::default` is the v1 table.
#[allow(clippy::too_many_arguments)]
fn inputs_for(
    class: CapabilityClass,
    depth: u32,
    o1_lookup: bool,
    needs_absent_tool: bool,
    fits_window: bool,
    single_skill: bool,
    cross_chunk_dominant: bool,
    large_window_available: bool,
    decomposable: bool,
) -> GateInputs {
    let policy = RoutingPolicy::default().for_class(class);
    GateInputs {
        o1_lookup,
        needs_absent_tool,
        fits_window,
        single_skill,
        cross_chunk_dominant,
        large_window_available,
        decomposable,
        depth,
        max_depth: policy.max_depth,
        // Routing semantics: a policy cap of 0 means "no spawning" (weak).
        can_spawn: policy.max_depth > 0,
    }
}

fn parse_class(raw: &str) -> Result<CapabilityClass, String> {
    match raw {
        "weak" => Ok(CapabilityClass::Weak),
        "medium" => Ok(CapabilityClass::Medium),
        "strong" => Ok(CapabilityClass::Strong),
        other => Err(format!(
            "--class: `{other}` is not one of weak | medium | strong"
        )),
    }
}

/// Convenience for tests and any caller that wants the decision, not the
/// printed line.
#[cfg(test)]
fn decide_for(
    class: CapabilityClass,
    depth: u32,
    decomposable: bool,
) -> fractality_core::needgate::Decision {
    decide(&inputs_for(
        class,
        depth,
        false,
        false,
        false,
        false,
        false,
        false,
        decomposable,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use fractality_core::needgate::Verdict;

    #[test]
    fn parse_class_accepts_the_three_classes_and_rejects_others() {
        assert_eq!(parse_class("weak"), Ok(CapabilityClass::Weak));
        assert_eq!(parse_class("medium"), Ok(CapabilityClass::Medium));
        assert_eq!(parse_class("strong"), Ok(CapabilityClass::Strong));
        assert!(parse_class("mega").is_err());
    }

    /// The weak class (routing cap 0) resolves to `can_spawn = false`, so a
    /// decomposable task folds instead of spawning — the overload is
    /// resolved at this seam, not left to `max_depth`.
    #[test]
    fn weak_class_folds_a_decomposable_task() {
        let inputs = inputs_for(
            CapabilityClass::Weak,
            0,
            false,
            false,
            false,
            false,
            false,
            false,
            true,
        );
        assert_eq!(inputs.max_depth, 0);
        assert!(!inputs.can_spawn);
        assert_eq!(decide(&inputs).verdict, Verdict::FoldLocal);
    }

    /// Medium (routing cap 1) may spawn: a decomposable task at depth 0
    /// spawns, and at the cap folds.
    #[test]
    fn medium_class_spawns_under_the_cap_and_folds_at_it() {
        assert_eq!(
            decide_for(CapabilityClass::Medium, 0, true).verdict,
            Verdict::Spawn
        );
        assert_eq!(
            decide_for(CapabilityClass::Medium, 1, true).verdict,
            Verdict::FoldLocal
        );
    }
}
