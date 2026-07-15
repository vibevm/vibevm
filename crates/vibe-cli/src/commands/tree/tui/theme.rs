//! The single source of colour, style, and glyphs for the TUI (PROP-037 §2.2 —
//! "the CSS"). A restyle touches only this file; no component hard-codes a
//! `Color`. The palette is **Rosé Pine** in truecolor — a violet, cosmic dark
//! look (a purple-tinted base with an iris/lavender accent). The main tree keeps
//! the terminal background (so a themed terminal shows through), while modals
//! paint a solid [`BASE`] panel so they read as windows over it.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#theme");

use ratatui_core::style::{Color, Modifier, Style};

// --- Rosé Pine palette (truecolor) ------------------------------------------
/// Window/panel background — a deep purple-tinted black.
pub const BASE: Color = Color::Rgb(25, 23, 36); // #191724
/// A raised surface (status bar, chrome).
pub const SURFACE0: Color = Color::Rgb(31, 29, 46); // #1f1d2e
/// A brighter surface (an off-flag, a subtle fill).
pub const SURFACE1: Color = Color::Rgb(38, 35, 58); // #26233a (overlay)
/// Muted foreground — borders, disabled text.
pub const MUTED: Color = Color::Rgb(110, 106, 134); // #6e6a86
/// Secondary foreground.
pub const SUBTEXT: Color = Color::Rgb(144, 140, 170); // #908caa
/// Primary foreground — a lavender-white.
pub const TEXT: Color = Color::Rgb(224, 222, 244); // #e0def4
/// The accent — selection, titles, the brand. The cosmic violet.
pub const IRIS: Color = Color::Rgb(196, 167, 231); // #c4a7e7
/// A pink accent — flags, warnings.
pub const LOVE: Color = Color::Rgb(235, 111, 146); // #eb6f92
/// A warm accent (a badge, a highlight).
pub const GOLD: Color = Color::Rgb(246, 193, 119); // #f6c177
/// A cool accent — static load, links.
pub const FOAM: Color = Color::Rgb(156, 207, 216); // #9ccfd8
/// A soft rose (a secondary badge).
pub const ROSE: Color = Color::Rgb(235, 188, 186); // #ebbcba

// --- Style helpers (the only styles components use) --------------------------

/// Plain body text.
pub fn text() -> Style {
    Style::new().fg(TEXT)
}

/// A muted secondary line — hints, "why disabled" reasons, chrome.
pub fn dim() -> Style {
    Style::new().fg(MUTED)
}

/// The accent colour (an active key label, a marker).
pub fn accent() -> Style {
    Style::new().fg(IRIS)
}

/// A panel / window title in the border.
pub fn title() -> Style {
    Style::new().fg(IRIS).add_modifier(Modifier::BOLD)
}

/// The selection bar (a highlighted row): base text on the iris accent.
pub fn selection() -> Style {
    Style::new().fg(BASE).bg(IRIS).add_modifier(Modifier::BOLD)
}

/// A subtler selection for the main table (surface fill, iris text) so the tree
/// stays readable over a themed terminal background.
pub fn row_selection() -> Style {
    Style::new()
        .fg(IRIS)
        .bg(SURFACE1)
        .add_modifier(Modifier::BOLD)
}

/// A modal window's solid background (painted under [`ratatui_widgets::clear`]).
pub fn panel() -> Style {
    Style::new().bg(BASE).fg(TEXT)
}

/// A rounded panel border.
pub fn border() -> Style {
    Style::new().fg(MUTED)
}

/// The top status bar.
pub fn status_bar() -> Style {
    Style::new().bg(SURFACE0).fg(SUBTEXT)
}

/// A status-bar value (highlit over the bar).
pub fn status_value() -> Style {
    Style::new()
        .bg(SURFACE0)
        .fg(TEXT)
        .add_modifier(Modifier::BOLD)
}

/// The table header row.
pub fn header() -> Style {
    Style::new().fg(IRIS).add_modifier(Modifier::BOLD)
}

/// The colour of a load-type cell (`static` / `dynamic` / `none`).
pub fn load(load: &str) -> Style {
    match load {
        "static" => Style::new().fg(FOAM),
        "dynamic" => Style::new().fg(IRIS),
        "none" => Style::new().fg(MUTED),
        _ => Style::new().fg(SUBTEXT),
    }
}

/// A set flag (`T`/`C`/`S` = `x`).
pub fn flag_on() -> Style {
    Style::new().fg(GOLD)
}

/// An unset flag (`.`).
pub fn flag_off() -> Style {
    Style::new().fg(SURFACE1)
}

/// A footer key label (the `F1`, `Enter`, … glyph).
pub fn key() -> Style {
    Style::new().fg(IRIS)
}

/// A footer key's description.
pub fn key_desc() -> Style {
    Style::new().fg(SUBTEXT)
}
