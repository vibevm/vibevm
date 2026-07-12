//! The boss-side triage and telemetry verbs (D16/D18/D-C3-6):
//! `questions`, `escalations`, `answer`, `stats`.

use std::collections::HashMap;

use fractality_core::ids::RunId;
use fractality_core::run::RunState;
use fractality_mc_client::{ClientError, connect_or_start};

use crate::{EXIT_NEGATIVE, EXIT_OK, err_code, fail_code, out, resolve_run};

specmark::scope!("spec://fractality/PROP-001#architecture");

/// `fractality questions`: the boss's triage inbox (D18).
pub(crate) async fn questions(home: &camino::Utf8Path, json: bool) -> u8 {
    let client = match connect_or_start(home).await {
        Ok(c) => c,
        Err(e) => return fail_code(err_code(&e), &e.to_string()),
    };
    match client.runs(Some(RunState::WaitingOnBoss), None).await {
        Ok(runs) => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&runs).expect("runs serialize")
                );
                return EXIT_OK;
            }
            for r in &runs {
                println!(
                    "{} {} {}",
                    r.run_id,
                    fractality_core::time::format_duration_ms(
                        fractality_core::time::now_ms().saturating_sub(r.updated_ts_ms)
                    ),
                    r.question.as_deref().unwrap_or("-"),
                );
            }
            EXIT_OK
        }
        Err(e) => fail_code(err_code(&e), &e.to_string()),
    }
}

/// `fractality escalations`: the boss's escalation inbox (D-C3-6) — the
/// runs that handed their task UP the tree. Each is attributed to the
/// ROOT of its parent chain (the top-level task a human owns), so the
/// climb to the top is visible at a glance. Exit: 0 (even when empty).
pub(crate) async fn escalations(home: &camino::Utf8Path, json: bool) -> u8 {
    let client = match connect_or_start(home).await {
        Ok(c) => c,
        Err(e) => return fail_code(err_code(&e), &e.to_string()),
    };
    // The whole registry in one read: the escalated runs to report, plus
    // every run's parent so the walk to the root resolves. The climb is
    // client-side over the `parent` edges — no new endpoint (the state
    // filter already serves `runs(Escalated)`).
    let all = match client.runs(None, None).await {
        Ok(runs) => runs,
        Err(e) => return fail_code(err_code(&e), &e.to_string()),
    };
    let parents: HashMap<RunId, Option<RunId>> = all.iter().map(|r| (r.run_id, r.parent)).collect();
    let mut escalated: Vec<_> = all
        .iter()
        .filter(|r| r.state == RunState::Escalated)
        .collect();
    escalated.sort_by_key(|r| r.updated_ts_ms);

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&escalated).expect("runs serialize")
        );
        return EXIT_OK;
    }
    for r in &escalated {
        let root = root_of(&parents, r.run_id);
        let (reason, needs) = r
            .escalation
            .as_ref()
            .map(|e| (e.reason.as_str(), e.needs.as_str()))
            .unwrap_or(("-", "-"));
        println!(
            "{} {} depth={} root={} — {} (needs: {})",
            r.run_id,
            fractality_core::time::format_duration_ms(
                fractality_core::time::now_ms().saturating_sub(r.updated_ts_ms)
            ),
            r.depth,
            root,
            reason,
            needs,
        );
    }
    EXIT_OK
}

/// Climbs the `parent` edges from `start` to the root of its call tree
/// (the run whose parent is `None`), returning that root's id — the
/// escalation's attribution to the human at the top. A dangling parent
/// (pruned or foreign-home) stops the walk at that id; a 64-hop guard
/// bounds any corrupted cycle so triage never hangs.
fn root_of(parents: &HashMap<RunId, Option<RunId>>, start: RunId) -> RunId {
    let mut cur = start;
    for _ in 0..64 {
        match parents.get(&cur).copied().flatten() {
            Some(parent) => cur = parent,
            None => break,
        }
    }
    cur
}

