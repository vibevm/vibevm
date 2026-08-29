//! The `xml-minify` binding: the FIRST production transform behavior
//! (R4 architecture §8), and the segmented emitted-tape adapter it drives.
//!
//! **This cell is binding and segmentation, never a serializer.** The strict
//! span-deletion kernel already exists
//! ([`crate::transforms::minify_emitted_xml`]) and stays exactly as strict:
//! nothing here relaxes it, and nothing here writes XML. What this cell adds
//! is *where* the kernel is allowed to look.
//!
//! **One stage, honestly.** The kernel's whole surface is `&str → Cow<str>`,
//! so the one carrier it can serve without inventing a serializer is the
//! EMITTED tape. A lane behavior receives a structured [`crate::compiler::ir::LaneIr`]
//! whose contributions are parsed documents; minifying those would mean
//! editing a tree and re-rendering it — a new serializer, which R4.2 is
//! explicitly not. So the behavior declares [`TransformStage::Emitted`] and
//! the registry's own stage law refuses every other stage
//! (`TransformRegistry::resolve` → `StageMismatch`, and the trait's
//! `wrong_stage` default underneath it).
//!
//! **Why the tape is SEGMENTED (R4 architecture §2.2's ruling).** The wire
//! tape gate demands the EXACT context-owned prologue — the three provenance
//! comments, the optional §7.1 transforms header, the blank separator, the
//! resolution preamble and any tombstone — and reconciles every contribution
//! marker against the carried witnesses. Those bytes are engine framing, not
//! plugin bytes: an artifact whose own framing a transform had rewritten
//! would be refused by its own gate. So the adapter minifies WITHIN document
//! segments only. Every engine-framed comment span and every inter-segment
//! byte is copied verbatim, and the segmenter — not the kernel's incidental
//! treatment of top-level whitespace — is what guarantees it.
//!
//! **The segmentation is the emit cell's own grammar, read back.** The spans
//! are found by [`framing::GENERATED_COMMENT_OPEN`] /
//! [`framing::GENERATED_COMMENT_CLOSE`], the constants the emitters WRITE
//! with, and a hoisted marker is recognised through
//! [`framing::hoisted_marker_origin`], the reader that shares one spelling
//! with `hoisted_marker_payload`. There is no second framing grammar in this
//! file.
//!
//! **The hoisted refusal (R4 architecture §8).** A hoisted contribution
//! writes a bare top-level `#use spec://…` line into an XML lane — text
//! outside any element, which is neither a document segment the kernel can
//! judge nor framing this adapter owns. Until an engine-owned segmented
//! adapter handles that line honestly, the active transform REFUSES the
//! artifact by name, carrying the marker's byte offset and a bounded preview
//! of the origin it names. It never skips the artifact silently and never
//! hands the line to the kernel.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#STAGE-EMITTED");

use std::borrow::Cow;
use std::ops::Range;

use crate::compiler::emit::framing;
use crate::transforms::{XmlMinifyError, minify_emitted_xml};

use super::behavior::{TransformBehavior, TransformBehaviorError};
use super::plan::{TransformConfig, TransformStage};
use super::plan_validate::{BoundedPreview, bounded};

/// The catalog name of the first production transform builtin — the exact
/// spelling a manifest writes in `handler = { kind = "builtin", name = … }`.
pub(crate) const XML_MINIFY_NAME: &str = "xml-minify";

/// Its registry-owned behavior epoch (ABI §4: a builtin's epoch is
/// registry-owned and must bump with observable behavior).
pub(crate) const XML_MINIFY_EPOCH: u32 = 1;

/// The `xml-minify` behavior: the segmented emitted-tape adapter over the
/// strict span-deletion kernel.
pub(crate) struct XmlMinify;

impl TransformBehavior for XmlMinify {
    fn name(&self) -> &str {
        XML_MINIFY_NAME
    }

    fn epoch(&self) -> u32 {
        XML_MINIFY_EPOCH
    }

    fn stage(&self) -> TransformStage {
        TransformStage::Emitted
    }

    /// Minify every document segment of the tape, or refuse it typed.
    ///
    /// Byte-equal output returns the caller's OWN bytes — the value T9's
    /// reconstruction compares against the original artifact, so an artifact
    /// this behavior did not change stays `Eq` to the untransformed compile.
    fn run_emitted(
        &self,
        _config: Option<&TransformConfig>,
        input: Vec<u8>,
    ) -> Result<Vec<u8>, TransformBehaviorError> {
        let minified = {
            let tape = std::str::from_utf8(&input).map_err(|error| {
                self.refused(XmlMinifyBindingError::NotUtf8 {
                    offset: error.valid_up_to(),
                })
            })?;
            match minify_tape(tape).map_err(|refusal| self.refused(refusal))? {
                // Borrowed IS the no-change answer: `render_segments` only
                // allocates once a segment really moved.
                Cow::Borrowed(_) => None,
                Cow::Owned(rendered) => Some(rendered.into_bytes()),
            }
        };
        Ok(match minified {
            Some(bytes) => bytes,
            None => input,
        })
    }
}

