//! The pivot document IR (PROP-045 §2: one model, every format a frontend
//! or a backend over it).
//!
//! The IR is isomorphic to the Markdown-expressible structure of the markup
//! contract (##XML-DIALECT-IS-THE-MD-SUBSET): sections nested by heading
//! depth, the five block kinds, anchored units carrying facts, and the
//! `<status>` element vocabulary — nothing more, so XML→MD loses nothing
//! semantic *by construction*: the dialect cannot express what MD cannot.
//!
//! Inline content is ONE text string per unit (##INLINE-STAYS-MARKDOWN):
//! emphasis, inline code, links, `##NAME` citations and `spec://` addresses
//! ride inside `Unit::text` as literal Markdown conventions in both
//! directions. The pivot does not model inline grammar, which is what keeps
//! round-trips byte-stable at the text level.
//!
//! The status vocabulary is NOT a second schema: `StatusEl` carries exactly
//! the attributes progress-core's `element.rs` decodes
//! (`stage`/`state`/`action`/`actionstage`/`audience`/`comment`/`ref`), over
//! that crate's own enums. The parse-artifact fields of a full
//! `progress_core::model::Marker` (form, granularity, source line) are not
//! IR semantics — an XML-authored status has no source line — so the IR
//! stores the semantic payload alone and both frontends build it from their
//! own parse.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-045#shape");

use progress_core::model::{Action, Audience, Stage, State};

/// One whole document: the H1 title, the document `<status>`, the
/// document-level blocks (everything not under a section — the preamble
/// before the first heading plus any blocks between the H1 and the first
/// subsection), and the top-level sections.
///
/// A document with no H1 (`title: None`) is legal — the redbook's
/// `LICENSE.md` is exactly that: pure preamble, no sections.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpecDoc {
    /// The first H1: its text and its `{#anchor}` when it carries one
    /// (##ADDRESSING-UNCHANGED — a document's address survives the change
    /// of serialisation, so the title's anchor is IR state, not spelling).
    pub title: Option<Title>,
    /// The document-level `<status>` element.
    pub status: Option<StatusEl>,
    /// Blocks that belong to no section.
    pub preamble: Vec<Block>,
    /// Top-level sections (H2-and-deeper in MD; `level` is implicit in the
    /// nesting — an Hn heading opens a section at depth n−1).
    pub sections: Vec<Section>,
}

/// The document title: the H1 text plus its optional anchor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Title {
    pub text: String,
    pub id: Option<String>,
}

/// The `<status>` element's attribute set — the progress-core vocabulary
/// verbatim, with `stage` and `state` required exactly as
/// `progress-core`'s marker builder requires them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusEl {
    pub stage: Stage,
    pub state: State,
    pub action: Option<Action>,
    pub actionstage: Option<Stage>,
    /// Empty ⇒ the `dev` default (PROP-043 §3.6).
    pub audience: Vec<Audience>,
    pub comment: Option<String>,
    pub r#ref: Option<String>,
}

/// One nested section: a heading plus everything it governs. The heading
/// level is implicit — a child section is one heading level deeper than its
/// parent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Section {
    /// The heading's `{#anchor}` / `<section id="…">`. `None` is legal: the
    /// book chapters' headings carry no explicit anchors.
    pub id: Option<String>,
    /// The heading text without the `{#…}` suffix.
    pub title: String,
    /// A standalone status marker placed directly under the heading.
    pub status: Option<StatusEl>,
    pub blocks: Vec<Block>,
    pub sections: Vec<Section>,
}

