//! The agentic relay — vibevm borrowing the calling agent's LLM
//! (PROP-018 §2.2, §2.7, §2.10).
//!
//! A reasoning operation that runs under the relay backend does not act: it
//! composes an [`Intent`] (a prompt) and parks it in the project's
//! `.vibe/agentic/` mailbox for the calling agent to drain with `vibe
//! command` and execute on its own LLM. This is the MVP realisation of the
//! pluggable inference backend (PROP-018 §2.2) — the only backend today;
//! a built-in `vibe-llm` backend is far-backlog (§6).

specmark::scope!("spec://vibevm/common/PROP-018#relay");

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;
use specmark::spec;

/// Which inference backend an operation can run on (PROP-018 §2.3).
/// Affinity is a property of the *work*, not a user choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[spec(implements = "spec://vibevm/common/PROP-018#affinity")]
pub enum Affinity {
    /// Needs the calling agent's LLM via the relay — no built-in engine yet.
    AgenticOnly,
    /// Pure algorithm, or the future built-in engine; needs no agent.
    StandaloneOnly,
    /// Expressible on either backend.
    Both,
}

/// A unit of reasoning vibevm hands back to the calling agent (PROP-018
/// §2.7) — a prompt with light frontmatter, rendered to markdown for the
/// `.vibe/agentic/command.md` mailbox.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[spec(implements = "spec://vibevm/common/PROP-018#relay")]
pub struct Intent {
    /// The command that produced it, e.g. `"agentic explain"`.
    pub source: String,
    /// One-line human title.
    pub title: String,
    /// The instruction for the agent to carry out, as markdown.
    pub body: String,
}

impl Intent {
    /// Render to the mailbox markdown form: a frontmatter block carrying
    /// the pending marker and the source, then the title and body.
    pub fn to_markdown(&self) -> String {
        format!(
            "---\nvibevm-intent: pending\nsource: {}\n---\n\n# {}\n\n{}\n",
            self.source, self.title, self.body
        )
    }
}

/// The outcome of submitting an [`Intent`] to a backend (PROP-018 §2.2).
#[derive(Debug, Clone)]
pub enum BackendOutcome {
    /// Parked for the calling agent to execute; carries a human pointer.
    Delegated { pointer: String },
    /// Executed in-process by a built-in engine; carries the result.
    /// Unreachable in the MVP (no engine) — present for forward-compat.
    Completed(String),
}

/// Where an operation's reasoning runs (PROP-018 §2.2). One impl today,
/// [`RelayBackend`]; a built-in `vibe-llm` backend is far-backlog (§6).
///
/// ```
/// use vibe_mcp::agentic::{BackendOutcome, InferenceBackend, Intent, RelayBackend};
///
/// let project = tempfile::tempdir().unwrap();
/// // The relay backend parks an intent for the calling agent instead of
/// // reasoning in-process — vibevm has no engine of its own yet.
/// let backend = RelayBackend::for_project(project.path());
/// let intent = Intent {
///     source: "agentic explain".into(),
///     title: "Explain this project".into(),
///     body: "Summarise the README in three paragraphs.".into(),
/// };
/// match backend.submit(&intent).unwrap() {
///     BackendOutcome::Delegated { pointer } => assert!(pointer.contains("vibe command")),
///     BackendOutcome::Completed(_) => unreachable!("the relay never completes in-process"),
/// }
/// ```
pub trait InferenceBackend {
    /// Hand a reasoning intent to the backend.
    fn submit(&self, intent: &Intent) -> Result<BackendOutcome>;
}

/// The agentic relay backend: it does not reason — it parks the intent in
/// the project's `.vibe/agentic/` mailbox for the calling agent to drain
/// with `vibe command` (PROP-018 §2.7).
#[derive(Debug, Clone)]
#[spec(implements = "spec://vibevm/common/PROP-018#pluggable-backend")]
pub struct RelayBackend {
    dir: PathBuf,
}

impl RelayBackend {
    /// Bind the relay to a project root — the mailbox is
    /// `<root>/.vibe/agentic/`.
    pub fn for_project(project_root: &Path) -> Self {
        RelayBackend {
            dir: relay_dir(project_root),
        }
    }
}

impl InferenceBackend for RelayBackend {
    fn submit(&self, intent: &Intent) -> Result<BackendOutcome> {
        park_intent(&self.dir, intent)?;
        Ok(BackendOutcome::Delegated {
            pointer: "intent queued — run `vibe command` to fetch it, then carry it out"
                .to_string(),
        })
    }
}

/// The relay directory for a project root: `<root>/.vibe/agentic`
/// (PROP-018 §3).
pub fn relay_dir(project_root: &Path) -> PathBuf {
    project_root.join(".vibe").join("agentic")
}

const MAILBOX: &str = "command.md";
const ARCHIVE: &str = "command.done.md";

/// Park an intent in the single-slot mailbox (PROP-018 §2.7). Ensures the
/// relay dir and a self-contained `*` gitignore exist — so the transient
/// state never reaches git even if the project has no `.vibe/.gitignore` —
/// then writes `command.md`.
#[spec(implements = "spec://vibevm/common/PROP-018#relay")]
pub fn park_intent(dir: &Path, intent: &Intent) -> Result<PathBuf> {
    fs::create_dir_all(dir).with_context(|| format!("creating relay dir `{}`", dir.display()))?;
    // A `*` here ignores every file in the dir, this `.gitignore` included.
    let ignore = dir.join(".gitignore");
    if !ignore.exists() {
        let _ = fs::write(&ignore, "*\n");
    }
    let path = dir.join(MAILBOX);
    fs::write(&path, intent.to_markdown())
        .with_context(|| format!("writing relay mailbox `{}`", path.display()))?;
    Ok(path)
}

