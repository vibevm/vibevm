//! The CLI projection of `vibe_core::progress` observations.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-060#DISPLAY");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#display");

use console::Style;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget};
use std::collections::BTreeMap;
use std::io::{self, Write};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};
use vibe_core::progress::{ProgressEvent, ProgressEventKind, ProgressObserver, TaskId};

mod panel;
mod sanitize;
mod vocabulary;
use panel::{Panel, clear_tail, push_tail};
#[cfg(test)]
use sanitize::MAX_MESSAGE_CHARS;
use sanitize::sanitize;
pub(crate) use sanitize::sanitize_progress_text;
use vocabulary::{
    TreeGlyphs, bar_style, bytes_bar_style, bytes_spinner_style, diagnostic_marker,
    diagnostic_word, format_elapsed, human_bytes, measured_spinner_style, spinner_style,
};

/// The marker of a step nothing ran on; its whole row is dim.
const SKIPPED: &str = "•";

const PLAIN_UPDATE_INTERVAL: Duration = Duration::from_secs(1);
const WAIT_INTERVAL: Duration = Duration::from_secs(15);

/// How this invocation may render progress on stderr.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProgressMode {
    /// The command owns its terminal/protocol, or progress was disabled.
    Disabled,
    /// Flushed line records, safe beside independently rendered stdout.
    Plain,
    /// Cursor-managed bars for command families that synchronize output.
    Interactive,
}

pub(super) struct ProgressRenderer {
    renderer: Renderer,
    verbose: bool,
}

enum Renderer {
    Interactive(InteractiveRenderer),
    Plain(Arc<PlainRenderer>),
}

struct InteractiveRenderer {
    multi: MultiProgress,
    tasks: Mutex<BTreeMap<TaskId, InteractiveTask>>,
    /// Set while an application step panel renders (PROP-065 `#display`).
    panel: Mutex<Option<Panel>>,
    /// The glyphs nested tasks are drawn with, decided once (PROP-060
    /// `#DISPLAY`).
    tree: TreeGlyphs,
}

struct InteractiveTask {
    label: String,
    depth: usize,
    bar: ProgressBar,
    /// The folded child-output lines of a step panel, newest last, the
    /// bars that draw them under `bar`, and when they were last rewritten.
    tail: Vec<String>,
    tail_bars: Vec<ProgressBar>,
    tail_drawn: Instant,
}

struct PlainRenderer {
    emission: Mutex<PlainEmission>,
    tasks: Mutex<BTreeMap<TaskId, PlainTask>>,
}

struct PlainEmission {
    writer: Box<dyn Write + Send>,
    suspended: usize,
}

struct PlainTask {
    label: String,
    started: Instant,
    last_update: Instant,
}

impl ProgressRenderer {
    pub(super) fn detect(verbose: bool, interactive: bool) -> Arc<Self> {
        if interactive {
            Arc::new(Self::interactive(verbose))
        } else {
            let plain = Arc::new(PlainRenderer {
                emission: Mutex::new(PlainEmission {
                    writer: Box::new(io::stderr()),
                    suspended: 0,
                }),
                tasks: Mutex::new(BTreeMap::new()),
            });
            spawn_wait_reporter(Arc::downgrade(&plain));
            Arc::new(Self {
                renderer: Renderer::Plain(plain),
                verbose,
            })
        }
    }

    fn interactive(verbose: bool) -> Self {
        let multi = MultiProgress::with_draw_target(ProgressDrawTarget::stderr_with_hz(12));
        Self {
            renderer: Renderer::Interactive(InteractiveRenderer {
                multi,
                tasks: Mutex::new(BTreeMap::new()),
                panel: Mutex::new(None),
                tree: TreeGlyphs::detect(),
            }),
            verbose,
        }
    }

    #[cfg(test)]
    fn interactive_target(target: ProgressDrawTarget, verbose: bool) -> Arc<Self> {
        Arc::new(Self {
            renderer: Renderer::Interactive(InteractiveRenderer {
                multi: MultiProgress::with_draw_target(target),
                tasks: Mutex::new(BTreeMap::new()),
                panel: Mutex::new(None),
                tree: TreeGlyphs::detect(),
            }),
            verbose,
        })
    }

