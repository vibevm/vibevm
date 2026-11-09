//! A box-drawn table: a header row, rules, and cells wrapped — never cut —
//! to the viewport. Used where a reader compares values across rows (the
//! response headers of `vibe dl --headers`, the failed checks of
//! `vibe dl --test-proxy`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#list");

use super::glyphs::GlyphSet;
use super::roles::{Role, paint};
use super::width::{measure, viewport, wrap};

/// The rows of one table under a header.
#[derive(Debug, Clone, Default)]
pub(crate) struct Table {
    pub(crate) header: Vec<String>,
    pub(crate) rows: Vec<Vec<String>>,
}

struct Frame {
    h: &'static str,
    v: &'static str,
    top: [&'static str; 3],
    mid: [&'static str; 3],
    bottom: [&'static str; 3],
}

const UNICODE: Frame = Frame {
    h: "─",
    v: "│",
    top: ["┌", "┬", "┐"],
    mid: ["├", "┼", "┤"],
    bottom: ["└", "┴", "┘"],
};
const ASCII: Frame = Frame {
    h: "-",
    v: "|",
    top: ["+", "+", "+"],
    mid: ["+", "+", "+"],
    bottom: ["+", "+", "+"],
};

impl Table {
    pub(crate) fn new(header: Vec<String>) -> Self {
        Self {
            header,
            rows: Vec::new(),
        }
    }

    pub(crate) fn row(mut self, cells: Vec<String>) -> Self {
        self.rows.push(cells);
        self
    }

    /// The table bounded to the viewport.
    ///
    /// ```text
    /// ┌──────────────┬────────────┐
    /// │ Header       │ Value      │
    /// ├──────────────┼────────────┤
    /// │ content-type │ text/plain │
    /// └──────────────┴────────────┘
    /// ```
    pub(crate) fn render(&self) -> String {
        self.render_at(viewport())
    }

    pub(crate) fn render_at(&self, width: usize) -> String {
        let frame = match GlyphSet::detect() {
            GlyphSet::Unicode => &UNICODE,
            GlyphSet::Ascii => &ASCII,
        };
        let columns = self.header.len().max(1);
        let natural: Vec<usize> = (0..columns)
            .map(|c| {
                self.rows
                    .iter()
                    .map(|row| row.get(c).map_or(0, |cell| measure(cell)))
                    .chain([self.header.get(c).map_or(0, |h| measure(h))])
                    .max()
                    .unwrap_or(0)
                    .max(1)
            })
            .collect();
        // Borders and one space of padding each side take 3 per column + 1.
        let budget = width.saturating_sub(3 * columns + 1).max(columns * 8);
        let mut widths = natural.clone();
        while widths.iter().sum::<usize>() > budget {
            // Narrow the widest column first; no column goes below 8.
            let Some((index, _)) = widths
                .iter()
                .enumerate()
                .filter(|(_, w)| **w > 8)
                .max_by_key(|(_, w)| **w)
            else {
                break;
            };
            widths[index] -= 1;
        }
        let rule = |ends: [&str; 3]| {
            let mut line = String::from(ends[0]);
            for (i, w) in widths.iter().enumerate() {
                line.push_str(&frame.h.repeat(w + 2));
                line.push_str(if i + 1 == widths.len() {
                    ends[2]
                } else {
                    ends[1]
                });
            }
            paint(Role::Dim, &line)
        };
        let mut out = vec![rule(frame.top)];
        out.extend(self.render_row(&self.header, &widths, frame, true));
        out.push(rule(frame.mid));
        for row in &self.rows {
            out.extend(self.render_row(row, &widths, frame, false));
        }
        out.push(rule(frame.bottom));
        out.join("\n")
    }

    fn render_row(
        &self,
        cells: &[String],
        widths: &[usize],
        frame: &Frame,
        header: bool,
    ) -> Vec<String> {
        let wrapped: Vec<Vec<String>> = widths
            .iter()
            .enumerate()
            .map(|(i, w)| wrap_hard(cells.get(i).map_or("", String::as_str), *w))
            .collect();
        let height = wrapped.iter().map(Vec::len).max().unwrap_or(1);
        (0..height)
            .map(|line| {
                let mut out = paint(Role::Dim, frame.v);
                for (i, w) in widths.iter().enumerate() {
                    let text = wrapped[i].get(line).map_or("", String::as_str);
                    let pad = " ".repeat(w.saturating_sub(measure(text)));
                    let text = if header {
                        paint(Role::Accent, text)
                    } else {
                        text.to_owned()
                    };
                    out.push(' ');
                    out.push_str(&text);
                    out.push_str(&pad);
                    out.push(' ');
                    out.push_str(&paint(Role::Dim, frame.v));
                }
                out
            })
            .collect()
    }
}

/// `text` wrapped at words, and a word wider than `width` split into
/// pieces: a table cell is never cut, every character is kept.
fn wrap_hard(text: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    for line in wrap(text, width) {
        if measure(&line) <= width {
            out.push(line);
            continue;
        }
        let mut piece = String::new();
        for ch in line.chars() {
            if measure(&piece) + 1 > width {
                out.push(std::mem::take(&mut piece));
            }
            piece.push(ch);
        }
        out.push(piece);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_values_wrap_and_keep_every_character() {
        let value = "x".repeat(70);
        let table = Table::new(vec!["Header".into(), "Value".into()])
            .row(vec!["etag".into(), value.clone()]);
        let rendered = table.render_at(40);
        for line in rendered.lines() {
            assert!(measure(line) <= 40, "{line}");
        }
        let kept: String = rendered
            .lines()
            .filter_map(|l| l.split('│').nth(2))
            .map(str::trim)
            .collect();
        assert!(kept.contains(&value));
    }
}
