//! The block of PROP-067 `#block`, and the shapes built from it: the
//! refusal of `#refusal`, the warning line and the trailer of `#trailer`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#block");

use super::glyphs::Glyph;
use super::meta::{Meta, join_meta};
use super::roles::{Role, paint};
use super::width::{cut, measure, viewport, wrap};

/// Where body lines start: column 2, under the headline.
const BODY: &str = "  ";

/// A gutter line and the lines under it.
#[derive(Debug, Clone)]
pub(crate) struct Block {
    pub(crate) glyph: Glyph,
    /// The glyph's role: `ok` for `✓`, `alert` for `✗`, `dim` for `○`.
    pub(crate) glyph_role: Role,
    pub(crate) headline: String,
    pub(crate) headline_role: Role,
    pub(crate) meta: Vec<Meta>,
    pub(crate) body: Vec<Line>,
}

/// One body line of a block, at column 2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Line {
    /// A kept line of a failed block: the fold glyph, the text role
    /// (`#collapse`).
    Evidence(String),
    /// A label column — `why`, `fix`, `see` — then its values; the second
    /// and later values stand on their own lines under the first.
    Label(String, Vec<LabelValue>),
    /// A child block: the branch glyph and its headline (`#children`).
    Child(String),
}

/// A value of a label line: prose in the text role, or a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LabelValue {
    Text(String),
    Command(String),
}

impl Block {
    /// A block with no metadata and no body.
    pub(crate) fn new(glyph: Glyph, headline: impl Into<String>) -> Self {
        let (glyph_role, headline_role) = match glyph {
            Glyph::Done => (Role::Ok, Role::Text),
            Glyph::Failed => (Role::Alert, Role::Text),
            Glyph::Skipped => (Role::Dim, Role::Dim),
            Glyph::Trailer => (Role::Ok, Role::Ok),
            Glyph::Warning => (Role::Warn, Role::Warn),
            Glyph::Question | Glyph::Running | Glyph::Fold | Glyph::Child => {
                (Role::Text, Role::Text)
            }
        };
        Self {
            glyph,
            glyph_role,
            headline: headline.into(),
            headline_role,
            meta: Vec::new(),
            body: Vec::new(),
        }
    }

    /// The same block with its metadata.
    pub(crate) fn with_meta(mut self, meta: Vec<Meta>) -> Self {
        self.meta = meta;
        self
    }

    /// The same block with its body.
    pub(crate) fn with_body(mut self, body: Vec<Line>) -> Self {
        self.body = body;
        self
    }

    /// The same block with its headline in another role.
    pub(crate) fn headline_in(mut self, role: Role) -> Self {
        self.headline_role = role;
        self
    }

    /// The block as lines, no trailing newline, bounded to the viewport.
    ///
    /// ```text
    /// ✓ build   Build vim · 2m 10s · 412 lines
    /// ```
    #[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-067#block")]
    pub(crate) fn render(&self) -> String {
        self.render_at(viewport())
    }

