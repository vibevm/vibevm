//! The paid agent step under an operation stop.
//!
//! The backend here stands in for a stop-capable transport: it records the
//! paid call, then plays the stop at one exact boundary. The project tree read
//! back afterwards is the evidence for what was (not) written or restored.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use vibe_core::cancellation::CancellationToken;

use super::support::{PROMPT, RecordingBackend, TWO_OUTPUTS, TWO_OUTPUTS_RESULT, context, row};
use super::{KEY, tree};
use crate::agent::{
    AgentBackend, AgentCompletion, AgentError, AgentRequest, PromptRequest, ResolvedPrompt,
    execute, prepare,
};

/// Where the backend plays the stop.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Stop {
    Never,
    /// The stop is acknowledged while the request is in flight; the backend
    /// closes the request, exactly as the production transport does.
    InFlight,
    /// The stop is requested just as the provider's answer arrives.
    OnAnswer,
}

struct Controlled {
    inner: RecordingBackend,
    stop: Stop,
}

impl AgentBackend for Controlled {
    fn resolve_prompt(&self, request: &PromptRequest) -> Result<ResolvedPrompt, String> {
        self.inner.resolve_prompt(request)
    }

    fn complete(&self, _request: &AgentRequest) -> Result<AgentCompletion, String> {
        Err("a controlled execution must not take the unsupervised path".into())
    }

    fn supports_control(&self) -> bool {
        true
    }

    fn complete_controlled(
        &self,
        request: &AgentRequest,
        cancellation: &CancellationToken,
    ) -> Result<AgentCompletion, String> {
        let answer = self.inner.complete(request);
        match self.stop {
            Stop::Never => answer,
            Stop::InFlight => {
                cancellation.cancel();
                cancellation
                    .checkpoint()
                    .map_err(|error| error.to_string())?;
                answer
            }
            Stop::OnAnswer => {
                cancellation.cancel();
                answer
            }
        }
    }
}

fn controlled(stop: Stop) -> Controlled {
    Controlled {
        inner: RecordingBackend::answering(TWO_OUTPUTS_RESULT),
        stop,
    }
}

fn run(
    project: &Path,
    backend: &Controlled,
    token: &CancellationToken,
) -> Result<super::Reply, AgentError> {
    let row = row(TWO_OUTPUTS, PROMPT);
    let context = context(project, &row);
    let prepared = prepare(backend, &row, &context)
        .unwrap()
        .expect("an agent row prepares");
    execute(backend, KEY, &context, &prepared, Some(token))
}

/// A stop requested once `trigger` exists under the project — the observable
/// moment between two output publications.
fn stop_once(project: &Path, relative: &str, also: fn(&Path)) -> CancellationToken {
    let watched: PathBuf = project.join(relative);
    CancellationToken::new().with_poll(Arc::new(move || {
        let present = watched.exists();
        if present {
            also(&watched);
        }
        present
    }))
}

fn stage_of(error: &AgentError) -> (&str, &[String], &[String]) {
    match error {
        AgentError::Cancelled {
            stage,
            restored,
            retained,
            ..
        } => (stage, restored, retained),
        other => panic!("expected a cancellation, got {other}"),
    }
}

#[test]
fn a_stop_while_the_request_is_in_flight_writes_nothing() {
    let project = tempfile::tempdir().unwrap();
    let backend = controlled(Stop::InFlight);
    let token = CancellationToken::new();
    let error = run(project.path(), &backend, &token).unwrap_err();
    let (stage, restored, retained) = stage_of(&error);
    assert!(stage.contains("in flight"), "{stage}");
    assert!(restored.is_empty() && retained.is_empty());
    assert_eq!(backend.inner.calls(), 1, "one paid call, never retried");
    assert!(tree(project.path()).is_empty());
    assert_eq!(token.publication_state(), "not-started");
    let rendered = error.to_string();
    assert!(
        rendered.contains("never retries a paid request"),
        "{rendered}"
    );
    assert!(
        rendered.contains("no declared output was published"),
        "{rendered}"
    );
}

#[test]
fn a_stop_arriving_with_the_answer_writes_nothing() {
    let project = tempfile::tempdir().unwrap();
    let backend = controlled(Stop::OnAnswer);
    let token = CancellationToken::new();
    let error = run(project.path(), &backend, &token).unwrap_err();
    let (stage, _, _) = stage_of(&error);
    assert!(stage.contains("before any declared output"), "{stage}");
    assert_eq!(backend.inner.calls(), 1);
    assert!(tree(project.path()).is_empty());
    assert!(token.acknowledged());
}