    #[cfg(test)]
    fn plain(writer: Box<dyn Write + Send>, verbose: bool) -> Arc<Self> {
        Arc::new(Self {
            renderer: Renderer::Plain(Arc::new(PlainRenderer {
                emission: Mutex::new(PlainEmission {
                    writer,
                    suspended: 0,
                }),
                tasks: Mutex::new(BTreeMap::new()),
            })),
            verbose,
        })
    }

    pub(super) fn suspend<R>(&self, render: impl FnOnce() -> R) -> R {
        match &self.renderer {
            Renderer::Interactive(interactive) => interactive.multi.suspend(render),
            Renderer::Plain(plain) => plain.suspend(render),
        }
    }

    /// Whether this invocation draws cursor-managed bars.
    pub(super) fn is_interactive(&self) -> bool {
        matches!(self.renderer, Renderer::Interactive(_))
    }

    #[cfg(test)]
    fn active_task_count(&self) -> usize {
        match &self.renderer {
            Renderer::Interactive(renderer) => {
                renderer.tasks.lock().map(|tasks| tasks.len()).unwrap_or(0)
            }
            Renderer::Plain(renderer) => {
                renderer.tasks.lock().map(|tasks| tasks.len()).unwrap_or(0)
            }
        }
    }
}

impl ProgressObserver for ProgressRenderer {
    fn observe(&self, event: ProgressEvent) {
        match &self.renderer {
            Renderer::Interactive(renderer) => renderer.observe(event, self.verbose),
            Renderer::Plain(renderer) => renderer.observe(event, self.verbose),
        }
    }
}

impl InteractiveRenderer {
    fn observe(&self, event: ProgressEvent, verbose: bool) {
        let Ok(mut tasks) = self.tasks.lock() else {
            return;
        };
        match event.kind {
            ProgressEventKind::Started { label } => {
                let label = sanitize(&label);
                let depth = event
                    .parent_id
                    .and_then(|parent| tasks.get(&parent).map(|task| task.depth + 1))
                    .unwrap_or(0);
                let bar = self.multi.add(ProgressBar::new_spinner());
                bar.set_style(spinner_style());
                bar.set_message(self.tree.label(depth, &label));
                bar.enable_steady_tick(Duration::from_millis(120));
                tasks.insert(
                    event.task_id,
                    InteractiveTask {
                        label,
                        depth,
                        bar,
                        tail: Vec::new(),
                        tail_bars: Vec::new(),
                        tail_drawn: Instant::now(),
                    },
                );
            }
            ProgressEventKind::Progress {
                completed,
                total,
                unit,
            } => {
                let Some(task) = tasks.get(&event.task_id) else {
                    return;
                };
                // The style is chosen before the measurement, so the one
                // draw the measurement triggers already carries the bar.
                if let Some(total) = total {
                    if unit.eq_ignore_ascii_case("bytes") {
                        task.bar.set_prefix(String::new());
                        task.bar.set_style(bytes_bar_style());
                    } else {
                        task.bar.set_prefix(sanitize(&unit));
                        task.bar.set_style(bar_style());
                    }
                    task.bar.set_length(total);
                    task.bar.set_position(completed.min(total));
                } else {
                    if unit.eq_ignore_ascii_case("bytes") {
                        task.bar.set_prefix(String::new());
                        task.bar.set_style(bytes_spinner_style());
                    } else {
                        task.bar.set_prefix(sanitize(&unit));
                        task.bar.set_style(measured_spinner_style());
                    }
                    task.bar.set_position(completed);
                }
            }
            ProgressEventKind::Detail { message } => {
                // A step panel owns child output: folded under its step, or
                // streamed above the bars while the fold is open
                // (PROP-065 `#display`).
                if let Some(panel) = self.panel.lock().ok().and_then(|panel| panel.clone()) {
                    let line = sanitize(&message);
                    if panel.expanded.load(Ordering::SeqCst) {
                        let _ = self.multi.println(line);
                    } else if let Some(task) = tasks.get_mut(&event.task_id) {
                        push_tail(&self.multi, &self.tree, task, &line, &panel);
                    }
                } else if verbose && let Some(task) = tasks.get(&event.task_id) {
                    let _ = self.multi.println(format!(
                        "{}· {}: {}",
                        self.tree.under(task.depth),
                        task.label,
                        sanitize(&message)
                    ));
                }
            }
            ProgressEventKind::Diagnostic { level, message } => {
                if let Some(task) = tasks.get(&event.task_id) {
                    let _ = self.multi.println(format!(
                        "{}{} {}: {}",
                        self.tree.under(task.depth),
                        diagnostic_marker(level),
                        task.label,
                        sanitize(&message)
                    ));
                }
            }
            ProgressEventKind::Finished => {
                self.terminal_row(&mut tasks, event.task_id, "✓", None, event.elapsed);
            }
            ProgressEventKind::Skipped { reason } => self.terminal_row(
                &mut tasks,
                event.task_id,
                SKIPPED,
                Some(&sanitize(&reason)),
                event.elapsed,
            ),
            ProgressEventKind::Failed { message } => self.terminal_row(
                &mut tasks,
                event.task_id,
                "✗",
                Some(&sanitize(&message)),
                event.elapsed,
            ),
            ProgressEventKind::Stopped => self.terminal_row(
                &mut tasks,
                event.task_id,
                "!",
                Some("stopped"),
                event.elapsed,
            ),
        }
    }

