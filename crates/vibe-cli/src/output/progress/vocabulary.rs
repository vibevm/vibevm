//! How one progress row reads: the bar and spinner templates a task takes
//! on, the indent of a nested label, and the short human spellings of a
//! diagnostic level, an elapsed duration and a byte count.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-060#DISPLAY");

use indicatif::ProgressStyle;
use std::time::Duration;
use vibe_core::progress::ProgressDiagnosticLevel;

pub(super) fn spinner_style() -> ProgressStyle {
    ProgressStyle::with_template("{spinner:.cyan} {wide_msg} [{elapsed_precise}]")
        .unwrap_or_else(|_| ProgressStyle::default_spinner())
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
}

pub(super) fn bar_style() -> ProgressStyle {
    ProgressStyle::with_template(
        "{bar:16.cyan/blue} {pos}/{len} {prefix} {wide_msg} [{elapsed_precise}]",
    )
    .unwrap_or_else(|_| ProgressStyle::default_bar())
    .progress_chars("=>-")
}

pub(super) fn bytes_bar_style() -> ProgressStyle {
    ProgressStyle::with_template(
        "{bar:16.cyan/blue} {bytes}/{total_bytes} {wide_msg} [{elapsed_precise}]",
    )
    .unwrap_or_else(|_| ProgressStyle::default_bar())
    .progress_chars("=>-")
}

pub(super) fn measured_spinner_style() -> ProgressStyle {
    ProgressStyle::with_template("{spinner:.cyan} {pos} {prefix} {wide_msg} [{elapsed_precise}]")
        .unwrap_or_else(|_| ProgressStyle::default_spinner())
}

pub(super) fn bytes_spinner_style() -> ProgressStyle {
    ProgressStyle::with_template("{spinner:.cyan} {bytes} {wide_msg} [{elapsed_precise}]")
        .unwrap_or_else(|_| ProgressStyle::default_spinner())
}

/// The width of the spinner-or-marker column every row starts with, so a
/// tree keeps its columns across live, finished and folded lines.
const GUTTER: &str = "  ";

/// The glyphs nested tasks are drawn with (PROP-060 `#DISPLAY`).
///
/// The decision is the renderer's one glyph decision, shared with the
/// prompt theme of `output::prompts`: redirected output keeps the glyphs
/// and loses only its colour, and `TermFamily::Dummy` is the one family
/// `console` reports as having no terminal vocabulary at all.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-060#DISPLAY")]
pub(super) struct TreeGlyphs {
    /// A child with siblings still to come.
    branch: &'static str,
    /// The vertical that carries past a child, and a continuation line.
    stem: &'static str,
}

impl TreeGlyphs {
    pub(super) const fn unicode() -> Self {
        Self {
            branch: "├─ ",
            stem: "│  ",
        }
    }

    /// Three columns wide like the Unicode set, so a tree keeps its
    /// columns whichever vocabulary draws it.
    pub(super) const fn ascii() -> Self {
        Self {
            branch: "|- ",
            stem: "|  ",
        }
    }

    pub(super) fn detect() -> Self {
        match console::Term::stderr().features().family() {
            console::TermFamily::Dummy => Self::ascii(),
            _ => Self::unicode(),
        }
    }

    /// One task's row: the verticals of its ancestors, then its own
    /// branch. A root carries none.
    ///
    /// The last child of a parent would read `└─`, but a row is printed
    /// the moment the task ends and nothing then knows whether a sibling
    /// follows; every child therefore branches, and the parent's own
    /// terminal row closes the group.
    pub(super) fn label(&self, depth: usize, label: &str) -> String {
        match depth {
            0 => label.to_owned(),
            depth => format!("{}{}{label}", self.stem.repeat(depth - 1), self.branch),
        }
    }

    /// A line that belongs to a task without being a task of its own — a
    /// folded child-output line, a diagnostic, a verbose detail: the
    /// task's own verticals, so the line starts in the task's label
    /// column.
    pub(super) fn under(&self, depth: usize) -> String {
        format!("{GUTTER}{}", self.stem.repeat(depth))
    }
}

pub(super) fn diagnostic_marker(level: ProgressDiagnosticLevel) -> &'static str {
    match level {
        ProgressDiagnosticLevel::Warning => "warning:",
        ProgressDiagnosticLevel::Error => "error:",
    }
}

pub(super) fn diagnostic_word(level: ProgressDiagnosticLevel) -> &'static str {
    match level {
        ProgressDiagnosticLevel::Warning => "warning",
        ProgressDiagnosticLevel::Error => "error",
    }
}

pub(super) fn format_elapsed(elapsed: Duration) -> String {
    let seconds = elapsed.as_secs();
    if seconds >= 60 {
        format!("{}m {:02}s", seconds / 60, seconds % 60)
    } else if seconds > 0 {
        format!("{seconds}s")
    } else {
        format!("{}ms", elapsed.as_millis())
    }
}

pub(super) fn human_bytes(bytes: u64) -> String {
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
