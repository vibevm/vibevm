//! The terminal display language of PROP-067: the one module that owns
//! every glyph, colour role, rule, block, list, trailer and refusal the CLI
//! writes for a human reader.
//!
//! Nothing outside this module composes a glyph, a colour or a separator.
//! A command builds a [`Block`] or a [`List`] and prints the string it
//! renders; the shape, the palette, the column measurement and the blank
//! line between blocks are decided here once, so `install`, `remap list`,
//! the wizard and a refusal read as one tool.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#coverage");

mod block;
mod glyphs;
mod list;
mod meta;
mod page;
mod roles;
mod rule;
mod width;

pub(crate) use block::{
    Block, LabelValue, Line, Refusal, fold_line, hint, parked_trailer, trailer, warning,
};
pub(crate) use glyphs::{Glyph, GlyphSet, PromptGlyphs, spinner_frames};
pub(crate) use list::{Cell, List, Row};
pub(crate) use meta::{Meta, bytes, elapsed, join_meta, short_path};
pub(crate) use page::{Kind, Stream, before, mark};
pub(crate) use roles::{Role, paint, style};
pub(crate) use rule::rule;
pub(crate) use width::{cut, viewport};

/// What the composition root decided about this invocation's terminal,
/// handed to the language once before anything is drawn.
#[derive(Debug, Clone, Default)]
pub(crate) struct Setup {
    /// Whether attributes may be written at all: false under `NO_COLOR`,
    /// `--plain`, a redirected stream, `--json` and `--quiet`.
    pub(crate) colour: bool,
    /// Whether the two streams land on one screen, so the blank line
    /// between blocks is decided over both together.
    pub(crate) one_screen: bool,
    /// The home directory a path is shortened against (`~`).
    pub(crate) home: Option<std::path::PathBuf>,
}

/// Apply the root's decision. Called once from `main`; a test that never
/// calls it draws with no attribute and the Unicode glyph set.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-067#palette")]
pub(crate) fn configure(setup: Setup) {
    roles::set_colour(setup.colour);
    page::set_one_screen(setup.one_screen);
    meta::set_home(setup.home);
}

#[cfg(test)]
#[path = "language/tests.rs"]
mod tests;
