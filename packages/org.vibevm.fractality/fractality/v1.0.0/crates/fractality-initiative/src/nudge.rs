//! Nudge policy (plan D5/D6, barkain study BD1): threshold-gated,
//! cooldown-bounded, two lines maximum, never blocking. Pure decisions
//! over bus facts + explicit time; the texts are templates filled with
//! measured numbers and the exact verb to run — a nudge cites facts,
//! not rules.

use fractality_core::api::SessionMetricsResponse;
use serde::Deserialize;

specmark::scope!("spec://fractality/PROP-001#sessions");

/// `<home>/initiative.toml` (machine-scoped, like profiles.toml). All
/// fields default so an absent file is a fully-working default posture;
/// unknown keys are ignored (forward tolerance).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(default)]
pub struct NudgeConfig {
    /// Master switch (`FRACTALITY_INITIATIVE=off` overrides it at the
    /// adapter edge).
    pub enabled: bool,
    /// Work-shaped tool calls since the last delegation before a
    /// prompt-time nudge fires (BD1 slate threshold).
    pub work_tool_threshold: u64,
    /// Minimum seconds between prompt-time nudges (anti-fatigue, R3).
    pub cooldown_secs: u64,
    /// Emit Stop-time parked-question alerts (once per question).
    pub question_alerts: bool,
    /// Emit the threshold nudge mid-turn through PostToolUse
    /// (DEF-C2-1, F23: single-prompt headless sessions have no second
    /// prompt for the prompt-time channel; this is the seam that
    /// exists there). Shares the session cooldown anchor, so the
    /// fatigue bound stays one nudge per window across ALL channels.
    pub midwork_nudges: bool,
}

impl Default for NudgeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            work_tool_threshold: 7,
            cooldown_secs: 300,
            question_alerts: true,
            midwork_nudges: true,
        }
    }
}

impl NudgeConfig {
    /// Parses `initiative.toml` text. Absent file → `default()` at the
    /// caller; a malformed file also degrades to defaults (availability
    /// law: a broken config must not break a session — the adapter may
    /// surface the parse error through its debug channel).
    pub fn from_toml_str(text: &str) -> Self {
        toml::from_str(text).unwrap_or_default()
    }
}

/// A prompt-time injection the adapter should emit.
#[derive(Debug, Clone, PartialEq)]
pub struct Nudge {
    /// Short trigger name — journaled as the NudgeSent reason.
    pub reason: String,
    /// ≤ 2 lines, facts + the verb to run (D6).
    pub text: String,
}

/// The UserPromptSubmit decision (D5): parked questions win over the
/// slate; one nudge per cooldown window; quiet otherwise.
pub fn decide_prompt_nudge(
    cfg: &NudgeConfig,
    session: &SessionMetricsResponse,
    now_ms: u64,
) -> Option<Nudge> {
    if !cfg.enabled {
        return None;
    }
    let cooling = session
        .session
        .last_nudge_ts_ms
        .is_some_and(|t| now_ms.saturating_sub(t) < cfg.cooldown_secs.saturating_mul(1000));
    if cooling {
        return None;
    }
    if !session.parked.is_empty() {
        let oldest = session
            .parked
            .iter()
            .map(|p| p.waiting_ms)
            .max()
            .unwrap_or(0);
        return Some(Nudge {
            reason: "parked-questions".to_owned(),
            text: format!(
                "fractality: {} worker(s) parked on a question (oldest {}). Triage with `fractality questions`, answer with `fractality answer <id> \"...\"`.",
                session.parked.len(),
                fractality_core::time::format_duration_ms(oldest),
            ),
        });
    }
    let slate = session.session.counters.work_tools_since_delegation;
    if slate >= cfg.work_tool_threshold {
        return Some(Nudge {
            reason: "work-tool-threshold".to_owned(),
            text: format!(
                "fractality: {slate} work-tool calls since your last delegation. If the task is a work order, score it (`fractality route --error-cost ... --context ... --verify ... --size ...`) and delegate per the matrix.",
            ),
        });
    }
    None
}