/// Drain the pending intent (PROP-018 §2.7): return its markdown and move
/// it to `command.done.md` (status flipped to `done`), emptying the slot.
/// `None` when nothing pends — re-running after a drain is a clean no-op.
#[spec(implements = "spec://vibevm/common/PROP-018#relay")]
pub fn drain_intent(dir: &Path) -> Result<Option<String>> {
    let path = dir.join(MAILBOX);
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("reading relay mailbox `{}`", path.display()))?;
    let archived = content.replacen("vibevm-intent: pending", "vibevm-intent: done", 1);
    fs::write(dir.join(ARCHIVE), archived)
        .with_context(|| format!("archiving relay intent in `{}`", dir.display()))?;
    fs::remove_file(&path)
        .with_context(|| format!("clearing relay mailbox `{}`", path.display()))?;
    Ok(Some(content))
}

/// The affinity of the `explain` operation: agentic-only until a built-in
/// inference backend exists (PROP-018 §2.3, §2.10).
pub const EXPLAIN_AFFINITY: Affinity = Affinity::AgenticOnly;

/// Compose the `vibe agentic explain` intent (PROP-018 §2.10). The op does
/// no LLM work and reads no file *content* — it only inspects which of
/// `README.md` / `vibe.toml` exist to tailor the instruction the agent
/// will execute.
#[spec(implements = "spec://vibevm/common/PROP-018#explain")]
pub fn explain_intent(project_root: &Path) -> Intent {
    let readme = ["README.md", "readme.md", "Readme.md"]
        .into_iter()
        .find(|f| project_root.join(f).exists());
    let has_manifest = project_root.join("vibe.toml").exists();

    let mut detected = Vec::new();
    if let Some(r) = readme {
        detected.push(r.to_string());
    }
    if has_manifest {
        detected.push("vibe.toml".to_string());
    }
    let detected_line = if detected.is_empty() {
        "Detected in this project: neither README.md nor vibe.toml found.".to_string()
    } else {
        format!("Detected in this project: {}.", detected.join(", "))
    };

    let body = format!(
        "{detected_line}\n\n\
         Explain this project to a developer seeing it for the first time, in \
         at most three short paragraphs of plain prose.\n\n\
         Work from these sources, in priority order:\n\
         1. `README.md` at the project root — read it and summarise what the \
         project is and does.\n\
         2. `vibe.toml`, if present — fold in what its structure reveals: the \
         package `kind`, what the project `requires`, and what it `provides`.\n\n\
         If `README.md` is absent, say so in one clause and explain from \
         `vibe.toml` alone. If neither is present, state that the project \
         carries no description and stop. Do not invent features the sources \
         do not support — prefer \"the sources don't say\" over guessing. Lead \
         with the explanation; no preamble."
    );

    Intent {
        source: "agentic explain".to_string(),
        title: "Explain this project".to_string(),
        body,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use specmark::verifies;

    #[test]
    fn intent_renders_pending_frontmatter() {
        let i = Intent {
            source: "agentic explain".to_string(),
            title: "T".to_string(),
            body: "B".to_string(),
        };
        let md = i.to_markdown();
        assert!(md.starts_with("---\nvibevm-intent: pending\n"));
        assert!(md.contains("source: agentic explain"));
        assert!(md.contains("# T"));
        assert!(md.ends_with("B\n"));
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-018#relay", r = 4)]
    fn park_then_drain_is_a_one_shot_mailbox() {
        let proj = tempfile::tempdir().unwrap();
        let dir = relay_dir(proj.path());
        let intent = Intent {
            source: "agentic explain".to_string(),
            title: "Explain this project".to_string(),
            body: "do the thing".to_string(),
        };

        // Park → mailbox + gitignore exist.
        park_intent(&dir, &intent).unwrap();
        assert!(dir.join("command.md").is_file());
        assert!(dir.join(".gitignore").is_file());

        // Drain → returns the pending markdown, archives it, empties slot.
        let drained = drain_intent(&dir).unwrap().expect("a pending intent");
        assert!(drained.contains("vibevm-intent: pending"));
        assert!(drained.contains("do the thing"));
        assert!(!dir.join("command.md").exists());
        let archived = std::fs::read_to_string(dir.join("command.done.md")).unwrap();
        assert!(archived.contains("vibevm-intent: done"));

        // Second drain → clean no-op.
        assert!(drain_intent(&dir).unwrap().is_none());
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-018#pluggable-backend", r = 2)]
    fn relay_backend_delegates() {
        let proj = tempfile::tempdir().unwrap();
        let backend = RelayBackend::for_project(proj.path());
        let intent = explain_intent(proj.path());
        match backend.submit(&intent).unwrap() {
            BackendOutcome::Delegated { pointer } => assert!(pointer.contains("vibe command")),
            BackendOutcome::Completed(_) => panic!("relay never completes in-process"),
        }
        assert!(relay_dir(proj.path()).join("command.md").is_file());
    }

    #[test]
    #[verifies("spec://vibevm/common/PROP-018#explain", r = 4)]
    fn explain_intent_tailors_to_detected_sources() {
        let proj = tempfile::tempdir().unwrap();
        // Neither file present.
        let none = explain_intent(proj.path());
        assert!(none.body.contains("neither README.md nor vibe.toml"));

        // With a README + manifest, both are named.
        std::fs::write(proj.path().join("README.md"), "hi").unwrap();
        std::fs::write(proj.path().join("vibe.toml"), "[project]\nname=\"x\"\n").unwrap();
        let both = explain_intent(proj.path());
        assert!(both.body.contains("README.md, vibe.toml"));
        assert_eq!(both.source, "agentic explain");
        assert_eq!(EXPLAIN_AFFINITY, Affinity::AgenticOnly);
    }
}
