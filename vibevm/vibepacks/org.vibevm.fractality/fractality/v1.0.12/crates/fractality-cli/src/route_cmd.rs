//! `fractality route` (Campaign 2 D6): the delegation matrix's verdict
//! procedure as a verb. Pure calculus from the engine; this cell only
//! parses axes and prints. Exit: 0 delegate, 1 keep (a truthful
//! negative — the task stays with the boss), 2 never (bad axes).

use fractality_initiative::route::{Axes, Verdict};

use crate::{EXIT_NEGATIVE, EXIT_OK, fail_code};

specmark::scope!("spec://fractality/PROP-001#sessions");

pub(crate) fn route(error_cost: &str, context: &str, verify: &str, size: &str, json: bool) -> u8 {
    let axes = match parse_axes(error_cost, context, verify, size) {
        Ok(a) => a,
        Err(m) => return fail_code(2, &m),
    };
    let verdict = fractality_initiative::route::route(&axes);
    if json {
        let doc = match &verdict {
            Verdict::Keep { rule, reason } => serde_json::json!({
                "verdict": "keep", "rule": rule, "reason": reason,
            }),
            Verdict::Delegate {
                slot,
                scenario,
                discretionary,
                rule,
                reason,
            } => serde_json::json!({
                "verdict": "delegate", "slot": slot, "scenario": scenario,
                "discretionary": discretionary, "rule": rule, "reason": reason,
            }),
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&doc).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
        );
    } else {
        match &verdict {
            Verdict::Keep { rule, reason } => {
                println!("keep ({rule}): {reason}");
            }
            Verdict::Delegate {
                slot,
                scenario,
                discretionary,
                rule,
                reason,
            } => {
                let disc = if *discretionary {
                    " · discretionary"
                } else {
                    ""
                };
                println!("delegate → slot={slot} scenario={scenario}{disc} ({rule}): {reason}");
            }
        }
    }
    match verdict {
        Verdict::Delegate { .. } => EXIT_OK,
        Verdict::Keep { .. } => EXIT_NEGATIVE,
    }
}

fn parse_axes(error_cost: &str, context: &str, verify: &str, size: &str) -> Result<Axes, String> {
    Ok(Axes {
        error_cost: error_cost
            .parse()
            .map_err(|e| format!("--error-cost: {e}"))?,
        context: context.parse().map_err(|e| format!("--context: {e}"))?,
        verify: verify.parse().map_err(|e| format!("--verify: {e}"))?,
        size: size.parse().map_err(|e| format!("--size: {e}"))?,
    })
}