/// `fractality answer <id> [<text>|--file <f>]` (D18).
pub(crate) async fn answer(
    home: &camino::Utf8Path,
    raw_id: &str,
    text: Option<&str>,
    file: Option<&camino::Utf8Path>,
) -> u8 {
    let body = match (text, file) {
        (Some(t), None) => t.to_owned(),
        (None, Some(path)) => match std::fs::read_to_string(path.as_std_path()) {
            Ok(t) => t,
            Err(e) => return fail_code(EXIT_NEGATIVE, &format!("reading `{path}`: {e}")),
        },
        _ => {
            return fail_code(
                EXIT_NEGATIVE,
                "give the answer as an argument or with --file",
            );
        }
    };
    let client = match connect_or_start(home).await {
        Ok(c) => c,
        Err(e) => return fail_code(err_code(&e), &e.to_string()),
    };
    let run = match resolve_run(&client, raw_id).await {
        Ok(r) => r,
        Err((code, message)) => return fail_code(code, &message),
    };
    match client.answer(run.run_id, &body).await {
        Ok(r) => {
            println!("{} {}", r.run_id, r.state);
            EXIT_OK
        }
        Err(ClientError::Api {
            status: 409,
            message,
            ..
        }) => fail_code(EXIT_NEGATIVE, &format!("run is not waiting: {message}")),
        Err(e) => fail_code(err_code(&e), &e.to_string()),
    }
}

/// `fractality stats`: a thin client over GET /v0/metrics (D16 — no
/// shadow accounting anywhere else).
pub(crate) async fn stats(home: &camino::Utf8Path, json: bool) -> u8 {
    let client = match connect_or_start(home).await {
        Ok(c) => c,
        Err(e) => return fail_code(err_code(&e), &e.to_string()),
    };
    match client.metrics().await {
        Ok(m) => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&m).expect("metrics serialize")
                );
            } else {
                out::print_metrics(&m);
                // The D12 monthly quota rollup (IGNITION §15 leftover,
                // absorbed by C2 Ф2): one consumer sum, engine-rendered.
                let month = fractality_core::time::utc_date_string(fractality_core::time::now_ms())
                    [..7]
                    .to_owned();
                println!(
                    "month {}: {} web-tool calls",
                    month,
                    fractality_initiative::month_web_calls(&m, &month),
                );
            }
            EXIT_OK
        }
        Err(e) => fail_code(err_code(&e), &e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> RunId {
        s.parse().expect("ulid")
    }

    #[test]
    fn root_of_climbs_to_the_top_and_guards_cycles() {
        let a = id("01ARZ3NDEKTSV4RRFFQ69G5FA1"); // root (parent None)
        let b = id("01ARZ3NDEKTSV4RRFFQ69G5FA2"); // child of a
        let c = id("01ARZ3NDEKTSV4RRFFQ69G5FA3"); // child of b
        let parents: HashMap<RunId, Option<RunId>> = [(a, None), (b, Some(a)), (c, Some(b))]
            .into_iter()
            .collect();
        assert_eq!(root_of(&parents, c), a, "climbs c -> b -> a");
        assert_eq!(root_of(&parents, b), a);
        assert_eq!(root_of(&parents, a), a, "a root is itself");

        // A dangling parent stops the walk at that id (an honest boundary
        // when a chain crosses a pruned or foreign-home run).
        let d = id("01ARZ3NDEKTSV4RRFFQ69G5FA4");
        let missing = id("01ARZ3NDEKTSV4RRFFQ69G5FA5");
        let dangling: HashMap<RunId, Option<RunId>> = [(d, Some(missing))].into_iter().collect();
        assert_eq!(root_of(&dangling, d), missing);

        // A cycle must terminate via the hop guard, never hang.
        let x = id("01ARZ3NDEKTSV4RRFFQ69G5FA6");
        let y = id("01ARZ3NDEKTSV4RRFFQ69G5FA7");
        let cyc: HashMap<RunId, Option<RunId>> = [(x, Some(y)), (y, Some(x))].into_iter().collect();
        let r = root_of(&cyc, x);
        assert!(r == x || r == y, "cycle resolves to one of its nodes");
    }
}