#[test]
fn a_stop_already_acknowledged_never_reaches_the_paid_call() {
    let project = tempfile::tempdir().unwrap();
    let backend = controlled(Stop::Never);
    let token = CancellationToken::new();
    token.cancel();
    let error = run(project.path(), &backend, &token).unwrap_err();
    let (stage, _, _) = stage_of(&error);
    assert!(stage.contains("before the provider request"), "{stage}");
    assert_eq!(backend.inner.calls(), 0);
    assert!(tree(project.path()).is_empty());
}

#[test]
fn a_stop_between_outputs_restores_the_first_and_its_directory() {
    let project = tempfile::tempdir().unwrap();
    let backend = controlled(Stop::Never);
    let token = stop_once(project.path(), "docs/guide.md", |_| {});
    let error = run(project.path(), &backend, &token).unwrap_err();
    let (stage, restored, retained) = stage_of(&error);
    assert_eq!(stage, "between declared outputs");
    assert_eq!(restored, ["docs/guide.md"]);
    assert!(retained.is_empty(), "{retained:?}");
    assert!(tree(project.path()).is_empty());
    assert!(
        !project.path().join("docs").exists(),
        "the directory the publication created is removed while empty"
    );
    // A clean inverse leaves nothing the enclosing operation must recover.
    assert_eq!(token.publication_state(), "not-started");
}

#[test]
fn a_stop_between_outputs_restores_a_replaced_file_to_its_before_image() {
    let project = tempfile::tempdir().unwrap();
    fs::create_dir_all(project.path().join("docs")).unwrap();
    fs::write(project.path().join("docs/guide.md"), "operator bytes\n").unwrap();
    let backend = controlled(Stop::Never);
    let watched = project.path().join("docs/guide.md");
    let token = CancellationToken::new().with_poll(Arc::new(move || {
        fs::read_to_string(&watched).is_ok_and(|text| text == "# Guide\n")
    }));
    let error = run(project.path(), &backend, &token).unwrap_err();
    let (_, restored, _) = stage_of(&error);
    assert_eq!(restored, ["docs/guide.md"]);
    assert_eq!(
        fs::read_to_string(project.path().join("docs/guide.md")).unwrap(),
        "operator bytes\n"
    );
    assert_eq!(tree(project.path()), ["docs/guide.md"]);
}

#[test]
fn a_stop_after_the_last_output_still_finds_every_output_reversible() {
    let project = tempfile::tempdir().unwrap();
    let backend = controlled(Stop::Never);
    let token = stop_once(project.path(), "docs/reference.md", |_| {});
    let error = run(project.path(), &backend, &token).unwrap_err();
    let (stage, restored, _) = stage_of(&error);
    assert!(stage.contains("after the last declared output"), "{stage}");
    assert_eq!(restored, ["docs/reference.md", "docs/guide.md"]);
    assert!(tree(project.path()).is_empty());
}

#[test]
fn a_foreign_replacement_is_left_untouched_and_named() {
    let project = tempfile::tempdir().unwrap();
    let backend = controlled(Stop::Never);
    let token = stop_once(project.path(), "docs/guide.md", |path| {
        if fs::read_to_string(path).is_ok_and(|text| text == "# Guide\n") {
            fs::remove_file(path).unwrap();
            fs::write(path, "foreign\n").unwrap();
        }
    });
    let error = run(project.path(), &backend, &token).unwrap_err();
    let (_, restored, retained) = stage_of(&error);
    assert!(restored.is_empty(), "{restored:?}");
    assert_eq!(retained, ["docs/guide.md"]);
    assert_eq!(
        fs::read_to_string(project.path().join("docs/guide.md")).unwrap(),
        "foreign\n"
    );
    assert_eq!(token.publication_state(), "recovery-required");
    assert!(error.to_string().contains("left untouched"), "{error}");
}

#[test]
fn a_controlled_run_without_a_stop_publishes_exactly_what_an_unsupervised_run_does() {
    let supervised = tempfile::tempdir().unwrap();
    let backend = controlled(Stop::Never);
    let token = CancellationToken::new();
    let reply = run(supervised.path(), &backend, &token).unwrap();

    let unsupervised = tempfile::tempdir().unwrap();
    let plain = RecordingBackend::answering(TWO_OUTPUTS_RESULT);
    let (_, plain_reply) =
        super::prepared_run(unsupervised.path(), TWO_OUTPUTS, PROMPT, &plain).unwrap();

    assert_eq!(reply.artifacts.len(), plain_reply.artifacts.len());
    assert_eq!(reply.message, plain_reply.message);
    assert_eq!(tree(supervised.path()), tree(unsupervised.path()));
    for path in ["docs/guide.md", "docs/reference.md"] {
        assert_eq!(
            fs::read(supervised.path().join(path)).unwrap(),
            fs::read(unsupervised.path().join(path)).unwrap()
        );
    }
    assert_eq!(backend.inner.calls(), 1);
    assert!(!token.acknowledged());
}
