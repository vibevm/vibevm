//! The built-in face the link card sets its title in (PROP-057
//! `##CARD-PREVIEW-COMPOSED`).
//!
//! A card composed at build time has to set text, and a text setter
//! needs letterforms. Loading a font file would make the picture depend
//! on a file the package does not carry and the machine may not have;
//! shelling out to a renderer would make it depend on a program. So the
//! face is here: five columns by seven rows of capitals, digits and the
//! punctuation a title uses, which is enough to set a name at the size a
//! link card shows it and nothing like enough to set a paragraph. That
//! is the correct amount of typography for this job.
//!
//! **A title is set in capitals**, and lower case is folded into them
//! rather than approximated: half a face is a worse lie than a
//! deliberate one. **A character the face does not have is dropped**,
//! and when dropping would leave less than half the title, the caller
//! is told so — see [`renderable`], which falls back to nothing rather
//! than to a row of blanks.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#CARD-PREVIEW-COMPOSED");

/// Columns in one glyph.
pub const WIDTH: u32 = 5;
/// Rows in one glyph.
pub const HEIGHT: u32 = 7;
/// Pen movement between glyph origins, in glyph units.
pub const ADVANCE: u32 = 6;
/// Baseline-to-baseline distance, in glyph units.
pub const LINE_ADVANCE: u32 = 10;

/// The rows of one character, top first, five bits each (bit 4 leftmost).
/// `None` for a character the face does not carry.
pub fn glyph(ch: char) -> Option<[u8; HEIGHT as usize]> {
    let upper = ch.to_ascii_uppercase();
    let rows = match upper {
        ' ' => [0, 0, 0, 0, 0, 0, 0],
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001,
        ],
        'X' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11111, 0b00010, 0b00100, 0b00010, 0b00001, 0b10001, 0b01110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
        ],
        '6' => [
            0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
        ],
        '.' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100,
        ],
        ',' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100, 0b01000,
        ],
        ':' => [
            0b00000, 0b01100, 0b01100, 0b00000, 0b01100, 0b01100, 0b00000,
        ],
        ';' => [
            0b00000, 0b01100, 0b01100, 0b00000, 0b01100, 0b01100, 0b01000,
        ],
        '-' => [
            0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
        ],
        '_' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b11111,
        ],
        '\'' => [
            0b00100, 0b00100, 0b01000, 0b00000, 0b00000, 0b00000, 0b00000,
        ],
        '"' => [
            0b01010, 0b01010, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000,
        ],
        '!' => [
            0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100,
        ],
        '?' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b00000, 0b00100,
        ],
        '(' => [
            0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0b00100, 0b00010,
        ],
        ')' => [
            0b01000, 0b00100, 0b00010, 0b00010, 0b00010, 0b00100, 0b01000,
        ],
        '/' => [
            0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000,
        ],
        '&' => [
            0b01100, 0b10010, 0b10100, 0b01000, 0b10101, 0b10010, 0b01101,
        ],
        '+' => [
            0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000,
        ],
        '#' => [
            0b01010, 0b01010, 0b11111, 0b01010, 0b11111, 0b01010, 0b01010,
        ],
        '@' => [
            0b01110, 0b10001, 0b10111, 0b10101, 0b10110, 0b10000, 0b01110,
        ],
        _ => return None,
    };
    Some(rows)
}

/// The part of `title` this face can set, with runs of dropped
/// characters collapsed to one space.
///
/// An empty answer is the honest one for a title written in a script the
/// face does not carry: the caller then draws the ground and the glyph
/// and no text, which is a card that says less, rather than a card that
/// says something wrong.
pub fn renderable(title: &str) -> String {
    let mut out = String::new();
    let mut kept = 0usize;
    let mut total = 0usize;
    for ch in title.chars() {
        if ch.is_whitespace() {
            if !out.ends_with(' ') {
                out.push(' ');
            }
            continue;
        }
        total += 1;
        if glyph(ch).is_some() {
            out.push(ch.to_ascii_uppercase());
            kept += 1;
        } else if !out.ends_with(' ') {
            out.push(' ');
        }
    }
    // Half is the line: below it the survivors are debris rather than a
    // name, and a card showing debris is worse than one showing none.
    if total == 0 || kept * 2 < total {
        return String::new();
    }
    out.trim().to_string()
}

/// One title, broken into lines at the largest scale that fits.
pub struct Lines {
    pub lines: Vec<String>,
    /// Pixels per glyph unit.
    pub scale: f64,
}

/// The largest scale, from a fixed ladder, at which `text` fits a
/// `column` wide and `band` tall — and the lines it makes.
///
/// BOTH measurements matter. A title sized only by the column runs off
/// the bottom of the card the moment it needs a third line, and a card
/// whose last line is half a card is worse than one set a step smaller.
///
/// The ladder is fixed rather than continuous so a one-character edit to
/// a title moves the type by a visible step or not at all, instead of by
/// a fraction nobody asked for.
pub fn wrap(text: &str, column: f64, band: f64) -> Lines {
    for scale in SCALES {
        let per_line = (column / (f64::from(ADVANCE) * scale)).floor() as usize;
        if per_line == 0 {
            continue;
        }
        let lines = break_words(text, per_line);
        if lines.len() <= MAX_LINES && block_height(lines.len(), scale) <= band {
            return Lines { lines, scale };
        }
    }
    // The smallest step, truncated to what the band holds: a title
    // longer than that is not a title, and the card shows what fits.
    let scale = SCALES[SCALES.len() - 1];
    let per_line = ((column / (f64::from(ADVANCE) * scale)).floor() as usize).max(1);
    let mut lines = break_words(text, per_line);
    let fits = ((band + f64::from(LINE_ADVANCE - HEIGHT) * scale)
        / (f64::from(LINE_ADVANCE) * scale))
        .floor()
        .max(1.0) as usize;
    lines.truncate(fits.min(MAX_LINES));
    Lines { lines, scale }
}

/// How tall `count` lines stand at `scale`: the advances between them
/// plus the last line's own body, which is what actually has to fit.
fn block_height(count: usize, scale: f64) -> f64 {
    if count == 0 {
        return 0.0;
    }
    (count.saturating_sub(1) as f64 * f64::from(LINE_ADVANCE) + f64::from(HEIGHT)) * scale
}

/// How many lines a card gives a title.
pub const MAX_LINES: usize = 4;

/// The scale ladder, largest first, in pixels per glyph unit.
const SCALES: [f64; 6] = [16.0, 13.0, 11.0, 9.0, 7.0, 5.0];

/// Greedy word wrap. A word longer than the line is set on its own line
/// and allowed to overhang: breaking inside a name is worse than a
/// ragged edge.
fn break_words(text: &str, per_line: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.chars().count() + 1 + word.chars().count() <= per_line {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current));
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}
