//! The CLI projection of `vibe_core::progress` observations.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-060#DISPLAY");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#display");

use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget};
use std::collections::BTreeMap;
use std::io;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use vibe_core::progress::{
    ProgressDiagnosticLevel, ProgressEvent, ProgressEventKind, ProgressObserver, TaskId,
};

mod panel;
mod plain;
mod rows;
use plain::{PlainRenderer, spawn_wait_reporter};
mod sanitize;
mod vocabulary;
use panel::{Panel, clear_tail, push_tail};
use rows::{End, Headline, StepLogs, collapsed};
#[cfg(test)]
use sanitize::MAX_MESSAGE_CHARS;
use sanitize::sanitize;
pub(crate) use sanitize::sanitize_progress_text;
use vocabulary::{
    bar_style, bytes_bar_style, bytes_spinner_style, measured_spinner_style, spinner_style,
};

use super::language::{self, Kind, Role, Stream, paint};

/// The waiting interval of PROP-060 `#DISPLAY`: a silent running block in a
/// plain stream says it is still running this often (PROP-067 `#plain`).
const WAIT_INTERVAL: Duration = Duration::from_secs(15);

/// The list the finished blocks of one command form (PROP-067 `#list`).
const STEPS: Kind = Kind::Row("steps");

/// Whether a task is the command's own fallback root, whose summary is the
/// trailer rather than a row (PROP-067 `#trailer`).
fn is_command(label: &str, parent: Option<TaskId>) -> bool {
    parent.is_none() && label.starts_with("Command ")
}

/// A diagnostic as the warning line of PROP-067 `#refusal`.
fn diagnostic_line(level: ProgressDiagnosticLevel, label: &str, message: &str) -> String {
    let word = match level {
        ProgressDiagnosticLevel::Warning => "",
        ProgressDiagnosticLevel::Error => "error: ",
    };
    language::warning(&format!("{word}{label}: {message}"))
}

/// The record one task leaves, with the blank line the page owes it.
fn row(text: String) -> String {
    format!("{}{text}", language::before(Stream::Err, STEPS))
}

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
    /// The step logs this run knows, shared by both renderers.
    logs: StepLogs,
}

enum Renderer {
    Interactive(InteractiveRenderer),
    Plain(Arc<PlainRenderer>),
}

/// The visible shape every renderer keeps per task: its headline and the
/// widest name of the list its row joins.
#[derive(Default)]
struct Shapes {
    headlines: BTreeMap<TaskId, Headline>,
    name_width: usize,
}

impl Shapes {
    /// Place a starting task: a child of the command's own task is a block
    /// of its own; any other child hangs one level under its parent.
    fn start(&mut self, id: TaskId, parent: Option<TaskId>, label: &str) -> Headline {
        let command = is_command(label, parent);
        let depth = match parent.and_then(|parent| self.headlines.get(&parent)) {
            Some(parent) if parent.command => 0,
            Some(parent) => parent.depth + 1,
            None => 0,
        };
        let headline = Headline::parse(label, depth, command);
        if depth == 0 && !command {
            self.name_width = self.name_width.max(headline.name.chars().count());
        }
        self.headlines.insert(id, headline.clone());
        headline
    }

    fn end(&mut self, id: TaskId) -> Option<Headline> {
        self.headlines.remove(&id)
    }
}

struct InteractiveRenderer {
    multi: MultiProgress,
    tasks: Mutex<BTreeMap<TaskId, InteractiveTask>>,
    shapes: Mutex<Shapes>,
    /// Set while an application step panel renders (PROP-065 `#display`).
    panel: Mutex<Option<Panel>>,
    /// Whether the bars draw on the process's own stderr, where a finished
    /// row is written around them; a test's in-memory terminal takes it as
    /// one of their lines.
    stderr: bool,
    /// The task of a download, drawn as the bytes of its step's block
    /// rather than as a row of its own (`PROP-068#display`): the step it
    /// belongs to, by the download's task.
    downloads: Mutex<BTreeMap<TaskId, TaskId>>,
}

struct InteractiveTask {
    headline: Headline,
    bar: ProgressBar,
    /// The fold of a step panel, newest last, the bars that draw it under
    /// `bar`, the hint bar under them, and when they were last rewritten.
    tail: Vec<String>,
    tail_bars: Vec<ProgressBar>,
    hint_bar: Option<ProgressBar>,
    tail_drawn: Instant,
}

