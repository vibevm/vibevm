//! The campaign journal and the generated `RESUME.md` (crash-safety law:
//! campaign plan §4; data shapes PROP-043 §7.4).
//!
//! Append-only JSONL; a torn tail line (killed mid-write) is discarded on
//! read. Recovery rule rendered into RESUME.md verbatim: step closed ⇒
//! edits stand; step open ⇒ `git restore` its files and redo.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#campaign-zone");

use crate::cache::{now_utc, write_atomic};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Event {
    StepStart {
        id: String,
        /// mark-file | verify-unit | close-obligation | execute-task | …
        step_type: String,
        target: String,
        actor: String,
        ts: String,
    },
    StepDone {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        result: Option<String>,
        ts: String,
    },
    /// A campaign phase transition. Append-only and machine-readable: the
    /// `value` of the LAST `phase` event is the campaign's current phase
    /// (with none present the phase is the opening `"A"`). The tool derives
    /// the phase from this event and never by parsing the plan's Markdown —
    /// PROP-043 §state: "the dashboard … computes nothing and parses no
    /// Markdown ever."
    Phase { value: String, ts: String },
}

impl Event {
    pub fn id(&self) -> &str {
        match self {
            Event::StepStart { id, .. } | Event::StepDone { id, .. } => id,
            // A phase transition is not a step; it carries no step id.
            Event::Phase { .. } => "",
        }
    }
}

/// Read the journal, tolerating a torn last line and unknown (newer) event
/// kinds.
///
/// A torn tail (writer killed mid-line) is *incomplete* JSON — stop there
/// and never guess past it. A **complete** JSON line whose `kind` this reader
/// does not model yet is a newer writer's event — skip it but keep reading,
/// so a forward-compatible event never truncates the log.
pub fn read_journal(path: &Path) -> Result<Vec<Event>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Stage 1: is the line even complete JSON? A torn tail is not.
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            break; // torn/incomplete tail — never guess past it
        };
        // Stage 2: a modeled event kind is kept; an unmodeled one is skipped
        // (forward compatibility), and reading continues past it.
        if let Ok(event) = serde_json::from_value::<Event>(value) {
            out.push(event);
        }
    }
    Ok(out)
}

/// Append one event (creates the file and parents when absent).
pub fn append_event(path: &Path, event: &Event) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("opening {}", path.display()))?;
    let mut line = serde_json::to_string(event)?;
    line.push('\n');
    f.write_all(line.as_bytes())
        .with_context(|| format!("appending to {}", path.display()))?;
    Ok(())
}

pub fn start_step(path: &Path, id: &str, step_type: &str, target: &str, actor: &str) -> Result<()> {
    append_event(
        path,
        &Event::StepStart {
            id: id.into(),
            step_type: step_type.into(),
            target: target.into(),
            actor: actor.into(),
            ts: now_utc(),
        },
    )
}

pub fn done_step(path: &Path, id: &str, result: Option<String>) -> Result<()> {
    append_event(
        path,
        &Event::StepDone {
            id: id.into(),
            result,
            ts: now_utc(),
        },
    )
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenStep {
    pub id: String,
    pub step_type: String,
    pub target: String,
    pub actor: String,
    pub started_at: String,
}

/// Steps with a start and no done — the crash residue to recover.
pub fn open_steps(events: &[Event]) -> Vec<OpenStep> {
    let mut open: BTreeMap<String, OpenStep> = BTreeMap::new();
    for e in events {
        match e {
            Event::StepStart {
                id,
                step_type,
                target,
                actor,
                ts,
            } => {
                open.insert(
                    id.clone(),
                    OpenStep {
                        id: id.clone(),
                        step_type: step_type.clone(),
                        target: target.clone(),
                        actor: actor.clone(),
                        started_at: ts.clone(),
                    },
                );
            }
            Event::StepDone { id, .. } => {
                open.remove(id);
            }
            // A phase transition touches no step's open/closed state.
            Event::Phase { .. } => {}
        }
    }
    open.into_values().collect()
}

/// The campaign phase, derived from the journal: the `value` of the LAST
/// `phase` event wins; with none present the phase is `"A"` (the campaign's
/// opening phase). This is the machine-readable derivation PROP-043 §state
/// requires — the phase is never read from the plan's Markdown.
pub fn derive_phase(events: &[Event]) -> String {
    events
        .iter()
        .rev()
        .find_map(|e| match e {
            Event::Phase { value, .. } => Some(value.clone()),
            _ => None,
        })
        .unwrap_or_else(|| "A".to_string())
}

/// Render RESUME.md — the one file a cold session reads first.
pub fn render_resume(
    campaign_id: &str,
    phase: &str,
    counters: &serde_json::Value,
    open: &[OpenStep],
    next_hint: &str,
) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "# RESUME — campaign `{campaign_id}`\n\n_Generated {} — do not edit; \
         regenerate with `vibe progress resume`._\n\n",
        now_utc()
    ));
    s.push_str(&format!("**Phase:** {phase}\n\n"));
    s.push_str(&format!(
        "**Where we are:** {}\n\n",
        serde_json::to_string(counters).unwrap_or_else(|_| "{}".into())
    ));
    s.push_str("## Unfinished (recover FIRST)\n\n");
    if open.is_empty() {
        s.push_str("Nothing open — the journal is clean.\n");
    } else {
        for st in open {
            s.push_str(&format!(
                "- step `{}` ({}, target `{}`, actor {}, started {}) is OPEN → \
                 `git restore` the files it touched, then redo the step.\n",
                st.id, st.step_type, st.target, st.actor, st.started_at
            ));
        }
    }
    s.push_str("\n## Next\n\n");
    s.push_str(next_hint);
    s.push_str(
        "\n\n## Rules of the road\n\n\
         - Every session: read this file, recover Unfinished, then take Next.\n\
         - Close the current step before ending a session; never start one you \
         cannot finish.\n\
         - Plan: `spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md` §4–§5. \
         Contract: `spec/modules/vibe-progress/PROP-043-progress-markup.md`.\n\
         - Dashboard: `node tools/progress-dashboard/serve.mjs`.\n",
    );
    s
}

