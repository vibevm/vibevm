//! The colour roles of PROP-067 `#palette` and the one function that puts
//! an attribute on text.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#palette");

use std::sync::atomic::{AtomicBool, Ordering};

use console::Style;

/// Every colour the CLI writes names one of these roles; there is no other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    /// The terminal's default foreground.
    Text,
    /// Rules, metadata, fold lines, hints, skipped blocks.
    Dim,
    /// Identifiers inline in prose; a command the reader can type.
    Accent,
    /// `✓`, an answered value, a successful trailer.
    Ok,
    /// `✗`, a refusal's headline, a `deny` row.
    Alert,
    /// `!` and the headline it marks.
    Warn,
}

/// Off until the composition root says otherwise, so a unit test and a
/// redirected run both get plain text.
static COLOUR: AtomicBool = AtomicBool::new(false);

pub(super) fn set_colour(on: bool) {
    COLOUR.store(on, Ordering::Relaxed);
    console::set_colors_enabled(on);
    console::set_colors_enabled_stderr(on);
}

/// Whether attributes are written at all in this process.
pub(crate) fn colour_on() -> bool {
    COLOUR.load(Ordering::Relaxed)
}

/// The terminal attribute of a role. Every one is an attribute the
/// terminal maps itself (no RGB), so a light and a dark theme both read.
///
/// The style writes its escapes exactly when this process draws colour,
/// whatever stream `console` would otherwise have asked about.
pub(crate) fn style(role: Role) -> Style {
    let style = Style::new().force_styling(colour_on());
    match role {
        Role::Text => style,
        Role::Dim => style.dim(),
        Role::Accent => style.blue(),
        Role::Ok => style.green(),
        Role::Alert => style.red(),
        Role::Warn => style.yellow(),
    }
}

/// `text` in `role`; the text itself when colour is off.
///
/// ```text
/// paint(Role::Dim, "3s")  ->  ESC[2m3sESC[0m   (colour off: 3s)
/// ```
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-067#palette")]
pub(crate) fn paint(role: Role, text: &str) -> String {
    if !colour_on() || role == Role::Text || text.is_empty() {
        return text.to_owned();
    }
    style(role).apply_to(text).to_string()
}

/// `text` in `role` and bold: a count that changes what the reader does,
/// or the value a question was answered with — nowhere else.
pub(crate) fn bold(role: Role, text: &str) -> String {
    if !colour_on() || text.is_empty() {
        return text.to_owned();
    }
    style(role).bold().apply_to(text).to_string()
}