impl XmlMinify {
    /// Project one adapter refusal onto the behavior family's one arm.
    fn refused(&self, source: XmlMinifyBindingError) -> TransformBehaviorError {
        TransformBehaviorError::EmittedTape {
            preview: bounded(self.name()),
            source,
        }
    }
}

/// Why the segmented emitted-tape adapter refused one artifact.
///
/// Typed by fault and bounded by the same law the plan refusals obey: an
/// origin spelling comes from a manifest and can be attacker-sized, so it
/// rides as a fixed-size preview plus its true length, beside the exact byte
/// offset that locates it in the tape.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum XmlMinifyBindingError {
    #[error("the emitted artifact is not valid UTF-8 (first invalid byte at {offset})")]
    NotUtf8 { offset: usize },
    #[error(
        "the emitted tape opens an engine-framed comment at byte {offset} that it never terminates"
    )]
    UnterminatedFrameComment { offset: usize },
    #[error(
        "the emitted tape carries a hoisted contribution ({origin}) at byte {offset}: its top-level `#use` line is not a document segment, and the segmented adapter refuses the artifact rather than skipping it silently or corrupting it"
    )]
    HoistedContribution {
        origin: BoundedPreview,
        offset: usize,
    },
    #[error("the emitted document segment at byte {offset} is not minifiable XML: {source}")]
    Segment {
        offset: usize,
        #[source]
        source: XmlMinifyError,
    },
}

/// XML 1.0 whitespace — the only characters this adapter treats as
/// inter-segment padding, matching the kernel's own definition.
fn is_xml_whitespace(character: char) -> bool {
    matches!(character, ' ' | '\t' | '\r' | '\n')
}

/// Minify every document segment of one emitted tape.
///
/// `Cow::Borrowed` means no segment moved, which is the byte-equal case T9
/// pins; `Cow::Owned` is strictly the frame bytes plus minified segments.
fn minify_tape(tape: &str) -> Result<Cow<'_, str>, XmlMinifyBindingError> {
    let mut rendered = String::new();
    let mut changed = false;
    let mut cursor = 0usize;
    loop {
        let frame = next_frame_comment(tape, cursor)?;
        let gap_end = frame.as_ref().map_or(tape.len(), |span| span.start);
        changed |= push_gap(&mut rendered, &tape[cursor..gap_end], cursor)?;
        let Some(span) = frame else {
            break;
        };
        let comment = &tape[span.start..span.end];
        if let Some(origin) = framing::hoisted_origin_in_comment(comment) {
            return Err(XmlMinifyBindingError::HoistedContribution {
                origin: bounded(&origin),
                offset: span.start,
            });
        }
        // Frame bytes, always verbatim: this is the whole of §2.2's ruling.
        rendered.push_str(comment);
        cursor = span.end;
    }
    Ok(if changed {
        Cow::Owned(rendered)
    } else {
        Cow::Borrowed(tape)
    })
}

/// The next engine-framed comment span at or after `from`, or `None`.
///
/// A generated comment always opens a LINE — every emitter writes it that
/// way — so an opening mid-line is content, not framing. The shared codec
/// guarantees an encoded payload contains no `--`, so the first
/// [`framing::GENERATED_COMMENT_CLOSE`] after an opening really is that
/// comment's terminator.
fn next_frame_comment(
    tape: &str,
    from: usize,
) -> Result<Option<Range<usize>>, XmlMinifyBindingError> {
    let mut search = from;
    while let Some(found) = tape[search..].find(framing::GENERATED_COMMENT_OPEN) {
        let open = search + found;
        if open == 0 || tape.as_bytes().get(open - 1) == Some(&b'\n') {
            let Some(close) = tape[open..].find(framing::GENERATED_COMMENT_CLOSE) else {
                return Err(XmlMinifyBindingError::UnterminatedFrameComment { offset: open });
            };
            let end = open + close + framing::GENERATED_COMMENT_CLOSE.len();
            return Ok(Some(open..end));
        }
        search = open + framing::GENERATED_COMMENT_OPEN.len();
    }
    Ok(None)
}

/// Push one inter-frame gap, minifying only its non-whitespace core.
///
/// The leading and trailing whitespace runs are the inter-segment newlines
/// the emitters write between framing and a document; they are copied
/// verbatim BY THIS FUNCTION rather than left to the kernel's treatment of
/// top-level whitespace, so "frame bytes are preserved" is a property of the
/// segmenter and not an incidental property of the kernel.
fn push_gap(rendered: &mut String, gap: &str, base: usize) -> Result<bool, XmlMinifyBindingError> {
    let lead = gap.len() - gap.trim_start_matches(is_xml_whitespace).len();
    if lead == gap.len() {
        // Whitespace only (or empty): no document segment lives here.
        rendered.push_str(gap);
        return Ok(false);
    }
    let trail = gap.len() - gap.trim_end_matches(is_xml_whitespace).len();
    let core = &gap[lead..gap.len() - trail];
    let minified = minify_emitted_xml(core).map_err(|source| XmlMinifyBindingError::Segment {
        offset: base + lead,
        source,
    })?;
    rendered.push_str(&gap[..lead]);
    rendered.push_str(minified.as_ref());
    rendered.push_str(&gap[gap.len() - trail..]);
    Ok(minified.as_ref() != core)
}
