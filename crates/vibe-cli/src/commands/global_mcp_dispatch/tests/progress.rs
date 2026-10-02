#![cfg(test)]

//! Typed global-install progress regression coverage.

use super::*;
use std::sync::Mutex;
use vibe_core::progress::{ProgressEvent, ProgressEventKind, ProgressObserver};

#[cfg(test)]
#[derive(Default)]
pub(super) struct Recorder(pub(super) Mutex<Vec<ProgressEvent>>);

#[cfg(test)]
impl ProgressObserver for Recorder {
    fn observe(&self, event: ProgressEvent) {
        self.0.lock().unwrap().push(event);
    }
}

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-060#PACKAGE-STAGES")]
fn global_install_reports_nested_typed_stages_and_terminal_success() {
    let harness = Harness::new();
    harness.seed("fpf", "1.0.0");
    let observer = Arc::new(Recorder::default());
    harness
        .run_observed(
            &[
                "install",
                "-g",
                "mcp:ai.lev/fpf",
                "--agent",
                "codex",
                "--server",
                "a,b",
                "--assume-yes",
            ],
            Some(observer.clone()),
        )
        .unwrap();
    let events = observer.0.lock().unwrap();
    let root = events
        .iter()
        .find(|event| {
            matches!(&event.kind,
        ProgressEventKind::Started { label } if label == "Installing global MCP package")
        })
        .unwrap();
    for label in [
        "Preparing user MCP project",
        "Installing packages",
        "Resolving dependencies",
        "Fetching package content",
        "Applying resolved packages",
        "Recording installation state",
        "Registering MCP package in selected agents",
    ] {
        let started = events
            .iter()
            .position(|event| {
                matches!(&event.kind,
            ProgressEventKind::Started { label: actual } if actual == label)
            })
            .unwrap_or_else(|| panic!("missing {label}: {events:?}"));
        let task = events[started].task_id;
        assert!(events[started].parent_id.is_some(), "{label}");
        assert!(
            events
                .iter()
                .skip(started + 1)
                .any(|event| event.task_id == task && event.kind == ProgressEventKind::Finished),
            "unfinished {label}"
        );
    }
    assert!(events.iter().any(|event| event.task_id == root.task_id && event.kind == ProgressEventKind::Finished));
    assert!(!events.iter().any(|event| matches!(
        event.kind,
        ProgressEventKind::Failed { .. } | ProgressEventKind::Stopped
    )));
}

#[test]
fn global_registration_failure_has_failed_root_without_finished_root() {
    let harness = Harness::new();
    harness.seed("fpf", "1.0.0");
    let observer = Arc::new(Recorder::default());
    assert!(
        harness
            .run_observed(
                &[
                    "install",
                    "-g",
                    "mcp:ai.lev/fpf",
                    "--agent",
                    "claude-desktop",
                    "--server",
                    "a,b",
                    "--assume-yes"
                ],
                Some(observer.clone())
            )
            .is_err()
    );
    let events = observer.0.lock().unwrap();
    let root = events
        .iter()
        .find(|event| {
            matches!(&event.kind,
        ProgressEventKind::Started { label } if label == "Installing global MCP package")
        })
        .unwrap();
    assert!(events.iter().any(|event| event.task_id == root.task_id
        && matches!(event.kind, ProgressEventKind::Failed { .. })));
    assert!(!events.iter().any(|event| event.task_id == root.task_id && event.kind == ProgressEventKind::Finished));
}

#[test]
fn quiet_json_and_no_progress_contexts_disable_observation_scope() {
    let harness = Harness::new();
    harness.seed("fpf", "1.0.0");
    for flag in ["--quiet", "--json", "--no-progress"] {
        let observer = Arc::new(Recorder::default());
        harness
            .run_observed(
                &[
                    "install",
                    "-g",
                    "mcp:ai.lev/fpf",
                    "--agent",
                    "codex",
                    "--server",
                    "a,b",
                    "--assume-yes",
                    flag,
                ],
                Some(observer.clone()),
            )
            .unwrap();
        assert!(observer.0.lock().unwrap().is_empty(), "{flag}");
    }
}