pub fn write_resume(path: &Path, body: &str) -> Result<()> {
    write_atomic(path, body.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journal_survives_torn_tail_and_tracks_open_steps() {
        let dir = tempfile::tempdir().expect("tempdir");
        let j = dir.path().join("journal.jsonl");
        start_step(&j, "b-001", "mark-file", "spec/a.md", "fable").expect("start");
        done_step(&j, "b-001", Some("ok".into())).expect("done");
        start_step(&j, "b-002", "mark-file", "spec/b.md", "fable").expect("start");
        // Simulate a torn tail from a killed writer.
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&j)
            .expect("open");
        f.write_all(b"{\"kind\":\"step-done\",\"id\":\"b-0")
            .expect("torn");
        drop(f);
        let events = read_journal(&j).expect("read");
        assert_eq!(events.len(), 3, "torn tail discarded");
        let open = open_steps(&events);
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].id, "b-002");
    }

    #[test]
    fn phase_absent_defaults_to_a() {
        // No events at all, and a journal of only step events, both read "A".
        assert_eq!(derive_phase(&[]), "A");
        let dir = tempfile::tempdir().expect("tempdir");
        let j = dir.path().join("journal.jsonl");
        start_step(&j, "b-001", "mark-file", "spec/a.md", "fable").expect("start");
        done_step(&j, "b-001", Some("ok".into())).expect("done");
        let events = read_journal(&j).expect("read");
        assert_eq!(
            derive_phase(&events),
            "A",
            "no phase event ⇒ opening phase A"
        );
    }

    #[test]
    fn phase_last_event_wins() {
        let dir = tempfile::tempdir().expect("tempdir");
        let j = dir.path().join("journal.jsonl");
        append_event(
            &j,
            &Event::Phase {
                value: "B".into(),
                ts: "2026-07-24T00:00:00Z".into(),
            },
        )
        .expect("phase b");
        start_step(&j, "c-001", "mark-file", "spec/a.md", "fable").expect("start");
        append_event(
            &j,
            &Event::Phase {
                value: "C".into(),
                ts: "2026-07-24T01:00:00Z".into(),
            },
        )
        .expect("phase c");
        let events = read_journal(&j).expect("read");
        assert_eq!(derive_phase(&events), "C", "the later phase event wins");
    }

    #[test]
    fn phase_survives_torn_tail() {
        let dir = tempfile::tempdir().expect("tempdir");
        let j = dir.path().join("journal.jsonl");
        append_event(
            &j,
            &Event::Phase {
                value: "B".into(),
                ts: "2026-07-24T00:00:00Z".into(),
            },
        )
        .expect("phase");
        // A torn tail (writer killed mid-line) must not erase the logged phase.
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&j)
            .expect("open");
        f.write_all(b"{\"kind\":\"phase\",\"value\":\"C")
            .expect("torn");
        drop(f);
        let events = read_journal(&j).expect("read");
        assert_eq!(events.len(), 1, "torn tail discarded");
        assert_eq!(derive_phase(&events), "B");
    }

    #[test]
    fn journal_tolerates_unknown_event_kinds() {
        let dir = tempfile::tempdir().expect("tempdir");
        let j = dir.path().join("journal.jsonl");
        start_step(&j, "b-001", "mark-file", "spec/a.md", "fable").expect("start");
        // A complete event of a kind a newer writer added that we do not model.
        // Forward compatibility: it is skipped, not read as a torn tail — the
        // known events after it must still read.
        {
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(&j)
                .expect("open");
            f.write_all(b"{\"kind\":\"future-thing\",\"whatever\":42}\n")
                .expect("unknown");
        }
        done_step(&j, "b-001", Some("ok".into())).expect("done");
        append_event(
            &j,
            &Event::Phase {
                value: "B".into(),
                ts: "2026-07-24T00:00:00Z".into(),
            },
        )
        .expect("phase");
        let events = read_journal(&j).expect("read");
        assert_eq!(
            events.len(),
            3,
            "unknown line skipped; known events survive"
        );
        assert!(open_steps(&events).is_empty(), "b-001 opened and closed");
        assert_eq!(derive_phase(&events), "B");
    }
}
