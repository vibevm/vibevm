//! The CLI projection of `vibe_core::progress` observations.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-060#DISPLAY");

use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::collections::BTreeMap;
use std::io::{self, Write};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};
use vibe_core::progress::{
    ProgressDiagnosticLevel, ProgressEvent, ProgressEventKind, ProgressObserver, TaskId,
};

mod sanitize;
#[cfg(test)]
use sanitize::MAX_MESSAGE_CHARS;
use sanitize::sanitize;
pub(crate) use sanitize::sanitize_progress_text;

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
}

struct InteractiveTask {
    label: String,
    depth: usize,
    bar: ProgressBar,
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
                bar.set_message(indent_label(depth, &label));
                bar.enable_steady_tick(Duration::from_millis(120));
                tasks.insert(event.task_id, InteractiveTask { label, depth, bar });
            }
            ProgressEventKind::Progress {
                completed,
                total,
                unit,
            } => {
                let Some(task) = tasks.get(&event.task_id) else {
                    return;
                };
                if let Some(total) = total {
                    task.bar.set_length(total);
                    task.bar.set_position(completed.min(total));
                    if unit.eq_ignore_ascii_case("bytes") {
                        task.bar.set_prefix(String::new());
                        task.bar.set_style(bytes_bar_style());
                    } else {
                        task.bar.set_prefix(sanitize(&unit));
                        task.bar.set_style(bar_style());
                    }
                } else {
                    task.bar.set_position(completed);
                    if unit.eq_ignore_ascii_case("bytes") {
                        task.bar.set_prefix(String::new());
                        task.bar.set_style(bytes_spinner_style());
                    } else {
                        task.bar.set_prefix(sanitize(&unit));
                        task.bar.set_style(measured_spinner_style());
                    }
                }
            }
            ProgressEventKind::Detail { message } => {
                if verbose && let Some(task) = tasks.get(&event.task_id) {
                    let _ = self.multi.println(format!(
                        "{}· {}: {}",
                        "  ".repeat(task.depth + 1),
                        task.label,
                        sanitize(&message)
                    ));
                }
            }
            ProgressEventKind::Diagnostic { level, message } => {
                if let Some(task) = tasks.get(&event.task_id) {
                    let _ = self.multi.println(format!(
                        "{}{} {}: {}",
                        "  ".repeat(task.depth + 1),
                        diagnostic_marker(level),
                        task.label,
                        sanitize(&message)
                    ));
                }
            }
            ProgressEventKind::Finished => {
                finish_interactive(
                    &self.multi,
                    &mut tasks,
                    event.task_id,
                    "✓",
                    None,
                    event.elapsed,
                );
            }
            ProgressEventKind::Skipped { reason } => finish_interactive(
                &self.multi,
                &mut tasks,
                event.task_id,
                "•",
                Some(&sanitize(&reason)),
                event.elapsed,
            ),
            ProgressEventKind::Failed { message } => finish_interactive(
                &self.multi,
                &mut tasks,
                event.task_id,
                "✗",
                Some(&sanitize(&message)),
                event.elapsed,
            ),
            ProgressEventKind::Stopped => finish_interactive(
                &self.multi,
                &mut tasks,
                event.task_id,
                "!",
                Some("stopped"),
                event.elapsed,
            ),
        }
    }
}

fn finish_interactive(
    multi: &MultiProgress,
    tasks: &mut BTreeMap<TaskId, InteractiveTask>,
    id: TaskId,
    marker: &str,
    detail: Option<&str>,
    elapsed: Duration,
) {
    let Some(task) = tasks.remove(&id) else {
        return;
    };
    task.bar.disable_steady_tick();
    task.bar.finish_and_clear();
    let detail = detail.map(|text| format!(" — {text}")).unwrap_or_default();
    let _ = multi.println(format!(
        "{}{} {}{} [{}]",
        "  ".repeat(task.depth),
        marker,
        task.label,
        detail,
        format_elapsed(elapsed)
    ));
}

impl PlainRenderer {
    fn suspend<R>(&self, render: impl FnOnce() -> R) -> R {
        if let Ok(mut emission) = self.emission.lock() {
            emission.suspended = emission.suspended.saturating_add(1);
        }
        let _guard = PlainSuspendGuard { renderer: self };
        render()
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

fn spinner_style() -> ProgressStyle {
    ProgressStyle::with_template("{spinner:.cyan} {wide_msg} [{elapsed_precise}]")
        .unwrap_or_else(|_| ProgressStyle::default_spinner())
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
}

fn bar_style() -> ProgressStyle {
    ProgressStyle::with_template(
        "{bar:16.cyan/blue} {pos}/{len} {prefix} {wide_msg} [{elapsed_precise}]",
    )
    .unwrap_or_else(|_| ProgressStyle::default_bar())
    .progress_chars("=>-")
}

fn bytes_bar_style() -> ProgressStyle {
    ProgressStyle::with_template(
        "{bar:16.cyan/blue} {bytes}/{total_bytes} {wide_msg} [{elapsed_precise}]",
    )
    .unwrap_or_else(|_| ProgressStyle::default_bar())
    .progress_chars("=>-")
}

fn measured_spinner_style() -> ProgressStyle {
    ProgressStyle::with_template("{spinner:.cyan} {pos} {prefix} {wide_msg} [{elapsed_precise}]")
        .unwrap_or_else(|_| ProgressStyle::default_spinner())
}

fn bytes_spinner_style() -> ProgressStyle {
    ProgressStyle::with_template("{spinner:.cyan} {bytes} {wide_msg} [{elapsed_precise}]")
        .unwrap_or_else(|_| ProgressStyle::default_spinner())
}

fn indent_label(depth: usize, label: &str) -> String {
    format!("{}{}", "  ".repeat(depth), label)
}

fn diagnostic_marker(level: ProgressDiagnosticLevel) -> &'static str {
    match level {
        ProgressDiagnosticLevel::Warning => "warning:",
        ProgressDiagnosticLevel::Error => "error:",
    }
}

fn diagnostic_word(level: ProgressDiagnosticLevel) -> &'static str {
    match level {
        ProgressDiagnosticLevel::Warning => "warning",
        ProgressDiagnosticLevel::Error => "error",
    }
}

fn format_elapsed(elapsed: Duration) -> String {
    let seconds = elapsed.as_secs();
    if seconds >= 60 {
        format!("{}m {:02}s", seconds / 60, seconds % 60)
    } else if seconds > 0 {
        format!("{seconds}s")
    } else {
        format!("{}ms", elapsed.as_millis())
    }
}

fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
#[path = "progress/tests.rs"]
mod tests;
