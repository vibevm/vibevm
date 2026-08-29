//! # vibe-specdoc — the PROP-045 pivot: one document IR, two frontends,
//! two backends
//!
//! Every spec-source format is a frontend (parse into
//! [`doc::SpecDoc`]) or a backend (emit from it); conversion between
//! formats is always parse → pivot → emit, never direct text rewriting
//! (PROP-045 ##PIVOT-MODEL). This crate owns the IR and all four edges:
//!
//! * [`from_markdown`] — an ADAPTER over `progress-core`'s own scanner
//!   (`parse_document`), not a fifth Markdown parser: it re-uses that
//!   crate's blocks, facts, markers and fence bindings and rebuilds
//!   section/block structure over them.
//! * [`from_xml`] — the closed-dialect XML reader (quick-xml); a foreign
//!   element, attribute, DTD, PI or entity is a loud error with a line
//!   and position, never a silent skip (##XML-DIALECT-IS-THE-MD-SUBSET).
//! * [`to_xml`] — the deterministic dialect writer (2-space indents, `\n`
//!   newlines, fixed attribute order); `from_xml(to_xml(d)) == d` for
//!   every document, so XML→IR→XML is byte-idempotent.
//! * [`to_markdown`] — the MD backend: nested sections → ATX headings
//!   with `{#id}`, facts → `@fact:<ID> … @status:<stage>/<state>` units,
//!   fences/tables/quotes/lists in their MD forms, `@fact/code:` + fence
//!   for typed facts.
//! * [`load_spec_text`] — the consumers' one dispatch (PROP-045
//!   ##PROJECTION-READ): `.md` verbatim, `.xml` as its canonical MD
//!   projection, with the [`SourceKind`] that marks projection-relative
//!   diagnostics; [`project_spec_text`] is its pure sibling for a caller
//!   that already owns the raw text; beside them, the
//!   one-document-one-form pair-collision law ([`pair_collisions_in`],
//!   [`discover_pair_collision`]).
//!
//! The dialect is isomorphic to the Markdown-expressible structure — it
//! cannot express what MD cannot, which is what makes the owner's
//! degradation law hold by construction rather than by a lossy converter.
//! Inline content stays Markdown (##INLINE-STAYS-MARKDOWN): the pivot does
//! not model inline grammar, and unit text round-trips verbatim.
//!
//! ```
//! use vibe_specdoc::{from_markdown, to_xml, from_xml, to_markdown};
//!
//! let md = "# T {#t}\n\n<status stage=\"spec\" state=\"done\"/>\n\n\
//!           @fact:ONLY One claim. @status:impl/done\n";
//! let ir = from_markdown(md).expect("parses");
//! let xml = to_xml(&ir);
//! // XML → IR → XML is byte-idempotent, and the IR survives the trip.
//! assert_eq!(to_xml(&from_xml(&xml).unwrap()), xml);
//! assert_eq!(from_xml(&xml).unwrap(), ir);
//! assert!(to_markdown(&ir).contains("@fact:ONLY"));
//! ```
//!
//! Separability follows progress-core's own law: this crate depends on no
//! vibevm subsystem — only on `progress-core` (the scanner and the status
//! vocabulary) and `quick-xml`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-045#root");

pub mod doc;
pub mod load;

mod convert;
mod md_in;
mod md_out;
mod xml_blocks;
mod xml_comment;
mod xml_facts;
mod xml_in;
mod xml_out;
mod xml_support;

#[cfg(test)]
mod convert_tests;
#[cfg(test)]
mod md_in_tests;
#[cfg(test)]
mod xml_comment_tests;
#[cfg(test)]
mod xml_in_tests;

pub use convert::{Conversion, Direction, convert};
pub use load::{
    LoadError, PROJECTION_NOTICE, PairCollision, SourceKind, discover_pair_collision,
    is_spec_source, load_spec_text, pair_collisions_in, project_spec_text,
};
pub use md_in::from_markdown;
pub use md_out::to_markdown;
pub use xml_comment::{
    XmlCommentCodecError, decode_generated_xml_comment, decode_generated_xml_comment_payload,
    encode_generated_xml_comment,
};
pub use xml_in::from_xml;
pub use xml_out::to_xml;

/// Versioned deterministic-converter identity recorded in every transformed
/// dependency slot. Bump only when identical source may intentionally emit
/// different canonical bytes.
pub const CONVERTER_RECIPE: &str = "specdoc/4";

use std::fmt;

/// A parse or emit failure. Every dialect violation carries the 1-based
/// source line (0 when the failure has no position) and names the offending
/// construct in the message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// 1-based line in the source text; 0 when positionless.
    pub line: usize,
    pub message: String,
}

impl Error {
    /// A positioned error.
    pub fn at(line: usize, message: impl Into<String>) -> Error {
        Error {
            line,
            message: message.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            f.write_str(&self.message)
        } else {
            write!(f, "line {}: {}", self.line, self.message)
        }
    }
}

impl std::error::Error for Error {}

/// The crate's result type.
pub type Result<T> = std::result::Result<T, Error>;
