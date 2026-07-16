//! The single source of colour, style, and glyphs for the TUI (PROP-037 §2.2 —
//! "the CSS"). A restyle touches only this module; no component hard-codes a
//! [`Color`]. Colour flows through a data-driven [`Palette`] of semantic
//! [`Role`] tokens (PROP-037 §2.2.1), glyphs through a [`Glyphs`] set (§2.2.2),
//! and both are carried by one [`Theme`] value-type that projects onto the
//! detected rendering [`Tier`] (§2.2.3).
//!
//! ## Phase 9a — the theme is threaded, not global (PROP-037 §9)
//!
//! Every component takes the active [`Theme`] *by reference* — there is no
//! process-wide default singleton. [`App`](super::state::App) owns the one
//! `Theme`, built from the resolved settings (palette + tier) on launch
//! ([`super::settings`]), and hands `&Theme` down through every render call.
//! The palette is therefore genuinely switchable: change `vibe.tree.palette`,
//! relaunch, and the whole UI re-skins. [`Theme::default`] (Rosé Pine, Tier 3)
//! remains the fallback for tests and the no-settings path.
//!
//! [`Color`]: ratatui_core::style::Color

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#theme");

use ratatui_core::style::{Color, Modifier, Style};
use specmark::spec;

// The flat `theme::` aliases for the submodule API. The primary types
// (`Theme`, `Role`, `Palette`, `Rgb`, `Tier`, `Glyphs`, `PaletteName`,
// `project_color`) compose the surface every component reaches through a
// `&Theme`; the per-palette structs and `resolve` are the registration points
// a palette identity flows through.
#[allow(unused_imports)]
pub use glyphs::{Glyphs, corners};
pub use palette::{Palette, Rgb, Role};
#[allow(unused_imports)]
pub use palettes::{Frappe, Latte, Macchiato, Mocha, PaletteName, RosePine, resolve};
#[allow(unused_imports)]
pub use tier::{Tier, detect_tier, project_color};

// The submodules are public so the staged Phase-3 API (`detect_tier`,
// `corners`, the Catppuccin palettes, …) is reachable as `theme::tier::…`
// etc.; `#[allow(dead_code)]` covers the items not yet wired through `App`.
#[allow(dead_code)]
pub mod glyphs;
#[allow(dead_code)]
pub mod palette;
#[allow(dead_code)]
pub mod palettes;
#[allow(dead_code)]
pub mod tier;

// ---------------------------------------------------------------------------
// Theme — the single source of truth (PROP-037 §1.4, §2.2)
// ---------------------------------------------------------------------------

/// The one value-type every component takes by reference (PROP-037 §2.2). It
/// bundles the active [`Palette`] (semantic role → colour), the active
/// [`Glyphs`] set (tier-appropriate vocabulary), and the detected [`Tier`]; its
/// [`color`](Theme::color) method is the only route from a [`Role`] to a
/// [`Color`], projecting onto the tier so a component never branches on
/// terminal capability itself.
pub struct Theme {
    palette: Box<dyn Palette>,
    glyphs: Glyphs,
    tier: Tier,
}

impl Theme {
    /// Build a theme for a named palette at a detected tier. Tier ≥ 1 gets the
    /// Unicode [`Glyphs::rich`] vocabulary; Tier 0 falls back to
    /// [`Glyphs::ascii`].
    #[must_use]
    pub fn new(name: PaletteName, tier: Tier) -> Self {
        let glyphs = if tier >= Tier::T1 {
            Glyphs::rich()
        } else {
            Glyphs::ascii()
        };
        Self {
            palette: palettes::resolve(name),
            glyphs,
            tier,
        }
    }

    /// Resolve a [`Role`] to its projected [`Color`] for this theme's tier
    /// (PROP-037 §2.2.3). The only route a component has from a semantic role
    /// to a terminal colour.
    #[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#rendering-tiers")]
    #[must_use]
    pub fn color(&self, role: Role) -> Color {
        project_color(
            self.palette.role(role),
            role,
            self.tier,
            self.palette.is_light(),
        )
    }

