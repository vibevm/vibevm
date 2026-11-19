use super::*;

#[test]
fn prompt_without_panel_holds_progress_without_blocking_render_callbacks() {
    let writer = SharedWriter::default();
    let renderer = ProgressRenderer::plain(Box::new(writer.clone()), true);
    let mut ctx = crate::output::Context::from_flags(
        false,
        false,
        None,
        false,
        crate::cli::AgentModeArg::Auto,
    );
    ctx.progress_renderer = Some(renderer.clone());
    let progress = Progress::new(renderer);
    let task = progress.task("waiting");
    ctx.with_terminal_released(|| {
        task.detail("must not interleave");
        assert!(!writer.text().contains("must not interleave"));
    });
    task.detail("after prompt");
    assert!(writer.text().contains("after prompt"));
    task.finish();
}
