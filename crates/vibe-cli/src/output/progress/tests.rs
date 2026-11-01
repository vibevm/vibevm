use super::*;
use indicatif::InMemoryTerm;
use std::io::Write;
use std::sync::MutexGuard;
use vibe_core::progress::{Progress, ProgressDiagnosticLevel};

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
    // PROP-067 `#plain`: nothing is written while a block runs.
    assert_eq!(writer.text(), "");
    task.set_progress(250_000_000, Some(300_000_000), "bytes");
    assert_eq!(writer.text(), "");
    task.finish();

    let text = writer.text();
    assert_eq!(text, "✓ download · 0s\n");
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
    assert!(
        quiet_text.contains("! resolve: compiler warning"),
        "{quiet_text}"
    );

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

/// The step panel of PROP-065 `#display` over an in-memory terminal.
/// The screen once `needle` has been drawn on it. A bar's steady tick
/// redraws from its own thread, and `InMemoryTerm` clears and rewrites a
/// line as separate operations, so one read can land between the two and
/// see a blank screen; waiting for the text is the only honest read.
fn drawn(terminal: &InMemoryTerm, needle: &str) -> String {
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        let contents = terminal.contents();
        if contents.contains(needle) || std::time::Instant::now() > deadline {
            return contents;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn panelled(
    rows: u16,
    columns: u16,
    tail: usize,
    expanded: bool,
) -> (InMemoryTerm, Arc<ProgressRenderer>) {
    let terminal = InMemoryTerm::new(rows, columns);
    let renderer = ProgressRenderer::interactive_target(
        ProgressDrawTarget::term_like(Box::new(terminal.clone())),
        false,
    );
    assert!(renderer.open_panel_every(tail, expanded, Duration::ZERO, false));
    (terminal, renderer)
}

#[test]
fn a_step_panel_folds_the_last_tail_lines_under_its_step_and_erases_them_on_finish() {
    let (terminal, renderer) = panelled(12, 60, 5, false);
    let progress = Progress::new(renderer.clone());
    let step = progress.task("build (recipe)  Build");
    for line in 1..=7 {
        step.detail(format!("line {line}"));
    }

    let folded = terminal.contents();
    assert!(folded.contains("build  Build"), "{folded:?}");
    for kept in ["line 3", "line 4", "line 5", "line 6", "line 7"] {
        assert!(folded.contains(kept), "{kept} missing from {folded:?}");
    }
    for replaced in ["line 1", "line 2"] {
        assert!(!folded.contains(replaced), "{replaced} kept in {folded:?}");
    }
    assert!(
        folded.lines().all(|line| line.chars().count() <= 60),
        "{folded:?}"
    );

    step.finish();
    renderer.suspend(|| ());
    let after = terminal.contents();
    assert!(after.contains("✓ build  Build · 0s"), "{after:?}");
    assert!(
        !after.contains("line 7"),
        "the fold goes with its step: {after:?}"
    );
}

#[test]
fn a_zero_tail_draws_the_step_row_alone() {
    let (terminal, renderer) = panelled(6, 60, 0, false);
    let progress = Progress::new(renderer.clone());
    let step = progress.task("build (recipe)  Build");
    step.detail("not shown");
    let contents = terminal.contents();
    assert!(contents.contains("build  Build"), "{contents:?}");
    assert!(!contents.contains("not shown"), "{contents:?}");
    step.finish();
}

#[test]
fn the_panel_toggle_replaces_the_fold_with_the_live_stream_and_folds_back() {
    let (terminal, renderer) = panelled(12, 60, 5, false);
    let progress = Progress::new(renderer.clone());
    let step = progress.task("build (recipe)  Build");
    step.detail("folded line");
    assert!(terminal.contents().contains("folded line"));

    renderer.toggle_panel();
    step.detail("streamed line");
    let expanded = terminal.contents();
    assert!(expanded.contains("streamed line"), "{expanded:?}");
    assert!(
        !expanded.contains("folded line"),
        "opening the fold erases it: {expanded:?}"
    );

    renderer.toggle_panel();
    step.detail("folded again");
    let refolded = terminal.contents();
    assert!(
        refolded.contains("streamed line"),
        "no history is redrawn: {refolded:?}"
    );
    assert!(refolded.contains("folded again"), "{refolded:?}");
    step.finish();
}

#[test]
fn a_closed_panel_restores_the_verbose_detail_record_and_plain_draws_none() {
    let (terminal, renderer) = panelled(12, 80, 5, false);
    let progress = Progress::new(renderer.clone());
    let step = progress.task("build (recipe)  Build");
    step.detail("folded line");
    renderer.close_panel();
    let closed = terminal.contents();
    assert!(!closed.contains("folded line"), "{closed:?}");
    step.detail("after the panel");
    assert!(
        !terminal.contents().contains("after the panel"),
        "a non-verbose renderer keeps detail to itself"
    );
    step.finish();

    let writer = SharedWriter::default();
    assert!(
        !ProgressRenderer::plain(Box::new(writer), false).open_panel(5, false, false),
        "a plain renderer draws no fold"
    );
}

#[test]
fn a_download_with_a_known_total_draws_a_bytes_bar() {
    let (terminal, renderer) = panelled(8, 80, 5, false);
    let progress = Progress::new(renderer.clone());
    let fetch = progress.task("fetch (sources)  Fetch the sources");
    fetch.set_progress(5 * 1024 * 1024, Some(10 * 1024 * 1024), "bytes");
    renderer.suspend(|| ());
    let bar = terminal.contents();
    assert!(
        bar.contains("of 10.00 MiB"),
        "a known total gives a bar: {bar:?}"
    );
    fetch.finish();

    let unknown = progress.task("fetch (sources)  Fetch an unsized input");
    unknown.set_progress(3 * 1024 * 1024, None, "bytes");
    renderer.suspend(|| ());
    let spinner = terminal.contents();
    assert!(
        spinner
            .lines()
            .any(|line| line.contains("Fetch an unsized input") && !line.contains('━')),
        "an unknown total gives the bytes spinner: {spinner:?}"
    );
    unknown.finish();
}

/// A download is the running block of its step (`PROP-068#stream`): its
/// samples draw the step's bytes bar with the sample's rate, and its own
/// task leaves no row.
#[test]
fn a_download_draws_on_its_step_with_the_samples_rate() {
    use std::io::Read;
    use vibe_core::progress::transfer::{Destination, Purpose, Transfer, TransferRecord};

    let (terminal, renderer) = panelled(8, 80, 5, false);
    let progress = Progress::new(renderer.clone());
    let fetch = progress.task("fetch (sources)  Fetch the sources");
    let record = TransferRecord::http(
        Purpose::PackageSource,
        "https://example.test/tool.tar.gz",
        Destination::Cache,
    )
    .total(Some(10 * 1024 * 1024));
    let body = vec![0u8; 5 * 1024 * 1024];
    let mut reader = Transfer::begin(record, Some(&fetch), None).reader(&body[..]);
    std::io::copy(
        &mut reader.by_ref().take(5 * 1024 * 1024),
        &mut std::io::sink(),
    )
    .unwrap();
    renderer.suspend(|| ());
    let running = terminal.contents();
    assert!(
        running.contains("Fetch the sources") && running.contains("of 10.00 MiB · "),
        "the step's bar carries the bytes and the rate: {running:?}"
    );
    assert!(running.contains("/s"), "{running:?}");
    assert!(
        !running.contains("Downloading"),
        "no row of its own: {running:?}"
    );
    reader.finish();
    fetch.finish();
    renderer.suspend(|| ());
    assert!(!terminal.contents().contains("Downloading"));
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
    assert!(text.contains("✓ first · 0s"), "{text}");
    assert!(text.contains("later output"));
    assert!(text.contains("✓ second · 0s"), "{text}");
    assert_eq!(renderer.active_task_count(), 0);
}

#[test]
fn plain_suspend_blocks_due_heartbeats_and_resumes_after_nested_unwind() {
    let writer = SharedWriter::default();
    let renderer = ProgressRenderer::plain(Box::new(writer.clone()), false);
    let progress = Progress::new(renderer.clone());
    let task = progress.task("owned terminal");
    let Renderer::Plain(plain) = &renderer.renderer else {
        panic!("plain renderer expected");
    };
    if let Ok(mut tasks) = plain.tasks.lock() {
        let state = tasks.get_mut(&task.id()).expect("active task");
        state.started = Instant::now() - WAIT_INTERVAL;
        state.last_update = Instant::now() - WAIT_INTERVAL;
    }
    let before = writer.text();
    let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        renderer.suspend(|| {
            renderer.suspend(|| plain.waiting());
            assert_eq!(writer.text(), before);
            panic!("exercise unwind-safe suspension");
        });
    }));
    assert!(unwind.is_err());
    assert_eq!(writer.text(), before);

    plain.waiting();
    assert!(
        writer
            .text()
            .contains("● owned terminal · still running · 15s"),
        "{}",
        writer.text()
    );
    task.finish();
}

