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
//!
//! One genre reopens the MD-subset law: the DOCUMENTATION vocabulary
//! (PROP-045 §7 ##DOC-VOCAB-REOPENING) adds seven blocks Markdown cannot
//! express — a verifiable example with its golden output, a live citation
//! of a rule, a generated reference, a call-out, a figure, an agent prompt
//! — plus the slot condition `when`. They are a SECOND vocabulary, not a
//! widening: a reader opens them only when the caller passes
//! [`Vocabulary::Doc`] (##DOC-VOCAB-BY-KIND), the spec vocabulary stays
//! exactly as measured, and their Markdown projection is one way by law
//! (##DOC-VOCAB-MD-ONE-WAY).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-045#shape");

use std::fmt;

use progress_core::model::{Action, ArtifactRequirements, Audience, Stage, State, nearest};

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
    /// Blocks that belong to no section, each in its slot.
    pub preamble: Vec<BlockNode>,
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
    /// The section's activation condition (`when`), documentation
    /// vocabulary only (##DOC-VOCAB-WHEN-SLOT). `None` under the spec
    /// vocabulary, always.
    pub when: Option<Cond>,
    pub blocks: Vec<BlockNode>,
    pub sections: Vec<Section>,
}

/// One block IN ITS SLOT: the block plus the condition that governs the
/// slot it occupies (##DOC-VOCAB-WHEN-SLOT — `when` is a property of the
/// position in the flow, not of a block kind, so it applies uniformly to a
/// paragraph and to an `example` and creates no per-variant field that
/// half the variants would have to carry).
///
/// `when` is `None` for every block read under [`Vocabulary::Spec`]: the
/// spec dialect has no conditional vocabulary, and a `when` attribute
/// there is still the loud closed-vocabulary error it has always been.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockNode {
    /// The slot's condition, or `None` for an unconditional slot.
    pub when: Option<Cond>,
    pub block: Block,
}

impl BlockNode {
    /// An unconditional slot — the only shape the spec vocabulary can make.
    pub fn plain(block: Block) -> BlockNode {
        BlockNode { when: None, block }
    }
}

impl From<Block> for BlockNode {
    fn from(block: Block) -> BlockNode {
        BlockNode::plain(block)
    }
}

/// One block inside a section (or the document preamble).
///
/// The first five variants are the SPEC vocabulary — the Markdown-subset
/// kinds, unchanged and still isomorphic to what Markdown can express. The
/// seven that follow are the DOCUMENTATION genre (PROP-045 §7), legal only
/// for a reader holding [`Vocabulary::Doc`]; they are what Markdown cannot
/// express, which is exactly why the dialect was reopened for them.
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
    /// `<example>` — a command and the output it must produce
    /// (##ROW-DOCVOCAB-EXAMPLE). Documentation vocabulary.
    Example {
        /// The example's own id, unique in its page: the handle a
        /// translation cites through [`Block::ExampleRef`].
        id: String,
        /// The hermetic fixture project the command runs in; the fixture
        /// declares the normalisation rules and the `--json` schema map.
        fixture: String,
        /// The info string the command fence projects with (`None` ⇒ `sh`).
        lang: Option<String>,
        /// The expected process exit code. `None` ⇒ the attribute was
        /// absent and the expected code is the default `0`; `Some(0)` ⇒ it
        /// was spelled out (so the author's bytes survive the round trip).
        exit: Option<i32>,
        /// `<run>` — the command line, verbatim.
        run: String,
        /// `<expect>` — the golden stdout, verbatim. Empty means «no
        /// output», which is an assertion, not an absence.
        expect: String,
        /// `<stderr>` — the golden stderr, verbatim. `None` asserts
        /// «stderr is empty» (##ROW-DOCVOCAB-EXAMPLE-PURPOSE).
        stderr: Option<String>,
    },
    /// `<example ref="…"/>` — a translation's reference to the source
    /// page's example instead of an example of its own
    /// (##ROW-DOCVOCAB-EXAMPLE-REF). It carries no body: the fences are
    /// copied from the source when the pipeline projects it.
    ExampleRef { id: String },
    /// `<rule ref="spec://…#ANCHOR"/>` — the insertion point of a rule
    /// from a specification (##ROW-DOCVOCAB-RULE). The cited TEXT is not
    /// pivot state: it is substituted at build from the current spec, so
    /// the page cannot drift from the rule it quotes.
    Rule {
        /// The citation address, revision pin dropped — citations are live
        /// and unpinned (##DOC-VOCAB-RULE-ADDRESS).
        uri: String,
        /// A `~rN` pin that strayed into the attribute: recorded so the
        /// author's bytes survive the round trip, and never honoured —
        /// the resolver reads `uri`.
        rev: Option<u32>,
    },
    /// `<derived kind="…" ref="…"/>` — a machine-derived reference
    /// generated at build (##ROW-DOCVOCAB-DERIVED). The generated text is
    /// not stored: storing it is how documentation starts lying.
    Derived {
        kind: DerivedKind,
        reference: String,
    },
    /// `<note kind="…">` — a call-out whose body is one unit, and so is
    /// addressable by a fact anchor like any other unit
    /// (##ROW-DOCVOCAB-NOTE).
    Note { kind: NoteKind, body: Unit },
    /// `<figure src alt>` with a `<caption>` — an image from the package
    /// tree with its caption unit (##ROW-DOCVOCAB-FIGURE).
    Figure {
        src: String,
        alt: String,
        caption: Unit,
    },
    /// `<prompt id>` — a task for an agent in the user's voice, with what
    /// the agent needs, what the person sees when it worked, and the shell
    /// commands that must exit zero afterwards (##ROW-DOCVOCAB-PROMPT).
    Prompt {
        id: String,
        /// The prompt body, verbatim but for the whitespace that separates
        /// it from the children (the writer owns that layout).
        text: String,
        /// `<needs>` — what the agent must have. Prose, one unit of text.
        needs: Option<String>,
        /// `<outcome>` — what the person sees when it worked.
        outcome: Option<String>,
        /// `<assert>` — shell commands that must exit zero after the
        /// agent's work. Empty ONLY for an illustrative prompt, which
        /// spells its emptiness `assert="none"`
        /// (PROP-057 ##STYLE-PROMPT-FIRST).
        asserts: Vec<String>,
    },
}

