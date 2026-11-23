//! A repeated finished child row is drawn once (PROP-067 `#collapse`).

use super::*;
use indicatif::InMemoryTerm;
use std::io::Write;
use vibe_core::progress::Progress;

/// The captured bytes of one renderer's output.
#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl Write for Captured {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .map_err(|_| io::Error::other("test writer poisoned"))?
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Captured {
    /// The renderer's captured output as UTF-8.
    fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

/// Two finished and one failed child of the same label under one parent.
fn walk(progress: &Progress) {
    let parent = progress.task("Discovering versions for org.example/lib");
    let first = parent.progress().task("Reading Git metadata");
    first.finish();
    let second = parent.progress().task("Reading Git metadata");
    second.finish();
    let third = parent.progress().task("Reading Git metadata");
    third.fail("git exited exit status: 128");
    parent.finish();
    let other = progress.task("Discovering versions for org.example/other");
    other.progress().task("Reading Git metadata").finish();
    other.finish();
}

#[test]
fn repeated_child_rows_collapse_on_a_plain_stream() {
    let writer = Captured::default();
    let renderer = ProgressRenderer::plain(Box::new(writer.clone()), false);
    walk(&Progress::new(renderer));
    let text = writer.text();
    assert_eq!(text.matches("✓ └ Reading Git metadata").count(), 2);
    assert_eq!(text.matches("✗ └ Reading Git metadata").count(), 1);
}

#[test]
fn repeated_child_rows_collapse_on_a_terminal() {
    let terminal = InMemoryTerm::new(12, 100);
    let renderer = ProgressRenderer::interactive_target(
        ProgressDrawTarget::term_like(Box::new(terminal.clone())),
        false,
    );
    walk(&Progress::new(renderer.clone()));
    renderer.suspend(|| ());
    let screen = terminal.contents();
    assert_eq!(
        screen.matches("✓ └ Reading Git metadata").count(),
        2,
        "{screen:?}"
    );
    assert_eq!(
        screen.matches("✗ └ Reading Git metadata").count(),
        1,
        "{screen:?}"
    );
}

#[test]
fn repeated_child_rows_discard_only_the_suppressed_task_log() {
    let terminal = InMemoryTerm::new(12, 100);
    let renderers = [
        ProgressRenderer::plain(Box::new(Captured::default()), false),
        ProgressRenderer::interactive_target(
            ProgressDrawTarget::term_like(Box::new(terminal)),
            false,
        ),
    ];
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("output.log");
    std::fs::write(&path, "retained evidence\n").unwrap();
    for renderer in renderers {
        let progress = Progress::new(renderer.clone());
        let parent = progress.task("Discovering versions");
        parent.progress().task("Reading Git metadata").finish();
        let duplicate = parent.progress().task("Reading Git metadata");
        let neighbor = parent.progress().task("Other task");
        let completed_id = duplicate.id();
        renderer.note_task_log(completed_id, &path);
        renderer.note_task_log(neighbor.id(), &path);
        assert!(renderer.logs.has_task_log(completed_id));
        assert!(renderer.logs.has_task_log(neighbor.id()));

        duplicate.finish();

        assert!(!renderer.logs.has_task_log(completed_id));
        assert!(renderer.logs.has_task_log(neighbor.id()));
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "retained evidence\n"
        );
        neighbor.finish();
        parent.finish();
    }
}
