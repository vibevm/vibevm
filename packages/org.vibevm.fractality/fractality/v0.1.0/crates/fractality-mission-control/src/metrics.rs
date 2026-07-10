//! `GET /v0/metrics` aggregation (I3/D16): the journal-backed registry
//! is the one profiling-metadata store; every consumer — `fractality
//! stats`, the Campaign-2 initiative system, future GUIs — reads these
//! aggregates and nothing else.

use std::collections::BTreeMap;

use fractality_core::api::{MetricsBucket, MetricsResponse};
use fractality_core::run::{RunRecord, RunState};
use fractality_core::time::utc_date_string;

specmark::scope!("spec://fractality/PROP-001#architecture");

/// Folds the registry snapshot into the metrics answer. Pure — trivially
/// testable, and the handler stays a one-liner.
pub fn compute(runs: &[RunRecord]) -> MetricsResponse {
    let mut resp = MetricsResponse::default();
    for run in runs {
        *resp
            .by_state
            .entry(run.state.as_str().to_owned())
            .or_default() += 1;
        fold(&mut resp.totals, run);
        fold(bucket(&mut resp.by_profile, &run.profile), run);
        fold(bucket(&mut resp.by_model, &run.model), run);
        fold(
            bucket(&mut resp.by_day, &utc_date_string(run.created_ts_ms)),
            run,
        );
    }
    resp
}

fn bucket<'a>(map: &'a mut BTreeMap<String, MetricsBucket>, key: &str) -> &'a mut MetricsBucket {
    map.entry(key.to_owned()).or_default()
}

/// Folds one run into a bucket — the one aggregation rule, shared by
/// the global answer and the session-scoped one (Campaign 2 D16).
pub(crate) fn fold_into(b: &mut MetricsBucket, run: &RunRecord) {
    fold(b, run);
}

fn fold(b: &mut MetricsBucket, run: &RunRecord) {
    b.runs += 1;
    match run.state {
        RunState::Completed => b.completed += 1,
        RunState::Failed => b.failed += 1,
        RunState::Killed => b.killed += 1,
        _ => b.open += 1,
    }
    b.input_tokens += run.usage.input_tokens;
    b.output_tokens += run.usage.output_tokens;
    b.cache_creation_input_tokens += run.usage.cache_creation_input_tokens;
    b.cache_read_input_tokens += run.usage.cache_read_input_tokens;
    b.total_cost_usd += run.usage.total_cost_usd;
    b.web_tool_calls += run.usage.web_tool_calls;
    if run.state.is_terminal()
        && let Some(started) = run.started_ts_ms
    {
        b.wall_ms += run.updated_ts_ms.saturating_sub(started);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fractality_core::packet::{BudgetSpec, WorkspaceMode};
    use fractality_core::run::UsageTotals;

    fn run(
        id: &str,
        state: RunState,
        profile: &str,
        model: &str,
        // (created, started, updated) — one timing triple.
        times: (u64, Option<u64>, u64),
        out_tokens: u64,
    ) -> RunRecord {
        RunRecord {
            run_id: id.parse().expect("fixed ulid"),
            title: "t".into(),
            state,
            profile: profile.into(),
            model: model.into(),
            workspace_mode: WorkspaceMode::Dir,
            parent: None,
            origin_session: None,
            depth: 0,
            spawn_requested: true,
            budget: BudgetSpec::default(),
            node_id: "n".into(),
            run_dir: "runs/x".into(),
            created_ts_ms: times.0,
            updated_ts_ms: times.2,
            started_ts_ms: times.1,
            pod: None,
            worker_pid: None,
            exit_code: None,
            failure: None,
            kill_reason: None,
            usage: UsageTotals {
                output_tokens: out_tokens,
                web_tool_calls: 1,
                ..Default::default()
            },
            collected: None,
            question: None,
            answer: None,
        }
    }

    #[test]
    fn buckets_split_by_profile_model_day_and_state() {
        // Two completed runs on day one (different profiles), one open
        // run on day two.
        let day1 = 1_783_641_600_000u64; // 2026-07-10T00:00Z
        let day2 = day1 + 86_400_000;
        let runs = vec![
            run(
                "01ARZ3NDEKTSV4RRFFQ69G5FAV",
                RunState::Completed,
                "glm",
                "big",
                (day1, Some(day1 + 1_000), day1 + 31_000),
                100,
            ),
            run(
                "01BX5ZZKBKACTAV9WEVGEMMVRY",
                RunState::Killed,
                "glm",
                "small",
                (day1, Some(day1 + 2_000), day1 + 12_000),
                50,
            ),
            run(
                "01BX5ZZKBKACTAV9WEVGEMMVS0",
                RunState::Running,
                "other",
                "big",
                (day2, Some(day2), day2 + 5_000),
                7,
            ),
        ];
        let m = compute(&runs);
        assert_eq!(m.totals.runs, 3);
        assert_eq!(m.totals.completed, 1);
        assert_eq!(m.totals.killed, 1);
        assert_eq!(m.totals.open, 1);
        assert_eq!(m.totals.output_tokens, 157);
        assert_eq!(m.totals.web_tool_calls, 3);
        // Wall time only for terminal runs: 30s + 10s.
        assert_eq!(m.totals.wall_ms, 40_000);
        assert_eq!(m.by_state.get("running"), Some(&1));
        assert_eq!(m.by_profile["glm"].runs, 2);
        assert_eq!(m.by_profile["other"].open, 1);
        assert_eq!(m.by_model["big"].runs, 2);
        assert_eq!(m.by_day["2026-07-10"].runs, 2);
        assert_eq!(m.by_day["2026-07-11"].runs, 1);
    }
}
