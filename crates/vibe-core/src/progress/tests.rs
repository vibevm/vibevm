use super::*;
use std::sync::Mutex;

#[derive(Default)]
struct RecordingObserver(Mutex<Vec<ProgressEvent>>);

impl ProgressObserver for RecordingObserver {
    fn observe(&self, event: ProgressEvent) {
        self.0.lock().expect("events lock").push(event);
    }
}

#[test]
fn hierarchy_and_explicit_parent_counting_are_observable() {
    let observer = Arc::new(RecordingObserver::default());
    let progress = Progress::new(observer.clone());
    let parent = progress.task("install");
    let child = parent.progress().task("package a");
    child.set_progress(3, Some(7), "files");
    child.finish();
    parent.set_progress(1, Some(1), "packages");
    parent.finish();

    let events = observer.0.lock().expect("events lock");
    assert_eq!(events[0].parent_id, None);
    assert_eq!(events[1].parent_id, Some(parent.id()));
    assert_eq!(events[2].task_id, child.id());
    assert_eq!(
        events[2].kind,
        ProgressEventKind::Progress {
            completed: 3,
            total: Some(7),
            unit: "files".to_string(),
        }
    );
    assert_eq!(events[4].task_id, parent.id());
    assert_eq!(
        events[4].kind,
        ProgressEventKind::Progress {
            completed: 1,
            total: Some(1),
            unit: "packages".to_string(),
        }
    );
}

#[test]
fn dropped_task_is_stopped_and_never_finished() {
    let observer = Arc::new(RecordingObserver::default());
    {
        let task = Progress::new(observer.clone()).task("abandoned");
        task.detail("waiting");
    }
    let events = observer.0.lock().expect("events lock");
    assert!(matches!(
        events.last().map(|event| &event.kind),
        Some(ProgressEventKind::Stopped)
    ));
    assert!(
        !events
            .iter()
            .any(|event| event.kind == ProgressEventKind::Finished)
    );
}

#[test]
fn terminal_outcome_is_emitted_once_and_suppresses_late_updates() {
    let observer = Arc::new(RecordingObserver::default());
    let task = Progress::new(observer.clone()).task("one outcome");
    task.skip("already current");
    task.finish();
    task.detail("too late");
    drop(task);

    let events = observer.0.lock().expect("events lock");
    assert_eq!(events.len(), 2);
    assert!(matches!(events[1].kind, ProgressEventKind::Skipped { .. }));
}

#[test]
fn default_progress_is_a_noop() {
    let task = Progress::default().task("silent");
    assert_eq!(task.id().get(), 0);
    task.set_progress(1, None, "items");
    task.detail("ignored");
    task.fail("ignored");
}

#[test]
fn child_diagnostics_keep_their_typed_severity() {
    let observer = Arc::new(RecordingObserver::default());
    let task = Progress::new(observer.clone()).task("compiler");
    task.diagnostic(ProgressDiagnosticLevel::Warning, "unused input");
    task.finish();

    let events = observer.0.lock().expect("events lock");
    assert_eq!(
        events[1].kind,
        ProgressEventKind::Diagnostic {
            level: ProgressDiagnosticLevel::Warning,
            message: "unused input".to_string(),
        }
    );
}