#[test]
fn plain_suspend_stays_silent_through_long_input_and_resumes_on_ok_or_error() {
    for fail_input in [false, true] {
        let writer = SharedWriter::default();
        let renderer = ProgressRenderer::plain(Box::new(writer.clone()), false);
        let progress = Progress::new(renderer.clone());
        let task = progress.task("Command init");
        let Renderer::Plain(plain) = &renderer.renderer else {
            panic!("plain renderer expected");
        };
        let before = writer.text();

        let result: Result<(), &str> = renderer.suspend(|| {
            // Advance the task's clock repeatedly without sleeping: input can
            // remain open for minutes, hours, or days with no timeout.
            for elapsed_seconds in [15, 45, 180, 3_600, 86_400] {
                let mut tasks = plain.tasks.lock().expect("active tasks lock");
                let state = tasks.get_mut(&task.id()).expect("active task");
                state.started = Instant::now() - Duration::from_secs(elapsed_seconds);
                state.last_update = Instant::now() - WAIT_INTERVAL;
                drop(tasks);
                plain.waiting();
                assert_eq!(writer.text(), before);
                assert_eq!(renderer.active_task_count(), 1);
            }
            if fail_input {
                Err("input interrupted")
            } else {
                Ok(())
            }
        });
        assert_eq!(result.is_err(), fail_input);
        assert_eq!(writer.text(), before);

        plain.waiting();
        assert!(
            writer.text().contains("● Command init · still running"),
            "{}",
            writer.text()
        );
        let waited = writer.text();
        task.finish();
        assert_eq!(
            writer.text(),
            waited,
            "the command's own task leaves no row: its summary is the trailer"
        );
        assert_eq!(renderer.active_task_count(), 0);
    }
}

