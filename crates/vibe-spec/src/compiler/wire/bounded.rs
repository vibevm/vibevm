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

/// Room left for the elision suffix, so a BYTE-capped render never overshoots
/// its ceiling by announcing how much it dropped.
const SUFFIX_RESERVE: usize = 48;

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
    let mut sink = Sink::chars(CAP);
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
    let mut sink = Sink::chars(CAP);
    let _ = write!(sink, "{value:?}");
    sink.finish()
}

/// [`debug`]'s discipline at a caller-chosen ceiling in UTF-8 BYTES — the
/// shape a wire epoch states its diagnostic cap in.
///
/// The sink stops keeping characters below the ceiling (leaving room for the
/// elision suffix), so the attacker-sized text is never materialized; the
/// final clamp only ever trims the already bounded result, on a character
/// boundary, and is total for a ceiling too small to hold the suffix at all.
pub(crate) fn debug_within(value: impl fmt::Debug, cap_bytes: usize) -> String {
    let mut sink = Sink::bytes(cap_bytes.saturating_sub(SUFFIX_RESERVE));
    let _ = write!(sink, "{value:?}");
    let mut rendered = sink.finish();
    if rendered.len() > cap_bytes {
        let mut end = cap_bytes;
        while end > 0 && !rendered.is_char_boundary(end) {
            end -= 1;
        }
        rendered.truncate(end);
    }
    rendered
}

struct Sink {
    text: String,
    kept: usize,
    dropped: usize,
    chars: usize,
    bytes: usize,
}

impl Sink {
    fn chars(cap: usize) -> Self {
        Self::new(cap, usize::MAX)
    }

    fn bytes(cap: usize) -> Self {
        Self::new(usize::MAX, cap)
    }

    fn new(chars: usize, bytes: usize) -> Self {
        Self {
            text: String::new(),
            kept: 0,
            dropped: 0,
            chars,
            bytes,
        }
    }

    fn accepts(&self, character: char) -> bool {
        self.kept < self.chars && self.text.len() + character.len_utf8() <= self.bytes
    }

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
            if self.accepts(character) {
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
    use vibe_wire::behaviour::compiler_trace_index::DIAGNOSTIC_CAP_BYTES;

    use super::{CAP, debug, debug_within, display, preview};

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

    /// The same trap value under the BYTE ceiling: `Display` is never reached,
    /// the variant family survives, and the render fits the trace epoch's cap
    /// even though the value would print megabytes.
    #[test]
    fn a_byte_capped_render_stays_within_the_trace_epoch_ceiling() {
        struct DisplayIsATrap;
        impl std::fmt::Display for DisplayIsATrap {
            fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                panic!("a bounded renderer must not call Display");
            }
        }
        impl std::fmt::Debug for DisplayIsATrap {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("PassFailed { source: ")?;
                for _ in 0..200_000 {
                    formatter.write_str("привет-мир, ")?;
                }
                formatter.write_str("}")
            }
        }
        let rendered = debug_within(DisplayIsATrap, DIAGNOSTIC_CAP_BYTES);
        assert!(rendered.starts_with("PassFailed"), "{rendered}");
        assert!(rendered.len() <= DIAGNOSTIC_CAP_BYTES, "{}", rendered.len());
        assert!(
            rendered.len() > DIAGNOSTIC_CAP_BYTES - 128,
            "{}",
            rendered.len()
        );
        assert!(rendered.contains("chars elided"), "{rendered}");
    }

    /// A ceiling too small for the elision suffix still yields a valid,
    /// in-budget string rather than an overshoot or a split character.
    #[test]
    fn a_tiny_byte_ceiling_is_total() {
        let rendered = debug_within("ёжик-ёжик-ёжик", 8);
        assert!(rendered.len() <= 8, "{rendered}");
        assert!(rendered.is_char_boundary(rendered.len()));
        assert_eq!(debug_within("ok", DIAGNOSTIC_CAP_BYTES), "\"ok\"");
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