/// The mid-work decision (DEF-C2-1): the slate threshold surfaced
/// through PostToolUse `additionalContext`, for sessions whose prompt
/// channel never re-fires (F23). Slate only — parked questions keep
/// their two dedicated channels (prompt + stop). Same threshold, same
/// shared cooldown anchor; a distinct journal reason so channel
/// fatigue can be told apart in the field data (MT-C2-05 measures it).
pub fn decide_midwork_nudge(
    cfg: &NudgeConfig,
    session: &SessionMetricsResponse,
    now_ms: u64,
) -> Option<Nudge> {
    if !cfg.enabled || !cfg.midwork_nudges {
        return None;
    }
    let cooling = session
        .session
        .last_nudge_ts_ms
        .is_some_and(|t| now_ms.saturating_sub(t) < cfg.cooldown_secs.saturating_mul(1000));
    if cooling {
        return None;
    }
    let slate = session.session.counters.work_tools_since_delegation;
    if slate >= cfg.work_tool_threshold {
        return Some(Nudge {
            reason: "work-tool-threshold-midwork".to_owned(),
            text: format!(
                "fractality: {slate} work-tool calls since your last delegation. If the current chunk is a work order, score it (`fractality route --error-cost ... --context ... --verify ... --size ...`) and delegate per the matrix.",
            ),
        });
    }
    None
}