/// Which vocabulary a reader has open (##DOC-VOCAB-BY-KIND).
///
/// The vocabulary is a parameter of the READER, never a declaration of the
/// document: a page says nothing about which element set may read it, and
/// the mapping «package kind → vocabulary» belongs to the caller, because
/// the pivot knows no `PackageKind` (the separability law — this crate
/// depends on no vibevm subsystem).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Vocabulary {
    /// The spec dialect: the closed MD-subset vocabulary, and nothing
    /// more. The default, so every existing caller keeps its contract.
    #[default]
    Spec,
    /// The spec dialect plus the documentation genre — open only inside
    /// packages of kind `doc`.
    Doc,
}

/// The kind of a call-out (##ROW-DOCVOCAB-NOTE).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteKind {
    Note,
    Tip,
    Warning,
}

impl NoteKind {
    pub const ALL: &'static [NoteKind] = &[NoteKind::Note, NoteKind::Tip, NoteKind::Warning];

    pub fn as_str(self) -> &'static str {
        match self {
            NoteKind::Note => "note",
            NoteKind::Tip => "tip",
            NoteKind::Warning => "warning",
        }
    }

    /// The label the Markdown projection prints, capitalised for a reader.
    pub fn label(self) -> &'static str {
        match self {
            NoteKind::Note => "Note",
            NoteKind::Tip => "Tip",
            NoteKind::Warning => "Warning",
        }
    }

    pub fn parse(s: &str) -> Option<NoteKind> {
        NoteKind::ALL.iter().copied().find(|k| k.as_str() == s)
    }
}

impl fmt::Display for NoteKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Which generator produces a `<derived>` block's text
/// (##ROW-DOCVOCAB-DERIVED). The generators themselves live in the
/// documentation pipeline, not in the pivot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DerivedKind {
    /// `vibe <command> --help`, run against the built binary.
    CliHelp,
    /// A field table generated from a JTD schema.
    JtdSchema,
    /// One field of the current package's manifest (`abstract`, `title`).
    ManifestField,
}

impl DerivedKind {
    pub const ALL: &'static [DerivedKind] = &[
        DerivedKind::CliHelp,
        DerivedKind::JtdSchema,
        DerivedKind::ManifestField,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            DerivedKind::CliHelp => "cli-help",
            DerivedKind::JtdSchema => "jtd-schema",
            DerivedKind::ManifestField => "manifest-field",
        }
    }

    pub fn parse(s: &str) -> Option<DerivedKind> {
        DerivedKind::ALL.iter().copied().find(|k| k.as_str() == s)
    }
}

