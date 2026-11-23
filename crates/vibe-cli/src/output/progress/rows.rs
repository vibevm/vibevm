//! The collapsed line a finished task leaves behind (PROP-067 `#collapse`),
//! shared by the interactive and the plain renderer so a log reads block
//! for block as the screen did (`#plain`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#collapse");

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use vibe_core::progress::TaskId;

use crate::output::language::{self, Glyph, Line, Meta, Role, paint};

/// How many lines of a failed step's log stay as its evidence.
pub(super) const EVIDENCE: usize = 30;

/// A child's identity within its immediate parent's rows for one run.
type ChildRowKey = (Option<TaskId>, usize, String, String);

/// What this run's rows have recorded: the step logs by step id, so a
/// finished row can count its lines and a failed one keep its tail, and how
/// many blocks the list of finished blocks holds — the trailer's count.
#[derive(Default)]
pub(super) struct StepLogs {
    logs: Mutex<BTreeMap<String, PathBuf>>,
    tasks: Mutex<BTreeMap<vibe_core::progress::TaskId, PathBuf>>,
    rows: AtomicUsize,
    last: Mutex<Option<PathBuf>>,
    seen: Mutex<std::collections::BTreeSet<ChildRowKey>>,
}

impl StepLogs {
    /// Whether this child headline is new in this run; root rows always draw.
    pub(super) fn first_sight(&self, headline: &Headline) -> bool {
        if headline.depth == 0 {
            return true;
        }
        self.seen
            .lock()
            .map(|mut seen| {
                seen.insert((
                    headline.parent_id,
                    headline.depth,
                    headline.name.clone(),
                    headline.title.clone(),
                ))
            })
            .unwrap_or(true)
    }

    pub(super) fn note(&self, step: &str, log: &Path) {
        if let Ok(mut logs) = self.logs.lock() {
            logs.insert(step.to_owned(), log.to_owned());
        }
        if let Ok(mut last) = self.last.lock() {
            *last = Some(log.to_owned());
        }
    }

    /// Forget only a completed task's log mapping when its row is suppressed.
    pub(super) fn discard_task_log(&self, id: TaskId) {
        if let Ok(mut tasks) = self.tasks.lock() {
            tasks.remove(&id);
        }
    }

    /// Whether a task-specific log mapping remains registered.
    #[cfg(test)]
    pub(super) fn has_task_log(&self, id: TaskId) -> bool {
        self.tasks
            .lock()
            .map(|tasks| tasks.contains_key(&id))
            .unwrap_or(false)
    }

    pub(super) fn note_task(&self, id: vibe_core::progress::TaskId, log: &Path) {
        if let Ok(mut tasks) = self.tasks.lock() {
            tasks.insert(id, log.to_owned());
        }
    }

    fn of(&self, id: vibe_core::progress::TaskId, step: &str) -> Option<PathBuf> {
        if let Some(path) = self
            .tasks
            .lock()
            .ok()
            .and_then(|mut tasks| tasks.remove(&id))
        {
            return Some(path);
        }
        self.logs
            .lock()
            .ok()
            .and_then(|logs| logs.get(step).cloned())
    }

    /// How many finished blocks were drawn, and the run directory the
    /// latest step log lies in (`<run>/steps/<id>/output.log`).
    pub(super) fn summary(&self) -> (usize, Option<PathBuf>) {
        let last = self.last.lock().ok().and_then(|last| last.clone());
        let run = last.and_then(|log| {
            let steps = log.parent()?.parent()?;
            (steps.file_name()? == "steps").then(|| steps.parent().map(Path::to_path_buf))?
        });
        (self.rows.load(Ordering::Relaxed), run)
    }
}

/// What one task is, as the renderers draw it.
#[derive(Debug, Clone)]
pub(super) struct Headline {
    /// The immediate parent task, independent of its visible depth.
    pub(super) parent_id: Option<TaskId>,
    /// The step id or the label's first column — never cut.
    pub(super) name: String,
    /// The rest of the label.
    pub(super) title: String,
    /// Visible depth: 0 for a block of its own, 1 and deeper for children.
    pub(super) depth: usize,
    /// The command's own task: never drawn as a row.
    pub(super) command: bool,
}

impl Headline {
    /// Read a task label: `<id> (<kind>)  <title>` for a step, any other
    /// text as a name alone. The kind is the plan's to show; a running or
    /// finished block names the step and its title (PROP-067 `#collapse`).
    pub(super) fn parse(label: &str, depth: usize, command: bool) -> Self {
        let (name, title) = match label.split_once("  ") {
            Some((name, title)) => (name, title.trim()),
            None => (label, ""),
        };
        let name = match name.rsplit_once(" (") {
            Some((id, kind)) if kind.ends_with(')') && !id.contains(' ') => id,
            _ => name,
        };
        // Each part is made terminal-safe on its own: the sanitiser folds
        // the two spaces that separate them.
        Self {
            parent_id: None,
            name: super::sanitize(name),
            title: super::sanitize(title),
            depth,
            command,
        }
    }

    /// The headline text after the gutter: the name padded to the list's
    /// name column, then the title; a child carries its branch glyph at its depth.
    pub(super) fn text(&self, name_width: usize) -> String {
        let indent = match self.depth {
            0 => String::new(),
            depth => format!("{}{} ", "  ".repeat(depth - 1), Glyph::Child.text()),
        };
        if self.title.is_empty() {
            return format!("{indent}{}", self.name);
        }
        let width = if self.depth == 0 { name_width } else { 0 };
        format!("{indent}{:<width$}  {}", self.name, self.title)
    }

