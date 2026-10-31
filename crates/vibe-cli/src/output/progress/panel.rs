//! The application step panel inside the interactive renderer (PROP-065
//! `#display`): the fold of a step's last child-output lines, the switch
//! between the fold and the live stream, and handing the terminal over.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#display");

use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use super::{InteractiveRenderer, InteractiveTask, ProgressRenderer, Renderer};

/// How often a step panel's fold is rewritten (PROP-060 `#COST`: a chatty
/// child must not repaint the panel per line).
const TAIL_REDRAW: Duration = Duration::from_millis(80);

/// A live step panel: how many child-output lines stay folded under the
/// running step, whether the fold is currently open, and how often it is
/// rewritten.
#[derive(Clone)]
pub(super) struct Panel {
    pub(super) tail: usize,
    pub(super) expanded: Arc<AtomicBool>,
    redraw: Duration,
}

impl ProgressRenderer {
    /// Start folding step child output under its step; `false` when this
    /// renderer draws no panel.
    pub(in crate::output) fn open_panel(&self, tail: usize, expanded: bool) -> bool {
        self.open_panel_every(tail, expanded, TAIL_REDRAW)
    }

    /// [`Self::open_panel`] with the fold's redraw interval named, so a
    /// test reads a settled fold without waiting for one.
    pub(super) fn open_panel_every(&self, tail: usize, expanded: bool, redraw: Duration) -> bool {
        let Renderer::Interactive(renderer) = &self.renderer else {
            return false;
        };
        if let Ok(mut panel) = renderer.panel.lock() {
            *panel = Some(Panel {
                tail,
                expanded: Arc::new(AtomicBool::new(expanded)),
                redraw,
            });
        }
        true
    }

    /// Fold the running step's output back, or replace the fold with the
    /// live stream; the new state takes effect at the next line.
    pub(in crate::output) fn toggle_panel(&self) {
        let Renderer::Interactive(renderer) = &self.renderer else {
            return;
        };
        let Ok(panel) = renderer.panel.lock() else {
            return;
        };
        let Some(panel) = panel.as_ref() else {
            return;
        };
        panel.expanded.fetch_xor(true, Ordering::SeqCst);
        renderer.clear_tails();
    }

    /// Stop drawing while a prompt or an attached child owns the terminal,
    /// and draw again afterwards. Unlike [`ProgressRenderer::suspend`] this
    /// is not scoped, because the caller's release and reclaim are separate
    /// events; the interactive renderer draws on stderr either way.
    pub(in crate::output) fn set_hidden(&self, hidden: bool) {
        match &self.renderer {
            Renderer::Interactive(renderer) => {
                if hidden {
                    renderer.clear_tails();
                    let _ = renderer.multi.clear();
                    renderer.multi.set_draw_target(ProgressDrawTarget::hidden());
                } else {
                    renderer
                        .multi
                        .set_draw_target(ProgressDrawTarget::stderr_with_hz(12));
                }
            }
            Renderer::Plain(plain) => plain.set_suspended(hidden),
        }
    }

    /// Render as before the panel opened.
    pub(in crate::output) fn close_panel(&self) {
        let Renderer::Interactive(renderer) = &self.renderer else {
            return;
        };
        renderer.clear_tails();
        if let Ok(mut panel) = renderer.panel.lock() {
            *panel = None;
        }
    }
}

impl InteractiveRenderer {
    /// Erase every folded tail line: the fold was toggled, the panel closed
    /// or the terminal handed to something else.
    pub(super) fn clear_tails(&self) {
        let Ok(mut tasks) = self.tasks.lock() else {
            return;
        };
        for task in tasks.values_mut() {
            clear_tail(&self.multi, task);
        }
    }
}

/// Add one child-output line to a step's fold, keeping the newest lines the
/// panel asks for and one bar per kept line under the step's own bar.
///
/// The fold is written as a whole and then drawn once: a per-bar message
/// draws at most once per the renderer's rate-limit window, so without the
/// forced frame the screen would keep a half-rewritten fold. Rewriting is
/// itself throttled, except while the fold is still filling and every line
/// is new.
pub(super) fn push_tail(
    multi: &MultiProgress,
    task: &mut InteractiveTask,
    line: &str,
    panel: &Panel,
) {
    if panel.tail == 0 {
        return;
    }
    task.tail.push(line.to_owned());
    while task.tail.len() > panel.tail {
        task.tail.remove(0);
    }
    let filling = task.tail_bars.len() < task.tail.len();
    if !filling && task.tail_drawn.elapsed() < panel.redraw {
        return;
    }
    while task.tail_bars.len() < task.tail.len() {
        let anchor = task.tail_bars.last().unwrap_or(&task.bar);
        let bar = multi.insert_after(anchor, ProgressBar::new_spinner());
        bar.set_style(tail_style());
        task.tail_bars.push(bar);
    }
    let indent = "  ".repeat(task.depth + 1);
    for (bar, line) in task.tail_bars.iter().zip(&task.tail) {
        bar.set_message(format!("{indent}{line}"));
    }
    multi.suspend(|| ());
    task.tail_drawn = Instant::now();
}

pub(super) fn clear_tail(multi: &MultiProgress, task: &mut InteractiveTask) {
    for bar in task.tail_bars.drain(..) {
        bar.finish_and_clear();
        multi.remove(&bar);
    }
    task.tail.clear();
}

/// One folded child-output line: dim, and never wider than the viewport.
fn tail_style() -> ProgressStyle {
    ProgressStyle::with_template("{wide_msg:.dim}")
        .unwrap_or_else(|_| ProgressStyle::default_spinner())
}
