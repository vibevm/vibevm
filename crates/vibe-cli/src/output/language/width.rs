//! Lines bounded to the viewport (PROP-067 `#width`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#width");

/// The width a rule is drawn at on a redirected stream, where nothing else
/// is bounded: a log reads as an ordinary terminal would have shown it.
const UNATTENDED_RULE: usize = 80;

/// The width of the terminal the blocks land on, when there is one.
fn terminal_width() -> Option<usize> {
    console::Term::stderr()
        .size_checked()
        .map(|(_, columns)| usize::from(columns))
        .filter(|columns| *columns >= 20)
}

/// The width lines are bounded to. A redirected stream has no viewport:
/// its lines are neither wrapped nor cut, so a log keeps every sentence on
/// the line a reader greps for (PROP-067 `#width`, `#plain`).
pub(crate) fn viewport() -> usize {
    terminal_width().unwrap_or(usize::MAX / 4)
}

/// The width a rule runs to: the viewport, or a terminal's usual width on a
/// redirected stream.
pub(crate) fn rule_width() -> usize {
    terminal_width().unwrap_or(UNATTENDED_RULE)
}

/// The display width of `text`, attributes not counted.
pub(crate) fn measure(text: &str) -> usize {
    console::measure_text_width(text)
}

/// `text` cut at `width` columns with `…` at the end; attributes survive.
pub(crate) fn cut(text: &str, width: usize) -> String {
    if measure(text) <= width {
        return text.to_owned();
    }
    console::truncate_str(text, width, "…").into_owned()
}

/// `text` wrapped at words to `width` columns. A word wider than the
/// width stands on its own line uncut: it may be an identifier.
pub(super) fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split(' ') {
        if !line.is_empty() && measure(&line) + 1 + measure(word) > width {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    lines.push(line);
    lines
}