/// One block inside a section (or the document preamble).
///
/// Lists are FLAT runs of items, mirroring the markup contract's own fact
/// model (a nested bullet is one more countable unit, not a sub-list);
/// table alignment and the delimiter row are MD spelling, not semantics;
/// thematic breaks and comment-only blocks are layout (the markup contract
/// exempts them from counting) and do not survive into the IR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// A paragraph: one unit (a plain paragraph or a block's lead lines).
    Paragraph(Unit),
    /// A run of list items. `ordered` records whether the MD form spelled
    /// the markers `N.`/`N)` (`true`) or `-`/`*`/`+` (`false`).
    List { ordered: bool, items: Vec<Unit> },
    /// A pipe table. `rows[0]` is the header row when the source had one;
    /// the delimiter row is structure and never appears here. Rows keep
    /// their own width — a ragged row is legal in both serialisations and
    /// is never padded (padding would change the cell count on re-parse).
    Table { rows: Vec<Vec<Unit>> },
    /// A fenced code block. `lang` is the info string (`None` for a bare
    /// fence); `fact` is the `@fact/code:<ID>` binding — the id of the fact
    /// in the immediately preceding block whose body this fence extends;
    /// `text` is the content between the fence lines, verbatim.
    Fence {
        lang: Option<String>,
        fact: Option<String>,
        text: String,
    },
    /// A blockquote: one unit whose text has the `>` prefixes stripped.
    Quote(Unit),
}

/// One countable unit — the carrier of a fact. A paragraph, a list item, a
/// table cell, a quote: each is one `text` plus, when the unit is anchored
/// or marked, its fact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    /// `Some` only when the unit carries an id, a status, or both; a unit
    /// with neither is plain text.
    pub fact: Option<Fact>,
    /// The unit's own text: inline Markdown conventions ride literally,
    /// the fact-anchor prefix and status spellings are stripped (they live
    /// in [`Fact`]), and leading/trailing whitespace of the whole unit is
    /// trimmed (it is spacing around the markup, not content). A list
    /// item's GFM task box (`[ ] `/`[x] `) is kept at the head of the text
    /// — the checkbox is the item's own content in the dialect.
    pub text: String,
}

/// A fact: the unit's address and its status. The fact's BODY is the unit's
/// `text` — plus, for a typed fact, the bound fence's text
/// (`@fact/code:<ID>`; the binding lives on [`Block::Fence`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fact {
    /// The `@fact:<ID>` / `##<ID>` anchor. `None` is legal only for a
    /// marked table cell (the cell exemption of the anchored-when-marked
    /// law); the XML frontend enforces exactly that.
    pub id: Option<String>,
    pub status: Option<StatusEl>,
}

impl Fact {
    /// A fact worth serialising: an id, a status, or both. A unit carrying
    /// neither is plain text and takes no `<fact>` element.
    pub fn is_meaningful(&self) -> bool {
        self.id.is_some() || self.status.is_some()
    }
}

impl From<&progress_core::model::Marker> for StatusEl {
    fn from(m: &progress_core::model::Marker) -> Self {
        StatusEl {
            stage: m.stage,
            state: m.state,
            action: m.action,
            actionstage: m.actionstage,
            audience: m.audience.clone(),
            comment: m.comment.clone(),
            r#ref: m.r#ref.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The vocabulary is progress-core's own: every enum value survives the
    /// `Marker → StatusEl` move, and nothing else does.
    #[test]
    fn status_el_carries_the_marker_payload_verbatim() {
        use progress_core::model::{
            Action, Audience, Granularity, Marker, MarkerForm, Stage, State,
        };
        let m = Marker {
            stage: Stage::Impl,
            state: State::Work,
            action: Some(Action::Drift),
            actionstage: Some(Stage::Spec),
            audience: vec![Audience::User, Audience::Dev],
            comment: Some("half-landed".into()),
            r#ref: Some("PROP-045#shape".into()),
            form: MarkerForm::Point,
            granularity: Granularity::Document,
            line: 12,
        };
        let s = StatusEl::from(&m);
        assert_eq!(s.stage, Stage::Impl);
        assert_eq!(s.state, State::Work);
        assert_eq!(s.action, Some(Action::Drift));
        assert_eq!(s.actionstage, Some(Stage::Spec));
        assert_eq!(s.audience, vec![Audience::User, Audience::Dev]);
        assert_eq!(s.comment.as_deref(), Some("half-landed"));
        assert_eq!(s.r#ref.as_deref(), Some("PROP-045#shape"));
        // and equality is the semantic equality: parse artifacts never enter
        let m2 = Marker {
            line: 999,
            form: MarkerForm::Wrapper,
            granularity: Granularity::Fragment,
            ..m.clone()
        };
        assert_eq!(StatusEl::from(&m), StatusEl::from(&m2));
    }
}