    /// The active palette's display name (e.g. `"rose-pine"`,
    /// `"catppuccin-mocha"`).
    #[allow(dead_code)] // introspection: read by settings tests + a future settings UI.
    #[must_use]
    pub fn palette_name(&self) -> &'static str {
        self.palette.name()
    }

    /// The active rendering tier.
    #[allow(dead_code)] // introspection: read by settings tests + a future settings UI.
    #[must_use]
    pub fn tier(&self) -> Tier {
        self.tier
    }

    /// Whether the active palette is a light-background palette.
    #[allow(dead_code)] // introspection: read by a future settings UI / AIUI.
    #[must_use]
    pub fn is_light(&self) -> bool {
        self.palette.is_light()
    }

    /// The active glyph set — the vocabulary components stamp into borders,
    /// tree connectors, and flag cells (PROP-037 §2.2.2).
    #[must_use]
    pub fn glyphs(&self) -> &Glyphs {
        &self.glyphs
    }

    // --- the "CSS": the only styles components use ------------------------

    /// Plain body text.
    #[must_use]
    pub fn text(&self) -> Style {
        Style::new().fg(self.color(Role::Text))
    }

    /// A muted secondary line — hints, "why disabled" reasons, chrome.
    #[must_use]
    pub fn dim(&self) -> Style {
        Style::new().fg(self.color(Role::Muted))
    }

    /// The accent colour (an active key label, a marker).
    #[must_use]
    pub fn accent(&self) -> Style {
        Style::new().fg(self.color(Role::Accent))
    }

    /// A panel / window title in the border.
    #[must_use]
    pub fn title(&self) -> Style {
        Style::new()
            .fg(self.color(Role::Accent))
            .add_modifier(Modifier::BOLD)
    }

    /// The selection bar (a highlighted row): base text on the accent ground.
    #[must_use]
    pub fn selection(&self) -> Style {
        Style::new()
            .fg(self.color(Role::Base))
            .bg(self.color(Role::Accent))
            .add_modifier(Modifier::BOLD)
    }

    /// A subtler selection for the main table (surface fill, accent text) so
    /// the tree stays readable over a themed terminal background.
    #[must_use]
    pub fn row_selection(&self) -> Style {
        Style::new()
            .fg(self.color(Role::Accent))
            .bg(self.color(Role::Surface1))
            .add_modifier(Modifier::BOLD)
    }

    /// A modal window's solid background (painted under `ratatui_widgets::clear`).
    #[must_use]
    pub fn panel(&self) -> Style {
        Style::new()
            .bg(self.color(Role::Base))
            .fg(self.color(Role::Text))
    }

    /// A rounded panel border.
    #[must_use]
    pub fn border(&self) -> Style {
        Style::new().fg(self.color(Role::Border))
    }

    /// The top status bar.
    #[must_use]
    pub fn status_bar(&self) -> Style {
        Style::new()
            .bg(self.color(Role::Surface0))
            .fg(self.color(Role::Subtext))
    }

    /// A status-bar value (highlighted over the bar).
    #[must_use]
    pub fn status_value(&self) -> Style {
        Style::new()
            .bg(self.color(Role::Surface0))
            .fg(self.color(Role::Text))
            .add_modifier(Modifier::BOLD)
    }

    /// The table header row.
    #[must_use]
    pub fn header(&self) -> Style {
        Style::new()
            .fg(self.color(Role::Accent))
            .add_modifier(Modifier::BOLD)
    }

    /// The colour of a load-type cell (`static` / `dynamic` / `none`).
    #[must_use]
    pub fn load(&self, load: &str) -> Style {
        let role = match load {
            "static" => Role::Foam,
            "dynamic" => Role::Accent,
            "none" => Role::Muted,
            _ => Role::Subtext,
        };
        Style::new().fg(self.color(role))
    }

    /// A set flag (`T`/`C`/`S` = `●`).
    #[must_use]
    pub fn flag_on(&self) -> Style {
        Style::new().fg(self.color(Role::Gold))
    }

    /// An unset flag (`○`).
    #[must_use]
    pub fn flag_off(&self) -> Style {
        Style::new().fg(self.color(Role::Surface1))
    }

    /// A footer key label (the `F1`, `Enter`, … glyph).
    #[must_use]
    pub fn key(&self) -> Style {
        Style::new().fg(self.color(Role::Accent))
    }

    /// A footer key's description.
    #[must_use]
    pub fn key_desc(&self) -> Style {
        Style::new().fg(self.color(Role::Subtext))
    }

    /// A validation warning — an inline schema-violation marker (PROP-041 §6
    /// `#validation-feedback`). Composes the warm `Gold` role so a violation
    /// reads as cautionary without being alarming (a deprecation hint or an
    /// out-of-range value); a harder error shade is a later addition.
    #[must_use]
    pub fn warning(&self) -> Style {
        Style::new().fg(self.color(Role::Gold))
    }
}