impl fmt::Display for DerivedKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A `when` condition — the boot lane's condition vocabulary, closed
/// (##DOC-VOCAB-WHEN-SLOT).
///
/// The two forms a page varies by are the platform and the agent. Both
/// spellings are a sanctioned duplication of a vocabulary owned elsewhere
/// (`os:` mirrors `vibe_core::manifest::WhenCondition`, `agent:` mirrors
/// `vibe_agent_projection::agents::Agent`): the pivot carries no edge to
/// either crate by the separability law, so the list is spelled here and
/// its single home is named. `installed:` is deliberately absent — a page
/// varies by the reader's platform and agent, never by what some project
/// happens to have installed.
///
/// ```
/// use vibe_specdoc::doc::Cond;
///
/// assert_eq!(Cond::parse("os:windows").unwrap().to_string(), "os:windows");
/// assert_eq!(Cond::parse("agent:codex").unwrap().to_string(), "agent:codex");
/// assert!(Cond::parse("os:beos").is_none());
/// assert!(Cond::parse("installed:org.vibevm.world/wal").is_none());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cond {
    /// `os:<name>` — the session's operating system.
    Os(CondOs),
    /// `agent:<name>` — the coding agent reading the page.
    Agent(CondAgent),
}

/// The operating systems a `when` condition names — the three
/// `vibe_core::manifest::TargetOs` values, spelled here (see [`Cond`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CondOs {
    Windows,
    Macos,
    Linux,
}

/// The agents a `when` condition names — the five
/// `vibe_agent_projection::agents::Agent` values, spelled here
/// (see [`Cond`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CondAgent {
    Claude,
    ClaudeDesktop,
    Cursor,
    OpenCode,
    Codex,
}

impl CondOs {
    pub const ALL: &'static [CondOs] = &[CondOs::Windows, CondOs::Macos, CondOs::Linux];

    pub fn as_str(self) -> &'static str {
        match self {
            CondOs::Windows => "windows",
            CondOs::Macos => "macos",
            CondOs::Linux => "linux",
        }
    }
}

impl CondAgent {
    pub const ALL: &'static [CondAgent] = &[
        CondAgent::Claude,
        CondAgent::ClaudeDesktop,
        CondAgent::Cursor,
        CondAgent::OpenCode,
        CondAgent::Codex,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            CondAgent::Claude => "claude",
            CondAgent::ClaudeDesktop => "claude-desktop",
            CondAgent::Cursor => "cursor",
            CondAgent::OpenCode => "opencode",
            CondAgent::Codex => "codex",
        }
    }
}

impl Cond {
    /// Every legal condition, in the order the hint offers them.
    pub fn all() -> Vec<Cond> {
        CondOs::ALL
            .iter()
            .copied()
            .map(Cond::Os)
            .chain(CondAgent::ALL.iter().copied().map(Cond::Agent))
            .collect()
    }

    /// Parse one condition, or `None` when it is outside the closed list.
    pub fn parse(s: &str) -> Option<Cond> {
        if let Some(os) = s.strip_prefix("os:") {
            return CondOs::ALL
                .iter()
                .copied()
                .find(|c| c.as_str() == os)
                .map(Cond::Os);
        }
        let agent = s.strip_prefix("agent:")?;
        CondAgent::ALL
            .iter()
            .copied()
            .find(|c| c.as_str() == agent)
            .map(Cond::Agent)
    }

    /// The nearest legal spelling to `s`, for the error's «did you mean».
    pub fn hint(s: &str) -> Option<String> {
        let spellings: Vec<String> = Cond::all().iter().map(Cond::to_string).collect();
        nearest(s, spellings.iter().map(String::as_str)).map(str::to_string)
    }
}

impl fmt::Display for Cond {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cond::Os(os) => write!(f, "os:{}", os.as_str()),
            Cond::Agent(a) => write!(f, "agent:{}", a.as_str()),
        }
    }
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
    /// Optional authored terminal artifact contract, in canonical order.
    pub requirements: Option<ArtifactRequirements>,
    pub status: Option<StatusEl>,
}

impl Fact {
    /// A fact worth serialising: an id, a status, or both. A unit carrying
    /// neither is plain text and takes no `<fact>` element.
    pub fn is_meaningful(&self) -> bool {
        self.id.is_some() || self.requirements.is_some() || self.status.is_some()
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