    /// The record one task leaves behind: its marker in the gutter every
    /// row starts with, then its own place in the tree. A skipped row is
    /// dim, because nothing happened on it.
    fn terminal_row(
        &self,
        tasks: &mut BTreeMap<TaskId, InteractiveTask>,
        id: TaskId,
        marker: &str,
        detail: Option<&str>,
        elapsed: Duration,
    ) {
        let Some(mut task) = tasks.remove(&id) else {
            return;
        };
        clear_tail(&self.multi, &mut task);
        task.bar.disable_steady_tick();
        task.bar.finish_and_clear();
        let detail = detail.map(|text| format!(" — {text}")).unwrap_or_default();
        let row = format!(
            "{marker} {}{detail} [{}]",
            self.tree.label(task.depth, &task.label),
            format_elapsed(elapsed)
        );
        let row = if marker == SKIPPED {
            Style::new().for_stderr().dim().apply_to(row).to_string()
        } else {
            row
        };
        let _ = self.multi.println(row);
    }
}

impl PlainRenderer {
    fn suspend<R>(&self, render: impl FnOnce() -> R) -> R {
        if let Ok(mut emission) = self.emission.lock() {
            emission.suspended = emission.suspended.saturating_add(1);
        }
        let _guard = PlainSuspendGuard { renderer: self };
        render()
    }

    /// The unscoped half of [`Self::suspend`], for a caller whose release
    /// and reclaim are separate events.
    fn set_suspended(&self, suspended: bool) {
        if let Ok(mut emission) = self.emission.lock() {
            emission.suspended = if suspended {
                emission.suspended.saturating_add(1)
            } else {
                emission.suspended.saturating_sub(1)
            };
        }
    }