    pub(crate) fn render_at(&self, width: usize) -> String {
        let mut lines = self.gutter_lines(width);
        let labels = self
            .body
            .iter()
            .filter_map(|line| match line {
                Line::Label(label, _) => Some(measure(label)),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        for line in &self.body {
            body_line(&mut lines, line, labels, width);
        }
        lines.join("\n")
    }

    /// The gutter line: the glyph in column 0, the headline from column 2
    /// wrapped at words, the metadata after it dropped item by item from
    /// the end while it does not fit — an identifier is never cut.
    fn gutter_lines(&self, width: usize) -> Vec<String> {
        let glyph = paint(self.glyph_role, self.glyph.text());
        let mut words = wrap(&self.headline, width.saturating_sub(2).max(10));
        let mut meta = self.meta.clone();
        if let Some(last) = words.last() {
            while !meta.is_empty() && 2 + measure(last) + measure(&join_meta(&meta)) > width {
                meta.pop();
            }
        }
        let tail = join_meta(&meta);
        let count = words.len();
        words
            .iter_mut()
            .enumerate()
            .map(|(index, line)| {
                let lead = if index == 0 {
                    format!("{glyph} ")
                } else {
                    BODY.to_owned()
                };
                let end = if index + 1 == count {
                    tail.as_str()
                } else {
                    ""
                };
                format!("{lead}{}{end}", paint(self.headline_role, line))
            })
            .collect()
    }
}

fn body_line(lines: &mut Vec<String>, line: &Line, labels: usize, width: usize) {
    match line {
        Line::Evidence(text) => lines.push(format!(
            "{BODY}{} {}",
            paint(Role::Dim, Glyph::Fold.text()),
            cut(text, width.saturating_sub(4))
        )),
        Line::Child(text) => lines.push(format!("{BODY}{} {text}", Glyph::Child.text())),
        Line::Label(label, values) => {
            let pad = " ".repeat(labels.saturating_sub(measure(label)) + 3);
            let indent = " ".repeat(labels + 3);
            for (index, value) in values.iter().enumerate() {
                let value = match value {
                    LabelValue::Text(text) => text.clone(),
                    LabelValue::Command(text) => paint(Role::Accent, text),
                };
                if index == 0 {
                    lines.push(format!("{BODY}{}{pad}{value}", paint(Role::Dim, label)));
                } else {
                    lines.push(format!("{BODY}{indent}{value}"));
                }
            }
        }
    }
}

/// A line of a running block's fold (PROP-067 `#fold`): the fold glyph
/// and the child's line, dim, cut at `width`, never coloured by what the
/// child wrote.
pub(crate) fn fold_line(text: &str, width: usize) -> String {
    paint(
        Role::Dim,
        &cut(&format!("{} {text}", Glyph::Fold.text()), width),
    )
}

/// A dim hint naming an expansion: `(l to show the live log)`.
pub(crate) fn hint(text: &str) -> String {
    paint(Role::Dim, text)
}

/// A refusal (PROP-067 `#refusal`): the code first in an `alert`
/// headline, then `why`, `fix` and `see` as label columns.
#[derive(Debug, Clone, Default)]
pub(crate) struct Refusal {
    pub(crate) code: Option<String>,
    pub(crate) headline: String,
    pub(crate) why: Option<String>,
    pub(crate) fix: Vec<String>,
    pub(crate) see: Option<String>,
}

impl Refusal {
    /// The refusal as a block.
    ///
    /// ```text
    /// ✗ consent_required  1 remap package offer needs an answer
    ///   why   org.corp/intranet is offered and neither accepted nor rejected
    ///   fix   vibe remap accept org.corp/intranet
    ///   see   spec://org.vibevm.core/vibevm/common/PROP-066#consent
    /// ```
    #[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-067#refusal")]
    pub(crate) fn block(&self) -> Block {
        let headline = match &self.code {
            Some(code) => format!("{code}  {}", self.headline),
            None => self.headline.clone(),
        };
        let mut body = Vec::new();
        if let Some(why) = &self.why {
            body.push(Line::Label(
                "why".into(),
                vec![LabelValue::Text(why.clone())],
            ));
        }
        if !self.fix.is_empty() {
            let fixes = self.fix.iter().cloned().map(LabelValue::Command).collect();
            body.push(Line::Label("fix".into(), fixes));
        }
        if let Some(see) = &self.see {
            body.push(Line::Label(
                "see".into(),
                vec![LabelValue::Text(see.clone())],
            ));
        }
        Block::new(Glyph::Failed, headline)
            .headline_in(Role::Alert)
            .with_body(body)
    }
}

/// A warning that does not stop the command: `! <sentence>`, one line.
pub(crate) fn warning(text: &str) -> String {
    format!(
        "{} {}",
        paint(Role::Warn, Glyph::Warning.text()),
        paint(Role::Warn, text)
    )
}

/// The one line a reader keeps (PROP-067 `#trailer`): `✻ <what> · <counts>
/// · <time> · log <full path>`, `ok` on success and `alert` on failure.
///
/// ```text
/// ✻ installed org.vibevm.apps/vim@9.1.0 · 3 steps · 2m 14s · log /home/u/.vibe/cache/app-runs/vim/1
/// ```
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-067#trailer")]
pub(crate) fn trailer(ok: bool, what: &str, mut meta: Vec<Meta>, log: Option<&str>) -> String {
    if let Some(log) = log {
        // The trailer is where the path is written in full.
        meta.push(Meta::Note(format!("log {log}")));
    }
    let role = if ok { Role::Ok } else { Role::Alert };
    format!(
        "{} {}{}",
        paint(role, Glyph::Trailer.text()),
        paint(role, what),
        join_meta(&meta)
    )
}

/// The trailer of a parked command: what it waits for, then the command
/// that resumes it on its own accent line.
pub(crate) fn parked_trailer(what: &str, resume: &str) -> String {
    format!(
        "{} {}\n{BODY}{}",
        paint(Role::Warn, Glyph::Trailer.text()),
        paint(Role::Warn, what),
        paint(Role::Accent, resume)
    )
}
