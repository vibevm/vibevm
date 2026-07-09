//! The tolerant incremental parser for Claude Code `--output-format
//! stream-json` transcripts (one JSON object per line).
//!
//! The pod reads the worker's stdout line by line and feeds each line to
//! [`StreamParser`]. Design law **D14**: the parser NEVER fatals on
//! unknown input — unknown event types are counted under their `type`
//! key, lines that fail to parse are counted as malformed, and parsing
//! always continues. A schema change that adds new fields or new event
//! kinds degrades to "counted, ignored" rather than a hard failure
//! (risk R2: schema drift tolerance). Navigation uses
//! [`serde_json::Value`] exclusively — no rigid event structs to break.
//!
//! The `result` event is **authoritative** for the token totals: on this
//! provider the assistant events under-report — the fixture's assistant
//! usage blocks carry zeros while the `result` event carries the real
//! numbers — so when a `result.usage` object arrives it overwrites the
//! accumulated token fields. [`StreamParser::totals`] snapshots the
//! running totals for the pod's heartbeat metering mid-stream; the
//! snapshot is cumulative, so replaying the latest one is idempotent.

use std::collections::BTreeMap;

use fractality_core::run::UsageTotals;
use serde_json::Value;

specmark::scope!("spec://fractality/PROP-001#architecture");

/// A tolerant, incremental parser for one `stream-json` transcript.
///
/// Feed it lines as they arrive from the worker's stdout; [`Self::totals`]
/// gives a live snapshot for heartbeat metering, [`Self::finish`] gives
/// the end-of-run summary. Unknown event kinds and malformed lines are
/// counted, never fatal (D14).
///
/// ```
/// use fractality_backend_claude_code::stream::StreamParser;
///
/// let mut p = StreamParser::new();
/// p.feed_line(r#"{"type":"assistant","message":{"model":"glm-5-turbo","usage":{"input_tokens":10,"output_tokens":7}}}"#);
/// p.feed_line(r#"{"type":"result","num_turns":1,"is_error":false,"total_cost_usd":0.5,"usage":{"input_tokens":12,"output_tokens":9},"result":"done"}"#);
/// let s = p.finish();
/// assert_eq!(s.totals.output_tokens, 9, "result usage overwrites the assistant's 7");
/// assert_eq!(s.final_text.as_deref(), Some("done"));
/// assert_eq!(s.model.as_deref(), Some("glm-5-turbo"));
/// ```
#[derive(Debug, Default)]
pub struct StreamParser {
    totals: UsageTotals,
    model: Option<String>,
    num_turns: Option<u64>,
    final_text: Option<String>,
    is_error: bool,
    event_counts: BTreeMap<String, u64>,
    malformed_lines: u64,
}

impl StreamParser {
    /// An empty parser ready to receive lines.
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds one transcript line. Tolerant: never panics, never errors —
    /// an unparseable or non-object line bumps [`StreamSummary::malformed_lines`]
    /// and is otherwise ignored.
    pub fn feed_line(&mut self, line: &str) {
        // D14: a bad line is counted, not fatal. Parsing continues.
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            self.malformed_lines += 1;
            return;
        };
        let Some(obj) = value.as_object() else {
            self.malformed_lines += 1;
            return;
        };

        self.totals.events += 1;

        let type_str = obj.get("type").and_then(Value::as_str);
        let key = match type_str {
            Some("system") => {
                let sub = obj.get("subtype").and_then(Value::as_str).unwrap_or("?");
                format!("system/{sub}")
            }
            Some(other) => other.to_owned(),
            None => "untyped".to_owned(),
        };
        *self.event_counts.entry(key).or_default() += 1;