    /// The prefix a line under this task starts with (fold, detail).
    pub(super) fn under(&self) -> String {
        "  ".repeat(self.depth + 1)
    }
}

/// How a task ended.
#[derive(Debug, Clone)]
pub(super) enum End {
    Finished,
    Skipped(String),
    Failed(String),
    Stopped,
}

/// The collapsed block of a task that ended: its gutter line and, for a
/// failure, the evidence kept from its log.
pub(super) fn collapsed(
    headline: &Headline,
    task_id: vibe_core::progress::TaskId,
    name_width: usize,
    end: &End,
    elapsed: Duration,
    logs: &StepLogs,
) -> String {
    // Both renderers exclude command roots and suppressed duplicates before
    // collapsing; every remaining row is a visible completed block, at any depth.
    logs.rows.fetch_add(1, Ordering::Relaxed);
    let log = logs.of(task_id, &headline.name);
    let (count, lines) = log.as_deref().map(read_lines).unwrap_or_default();
    let counted = |meta: &mut Vec<Meta>| {
        if count > 0 {
            let unit = if count == 1 { "line" } else { "lines" };
            meta.push(Meta::Count(format!("{count} {unit}")));
        }
    };
    let text = headline.text(name_width);
    let (glyph, role, mut meta, dim) = match end {
        End::Finished => {
            let mut meta = vec![Meta::Time(elapsed)];
            counted(&mut meta);
            (Glyph::Done, Role::Ok, meta, false)
        }
        End::Skipped(reason) => (
            Glyph::Skipped,
            Role::Dim,
            vec![Meta::Note(format!("skipped: {reason}"))],
            true,
        ),
        End::Stopped => (Glyph::Skipped, Role::Dim, vec![Meta::note("stopped")], true),
        End::Failed(message) => {
            let mut meta = vec![Meta::Time(elapsed)];
            match exit_status(message) {
                Some(code) => meta.push(Meta::Alert(format!("exit {code}"))),
                None if log.is_none() => meta.push(Meta::Alert(message.clone())),
                None => {}
            }
            (Glyph::Failed, Role::Alert, meta, false)
        }
    };
    let mut body = Vec::new();
    if let (End::Failed(_), Some(path)) = (end, &log) {
        meta.push(Meta::Note(format!(
            "log {}",
            language::short_path(&path.display().to_string())
        )));
        let kept = lines.len().saturating_sub(EVIDENCE);
        body = lines[kept..]
            .iter()
            .map(|line| Line::Evidence(crate::output::sanitize_progress_text(line)))
            .collect();
    }
    let mut block = language::Block::new(glyph, text)
        .with_meta(meta)
        .with_body(body);
    block.glyph_role = role;
    if dim {
        block.headline_role = Role::Dim;
    }
    let width = language::viewport();
    let rendered = block.render_at(width);
    if dim {
        // A skipped row is dim through its metadata too.
        return paint(Role::Dim, &language::cut(&strip(&rendered), width));
    }
    rendered
}

/// The text of a row without attributes, so a dim row is dim end to end.
fn strip(text: &str) -> String {
    console::strip_ansi_codes(text).into_owned()
}

/// The exit status a child's failure names, when it names one.
fn exit_status(message: &str) -> Option<i32> {
    for marker in [
        "exit status: ",
        "exit status ",
        "exit code: ",
        "exit code ",
        "with code ",
        "status ",
    ] {
        if let Some(at) = message.find(marker) {
            let digits: String = message[at + marker.len()..]
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '-')
                .collect();
            if let Ok(code) = digits.parse() {
                return Some(code);
            }
        }
    }
    None
}

fn read_lines(path: &Path) -> (usize, Vec<String>) {
    use std::io::Read;
    let Ok(mut file) = std::fs::File::open(path) else {
        return (0, Vec::new());
    };
    let mut tail = std::collections::VecDeque::new();
    let mut line = Vec::new();
    let mut bytes = [0u8; 16384];
    let mut count = 0;
    let mut pending = false;
    while let Ok(n) = file.read(&mut bytes) {
        if n == 0 {
            break;
        }
        for &b in &bytes[..n] {
            if b == b'\n' {
                count += 1;
                tail.push_back(String::from_utf8_lossy(&line).into_owned());
                if tail.len() > EVIDENCE {
                    tail.pop_front();
                }
                line.clear();
                pending = false;
            } else {
                pending = true;
                if line.len() < 4096 {
                    line.push(b);
                }
            }
        }
    }
    if pending {
        count += 1;
        tail.push_back(String::from_utf8_lossy(&line).into_owned());
        if tail.len() > EVIDENCE {
            tail.pop_front();
        }
    }
    (count, tail.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn log_tail_memory_is_bounded_for_long_invalid_lines() {
        use std::io::Write;
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("huge.log");
        let mut file = std::fs::File::create(&path).unwrap();
        for _ in 0..40 {
            file.write_all(&vec![255; 100000]).unwrap();
            file.write_all(b"\n").unwrap();
        }
        file.write_all(b"FINAL").unwrap();
        drop(file);
        let (count, tail) = read_lines(&path);
        assert_eq!(count, 41);
        assert_eq!(tail.len(), EVIDENCE);
        assert_eq!(tail.last().unwrap(), "FINAL");
        assert!(tail.iter().all(|line| line.len() <= 4096 * 3));
    }
}