    fn observe(&self, event: ProgressEvent, verbose: bool) {
        match event.kind {
            ProgressEventKind::Started { label } => {
                let label = sanitize(&label);
                if let Ok(mut tasks) = self.tasks.lock() {
                    tasks.insert(
                        event.task_id,
                        PlainTask {
                            label: label.clone(),
                            started: Instant::now(),
                            // The first real measurement is useful immediately;
                            // later high-frequency measurements are throttled.
                            last_update: Instant::now() - PLAIN_UPDATE_INTERVAL,
                        },
                    );
                }
                self.line(&format!("[start] {label}"));
            }
            ProgressEventKind::Progress {
                completed,
                total,
                unit,
            } => {
                let mut line = None;
                if let Ok(mut tasks) = self.tasks.lock()
                    && let Some(task) = tasks.get_mut(&event.task_id)
                {
                    let should_print = task.last_update.elapsed() >= PLAIN_UPDATE_INTERVAL
                        || total.is_some_and(|total| completed >= total);
                    task.last_update = Instant::now();
                    if should_print {
                        let measurement = if unit.eq_ignore_ascii_case("bytes") {
                            total.map_or_else(
                                || human_bytes(completed),
                                |total| {
                                    format!("{}/{}", human_bytes(completed), human_bytes(total))
                                },
                            )
                        } else {
                            total.map_or_else(
                                || format!("{completed} {}", sanitize(&unit)),
                                |total| format!("{completed}/{total} {}", sanitize(&unit)),
                            )
                        };
                        line = Some(format!(
                            "[progress] {}: {measurement} [{}]",
                            task.label,
                            format_elapsed(event.elapsed)
                        ));
                    }
                }
                if let Some(line) = line {
                    self.line(&line);
                }
            }
            ProgressEventKind::Detail { message } => {
                if verbose && let Some(label) = self.label(event.task_id) {
                    self.touch(event.task_id);
                    self.line(&format!("  · {label}: {}", sanitize(&message)));
                }
            }
            ProgressEventKind::Diagnostic { level, message } => {
                if let Some(label) = self.label(event.task_id) {
                    self.touch(event.task_id);
                    self.line(&format!(
                        "[{}] {label}: {}",
                        diagnostic_word(level),
                        sanitize(&message)
                    ));
                }
            }
            ProgressEventKind::Finished => {
                self.terminal(event.task_id, "done", None, event.elapsed)
            }
            ProgressEventKind::Skipped { reason } => self.terminal(
                event.task_id,
                "skip",
                Some(&sanitize(&reason)),
                event.elapsed,
            ),
            ProgressEventKind::Failed { message } => self.terminal(
                event.task_id,
                "fail",
                Some(&sanitize(&message)),
                event.elapsed,
            ),
            ProgressEventKind::Stopped => {
                self.terminal(event.task_id, "stopped", None, event.elapsed)
            }
        }
    }

    fn label(&self, id: TaskId) -> Option<String> {
        self.tasks
            .lock()
            .ok()
            .and_then(|tasks| tasks.get(&id).map(|task| task.label.clone()))
    }

    fn touch(&self, id: TaskId) {
        if let Ok(mut tasks) = self.tasks.lock()
            && let Some(task) = tasks.get_mut(&id)
        {
            task.last_update = Instant::now();
        }
    }

    fn terminal(&self, id: TaskId, marker: &str, detail: Option<&str>, elapsed: Duration) {
        let label = self
            .tasks
            .lock()
            .ok()
            .and_then(|mut tasks| tasks.remove(&id).map(|task| task.label));
        let Some(label) = label else {
            return;
        };
        let detail = detail.map(|text| format!(" — {text}")).unwrap_or_default();
        self.line(&format!(
            "[{marker}] {label}{detail} [{}]",
            format_elapsed(elapsed)
        ));
    }

    fn waiting(&self) {
        let lines = self
            .tasks
            .lock()
            .map(|tasks| {
                let now = Instant::now();
                tasks
                    .iter()
                    .filter_map(|task| {
                        let (id, task) = task;
                        if now.duration_since(task.last_update) < WAIT_INTERVAL {
                            return None;
                        }
                        Some((
                            *id,
                            format!(
                                "[wait] {} [{}]",
                                task.label,
                                format_elapsed(now.duration_since(task.started))
                            ),
                        ))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for (id, line) in lines {
            if self.line(&line) {
                self.touch(id);
            }
        }
    }

    fn line(&self, line: &str) -> bool {
        let Ok(mut emission) = self.emission.lock() else {
            return false;
        };
        if emission.suspended > 0 {
            return false;
        }
        let _ = writeln!(emission.writer, "{line}");
        let _ = emission.writer.flush();
        true
    }
}

struct PlainSuspendGuard<'a> {
    renderer: &'a PlainRenderer,
}

impl Drop for PlainSuspendGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut emission) = self.renderer.emission.lock() {
            emission.suspended = emission.suspended.saturating_sub(1);
        }
    }
}

fn spawn_wait_reporter(renderer: Weak<PlainRenderer>) {
    let _ = std::thread::Builder::new()
        .name("vibe-progress".to_string())
        .spawn(move || {
            loop {
                std::thread::sleep(WAIT_INTERVAL);
                let Some(renderer) = renderer.upgrade() else {
                    break;
                };
                renderer.waiting();
            }
        });
}

#[cfg(test)]
#[path = "progress/tests.rs"]
mod tests;