        match type_str {
            Some("assistant") => self.absorb_assistant(obj),
            Some("system") => self.absorb_system(obj),
            Some("result") => self.absorb_result(obj),
            _ => {}
        }
    }

    /// Running snapshot for live metering (the heartbeat sends these).
    pub fn totals(&self) -> UsageTotals {
        self.totals
    }

    /// Consumes the parser into the end-of-run summary.
    pub fn finish(self) -> StreamSummary {
        StreamSummary {
            totals: self.totals,
            model: self.model,
            num_turns: self.num_turns,
            final_text: self.final_text,
            is_error: self.is_error,
            event_counts: self.event_counts,
            malformed_lines: self.malformed_lines,
        }
    }

    /// Accumulates an assistant event's usage into the running totals.
    /// Each missing field contributes zero; `message.model` is recorded
    /// only when no earlier event (usually `system/init`) already set it
    /// — first-set wins.
    fn absorb_assistant(&mut self, obj: &serde_json::Map<String, Value>) {
        let Some(message) = obj.get("message") else {
            return;
        };
        let usage = message.get("usage");
        self.totals.input_tokens += u64_of(usage.and_then(|u| u.get("input_tokens")));
        self.totals.output_tokens += u64_of(usage.and_then(|u| u.get("output_tokens")));
        self.totals.cache_creation_input_tokens +=
            u64_of(usage.and_then(|u| u.get("cache_creation_input_tokens")));
        self.totals.cache_read_input_tokens +=
            u64_of(usage.and_then(|u| u.get("cache_read_input_tokens")));
        if self.model.is_none()
            && let Some(m) = message.get("model").and_then(Value::as_str)
        {
            self.model = Some(m.to_owned());
        }
    }

    /// A system/init event may carry the negotiated model; first-set wins.
    fn absorb_system(&mut self, obj: &serde_json::Map<String, Value>) {
        let is_init = obj.get("subtype").and_then(Value::as_str) == Some("init");
        if is_init
            && self.model.is_none()
            && let Some(m) = obj.get("model").and_then(Value::as_str)
        {
            self.model = Some(m.to_owned());
        }
    }

    /// The result event is authoritative: it overwrites the token fields
    /// (per-field; a missing field keeps the accumulated value), and sets
    /// cost, turns, final text, and error flag.
    fn absorb_result(&mut self, obj: &serde_json::Map<String, Value>) {
        if let Some(usage) = obj.get("usage") {
            if let Some(v) = usage.get("input_tokens").and_then(Value::as_u64) {
                self.totals.input_tokens = v;
            }
            if let Some(v) = usage.get("output_tokens").and_then(Value::as_u64) {
                self.totals.output_tokens = v;
            }
            if let Some(v) = usage
                .get("cache_creation_input_tokens")
                .and_then(Value::as_u64)
            {
                self.totals.cache_creation_input_tokens = v;
            }
            if let Some(v) = usage.get("cache_read_input_tokens").and_then(Value::as_u64) {
                self.totals.cache_read_input_tokens = v;
            }
        }
        if let Some(c) = obj.get("total_cost_usd").and_then(Value::as_f64) {
            self.totals.total_cost_usd = c;
        }
        if let Some(n) = obj.get("num_turns").and_then(Value::as_u64) {
            self.num_turns = Some(n);
        }
        if let Some(s) = obj.get("result").and_then(Value::as_str) {
            self.final_text = Some(s.to_owned());
        }
        self.is_error = obj
            .get("is_error")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    }
}

/// Reads a `usage`-shaped field as `u64`, defaulting a missing/non-integer
/// value to zero — the tolerant accumulation contract.
fn u64_of(v: Option<&Value>) -> u64 {
    v.and_then(Value::as_u64).unwrap_or(0)
}

/// The end-of-run summary produced by [`StreamParser::finish`].
#[derive(Debug, Clone, PartialEq)]
pub struct StreamSummary {
    /// Cumulative token totals (the `result` event is authoritative).
    pub totals: UsageTotals,
    /// The model negotiated for the run, if ever reported.
    pub model: Option<String>,
    /// `num_turns` from the `result` event, if reported.
    pub num_turns: Option<u64>,
    /// The worker's final report text, from the `result` event.
    pub final_text: Option<String>,
    /// `is_error` from the `result` event (missing ⇒ false).
    pub is_error: bool,
    /// Counts per event kind; system events count under `system/<subtype>`.
    pub event_counts: BTreeMap<String, u64>,
    /// Lines that failed to parse or parsed to a non-object.
    pub malformed_lines: u64,
}
