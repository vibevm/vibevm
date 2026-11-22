//! Scoreboard rendering (plan D5/D7): strictly factual, compact, and
//! deterministic — the caller supplies every timestamp.
//!
//! Two shapes ship in Ф2: the one-line strip (statusline / hook
//! injections) and the multi-line board (`fractality scoreboard`,
//! SessionStart injection). Both render only measured facts: runs,
//! outcomes, tokens, parked questions with ages, the monthly web-tool
//! quota burn. No invented savings numbers (D7).

use fractality_core::CredibilityFact;
use fractality_core::api::{MetricsResponse, SessionMetricsResponse};

specmark::scope!("spec://fractality/PROP-001#sessions");

/// The one-line strip: `frl: 3 deleg · 2 done · 1 parked · slate 4`.
/// Compact enough for a statusline segment and a hook injection alike;
/// `mc: down` states are the caller's business (availability law — the
/// engine never learns transport).
pub fn render_line(session: &SessionMetricsResponse) -> String {
    let c = &session.session.counters;
    let mut parts = vec![
        format!("{} deleg", c.delegations),
        format!("{} done", session.runs.completed),
    ];
    if session.runs.failed + session.runs.killed > 0 {
        parts.push(format!(
            "{} failed",
            session.runs.failed + session.runs.killed
        ));
    }
    if !session.parked.is_empty() {
        parts.push(format!("{} parked", session.parked.len()));
    }
    parts.push(format!("slate {}", c.work_tools_since_delegation));
    format!("frl: {}", parts.join(" · "))
}

/// Web-tool calls burned in one month: the D12 quota rollup (the
/// IGNITION §15 leftover, absorbed here). `month` is a `YYYY-MM`
/// prefix matched against the by-day bucket keys.
pub fn month_web_calls(global: &MetricsResponse, month: &str) -> u64 {
    global
        .by_day
        .iter()
        .filter(|(day, _)| day.starts_with(month))
        .map(|(_, bucket)| bucket.web_tool_calls)
        .sum()
}