impl ProgressRenderer {
    pub(super) fn detect(verbose: bool, interactive: bool) -> Arc<Self> {
        if interactive {
            Arc::new(Self::interactive(verbose))
        } else {
            let plain = Arc::new(PlainRenderer::new(Box::new(io::stderr())));
            spawn_wait_reporter(Arc::downgrade(&plain));
            Arc::new(Self {
                renderer: Renderer::Plain(plain),
                verbose,
                logs: StepLogs::default(),
            })
        }
    }

    fn interactive(verbose: bool) -> Self {
        let multi = MultiProgress::with_draw_target(ProgressDrawTarget::stderr_with_hz(12));
        Self::over(multi, verbose, true)
    }

    fn over(multi: MultiProgress, verbose: bool, stderr: bool) -> Self {
        Self {
            renderer: Renderer::Interactive(InteractiveRenderer {
                multi,
                tasks: Mutex::new(BTreeMap::new()),
                shapes: Mutex::new(Shapes::default()),
                panel: Mutex::new(None),
                stderr,
                downloads: Mutex::new(BTreeMap::new()),
            }),
            verbose,
            logs: StepLogs::default(),
        }
    }

    #[cfg(test)]
    fn interactive_target(target: ProgressDrawTarget, verbose: bool) -> Arc<Self> {
        Arc::new(Self::over(
            MultiProgress::with_draw_target(target),
            verbose,
            false,
        ))
    }

