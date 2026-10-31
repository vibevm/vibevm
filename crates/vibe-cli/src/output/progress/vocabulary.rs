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

pub(super) fn indent_label(depth: usize, label: &str) -> String {
    format!("{}{}", "  ".repeat(depth), label)
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
