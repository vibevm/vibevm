//! The closed glyph set of PROP-067 `#gutter`, in its Unicode and its ASCII
//! spelling, and the one decision of which spelling this process draws.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#gutter");

use std::sync::atomic::{AtomicBool, Ordering};

static ASCII: AtomicBool = AtomicBool::new(false);

/// Draw the ASCII set for the rest of the process; `main` calls it once, from
/// TERM, before anything is drawn.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#plain")]
pub(crate) fn draw_ascii(ascii: bool) {
    ASCII.store(ascii, Ordering::Relaxed);
}

/// The rule stroke in the set this process draws with.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#heading")]
pub(crate) fn rule_glyph() -> &'static str {
    match GlyphSet::detect() {
        GlyphSet::Ascii => "-",
        GlyphSet::Unicode => "─",
    }
}

/// The heading separator in the set this process draws with.
#[allow(dead_code)] // F4a consumes this helper; existing callers stay unchanged here.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#heading")]
pub(crate) fn heading_separator() -> &'static str {
    match GlyphSet::detect() {
        GlyphSet::Ascii => "*",
        GlyphSet::Unicode => "·",
    }
}

/// Every glyph the CLI may put in column 0, plus the two that draw inside
/// a block (the fold and the child branch). The set is closed: a shape
/// that needs another glyph amends PROP-067 first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Glyph {
    /// A block that is running, or a message being written now.
    Running,
    /// A block that finished well.
    Done,
    /// A block that failed.
    Failed,
    /// A block that was skipped or is waiting its turn.
    Skipped,
    /// The trailer of a command.
    Trailer,
    /// A question.
    Question,
    /// A warning that is not a block of its own.
    Warning,
    /// A line of a running block's fold.
    Fold,
    /// The child of a block.
    Child,
}

/// Which spelling of the set this process draws with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GlyphSet {
    Unicode,
    Ascii,
}

impl GlyphSet {
    /// The set this process draws with, selected by `draw_ascii`.
    /// Redirected output is not a reason to degrade: a log keeps its glyphs
    /// and loses only its colour (PROP-067 `#plain`).
    pub(crate) fn detect() -> Self {
        if ASCII.load(Ordering::Relaxed) {
            GlyphSet::Ascii
        } else {
            GlyphSet::Unicode
        }
    }
}

impl Glyph {
    /// The glyph in the set this process draws with.
    ///
    /// ```text
    /// Glyph::Done  ->  ✓   (ASCII: +)
    /// ```
    pub(crate) fn text(self) -> &'static str {
        self.in_set(GlyphSet::detect())
    }

    /// The glyph in a named set — the pure half, so a test reads both.
    pub(crate) fn in_set(self, set: GlyphSet) -> &'static str {
        let (unicode, ascii) = match self {
            Glyph::Running => ("●", "*"),
            Glyph::Done => ("✓", "+"),
            Glyph::Failed => ("✗", "x"),
            Glyph::Skipped => ("○", "-"),
            Glyph::Trailer => ("✻", "*"),
            Glyph::Question => ("?", "?"),
            Glyph::Warning => ("!", "!"),
            Glyph::Fold => ("│", "|"),
            Glyph::Child => ("└", "`"),
        };
        match set {
            GlyphSet::Unicode => unicode,
            GlyphSet::Ascii => ascii,
        }
    }
}

/// The spinner frames that stand in for a running block's `●` while it
/// spins; the last frame is the `●` itself, which indicatif shows once the
/// spinner stops.
pub(crate) fn spinner_frames() -> &'static [&'static str] {
    match GlyphSet::detect() {
        GlyphSet::Unicode => &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "●"],
        GlyphSet::Ascii => &["|", "/", "-", "\\", "*"],
    }
}

/// The glyphs a question draws inside its own block (PROP-067
/// `#question`): the row cursor, the multi-choice marks and the key
/// legend. They live at column 2 and deeper, never in the gutter.
pub(crate) struct PromptGlyphs {
    /// Before the row under the cursor.
    pub(crate) active: &'static str,
    /// A selected multi-choice row.
    pub(crate) checked: &'static str,
    /// An unselected multi-choice row.
    pub(crate) unchecked: &'static str,
    /// The key legend of a list question.
    pub(crate) keys: &'static str,
}

impl PromptGlyphs {
    /// The set this process draws with.
    pub(crate) fn detect() -> Self {
        Self::in_set(GlyphSet::detect())
    }

    /// The prompt glyphs of a named set.
    pub(crate) fn in_set(set: GlyphSet) -> Self {
        match set {
            GlyphSet::Unicode => Self {
                active: "❯",
                checked: "◉",
                unchecked: "◯",
                keys: "↑↓ move · enter choose · space toggle · esc cancel",
            },
            GlyphSet::Ascii => Self {
                active: ">",
                checked: "[x]",
                unchecked: "[ ]",
                keys: "up/down move, enter choose, space toggle, esc cancel",
            },
        }
    }
}

/// The branches of `vibe tree`'s plain view (PROP-067 `#coverage`): the
/// one listing that keeps a full tree of `├`/`└`, because its rows are all
/// known before the first is written and a reader follows the edges of a
/// dependency graph down them. They live at column 0 of a tree that has no
/// gutter, so they are named here rather than in the gutter set.
pub(crate) struct TreeGlyphs {
    /// Before a child that has a sibling after it.
    pub(crate) branch: &'static str,
    /// Before the last child of its parent.
    pub(crate) last: &'static str,
    /// The prefix under a child that has a sibling after it.
    pub(crate) through: &'static str,
}

impl TreeGlyphs {
    /// The set this process draws with.
    pub(crate) fn detect() -> Self {
        Self::in_set(GlyphSet::detect())
    }

    /// The branches of a named set; every spelling is three columns wide,
    /// so a deeper row stays under its parent in both.
    pub(crate) fn in_set(set: GlyphSet) -> Self {
        match set {
            GlyphSet::Unicode => Self {
                branch: "├─ ",
                last: "└─ ",
                through: "│  ",
            },
            GlyphSet::Ascii => Self {
                branch: "|- ",
                last: "`- ",
                through: "|  ",
            },
        }
    }
}
