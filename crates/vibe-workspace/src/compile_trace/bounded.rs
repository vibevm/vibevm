//! Diagnostics that are bounded *while they are built*, not after.
//!
//! Everything this writer reports about can be hostile. A scope label, a
//! fingerprint, a filename read off disk and a compiler diagnostic all arrive
//! from somewhere the writer does not control, and a trace is precisely the
//! subsystem that quotes them back. Two mistakes are easy here and both are
//! made by the obvious code:
//!
//! * `format!(…)` first, clamp second. The full multi-megabyte string is
//!   already allocated by the time the clamp runs, so the clamp bounds the
//!   *result* and not the *cost*.
//! * clamp to `cap` bytes and then append an ellipsis. The result is
//!   `cap + 3` bytes — over the wire epoch's ceiling, which is the one number
//!   the validator will actually refuse.
//!
//! So this cell is a streaming [`fmt::Write`] sink with the marker's width
//! RESERVED up front. Callers pass `format_args!`, so no intermediate string
//! exists; the sink copies characters until the reserve boundary and then
//! stops answering, so a 4 MiB label costs a bounded number of pushes rather
//! than a 4 MiB allocation. The final length is `≤ cap` with the marker
//! already counted, and the cut lands on a character boundary because whole
//! `char`s are what get pushed.
//!
//! The two ceilings are the wire epoch's own — [`DIAGNOSTIC_CAP_BYTES`] for a
//! diagnostic the index may carry, [`SCALAR_PREVIEW_BYTES`] for an untrusted
//! identity quoted inside a refusal — so a producer and its validator cannot
//! drift apart over a copied number.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE");

use std::fmt::{self, Write};
use std::path::Path;

use vibe_wire::behaviour::compiler_trace_index::{DIAGNOSTIC_CAP_BYTES, SCALAR_PREVIEW_BYTES};

/// What a truncated text ends with. Counted against the cap, never added on
/// top of it.
const MARK: &str = "…";

/// Render `args` into at most [`DIAGNOSTIC_CAP_BYTES`] bytes, marker
/// included. Nothing larger is ever materialised.
pub(super) fn diagnostic(args: fmt::Arguments<'_>) -> String {
    render(args, DIAGNOSTIC_CAP_BYTES)
}

/// The same for an untrusted identity quoted inside a refusal — a run id, a
/// scope id, a filename read off disk — at the epoch's preview ceiling.
pub(super) fn preview(value: &str) -> String {
    render(format_args!("{value}"), SCALAR_PREVIEW_BYTES)
}

/// A path as a bounded display string. Paths are ours, but the last
/// component of one can be an arbitrary directory name this writer merely
/// listed, so it gets the same treatment as any other quoted text.
pub(super) fn path(value: &Path) -> String {
    diagnostic(format_args!("{}", value.display()))
}

fn render(args: fmt::Arguments<'_>, cap: usize) -> String {
    let mut sink = Sink::new(cap);
    // `fmt::Write` for this sink never fails, so the result is `Ok` for every
    // well-behaved `Display`; a `Display` that returns `Err` on its own has
    // simply written less, and the partial text is still the honest answer.
    let _ = sink.write_fmt(args);
    sink.finish()
}

/// A `fmt::Write` sink that never holds more than `cap` bytes.
///
/// The reserve is taken at the END rather than up front, so a text that fits
/// exactly is verbatim and only a text that genuinely overran pays for the
/// marker. Nothing larger than the cap is ever allocated either way: the sink
/// stops copying the moment the next character would pass it, and the trim is
/// a pop from a buffer already at most `cap` bytes long.
struct Sink {
    text: String,
    cap: usize,
    overflowed: bool,
}

impl Sink {
    fn new(cap: usize) -> Self {
        Self {
            text: String::new(),
            cap,
            overflowed: false,
        }
    }

    fn finish(mut self) -> String {
        // Not truncated: the text IS the answer, marker or not.
        if !self.overflowed || self.cap < MARK.len() {
            return self.text;
        }
        // Make exactly enough room for the marker by popping whole
        // characters — the only thing ever pushed, so the cut cannot land
        // inside one.
        while self.text.len() + MARK.len() > self.cap && self.text.pop().is_some() {}
        self.text.push_str(MARK);
        self.text
    }
}

impl Write for Sink {
    fn write_str(&mut self, chunk: &str) -> fmt::Result {
        if self.overflowed {
            // Already full. Returning `Ok` rather than `Err` matters: an
            // `Err` would make some `Display` impls behave differently
            // depending on how much of them fit, and the point here is to
            // observe them, not to change them.
            return Ok(());
        }
        for character in chunk.chars() {
            if self.text.len() + character.len_utf8() > self.cap {
                self.overflowed = true;
                return Ok(());
            }
            self.text.push(character);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole contract in one place: never over the cap, marker included;
    /// never a split character; and a hostile input costs a bounded answer.
    #[test]
    fn a_hostile_render_stays_under_the_cap_with_its_marker_counted() {
        // A three-byte character does not tile either cap evenly, so the cut
        // genuinely has to walk to a boundary.
        let hostile = "☃".repeat(4 * 1024 * 1024);
        for (rendered, cap) in [
            (diagnostic(format_args!("{hostile}")), DIAGNOSTIC_CAP_BYTES),
            (preview(&hostile), SCALAR_PREVIEW_BYTES),
            (
                diagnostic(format_args!("publishing `{hostile}`: {hostile}")),
                DIAGNOSTIC_CAP_BYTES,
            ),
        ] {
            assert!(rendered.len() <= cap, "{} > {cap}", rendered.len());
            assert!(rendered.ends_with(MARK), "a truncated text says so");
            assert!(
                rendered
                    .chars()
                    .all(|c| c == '☃' || c == '…' || c.is_ascii()),
                "no character was split",
            );
        }
    }

    /// Under the cap nothing is added and nothing is lost.
    #[test]
    fn a_small_render_is_verbatim() {
        assert_eq!(diagnostic(format_args!("a {} c", "b")), "a b c");
        assert_eq!(preview("node:."), "node:.");
        assert!(!diagnostic(format_args!("short")).ends_with(MARK));
    }

    /// Exactly at the cap is not a truncation; one byte more is.
    #[test]
    fn the_boundary_is_the_cap_itself() {
        let exact = "a".repeat(SCALAR_PREVIEW_BYTES);
        assert_eq!(preview(&exact), exact);
        let over = "a".repeat(SCALAR_PREVIEW_BYTES + 1);
        let rendered = preview(&over);
        assert_eq!(rendered.len(), SCALAR_PREVIEW_BYTES);
        assert!(rendered.ends_with(MARK));
    }

    /// A cap too small for the marker truncates without one rather than
    /// overrunning to announce itself.
    #[test]
    fn a_cap_below_the_marker_never_overruns() {
        for cap in 0..MARK.len() {
            let rendered = render(format_args!("{}", "x".repeat(100)), cap);
            assert_eq!(rendered.len(), cap);
        }
    }
}
