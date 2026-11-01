//! The plain renderer (PROP-067 `#plain`): no cursor movement, no spinner,
//! no fold; a block is written once, when it ends, as its collapsed line,
//! and a silent running block says it is still running at the waiting
//! interval.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#plain");

use std::collections::BTreeMap;
use std::io::Write;
use std::sync::{Mutex, Weak};
use std::time::{Duration, Instant};

use vibe_core::progress::{ProgressEvent, ProgressEventKind, TaskId};

use super::rows::{End, Headline, StepLogs, collapsed};
use super::{Shapes, WAIT_INTERVAL, diagnostic_line, ending, row, sanitize};
use crate::output::language::{self, Kind, Role, Stream, paint};

pub(super) struct PlainRenderer {
    pub(super) emission: Mutex<PlainEmission>,
    pub(super) tasks: Mutex<BTreeMap<TaskId, PlainTask>>,
    shapes: Mutex<Shapes>,
}

pub(super) struct PlainEmission {
    writer: Box<dyn Write + Send>,
    suspended: usize,
}

pub(super) struct PlainTask {
    headline: Headline,
    pub(super) started: Instant,
    pub(super) last_update: Instant,
}

impl PlainRenderer {
    pub(super) fn new(writer: Box<dyn Write + Send>) -> Self {
        Self {
            emission: Mutex::new(PlainEmission {
                writer,
                suspended: 0,
            }),
            tasks: Mutex::new(BTreeMap::new()),
            shapes: Mutex::new(Shapes::default()),
        }
    }

    pub(super) fn suspend<R>(&self, render: impl FnOnce() -> R) -> R {
        if let Ok(mut emission) = self.emission.lock() {
            emission.suspended = emission.suspended.saturating_add(1);
        }
        let _guard = PlainSuspendGuard { renderer: self };
        render()
    }

    /// The unscoped half of [`Self::suspend`], for a caller whose release
    /// and reclaim are separate events.
    pub(super) fn set_suspended(&self, suspended: bool) {
        if let Ok(mut emission) = self.emission.lock() {
            emission.suspended = if suspended {
                emission.suspended.saturating_add(1)
            } else {
                emission.suspended.saturating_sub(1)
            };
        }
    }

    /// PROP-067 `#plain`: nothing is written while a block runs; it is
    /// written once, when it ends, as its collapsed line.
    pub(super) fn observe(&self, event: ProgressEvent, verbose: bool, logs: &StepLogs) {
        if let Some(end) = ending(&event.kind) {
            self.terminal(event.task_id, &end, event.elapsed, logs);
            return;
        }
        match event.kind {
            ProgressEventKind::Started { label } => {
                let Ok(mut shapes) = self.shapes.lock() else {
                    return;
                };
                let headline = shapes.start(event.task_id, event.parent_id, &label);
                if let Ok(mut tasks) = self.tasks.lock() {
                    tasks.insert(
                        event.task_id,
                        PlainTask {
                            headline,
                            started: Instant::now(),
                            last_update: Instant::now(),
                        },
                    );
                }
            }
            ProgressEventKind::Progress { .. } => self.touch(event.task_id),
            ProgressEventKind::Detail { message } => {
                if verbose && let Some(headline) = self.headline(event.task_id) {
                    self.touch(event.task_id);
                    self.line(&format!(
                        "{}{}",
                        headline.under(),
                        paint(
                            Role::Dim,
                            &format!("{}: {}", headline.name, sanitize(&message))
                        )
                    ));
                }
            }
            ProgressEventKind::Diagnostic { level, message } => {
                if let Some(headline) = self.headline(event.task_id) {
                    self.touch(event.task_id);
                    self.line(&diagnostic_line(level, &headline.name, &sanitize(&message)));
                }
            }
            _ => {}
        }
    }

    fn headline(&self, id: TaskId) -> Option<Headline> {
        self.tasks
            .lock()
            .ok()
            .and_then(|tasks| tasks.get(&id).map(|task| task.headline.clone()))
    }

    fn touch(&self, id: TaskId) {
        if let Ok(mut tasks) = self.tasks.lock()
            && let Some(task) = tasks.get_mut(&id)
        {
            task.last_update = Instant::now();
        }
    }

    fn terminal(&self, id: TaskId, end: &End, elapsed: Duration, logs: &StepLogs) {
        let width = self.shapes.lock().map(|s| s.name_width).unwrap_or(0);
        if let Ok(mut shapes) = self.shapes.lock() {
            shapes.end(id);
        }
        let headline = self
            .tasks
            .lock()
            .ok()
            .and_then(|mut tasks| tasks.remove(&id).map(|task| task.headline));
        let Some(headline) = headline else {
            return;
        };
        if headline.command {
            return;
        }
        self.line(&row(collapsed(&headline, width, end, elapsed, logs)));
    }

    /// One dim running line, `<headline> · still running · 30s`, per silent
    /// block.
    pub(super) fn waiting(&self) {
        let width = self.shapes.lock().map(|s| s.name_width).unwrap_or(0);
        let lines = self
            .tasks
            .lock()
            .map(|tasks| {
                let now = Instant::now();
                tasks
                    .iter()
                    .filter(|(_, task)| now.duration_since(task.last_update) >= WAIT_INTERVAL)
                    .map(|(id, task)| {
                        let block = language::Block::new(
                            language::Glyph::Running,
                            task.headline.text(width),
                        )
                        .with_meta(vec![
                            language::Meta::note("still running"),
                            language::Meta::Time(now.duration_since(task.started)),
                        ]);
                        (
                            *id,
                            paint(Role::Dim, &console::strip_ansi_codes(&block.render())),
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for (id, line) in lines {
            let line = format!("{}{line}", language::before(Stream::Err, Kind::Block));
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

pub(super) fn spawn_wait_reporter(renderer: Weak<PlainRenderer>) {
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
