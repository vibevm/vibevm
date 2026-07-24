//! The campaign journal and the generated `RESUME.md` (crash-safety law:
//! campaign plan §4; data shapes PROP-043 §7.4).
//!
//! Append-only JSONL; a torn tail line (killed mid-write) is discarded on
//! read. Recovery rule rendered into RESUME.md verbatim: step closed ⇒
//! edits stand; step open ⇒ `git restore` its files and redo.

specmark::scope!("spec://vibevm/modules/vibe-progress/PROP-043#campaign-zone");

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
}

impl Event {
    pub fn id(&self) -> &str {
        match self {
            Event::StepStart { id, .. } | Event::StepDone { id, .. } => id,
        }
    }
}

/// Read the journal, tolerating a torn last line.
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
        match serde_json::from_str::<Event>(line) {
            Ok(e) => out.push(e),
            Err(_) => break, // torn tail — stop here, never guess past it
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
        }
    }
    open.into_values().collect()
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
}