    #[cfg(test)]
    fn plain(writer: Box<dyn io::Write + Send>, verbose: bool) -> Arc<Self> {
        Arc::new(Self {
            renderer: Renderer::Plain(Arc::new(PlainRenderer::new(writer))),
            verbose,
            logs: StepLogs::default(),
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

    /// Remember where a step writes its output: its finished row counts
    /// the lines, its failed row keeps the tail (PROP-067 `#collapse`).
    pub(super) fn note_log(&self, step: &str, log: &std::path::Path) {
        self.logs.note(step, log);
    }

    /// How many finished blocks this run drew and where its run record is.
    pub(super) fn run_summary(&self) -> (usize, Option<std::path::PathBuf>) {
        self.logs.summary()
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
            Renderer::Interactive(renderer) => renderer.observe(event, self.verbose, &self.logs),
            Renderer::Plain(renderer) => renderer.observe(event, self.verbose, &self.logs),
        }
    }
}

/// How a terminal event ends its task, or `None` for a live event.
fn ending(kind: &ProgressEventKind) -> Option<End> {
    match kind {
        ProgressEventKind::Finished { .. } => Some(End::Finished),
        ProgressEventKind::Skipped { reason } => Some(End::Skipped(sanitize(reason))),
        ProgressEventKind::Failed { message, .. } => Some(End::Failed(sanitize(message))),
        ProgressEventKind::Stopped => Some(End::Stopped),
        _ => None,
    }
}

impl InteractiveRenderer {
    fn observe(&self, event: ProgressEvent, verbose: bool, logs: &StepLogs) {
        let Ok(mut tasks) = self.tasks.lock() else {
            return;
        };
        let Some(id) = self.download_step(&event) else {
            return;
        };
        if let Some(end) = ending(&event.kind) {
            self.terminal_row(&mut tasks, event.task_id, &end, event.elapsed, logs);
            return;
        }
        match event.kind {
            ProgressEventKind::Started { label, .. } => {
                let (headline, width) = match self.shapes.lock() {
                    Ok(mut shapes) => {
                        let headline = shapes.start(event.task_id, event.parent_id, &label);
                        (headline, shapes.name_width)
                    }
                    Err(_) => return,
                };
                // The command's own task draws nothing: its summary is the
                // trailer, and on an interactive terminal its blocks and
                // questions already show that it is alive (PROP-067
                // `#trailer`). A plain stream still says so while it waits.
                // The first block of the list stands one blank line off the
                // block before it; its row will then join it with none.
                if !headline.command
                    && headline.depth == 0
                    && language::before(Stream::Err, STEPS) == "\n"
                {
                    if self.stderr {
                        self.multi.suspend(|| eprint!("\r\n"));
                    } else {
                        let _ = self.multi.println("");
                    }
                }
                let bar = if headline.command {
                    ProgressBar::hidden()
                } else {
                    let bar = self.multi.add(ProgressBar::new_spinner());
                    bar.set_style(spinner_style());
                    bar.set_message(language::cut(
                        &headline.text(width),
                        language::viewport().saturating_sub(12),
                    ));
                    bar.enable_steady_tick(Duration::from_millis(120));
                    bar
                };
                tasks.insert(
                    event.task_id,
                    InteractiveTask {
                        headline,
                        bar,
                        tail: Vec::new(),
                        tail_bars: Vec::new(),
                        hint_bar: None,
                        tail_drawn: Instant::now(),
                    },
                );
            }
            ProgressEventKind::Progress {
                completed,
                total,
                unit,
                rate_bps,
                ..
            } => {
                let Some(task) = tasks.get(&id) else {
                    return;
                };
                // The style is chosen before the measurement, so the one
                // draw the measurement triggers already carries the bar.
                // A download's rate is its sample's, over the last second
                // (`PROP-068#stream`); nothing here computes one.
                let bytes = unit.eq_ignore_ascii_case("bytes");
                task.bar.set_prefix(match (bytes, rate_bps) {
                    // In the bar's own units, so `12.40 MiB of 40.10 MiB`
                    // and its rate read as one figure.
                    (true, Some(rate)) => format!(" · {}/s", indicatif::HumanBytes(rate)),
                    (true, None) => String::new(),
                    (false, _) => sanitize(&unit),
                });
                match (total, bytes) {
                    (Some(_), true) => task.bar.set_style(bytes_bar_style()),
                    (Some(_), false) => task.bar.set_style(bar_style()),
                    (None, true) => task.bar.set_style(bytes_spinner_style()),
                    (None, false) => task.bar.set_style(measured_spinner_style()),
                }
                if let Some(total) = total {
                    task.bar.set_length(total);
                    task.bar.set_position(completed.min(total));
                } else {
                    task.bar.set_position(completed);
                }
            }
            ProgressEventKind::Detail { message } => {
                // A step panel owns child output: folded under its step, or
                // streamed above the bars while the fold is open
                // (PROP-067 `#fold`).
                if let Some(panel) = self.panel.lock().ok().and_then(|panel| panel.clone()) {
                    let line = sanitize(&message);
                    if panel.expanded.load(Ordering::SeqCst) {
                        let _ = self.multi.println(line);
                    } else if let Some(task) = tasks.get_mut(&event.task_id) {
                        push_tail(&self.multi, task, &line, &panel);
                    }
                } else if verbose && let Some(task) = tasks.get(&event.task_id) {
                    let _ = self.multi.println(format!(
                        "{}{}",
                        task.headline.under(),
                        paint(
                            Role::Dim,
                            &format!("{}: {}", task.headline.name, sanitize(&message))
                        )
                    ));
                }
            }
            ProgressEventKind::Diagnostic { level, message } => {
                if let Some(task) = tasks.get(&event.task_id) {
                    let _ = self.multi.println(diagnostic_line(
                        level,
                        &task.headline.name,
                        &sanitize(&message),
                    ));
                }
            }
            _ => {}
        }
    }

    /// The task an event draws on: a download's step for the download's
    /// own task, the task itself otherwise; `None` for a download's start
    /// and end, which draw nothing of their own.
    fn download_step(&self, event: &ProgressEvent) -> Option<TaskId> {
        let mut downloads = self.downloads.lock().ok()?;
        if let ProgressEventKind::Started {
            transfer: Some(_), ..
        } = &event.kind
            && let Some(step) = event.parent_id
        {
            downloads.insert(event.task_id, step);
            return None;
        }
        let Some(step) = downloads.get(&event.task_id).copied() else {
            return Some(event.task_id);
        };
        if ending(&event.kind).is_some() {
            downloads.remove(&event.task_id);
            return None;
        }
        Some(step)
    }

    /// The record one task leaves behind: its collapsed block in the list
    /// of finished blocks. The command's own task leaves none.
    fn terminal_row(
        &self,
        tasks: &mut BTreeMap<TaskId, InteractiveTask>,
        id: TaskId,
        end: &End,
        elapsed: Duration,
        logs: &StepLogs,
    ) {
        let width = self.shapes.lock().map(|s| s.name_width).unwrap_or(0);
        if let Ok(mut shapes) = self.shapes.lock() {
            shapes.end(id);
        }
        let Some(mut task) = tasks.remove(&id) else {
            return;
        };
        clear_tail(&self.multi, &mut task);
        task.bar.disable_steady_tick();
        task.bar.finish_and_clear();
        if task.headline.command {
            return;
        }
        let text = row(collapsed(&task.headline, width, end, elapsed, logs));
        if !self.stderr {
            let _ = self.multi.println(text);
            return;
        }
        // Written around the bars rather than as one of their lines, so the
        // row ends with its own newline even when it is the last thing
        // drawn and the next block's blank line is not swallowed. The
        // panel's key reader holds the terminal in raw mode, where a bare
        // newline does not return the carriage.
        let text = text.replace('\n', "\r\n");
        self.multi.suspend(|| eprint!("{text}\r\n"));
    }
}

#[cfg(test)]
#[path = "progress/tests.rs"]
mod tests;
