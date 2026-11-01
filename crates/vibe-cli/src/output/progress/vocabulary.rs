//! How a running block reads on an interactive terminal: the spinner that
//! stands in for its `●`, the bars of a measured task, and its elapsed time
//! after ` · ` (PROP-067 `#block`, `#meta`). The glyphs and roles are the
//! display language's; only the indicatif templates live here.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-060#DISPLAY");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#block");

use indicatif::{ProgressState, ProgressStyle};

use crate::output::language::{self, Role, paint};

/// The elapsed time of a running block, in the metadata role.
fn with_elapsed(template: &str) -> ProgressStyle {
    ProgressStyle::with_template(template)
        .unwrap_or_else(|_| ProgressStyle::default_spinner())
        .with_key(
            "t",
            |state: &ProgressState, out: &mut dyn std::fmt::Write| {
                let _ = write!(
                    out,
                    "{}",
                    paint(
                        Role::Dim,
                        &format!(" · {}", language::elapsed(state.elapsed()))
                    )
                );
            },
        )
        .tick_strings(language::spinner_frames())
}

/// A running block: the spinner in the gutter, the headline, the time.
pub(super) fn spinner_style() -> ProgressStyle {
    with_elapsed("{spinner} {msg}{t}")
}

pub(super) fn bar_style() -> ProgressStyle {
    with_elapsed("{spinner} {msg} {bar:16} {pos}/{len} {prefix}{t}").progress_chars("━╸ ")
}

pub(super) fn bytes_bar_style() -> ProgressStyle {
    with_elapsed("{spinner} {msg} {bar:16} {bytes} of {total_bytes}{t}").progress_chars("━╸ ")
}

pub(super) fn measured_spinner_style() -> ProgressStyle {
    with_elapsed("{spinner} {msg} · {pos} {prefix}{t}")
}

pub(super) fn bytes_spinner_style() -> ProgressStyle {
    with_elapsed("{spinner} {msg} · {bytes}{t}")
}

/// A fold or hint line under a running block: drawn as composed, never
/// wider than the viewport.
pub(super) fn line_style() -> ProgressStyle {
    ProgressStyle::with_template("{wide_msg}").unwrap_or_else(|_| ProgressStyle::default_spinner())
}