impl Default for Theme {
    /// The canonical fallback when no settings are loaded: Rosé Pine at
    /// Tier 3 — the byte-identical pre-theme-module look (PROP-037 §2.2.1 R8).
    fn default() -> Self {
        Theme::new(PaletteName::RosePine, Tier::T3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_theme_is_rose_pine_t3() {
        let t = Theme::default();
        assert_eq!(t.palette_name(), "rose-pine");
        assert_eq!(t.tier(), Tier::T3);
        assert!(!t.is_light());
    }

    #[test]
    fn color_projects_truecolor_at_t3() {
        let t = Theme::new(PaletteName::RosePine, Tier::T3);
        assert_eq!(t.color(Role::Accent), Color::Rgb(196, 167, 231));
        assert_eq!(t.color(Role::Base), Color::Rgb(25, 23, 36));
    }

    #[test]
    fn color_degrades_with_tier() {
        let t3 = Theme::new(PaletteName::RosePine, Tier::T3);
        let t1 = Theme::new(PaletteName::RosePine, Tier::T1);
        let t0 = Theme::new(PaletteName::RosePine, Tier::T0);
        assert_eq!(t3.color(Role::Accent), Color::Rgb(196, 167, 231));
        assert_eq!(t1.color(Role::Accent), Color::Magenta);
        assert_eq!(t0.color(Role::Accent), Color::Reset);
    }

    #[test]
    fn tier0_gets_ascii_glyphs_tier1_gets_rich() {
        let t0 = Theme::new(PaletteName::RosePine, Tier::T0);
        let t1 = Theme::new(PaletteName::RosePine, Tier::T1);
        assert_eq!(t0.glyphs().fold_collapsed, "+");
        assert_eq!(t1.glyphs().fold_collapsed, "▸");
    }

    #[test]
    fn load_style_matches_role() {
        let t = Theme::new(PaletteName::RosePine, Tier::T3);
        assert_eq!(t.load("static").fg, t.color(Role::Foam).into());
        assert_eq!(t.load("dynamic").fg, t.color(Role::Accent).into());
        assert_eq!(t.load("none").fg, t.color(Role::Muted).into());
        assert_eq!(t.load("?").fg, t.color(Role::Subtext).into());
    }

    #[test]
    fn latte_theme_is_light_and_projects() {
        let t = Theme::new(PaletteName::Latte, Tier::T3);
        assert!(t.is_light());
        assert_eq!(t.color(Role::Base), Color::Rgb(239, 241, 245));
        // T1 polarity inverts on a light palette.
        let t1 = Theme::new(PaletteName::Latte, Tier::T1);
        assert_eq!(t1.color(Role::Text), Color::Black);
    }

    /// The default theme is byte-identical to the former legacy colour-const
    /// surface — the canonical-locked Rosé Pine base roles (R8 fidelity).
    #[test]
    fn default_theme_roles_match_canonical_rose_pine() {
        let t = Theme::default();
        assert_eq!(t.color(Role::Base), Color::Rgb(25, 23, 36));
        assert_eq!(t.color(Role::Surface0), Color::Rgb(31, 29, 46));
        assert_eq!(t.color(Role::Surface1), Color::Rgb(38, 35, 58));
        assert_eq!(t.color(Role::Muted), Color::Rgb(110, 106, 134));
        assert_eq!(t.color(Role::Subtext), Color::Rgb(144, 140, 170));
        assert_eq!(t.color(Role::Text), Color::Rgb(224, 222, 244));
        assert_eq!(t.color(Role::Accent), Color::Rgb(196, 167, 231));
        assert_eq!(t.color(Role::Love), Color::Rgb(235, 111, 146));
        assert_eq!(t.color(Role::Gold), Color::Rgb(246, 193, 119));
        assert_eq!(t.color(Role::Foam), Color::Rgb(156, 207, 216));
        assert_eq!(t.color(Role::Rose), Color::Rgb(235, 188, 186));
    }
}