/// The Stop-time alert (D5): the first parked question not yet
/// alerted, once per run — bounded exactly as barkain's forced
/// continuation is not. The adapter must also respect
/// `stop_hook_active` before asking for this decision.
pub fn decide_stop_alert(
    cfg: &NudgeConfig,
    session: &SessionMetricsResponse,
) -> Option<(fractality_core::RunId, String)> {
    if !cfg.enabled || !cfg.question_alerts {
        return None;
    }
    let unalerted = session
        .parked
        .iter()
        .find(|p| !session.session.alerted_runs.contains(&p.run_id))?;
    Some((
        unalerted.run_id,
        format!(
            "fractality: worker {} is parked on a question ({} waiting): {} — answer with `fractality answer {} \"...\"` before closing out.",
            unalerted.run_id,
            fractality_core::time::format_duration_ms(unalerted.waiting_ms),
            unalerted.question.lines().next().unwrap_or(""),
            unalerted.run_id,
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use fractality_core::api::{MetricsBucket, ParkedQuestion, SessionMetricsResponse};
    use fractality_core::session::InitiativeCounters;
    use fractality_core::{RunId, SessionRecord};

    const RUN: &str = "01BX5ZZKBKACTAV9WEVGEMMVRY";

    fn fixture(slate: u64, parked: Vec<ParkedQuestion>) -> SessionMetricsResponse {
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
                    ..Default::default()
                },
            },
            runs: MetricsBucket::default(),
            parked,
        }
    }

    fn parked(waiting_ms: u64) -> ParkedQuestion {
        ParkedQuestion {
            run_id: RUN.parse::<RunId>().expect("ulid"),
            question: "Which branch?\ndetails...".into(),
            waiting_ms,
        }
    }

    #[test]
    fn quiet_below_threshold_and_without_parked_questions() {
        let cfg = NudgeConfig::default();
        assert_eq!(
            decide_prompt_nudge(&cfg, &fixture(6, vec![]), 1_000_000),
            None
        );
    }

    #[test]
    fn the_slate_threshold_fires_with_the_route_verb_in_the_text() {
        let cfg = NudgeConfig::default();
        let n = decide_prompt_nudge(&cfg, &fixture(7, vec![]), 1_000_000).expect("fires");
        assert_eq!(n.reason, "work-tool-threshold");
        assert!(n.text.contains("7 work-tool calls"));
        assert!(n.text.contains("fractality route"));
    }

    #[test]
    fn parked_questions_win_over_the_slate() {
        let cfg = NudgeConfig::default();
        let n = decide_prompt_nudge(&cfg, &fixture(20, vec![parked(120_000)]), 1_000_000)
            .expect("fires");
        assert_eq!(n.reason, "parked-questions");
        assert!(n.text.contains("fractality questions"));
        assert!(n.text.contains("2m00s"));
    }

    #[test]
    fn the_cooldown_silences_and_expiry_reopens() {
        let cfg = NudgeConfig::default();
        let mut s = fixture(9, vec![]);
        s.session.last_nudge_ts_ms = Some(1_000_000);
        assert_eq!(
            decide_prompt_nudge(&cfg, &s, 1_000_000 + 299_999),
            None,
            "inside the 300 s window"
        );
        assert!(
            decide_prompt_nudge(&cfg, &s, 1_000_000 + 300_000).is_some(),
            "the window closes at exactly cooldown_secs"
        );
    }

    #[test]
    fn disabled_config_silences_everything() {
        let cfg = NudgeConfig {
            enabled: false,
            ..Default::default()
        };
        assert_eq!(
            decide_prompt_nudge(&cfg, &fixture(50, vec![parked(1)]), 1_000_000),
            None
        );
        assert_eq!(decide_stop_alert(&cfg, &fixture(0, vec![parked(1)])), None);
    }

    #[test]
    fn stop_alert_fires_once_per_run() {
        let cfg = NudgeConfig::default();
        let mut s = fixture(0, vec![parked(60_000)]);
        let (run, text) = decide_stop_alert(&cfg, &s).expect("first alert");
        assert_eq!(run.to_string(), RUN);
        assert!(text.contains("Which branch?"));
        assert!(text.contains(&format!("fractality answer {RUN}")));

        s.session.alerted_runs.push(RUN.parse().expect("ulid"));
        assert_eq!(decide_stop_alert(&cfg, &s), None, "already alerted");
    }

    #[test]
    fn config_parses_partial_toml_and_degrades_on_garbage() {
        let cfg = NudgeConfig::from_toml_str("work_tool_threshold = 3\n");
        assert_eq!(cfg.work_tool_threshold, 3);
        assert!(cfg.enabled, "unset fields keep defaults");
        assert!(cfg.midwork_nudges, "the mid-work channel defaults on");
        assert_eq!(
            NudgeConfig::from_toml_str("not toml ["),
            NudgeConfig::default()
        );
    }

    #[test]
    fn the_midwork_nudge_fires_at_the_threshold_with_its_own_reason() {
        let cfg = NudgeConfig::default();
        assert_eq!(
            decide_midwork_nudge(&cfg, &fixture(6, vec![]), 1_000_000),
            None,
            "below threshold"
        );
        let n = decide_midwork_nudge(&cfg, &fixture(7, vec![]), 1_000_000).expect("fires");
        assert_eq!(n.reason, "work-tool-threshold-midwork");
        assert!(n.text.contains("7 work-tool calls"));
        assert!(n.text.contains("fractality route"));
    }

    #[test]
    fn the_midwork_nudge_shares_the_cooldown_anchor() {
        let cfg = NudgeConfig::default();
        let mut s = fixture(9, vec![]);
        s.session.last_nudge_ts_ms = Some(1_000_000);
        assert_eq!(
            decide_midwork_nudge(&cfg, &s, 1_000_000 + 299_999),
            None,
            "one nudge per window across channels"
        );
        assert!(decide_midwork_nudge(&cfg, &s, 1_000_000 + 300_000).is_some());
    }

    #[test]
    fn the_midwork_channel_has_its_own_switch_and_ignores_parked() {
        let off = NudgeConfig {
            midwork_nudges: false,
            ..Default::default()
        };
        assert_eq!(decide_midwork_nudge(&off, &fixture(50, vec![]), 1), None);
        let cfg = NudgeConfig::default();
        let n = decide_midwork_nudge(&cfg, &fixture(9, vec![parked(60_000)]), 1_000_000)
            .expect("slate fires");
        assert_eq!(
            n.reason, "work-tool-threshold-midwork",
            "parked questions belong to the prompt/stop channels"
        );
    }
}
