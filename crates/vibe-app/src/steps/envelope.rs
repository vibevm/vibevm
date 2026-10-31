//! The envelope of one agent run (`PROP-064#envelope`, `#answer`,
//! `#parking`, `#record`): `prompt.md` and `context.json` in the step's run
//! directory `<work>/steps/<id>/`, the `result.json` an agent may write
//! back, the outbox task of a parked step, and the copy kept in the run
//! record.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#envelope");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#answer");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#record");

use std::fs;
use std::path::{Path, PathBuf};

use vibe_wire::generated::application::e1::agent_hook as hook;
use vibe_wire::generated::application::e1::agent_hook_result as answer;

use crate::agents::{AgentError, RunMode, io};
use crate::index::PackageCoordinate;

pub(crate) const HOOK_PROTOCOL: &str = "vibe-agent-hook/1";
pub(crate) const RESULT_PROTOCOL: &str = "vibe-agent-hook-result/1";

/// The files of one step's run directory.
pub(crate) struct Envelope {
    pub run: PathBuf,
    pub prompt: PathBuf,
    pub context: PathBuf,
    pub result: PathBuf,
    pub transcript: PathBuf,
}

impl Envelope {
    pub fn at(work: &Path, step: &str) -> Self {
        let run = work.join("steps").join(step);
        Self {
            prompt: run.join("prompt.md"),
            context: run.join("context.json"),
            result: run.join("result.json"),
            transcript: run.join("transcript.log"),
            run,
        }
    }

    /// Write `prompt.md` and `context.json`; returns the context text.
    pub fn write(&self, prompt: &str, context: &hook::AgentHook) -> Result<String, AgentError> {
        fs::create_dir_all(&self.run).map_err(|error| io("creating", &self.run, error))?;
        fs::write(&self.prompt, prompt).map_err(|error| io("writing", &self.prompt, error))?;
        let mut text = serde_json::to_string_pretty(context).map_err(|error| {
            io(
                "serialising",
                &self.context,
                std::io::Error::other(error.to_string()),
            )
        })?;
        text.push('\n');
        fs::write(&self.context, &text).map_err(|error| io("writing", &self.context, error))?;
        Ok(text)
    }

    /// Remove an answer left by an earlier attempt before a new one.
    pub fn clear_answer(&self) -> Result<(), AgentError> {
        match fs::remove_file(&self.result) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                Err(io("removing", &self.result, error))
            }
            _ => Ok(()),
        }
    }

    /// The agent's `result.json`: `None` when it wrote none, `Err` with the
    /// reason when it is not a valid answer.
    pub fn answer(&self) -> Option<Result<answer::AgentHookResult, String>> {
        let bytes = fs::read(&self.result).ok()?;
        Some(
            serde_json::from_slice::<answer::AgentHookResult>(&bytes)
                .map_err(|error| format!("result.json is not a {RESULT_PROTOCOL} answer: {error}"))
                .and_then(|answer| match answer.protocol.as_deref() {
                    Some(protocol) if protocol != RESULT_PROTOCOL => Err(format!(
                        "result.json declares protocol `{protocol}`, not `{RESULT_PROTOCOL}`"
                    )),
                    _ => Ok(answer),
                }),
        )
    }

    /// Copy the envelope, transcript and answer into the run record's step
    /// directory, so they survive the work directory; best effort.
    pub fn keep(&self, record_step: &Path) {
        if fs::create_dir_all(record_step).is_err() {
            return;
        }
        for file in [&self.prompt, &self.context, &self.transcript, &self.result] {
            if let Some(name) = file.file_name()
                && file.is_file()
            {
                let _ = fs::copy(file, record_step.join(name));
            }
        }
    }
}

/// Everything `context.json` states besides the paths of the envelope.
pub(crate) struct ContextFacts<'a> {
    pub package: &'a PackageCoordinate,
    pub option: &'a str,
    pub step: &'a str,
    pub mode: RunMode,
    pub work: &'a Path,
    pub agent_work: &'a Path,
    pub package_root: &'a Path,
    pub app_root: Option<&'a Path>,
    pub features: &'a [String],
    /// `(id, kind, outcome)` of every step reached before this one.
    pub earlier: Vec<(String, String, String)>,
}

