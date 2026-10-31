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
        renderer.open_panel(tail, expanded);
        Some(ProgressPanel { renderer })
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
