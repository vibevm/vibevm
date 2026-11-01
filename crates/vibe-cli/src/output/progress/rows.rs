//! The collapsed line a finished task leaves behind (PROP-067 `#collapse`),
//! shared by the interactive and the plain renderer so a log reads block
//! for block as the screen did (`#plain`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#collapse");

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use crate::output::language::{self, Glyph, Line, Meta, Role, paint};

/// How many lines of a failed step's log stay as its evidence.
pub(super) const EVIDENCE: usize = 30;

/// What this run's rows have recorded: the step logs by step id, so a
/// finished row can count its lines and a failed one keep its tail, and how
/// many blocks the list of finished blocks holds — the trailer's count.
#[derive(Default)]
pub(super) struct StepLogs {
    logs: Mutex<BTreeMap<String, PathBuf>>,
    rows: AtomicUsize,
    last: Mutex<Option<PathBuf>>,
}

impl StepLogs {
    pub(super) fn note(&self, step: &str, log: &Path) {
        if let Ok(mut logs) = self.logs.lock() {
            logs.insert(step.to_owned(), log.to_owned());
        }
        if let Ok(mut last) = self.last.lock() {
            *last = Some(log.to_owned());
        }
    }

    fn of(&self, step: &str) -> Option<PathBuf> {
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
    name_width: usize,
    end: &End,
    elapsed: Duration,
    logs: &StepLogs,
) -> String {
    if headline.depth == 0 {
        logs.rows.fetch_add(1, Ordering::Relaxed);
    }
    let log = logs.of(&headline.name);
    let lines = log.as_deref().map(read_lines).unwrap_or_default();
    let counted = |meta: &mut Vec<Meta>| {
        if !lines.is_empty() {
            let unit = if lines.len() == 1 { "line" } else { "lines" };
            meta.push(Meta::Count(format!("{} {unit}", lines.len())));
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

fn read_lines(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .map(|text| text.lines().map(str::to_owned).collect())
        .unwrap_or_default()
}
