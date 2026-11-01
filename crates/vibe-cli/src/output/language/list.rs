//! Lists (PROP-067 `#list`, `#width`): adjacent gutter lines whose columns
//! are measured on plain text over the whole list before it is drawn.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#list");

use super::glyphs::Glyph;
use super::meta::{Meta, SEPARATOR, join_meta};
use super::roles::{Role, paint};
use super::width::{cut, measure, viewport};

/// The gap between two columns.
const GAP: usize = 2;

/// One cell of a row, in its role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Cell {
    pub(crate) text: String,
    pub(crate) role: Role,
}

impl Cell {
    pub(crate) fn new(text: impl Into<String>, role: Role) -> Self {
        Self {
            text: text.into(),
            role,
        }
    }

    pub(crate) fn text(text: impl Into<String>) -> Self {
        Self::new(text, Role::Text)
    }
}

/// One row: an optional gutter glyph, its cells, its metadata and its
/// children.
#[derive(Debug, Clone)]
pub(crate) struct Row {
    pub(crate) glyph: Option<(Glyph, Role)>,
    pub(crate) cells: Vec<Cell>,
    pub(crate) meta: Vec<Meta>,
    /// A skipped row: every cell dim.
    pub(crate) dim: bool,
    /// Child lines, each drawn `└ ` under the name column.
    pub(crate) children: Vec<String>,
}

impl Row {
    pub(crate) fn new(cells: Vec<Cell>) -> Self {
        Self {
            glyph: None,
            cells,
            meta: Vec::new(),
            dim: false,
            children: Vec::new(),
        }
    }

    pub(crate) fn glyph(mut self, glyph: Glyph, role: Role) -> Self {
        self.glyph = Some((glyph, role));
        self
    }

    pub(crate) fn meta(mut self, meta: Vec<Meta>) -> Self {
        self.meta = meta;
        self
    }

    pub(crate) fn dim(mut self, dim: bool) -> Self {
        self.dim = dim;
        self
    }

    pub(crate) fn child(mut self, line: impl Into<String>) -> Self {
        self.children.push(line.into());
        self
    }
}

/// A list of rows drawn as one block.
#[derive(Debug, Clone, Default)]
pub(crate) struct List {
    pub(crate) rows: Vec<Row>,
    /// Metadata as its own aligned column (a plan, a listing) rather than
    /// inline after ` · ` (finished blocks, answers).
    pub(crate) meta_column: bool,
    /// The cell index children are drawn under.
    pub(crate) child_column: usize,
}

impl List {
    pub(crate) fn new(rows: Vec<Row>) -> Self {
        Self {
            rows,
            meta_column: false,
            child_column: 0,
        }
    }

    pub(crate) fn meta_column(mut self) -> Self {
        self.meta_column = true;
        self
    }

    pub(crate) fn children_under(mut self, column: usize) -> Self {
        self.child_column = column;
        self
    }

    /// The list as lines, no trailing newline.
    ///
    /// ```text
    ///   1. fetch  (sources)  Download the sources
    ///   2. build  (recipe)   Build vim             · needs command:gcc
    /// ```
    #[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
    pub(crate) fn render(&self) -> String {
        self.render_at(viewport())
    }

    pub(crate) fn render_at(&self, width: usize) -> String {
        let columns = self
            .rows
            .iter()
            .map(|row| row.cells.len())
            .max()
            .unwrap_or(0);
        let mut widths: Vec<usize> = (0..columns)
            .map(|column| {
                self.rows
                    .iter()
                    .filter_map(|row| row.cells.get(column))
                    .map(|cell| measure(&cell.text))
                    .max()
                    .unwrap_or(0)
            })
            .collect();
        let meta_width = self
            .rows
            .iter()
            .map(|row| measure(&join_meta(&row.meta)))
            .max()
            .unwrap_or(0);
        let lead = 2;
        let total = |widths: &[usize], meta: usize| {
            lead + widths
                .iter()
                .filter(|w| **w > 0)
                .map(|w| w + GAP)
                .sum::<usize>()
                + meta
        };
        // `#width`: the metadata leaves the row first — onto a dim line of
        // its own under the title, so nothing the row says is lost — then
        // the last (title) column goes, never the name.
        let mut show_meta = meta_width > 0;
        let mut meta_below = false;
        if show_meta && total(&widths, meta_width) > width {
            show_meta = false;
            meta_below = true;
        }
        while widths.len() > 2 && total(&widths, 0) > width {
            widths.pop();
        }
        let child_offset = lead
            + widths[..self.child_column.min(widths.len())]
                .iter()
                .filter(|w| **w > 0)
                .map(|w| w + GAP)
                .sum::<usize>();
        let mut lines = Vec::new();
        let title_offset = lead
            + widths[..widths.len().saturating_sub(1)]
                .iter()
                .filter(|w| **w > 0)
                .map(|w| w + GAP)
                .sum::<usize>();
        for row in &self.rows {
            lines.push(self.row_line(row, &widths, show_meta, width));
            if meta_below && !row.meta.is_empty() {
                let meta = join_meta(&row.meta);
                let meta = meta
                    .strip_prefix(&paint(Role::Dim, SEPARATOR))
                    .unwrap_or(&meta)
                    .trim_start();
                lines.push(format!("{}{meta}", " ".repeat(title_offset)));
            }
            for child in &row.children {
                lines.push(format!(
                    "{}{} {child}",
                    " ".repeat(child_offset),
                    Glyph::Child.text()
                ));
            }
        }
        lines.join("\n")
    }

    fn row_line(&self, row: &Row, widths: &[usize], show_meta: bool, width: usize) -> String {
        let mut line = match row.glyph {
            Some((glyph, role)) => format!("{} ", paint(role, glyph.text())),
            None => "  ".to_owned(),
        };
        let last = widths.iter().rposition(|w| *w > 0).unwrap_or(0);
        let mut filled = String::new();
        for (column, column_width) in widths.iter().enumerate() {
            if *column_width == 0 {
                continue;
            }
            let text = row
                .cells
                .get(column)
                .map(|cell| cell.text.as_str())
                .unwrap_or("");
            let role = if row.dim {
                Role::Dim
            } else {
                row.cells.get(column).map_or(Role::Text, |cell| cell.role)
            };
            let pad_to = if column == last && !(self.meta_column && show_meta) {
                0
            } else {
                column_width + GAP
            };
            let padding = " ".repeat(pad_to.saturating_sub(measure(text)));
            filled.push_str(&paint(role, text));
            filled.push_str(&padding);
        }
        line.push_str(filled.trim_end_matches(' '));
        if show_meta && !row.meta.is_empty() {
            let meta = join_meta(&row.meta);
            if self.meta_column {
                // The column already stands two off; the leading ` · `
                // would only push it further.
                let stripped = meta
                    .strip_prefix(&paint(Role::Dim, SEPARATOR))
                    .unwrap_or(&meta)
                    .trim_start()
                    .to_owned();
                line = pad_plain(&line, self.meta_start(widths));
                line.push_str(&stripped);
            } else {
                line.push_str(&meta);
            }
        }
        cut(&line, width)
    }

    fn meta_start(&self, widths: &[usize]) -> usize {
        2 + widths
            .iter()
            .filter(|w| **w > 0)
            .map(|w| w + GAP)
            .sum::<usize>()
    }
}

/// `line` padded with spaces to `column` display columns.
fn pad_plain(line: &str, column: usize) -> String {
    format!("{line}{}", " ".repeat(column.saturating_sub(measure(line))))
}