/// The multi-line board. With a session: its facts first (the D5
/// SessionStart shape); always: global today + month lines. `today` is
/// `YYYY-MM-DD`, `month` is `YYYY-MM`, `now_ms` anchors parked ages —
/// all caller-supplied (the engine reads no clock).
pub fn render_board(
    global: &MetricsResponse,
    session: Option<&SessionMetricsResponse>,
    credibility: Option<&CredibilityFact>,
    now_ms: u64,
    today: &str,
    month: &str,
) -> String {
    let mut out = String::new();
    if let Some(s) = session {
        let c = &s.session.counters;
        out.push_str(&format!(
            "session {}: {} delegated · {} done · {} failed · slate {} (work tools since last delegation)\n",
            s.session.session_id,
            c.delegations,
            s.runs.completed,
            s.runs.failed + s.runs.killed,
            c.work_tools_since_delegation,
        ));
        for p in &s.parked {
            out.push_str(&format!(
                "  parked {} ({}): {}\n",
                p.run_id,
                fractality_core::time::format_duration_ms(p.waiting_ms),
                p.question.lines().next().unwrap_or(""),
            ));
        }
    }
    if global.totals.runs == 0 {
        // The cold start (DEF-C2-3, F25): an all-zero counter block is
        // anti-proof at the one moment the injection speaks. Still
        // strictly factual (D7): "no runs yet" IS the measured fact;
        // the rest is the verb to run, not an invented number.
        out.push_str(
            "fabric ready — no delegated runs on this box yet.\n\
             first delegation: score the task (`fractality route --error-cost reversible --context compilable --verify mechanical --size S` → delegate/small), then `fractality spawn`; discovery: the fractality-delegate skill.\n",
        );
        return out;
    }
    let today_bucket = global.by_day.get(today);
    let (t_runs, t_done) = today_bucket.map_or((0, 0), |b| (b.runs, b.completed));
    out.push_str(&format!(
        "today: {} runs · {} completed · all-time: {} runs · {} completed · {} failed\n",
        t_runs, t_done, global.totals.runs, global.totals.completed, global.totals.failed,
    ));
    out.push_str(&format!(
        "month {}: {} of the web-tool quota burned · {} output tokens all-time\n",
        month,
        month_web_calls(global, month),
        global.totals.output_tokens,
    ));
    if let Some(cred) = credibility {
        // PP-002 (D7): rendered ONLY when a real completed-green acceptance
        // backs it (`worker_credibility` returned Some). This is the answer to
        // the Ф6 F24 keep-reason ("workers can't self-verify here") — dated
        // proof, on the surface the boss reads before it decides to delegate.
        out.push_str(&format!(
            "workers self-verify here: acceptance {}/{} green, last proven {} ago (profile {})\n",
            cred.acceptance_passed,
            cred.acceptance_total,
            fractality_core::time::format_duration_ms(now_ms.saturating_sub(cred.proven_ts_ms)),
            cred.profile,
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use fractality_core::api::{MetricsBucket, ParkedQuestion, SessionMetricsResponse};
    use fractality_core::session::InitiativeCounters;
    use fractality_core::{RunId, SessionRecord};

    fn session_fixture(
        delegations: u64,
        slate: u64,
        completed: u64,
        failed: u64,
        parked: Vec<ParkedQuestion>,
    ) -> SessionMetricsResponse {
        SessionMetricsResponse {
            session: SessionRecord {
                session_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().expect("ulid"),
                harness: "claude-code".into(),
                external_id: "cc-1".into(),
                cwd: "proj".into(),
                node_id: "n".into(),
                started_ts_ms: 1,
                updated_ts_ms: 1,
                ended_ts_ms: None,
                last_nudge_ts_ms: None,
                alerted_runs: Vec::new(),
                counters: InitiativeCounters {
                    work_tools_since_delegation: slate,
                    work_tools_total: slate + 3,
                    work_tool_ms_total: 1000,
                    delegations,
                    nudges_sent: 0,
                    question_alerts: 0,
                },
            },
            runs: MetricsBucket {
                runs: completed + failed,
                completed,
                failed,
                ..Default::default()
            },
            parked,
        }
    }

    fn parked(question: &str, waiting_ms: u64) -> ParkedQuestion {
        ParkedQuestion {
            run_id: "01BX5ZZKBKACTAV9WEVGEMMVRY".parse::<RunId>().expect("ulid"),
            question: question.into(),
            waiting_ms,
        }
    }

    #[test]
    fn the_line_is_compact_and_omits_empty_facts() {
        let s = session_fixture(3, 4, 2, 0, vec![]);
        assert_eq!(render_line(&s), "frl: 3 deleg · 2 done · slate 4");
    }

    #[test]
    fn the_line_surfaces_failures_and_parked_questions() {
        let s = session_fixture(1, 0, 0, 2, vec![parked("May I push?", 60_000)]);
        assert_eq!(
            render_line(&s),
            "frl: 1 deleg · 0 done · 2 failed · 1 parked · slate 0"
        );
    }

    #[test]
    fn month_rollup_sums_only_matching_days() {
        let mut global = MetricsResponse::default();
        for (day, calls) in [("2026-07-01", 5), ("2026-07-10", 7), ("2026-06-30", 100)] {
            global.by_day.insert(
                day.into(),
                MetricsBucket {
                    web_tool_calls: calls,
                    ..Default::default()
                },
            );
        }
        assert_eq!(month_web_calls(&global, "2026-07"), 12);
        assert_eq!(month_web_calls(&global, "2026-06"), 100);
        assert_eq!(month_web_calls(&global, "2026-01"), 0);
    }

    #[test]
    fn the_board_renders_session_first_then_global_lines() {
        let mut global = MetricsResponse::default();
        global.totals.runs = 10;
        global.totals.completed = 8;
        global.totals.failed = 1;
        global.totals.output_tokens = 42_000;
        global.by_day.insert(
            "2026-07-10".into(),
            MetricsBucket {
                runs: 4,
                completed: 3,
                web_tool_calls: 2,
                ..Default::default()
            },
        );
        let s = session_fixture(2, 1, 2, 0, vec![parked("Which branch?", 120_000)]);
        let board = render_board(&global, Some(&s), None, 0, "2026-07-10", "2026-07");
        let lines: Vec<&str> = board.lines().collect();
        assert_eq!(
            lines[0],
            "session 01ARZ3NDEKTSV4RRFFQ69G5FAV: 2 delegated · 2 done · 0 failed · slate 1 (work tools since last delegation)"
        );
        assert_eq!(
            lines[1],
            "  parked 01BX5ZZKBKACTAV9WEVGEMMVRY (2m00s): Which branch?"
        );
        assert_eq!(
            lines[2],
            "today: 4 runs · 3 completed · all-time: 10 runs · 8 completed · 1 failed"
        );
        assert_eq!(
            lines[3],
            "month 2026-07: 2 of the web-tool quota burned · 42000 output tokens all-time"
        );
    }

    #[test]
    fn a_zero_run_box_renders_the_cold_start_board_not_zero_counters() {
        let global = MetricsResponse::default();
        let board = render_board(&global, None, None, 0, "2026-07-10", "2026-07");
        let lines: Vec<&str> = board.lines().collect();
        assert_eq!(
            lines[0],
            "fabric ready — no delegated runs on this box yet."
        );
        assert!(lines[1].contains("fractality route"), "leads with the verb");
        assert!(lines[1].contains("fractality spawn"));
        assert!(lines[1].contains("fractality-delegate skill"));
        assert!(
            !board.contains("all-time: 0 runs"),
            "zero counters must not render (F25 anti-proof)"
        );
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn the_cold_board_keeps_the_live_session_line() {
        let global = MetricsResponse::default();
        let s = session_fixture(0, 3, 0, 0, vec![]);
        let board = render_board(&global, Some(&s), None, 0, "2026-07-10", "2026-07");
        let lines: Vec<&str> = board.lines().collect();
        assert!(lines[0].starts_with("session 01ARZ3NDEKTSV4RRFFQ69G5FAV"));
        assert_eq!(
            lines[1],
            "fabric ready — no delegated runs on this box yet."
        );
    }

    #[test]
    fn one_recorded_run_switches_the_board_back_to_counters() {
        let mut global = MetricsResponse::default();
        global.totals.runs = 1;
        global.totals.completed = 1;
        let board = render_board(&global, None, None, 0, "2026-07-10", "2026-07");
        assert!(board.starts_with("today: 0 runs"));
        assert!(board.contains("all-time: 1 runs · 1 completed"));
        assert!(!board.contains("fabric ready"));
    }

    #[test]
    fn the_board_surfaces_worker_credibility_when_a_fact_backs_it() {
        use fractality_core::CredibilityFact;
        let mut global = MetricsResponse::default();
        global.totals.runs = 3;
        global.totals.completed = 3;
        let now = 3_600_000_u64;
        let cred = CredibilityFact {
            proven_ts_ms: 0, // one hour before `now`
            profile: "glm".into(),
            acceptance_passed: 4,
            acceptance_total: 4,
        };
        let board = render_board(&global, None, Some(&cred), now, "2026-07-10", "2026-07");
        assert!(
            board.contains("workers self-verify here: acceptance 4/4 green")
                && board.contains("last proven")
                && board.contains("(profile glm)"),
            "the credibility line renders the dated fact:\n{board}"
        );
        // Absent the fact, no credibility line (D7 — never invented).
        let bare = render_board(&global, None, None, now, "2026-07-10", "2026-07");
        assert!(
            !bare.contains("self-verify"),
            "no backing fact → no credibility line"
        );
    }
}