#[test]
fn a_step_label_reads_as_its_id_and_title_and_a_child_hangs_from_its_branch() {
    let step = Headline::parse("build (recipe)  Build vim", 0, false);
    assert_eq!(
        (step.name.as_str(), step.title.as_str()),
        ("build", "Build vim")
    );
    assert_eq!(step.text(7), "build    Build vim");
    let child = Headline::parse("fetch (archive)  Fetch the toolchain", 1, false);
    assert_eq!(child.text(7), "└ fetch  Fetch the toolchain");
    let deeper = Headline::parse("requires org.example/tool@1.0.0 (build)", 2, false);
    assert_eq!(
        deeper.text(0),
        "  └ requires org.example/tool@1.0.0 (build)"
    );
}

#[test]
fn the_panel_draws_the_steps_as_blocks_and_a_requirement_with_its_children() {
    let (terminal, renderer) = panelled(16, 72, 3, false);
    let progress = Progress::new(renderer.clone());
    let command = progress.task("Command global install");
    let steps = command.progress();

    // A required package's whole install is one block of its own, and its
    // steps are its children (PROP-067 `#children`).
    let requirement = steps.task("requires org.example/toolchain@1.0.0 (build) for step build");
    let required_step = requirement
        .progress()
        .task("fetch (archive)  Fetch the toolchain");
    let nested = drawn(&terminal, "└ fetch  Fetch the toolchain");
    assert!(
        !nested.contains("Command global install"),
        "the command's own task draws nothing: {nested:?}"
    );
    assert!(
        nested.contains("requires org.example/toolchain@1.0.0"),
        "{nested:?}"
    );
    assert!(
        nested.contains("└ fetch  Fetch the toolchain"),
        "{nested:?}"
    );
    required_step.finish();
    requirement.finish();
    renderer.suspend(|| ());
    let done = drawn(&terminal, "✓ └ fetch  Fetch the toolchain · 0s");
    assert!(
        done.contains("✓ └ fetch  Fetch the toolchain · 0s"),
        "a finished child keeps its branch: {done:?}"
    );

    let fetch = steps.task("fetch (sources)  Fetch the sources");
    fetch.finish();
    let build = steps.task("build (recipe)  Build");
    build.detail("configure: done");
    build.detail("cc main.c");
    let folded = drawn(&terminal, "  │ cc main.c");
    assert!(
        folded.contains("  │ cc main.c"),
        "the fold sits at column 2 under the running block: {folded:?}"
    );
    assert!(
        folded.lines().all(|line| line.chars().count() <= 72),
        "{folded:?}"
    );

    build.finish();
    command.finish();
    renderer.suspend(|| ());
    let after = drawn(&terminal, "✓ build");
    assert!(after.contains("✓ fetch"), "{after:?}");
    assert!(after.contains("✓ build"), "{after:?}");
    assert!(
        !after.contains("Command global install"),
        "the root leaves no row: {after:?}"
    );
    assert!(
        !after.contains("cc main.c"),
        "the fold goes with its step: {after:?}"
    );
}

#[test]
fn a_skipped_and_a_failed_step_collapse_to_their_glyphs_and_metadata() {
    let terminal = InMemoryTerm::new(8, 72);
    let renderer = ProgressRenderer::interactive_target(
        ProgressDrawTarget::term_like(Box::new(terminal.clone())),
        false,
    );
    let progress = Progress::new(renderer.clone());
    let command = progress.task("Command global install");
    let skipped = command.progress().task("fix (agent)  Fix");
    skipped.skip("no agent configured");
    let failed = command.progress().task("build (recipe)  Build");
    failed.fail("recipe step build exited with code 1");
    command.finish();
    renderer.suspend(|| ());
    let after = terminal.contents();
    assert!(
        after.contains("○ fix  Fix · skipped: no agent configured"),
        "{after:?}"
    );
    assert!(after.contains("✗ build  Build · 0s · exit 1"), "{after:?}");
}
