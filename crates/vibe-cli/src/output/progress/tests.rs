use super::*;
use indicatif::InMemoryTerm;
use std::sync::MutexGuard;
use vibe_core::progress::Progress;

#[derive(Clone, Default)]
struct SharedWriter(Arc<Mutex<Vec<u8>>>);

impl Write for SharedWriter {
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

impl SharedWriter {
    fn text(&self) -> String {
        String::from_utf8(self.bytes().clone()).expect("UTF-8 progress")
    }

    fn bytes(&self) -> MutexGuard<'_, Vec<u8>> {
        self.0.lock().expect("test output lock")
    }
}

#[test]
fn plain_rendering_has_no_ansi_and_starts_before_delayed_completion() {
    let writer = SharedWriter::default();
    let renderer = ProgressRenderer::plain(Box::new(writer.clone()), false);
    let progress = Progress::new(renderer);
    let task = progress.task("download");
    assert_eq!(writer.text(), "[start] download\n");
    std::thread::sleep(Duration::from_millis(10));
    task.set_progress(250_000_000, Some(300_000_000), "bytes");
    task.finish();

    let text = writer.text();
    assert!(text.contains("238.4 MiB/286.1 MiB"), "{text:?}");
    assert!(!text.contains("250000000"));
    assert!(text.contains("[done] download"));
    assert!(!text.contains('\u{1b}'));
}

#[test]
fn default_and_verbose_detail_are_distinct() {
    let quiet_writer = SharedWriter::default();
    let quiet = Progress::new(ProgressRenderer::plain(
        Box::new(quiet_writer.clone()),
        false,
    ));
    let quiet_task = quiet.task("resolve");
    quiet_task.detail("registry selected");
    quiet_task.diagnostic(ProgressDiagnosticLevel::Warning, "compiler warning");
    quiet_task.finish();
    let quiet_text = quiet_writer.text();
    assert!(!quiet_text.contains("registry selected"));
    assert!(quiet_text.contains("[warning] resolve: compiler warning"));

    let verbose_writer = SharedWriter::default();
    let verbose = Progress::new(ProgressRenderer::plain(
        Box::new(verbose_writer.clone()),
        true,
    ));
    let verbose_task = verbose.task("resolve");
    verbose_task.detail("registry selected");
    verbose_task.finish();
    assert!(verbose_writer.text().contains("registry selected"));
}

#[test]
fn external_text_is_control_safe_bounded_and_redacted() {
    let rendered = sanitize_progress_text(
        "fetch\u{1b}[31m https://alice:hunter2@example.test/pkg?token=secret \
         authorization: Bearer also-secret password=hidden",
    );
    assert!(!rendered.contains('\u{1b}'));
    assert!(!rendered.contains("hunter2"));
    assert!(!rendered.contains("token=secret"));
    assert!(!rendered.contains("also-secret"));
    assert!(!rendered.contains("hidden"));
    assert!(rendered.contains("https://[redacted]@example.test/pkg?[redacted]"));
    assert!(rendered.chars().count() <= MAX_MESSAGE_CHARS + 1);

    for candidate in [
        "token: SENTINEL_ONE",
        "password = SENTINEL_TWO",
        r#"{"token": "SENTINEL_THREE"}"#,
        "Authorization: Bearer SENTINEL_FOUR",
        "cargo --token SENTINEL_FIVE next",
        "https://example.test/path#SENTINEL_SIX",
    ] {
        let rendered = sanitize_progress_text(candidate);
        assert!(
            !rendered.contains("SENTINEL"),
            "credential sentinel leaked from {candidate:?}: {rendered:?}"
        );
    }
}

#[test]
fn interactive_parent_active_and_completed_child_coexist_without_zombies() {
    let terminal = InMemoryTerm::new(12, 100);
    let renderer = ProgressRenderer::interactive_target(
        ProgressDrawTarget::term_like(Box::new(terminal.clone())),
        true,
    );
    let progress = Progress::new(renderer.clone());
    let parent = progress.task("overall install");
    let child = parent.progress().task("package one");
    child.finish();
    renderer.suspend(|| ());

    let during = terminal.contents();
    assert!(during.contains("overall install"), "{during:?}");
    assert!(during.contains("package one"), "{during:?}");
    assert_eq!(renderer.active_task_count(), 1);

    parent.detail("ordinary synchronized detail");
    parent.finish();
    renderer.suspend(|| ());
    let after = terminal.contents();
    assert!(after.contains("package one"), "{after:?}");
    assert!(after.contains("ordinary synchronized detail"), "{after:?}");
    assert!(after.contains("overall install"), "{after:?}");
    assert_eq!(renderer.active_task_count(), 0);
}

#[test]
fn interactive_long_nested_label_stays_within_eighty_columns() {
    let terminal = InMemoryTerm::new(12, 80);
    let renderer = ProgressRenderer::interactive_target(
        ProgressDrawTarget::term_like(Box::new(terminal.clone())),
        false,
    );
    let progress = Progress::new(renderer);
    let parent = progress.task("Installing package closure");
    let child = parent
        .progress()
        .task("Reading registry metadata for org.vibevm.world/multi-user-planning@1.0.0");
    child.set_progress(250_000_000, Some(300_000_000), "bytes");

    let contents = terminal.contents();
    assert!(
        contents.contains("Installing package closure"),
        "{contents:?}"
    );
    assert!(
        contents.contains("Reading registry metadata"),
        "{contents:?}"
    );
    assert!(contents.lines().count() >= 2, "{contents:?}");
    assert!(contents.lines().all(|line| line.chars().count() <= 80));
    child.finish();
    parent.finish();
}

#[test]
fn terminal_rows_persist_in_plain_transcript_and_active_state_is_bounded() {
    let writer = SharedWriter::default();
    let renderer = ProgressRenderer::plain(Box::new(writer.clone()), true);
    let progress = Progress::new(renderer.clone());
    let first = progress.task("first");
    first.finish();
    assert_eq!(renderer.active_task_count(), 0);

    let second = progress.task("second");
    second.detail("later output");
    second.finish();
    let text = writer.text();
    assert!(text.contains("[done] first"));
    assert!(text.contains("later output"));
    assert!(text.contains("[done] second"));
    assert_eq!(renderer.active_task_count(), 0);
}
