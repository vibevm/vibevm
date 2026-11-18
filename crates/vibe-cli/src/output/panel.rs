//! The invocation's handle on its interactive step panel (PROP-065
//! `#display`) and on holding progress back while something else owns the
//! terminal. The panel itself lives in the renderer; this is the one door
//! command adapters reach it through.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#display");

use std::sync::Arc;

use super::{Context, ProgressRenderer};

/// The invocation's live application step panel: the fold under the running
/// step, released while something else owns the terminal, closed when the
/// step run ends.
#[derive(Clone)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#display")]
pub(crate) struct ProgressPanel {
    renderer: Arc<ProgressRenderer>,
}

impl ProgressPanel {
    #[cfg(test)]
    pub(crate) fn for_test(ctx: &Context) -> Self {
        Self {
            renderer: ctx.progress_renderer.clone().expect("test renderer"),
        }
    }
    /// Replace the fold with the live stream, or fold it back.
    pub(crate) fn toggle(&self) {
        self.renderer.toggle_panel();
    }

    /// Stop drawing: a prompt or an attached child owns the terminal.
    pub(crate) fn hide(&self) {
        self.renderer.set_hidden(true);
    }

    /// Draw again after [`Self::hide`].
    pub(crate) fn show(&self) {
        self.renderer.set_hidden(false);
    }

    /// Render as before the panel opened.
    pub(crate) fn close(&self) {
        self.renderer.close_panel();
    }
}

/// Progress is held back for as long as this guard lives.
pub(crate) struct ProgressHold<'a> {
    renderer: Option<&'a Arc<ProgressRenderer>>,
}

impl Drop for ProgressHold<'_> {
    fn drop(&mut self) {
        if let Some(renderer) = self.renderer {
            renderer.set_hidden(false);
        }
    }
}

impl Context {
    /// Records whether standard input is a terminal. Only the composition
    /// root reads the ambient streams; everything below takes the value.
    pub fn with_terminal_input(mut self, terminal: bool) -> Self {
        self.terminal_input = terminal;
        self
    }

    /// Records `--plain` (PROP-067 `#plain`).
    pub fn with_plain(mut self, plain: bool) -> Self {
        self.plain = plain;
        self
    }

    /// Whether `--plain` asked for blocks alone, with no child stream.
    pub(crate) fn is_plain(&self) -> bool {
        self.plain
    }

    /// Whether keys may be read from standard input.
    pub(crate) fn reads_terminal_keys(&self) -> bool {
        self.terminal_input
    }

    /// Whether this invocation draws cursor-managed bars on stderr: human
    /// output, an interactive policy, progress neither disabled nor machine.
    pub(crate) fn has_interactive_progress(&self) -> bool {
        !self.machine_progress
            && self
                .progress_renderer
                .as_ref()
                .is_some_and(|renderer| renderer.is_interactive())
    }

    /// Open an application step panel on this invocation's interactive
    /// renderer: `None` in quiet, JSON, machine, disabled and plain modes,
    /// which all stay flat.
    pub(crate) fn open_progress_panel(&self, tail: usize, expanded: bool) -> Option<ProgressPanel> {
        if !self.has_interactive_progress() {
            return None;
        }
        let renderer = self.progress_renderer.clone()?;
        renderer.open_panel(tail, expanded, self.terminal_input);
        Some(ProgressPanel { renderer })
    }

    /// Whether a progress renderer draws this invocation's blocks.
    pub(crate) fn draws_progress(&self) -> bool {
        self.progress_renderer.is_some()
    }

    /// Remember where a step writes its output, so its finished block can
    /// count the lines and a failed one keep its tail (PROP-067
    /// `#collapse`).
    pub(crate) fn note_step_log(&self, step: &str, log: &std::path::Path) {
        if let Some(renderer) = &self.progress_renderer {
            renderer.note_log(step, log);
        }
    }

    pub(crate) fn note_task_log(&self, id: vibe_core::progress::TaskId, log: &std::path::Path) {
        if let Some(renderer) = &self.progress_renderer {
            renderer.note_task_log(id, log);
        }
    }

    /// Hold this invocation's progress back until the guard drops, so the
    /// held work prints and prompts on its own streams. Unlike
    /// [`Context::suspend_progress`] it keeps no renderer lock, so output
    /// and prompts inside it may still go through the renderer.
    pub(crate) fn hold_progress(&self) -> ProgressHold<'_> {
        if let Some(renderer) = &self.progress_renderer {
            renderer.set_hidden(true);
        }
        ProgressHold {
            renderer: self.progress_renderer.as_ref(),
        }
    }
}

/// Invocation-scoped input ownership, shared by context clones and prompt threads.
pub(crate) trait TerminalPanel: Send + Sync {
    fn release(&self);
    fn reclaim(&self);
}
struct TerminalRestore(Arc<dyn TerminalPanel>);
impl Drop for TerminalRestore {
    fn drop(&mut self) {
        self.0.reclaim();
    }
}
impl Context {
    #[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#display")]
    pub(crate) fn register_terminal_panel(&self, panel: Arc<dyn TerminalPanel>) {
        if let Ok(mut slot) = self.terminal_panel.lock() {
            *slot = Some(Arc::downgrade(&panel));
        }
    }
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-073#PROJECT-COOPERATIVE-STOP"
    )]
    pub(crate) fn with_terminal_released<R>(&self, ask: impl FnOnce() -> R) -> R {
        let panel = self
            .terminal_panel
            .lock()
            .ok()
            .and_then(|slot| slot.as_ref().and_then(std::sync::Weak::upgrade));
        if let Some(panel) = panel {
            panel.release();
            let _restore = TerminalRestore(panel);
            // No renderer lock is held over a blocking question.
            ask()
        } else {
            let _hold = self.hold_progress();
            ask()
        }
    }
}

#[cfg(test)]
mod terminal_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[derive(Default)]
    struct Panel {
        released: AtomicUsize,
        reclaimed: AtomicUsize,
    }
    impl TerminalPanel for Panel {
        fn release(&self) {
            self.released.fetch_add(1, Ordering::SeqCst);
        }
        fn reclaim(&self) {
            self.reclaimed.fetch_add(1, Ordering::SeqCst);
        }
    }
    fn context() -> Context {
        Context::from_flags(false, false, None, false, crate::cli::AgentModeArg::Auto)
    }
    #[test]
    fn context_clones_share_terminal_release_across_threads_and_restore_after_unwind() {
        let ctx = context();
        let panel = Arc::new(Panel::default());
        ctx.register_terminal_panel(panel.clone());
        let other = ctx.clone();
        let held = panel.clone();
        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                other.with_terminal_released(|| {
                    assert_eq!(held.released.load(Ordering::SeqCst), 1);
                    // Reentrant invocation access must not encounter the slot mutex.
                    other.with_terminal_released(|| {});
                    panic!("test prompt failed");
                });
            }));
            assert!(result.is_err());
        })
        .join()
        .unwrap();
        assert_eq!(panel.released.load(Ordering::SeqCst), 2);
        assert_eq!(panel.reclaimed.load(Ordering::SeqCst), 2);
        let separate = context();
        separate.with_terminal_released(|| {});
        assert_eq!(panel.released.load(Ordering::SeqCst), 2);
    }
}