/// The `vibe-agent-hook/1` document of one run.
pub(crate) fn context(envelope: &Envelope, facts: &ContextFacts<'_>) -> hook::AgentHook {
    let text = |path: &Path| path.to_string_lossy().into_owned();
    hook::AgentHook {
        protocol: HOOK_PROTOCOL.into(),
        vibe: env!("CARGO_PKG_VERSION").into(),
        package: hook::Package {
            group: facts.package.group.clone(),
            name: facts.package.name.clone(),
            version: facts.package.version.clone(),
        },
        option: facts.option.into(),
        step: facts.step.into(),
        mode: match facts.mode {
            RunMode::Interactive => hook::Mode::Interactive,
            RunMode::Unattended => hook::Mode::Unattended,
        },
        paths: hook::Paths {
            run: text(&envelope.run),
            prompt_file: text(&envelope.prompt),
            context_file: text(&envelope.context),
            result_file: text(&envelope.result),
            work: text(facts.agent_work),
            sources: text(&facts.work.join("sources")),
            out: text(&facts.work.join("out")),
            package: text(facts.package_root),
            app_root: facts.app_root.map(text),
        },
        features: facts.features.to_vec(),
        steps: facts
            .earlier
            .iter()
            .map(|(id, kind, outcome)| hook::Earlier {
                id: id.clone(),
                kind: kind.clone(),
                outcome: outcome.clone(),
            })
            .collect(),
    }
}

/// Write the parked task `<outbox>/<step>.md` for the hosting agent
/// (`PROP-064#parking`, `PROP-054#AGENT-HANDSHAKE`).
pub(crate) fn park(
    outbox: &Path,
    step: &str,
    title: &str,
    prompt: &str,
    context: &str,
    envelope: &Envelope,
    resume: &str,
) -> Result<PathBuf, AgentError> {
    fs::create_dir_all(outbox).map_err(|error| io("creating", outbox, error))?;
    let task = outbox.join(format!("{step}.md"));
    let body = format!(
        "# vibe agent task: `{step}` — {title}\n\n\
         An installation step was handed to you because vibe runs inside your session.\n\n\
         ## Prompt\n\n{prompt}\n\n\
         ## Context\n\n```json\n{context}```\n\n\
         ## Answer contract\n\n\
         1. Perform the prompt above; the working directory is the `work` path of the context.\n\
         2. Write `{result}`:\n\n\
         ```json\n{{\"protocol\": \"{RESULT_PROTOCOL}\", \"status\": \"ok\", \"message\": \"what was done\"}}\n```\n\n\
         \x20  Use `\"status\": \"fail\"` with a message when the task cannot be done.\n\
         3. Continue the installation: `{resume}`\n",
        result = envelope.result.display(),
    );
    fs::write(&task, body).map_err(|error| io("writing", &task, error))?;
    Ok(task)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_answer_is_read_only_when_valid() {
        let dir = tempfile::tempdir().unwrap();
        let envelope = Envelope::at(dir.path(), "fix");
        assert!(envelope.answer().is_none());
        fs::create_dir_all(&envelope.run).unwrap();
        fs::write(
            &envelope.result,
            r#"{"status":"fail","message":"no","extra":1}"#,
        )
        .unwrap();
        let answer = envelope.answer().unwrap().unwrap();
        assert_eq!(answer.status, answer::Status::Fail);
        fs::write(&envelope.result, r#"{"status":"maybe"}"#).unwrap();
        assert!(envelope.answer().unwrap().is_err());
        fs::write(&envelope.result, r#"{"protocol":"other/1","status":"ok"}"#).unwrap();
        assert!(envelope.answer().unwrap().unwrap_err().contains("other/1"));
        envelope.clear_answer().unwrap();
        envelope.clear_answer().unwrap();
        assert!(envelope.answer().is_none());
    }
}
