//! One bounded display discipline for every wire diagnostic.
//!
//! Hostile input is attacker-sized: a raw address, a backend id, an object
//! key or a parser's own error message can be megabytes. A refusal that
//! echoes it amplifies the attack into the log, and a refusal that formats it
//! and *then* truncates has already materialized it. So every diagnostic in
//! this conversion renders through this cell — a fixed character cap, the
//! TRUE byte length beside it, and, for a `Display` value, a `fmt::Write`
//! sink that stops keeping characters once the cap is reached.

use std::fmt::{self, Write as _};

/// Characters kept from any previewed value. Enough to recognise the input,
/// far too little to amplify it.
const CAP: usize = 48;

/// A borrowed value: the true byte length plus a capped prefix.
pub(super) fn preview(value: &str) -> String {
    let kept: String = value.chars().take(CAP).collect();
    // O(1) after the take: fewer bytes kept than the value has means elision.
    let elided = if kept.len() < value.len() { "…" } else { "" };
    format!("{} bytes, starts `{kept}{elided}`", value.len())
}

/// A `Display` value — a parser error, a source error — rendered through the
/// bounded sink, so its full text is never built in order to be shortened.
pub(super) fn display(value: impl fmt::Display) -> String {
    let mut sink = Sink::default();
    // `Sink` never fails, and a `Display` impl that does is its own bug: the
    // bounded text collected so far is still the honest diagnostic.
    let _ = write!(sink, "{value}");
    sink.finish()
}

/// A typed error rendered through its DERIVED `Debug`, never its `Display`.
///
/// Some `Display` impls build their text before the formatter ever sees it —
/// `VerificationError`'s cycle variant joins the whole path with `" -> "` —
/// so an outer sink could only truncate an allocation that already happened.
/// The derived `Debug` writes field by field through the formatter, so this
/// sink stops KEEPING characters at the cap while the value is still being
/// walked. The variant name leads the output, so the typed family survives.
pub(super) fn debug(value: impl fmt::Debug) -> String {
    let mut sink = Sink::default();
    let _ = write!(sink, "{value:?}");
    sink.finish()
}

#[derive(Default)]
struct Sink {
    text: String,
    kept: usize,
    dropped: usize,
}

impl Sink {
    fn finish(self) -> String {
        if self.dropped == 0 {
            self.text
        } else {
            format!("{}… (+{} chars elided)", self.text, self.dropped)
        }
    }
}

impl fmt::Write for Sink {
    fn write_str(&mut self, chunk: &str) -> fmt::Result {
        for character in chunk.chars() {
            if self.kept < CAP {
                self.text.push(character);
                self.kept += 1;
            } else {
                self.dropped += 1;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{CAP, debug, display, preview};

    #[test]
    fn a_huge_value_previews_to_a_bounded_string_with_its_true_length() {
        let value = "x".repeat(4 * 1024 * 1024);
        let rendered = preview(&value);
        assert!(rendered.contains("4194304 bytes"), "{rendered}");
        assert!(rendered.chars().count() < CAP + 64, "{rendered}");
    }

    #[test]
    fn a_short_value_is_previewed_whole_without_an_ellipsis() {
        let rendered = preview("static-md");
        assert_eq!(rendered, "9 bytes, starts `static-md`");
    }

    /// `debug` must never reach `Display`. This value's `Display` PANICS, so
    /// the test itself is the structural proof — and its `Debug` is huge, so
    /// the cap is exercised at the same time.
    #[test]
    fn bounded_debug_never_invokes_display() {
        struct DisplayIsATrap;
        impl std::fmt::Display for DisplayIsATrap {
            fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                panic!("a bounded renderer must not call Display");
            }
        }
        impl std::fmt::Debug for DisplayIsATrap {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("CycleDetected { path: [")?;
                for _ in 0..100_000 {
                    formatter.write_str("spec://org.demo/lib/manual/part.md, ")?;
                }
                formatter.write_str("] }")
            }
        }
        let rendered = debug(DisplayIsATrap);
        assert!(rendered.starts_with("CycleDetected"), "{rendered}");
        assert!(rendered.chars().count() < CAP + 48, "{rendered}");
        assert!(rendered.contains("chars elided"), "{rendered}");
    }

    #[test]
    fn a_huge_display_is_never_rendered_in_full() {
        struct Huge;
        impl std::fmt::Display for Huge {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                for _ in 0..100_000 {
                    formatter.write_str("chunk ")?;
                }
                Ok(())
            }
        }
        let rendered = display(Huge);
        assert!(rendered.chars().count() < CAP + 48, "{rendered}");
        assert!(rendered.contains("chars elided"), "{rendered}");
    }
}
