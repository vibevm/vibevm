//! The single source of colour, style, and glyphs for the TUI (PROP-037 §2.2 —
//! "the CSS"). A restyle touches only this module; no component hard-codes a
//! [`Color`]. Colour flows through a data-driven [`Palette`] of semantic
//! [`Role`] tokens (PROP-037 §2.2.1), glyphs through a [`Glyphs`] set (§2.2.2),
//! and both are carried by one [`Theme`] value-type that projects onto the
//! detected rendering [`Tier`] (§2.2.3).
//!
//! This module directory supersedes the legacy single `theme.rs`, but its
//! original public surface — the eleven `pub const` colours (`BASE`, …, `ROSE`)
//! and the sixteen `pub fn` style helpers (`text`, `dim`, `accent`, …) — is kept
//! as a thin delegating **compatibility shim** over a default [`Theme`] (Rosé
//! Pine, Tier 3), so every existing call site in `render`/`modal`/`menu`/`search`
//! keeps compiling unchanged. The next increment threads `&Theme` through `App`
//! explicitly and retires the shim.
//!
//! [`Color`]: ratatui_core::style::Color

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#theme");

use std::sync::OnceLock;

use ratatui_core::style::{Color, Modifier, Style};
use specmark::spec;

// The flat `theme::` aliases for the submodule API. The primary types
// (`Theme`, `Role`, `Palette`, `Rgb`, `Tier`, `Glyphs`, `PaletteName`,
// `project_color`) are exercised today by the legacy shim below; the rest
// (`corners`, the per-palette structs, `resolve`, `detect_tier`) compose the
// Phase-3 surface that lights up once `&Theme` is threaded through `App`, so
// their re-export aliases are unused for now.
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
    // Read by `glyphs()` below; both are the Phase-3 introspection surface.
    #[allow(dead_code)]
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

    // The introspection accessors below are the Phase-3 surface: they light up
    // once `&Theme` is threaded through `App` and components query the active
    // palette/tier. The style methods that follow are live today (the legacy
    // shim delegates to them), so they stay warning-clean unaided.
    /// The active palette's display name.
    #[allow(dead_code)]
    #[must_use]
    pub fn palette_name(&self) -> &'static str {
        self.palette.name()
    }

    /// The detected rendering tier.
    #[allow(dead_code)]
    #[must_use]
    pub fn tier(&self) -> Tier {
        self.tier
    }

    /// Whether the active palette is a light-background palette.
    #[allow(dead_code)]
    #[must_use]
    pub fn is_light(&self) -> bool {
        self.palette.is_light()
    }

    /// The active glyph set.
    #[allow(dead_code)]
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
}

// ---------------------------------------------------------------------------
// Legacy compatibility shim (PROP-037 §2.2) — keeps the pre-theme-module
// public surface resolving: `theme::BASE` … `theme::ROSE` and `theme::text()` …
// `theme::key_desc()`, now delegating through a default Rosé Pine / Tier-3
// Theme. Call sites in render/modal/menu/search do not change; a later change
// threads &Theme through App and retires this.
// ---------------------------------------------------------------------------

/// The default theme the legacy shim delegates to: Rosé Pine at Tier 3, exactly
/// the pre-module look. Built once behind an [`OnceLock`] (a `Theme` owns a
/// `Box<dyn Palette>`, so it cannot be `const`).
static DEFAULT_THEME: OnceLock<Theme> = OnceLock::new();

/// Fetch the process-wide default [`Theme`] used by the legacy free-fn /
/// free-const shim.
fn default_theme() -> &'static Theme {
    DEFAULT_THEME.get_or_init(|| Theme::new(PaletteName::RosePine, Tier::T3))
}

/// const lookup of a Rosé Pine role → [`Rgb`], straight off the canonical table
/// (the single source). `#[repr(u8)]` on [`Role`] makes the discriminant
/// comparison const-evaluable.
const fn rose_pine_rgb(role: Role) -> Rgb {
    let table = palettes::rose_pine::TABLE;
    let mut i = 0;
    while i < table.len() {
        if table[i].0 as u8 == role as u8 {
            return table[i].1;
        }
        i += 1;
    }
    // TABLE is total over Role; this is unreachable.
    Rgb(0, 0, 0)
}

/// const lift of a Rosé Pine role → [`Color::Rgb`], the value the legacy `pub
/// const` colours carry.
///
/// [`Color::Rgb`]: Color::Rgb
const fn rose_pine_color(role: Role) -> Color {
    let rgb = rose_pine_rgb(role);
    Color::Rgb(rgb.0, rgb.1, rgb.2)
}

// The eleven legacy colour constants, now derived from the Rosé Pine table
// (canonical-locked — byte-identical to the former literals). Each is a `const`
// lookup off `rose_pine_color`, so the canonical hex appears exactly once — in
// `palettes::rose_pine::TABLE` — and the legacy names never drift from it.
//
// Seven (`BASE`, `SURFACE0`, `IRIS`, `LOVE`, `GOLD`, `FOAM`, `ROSE`) are
// referenced by current call sites in `render` / `search`; the other four are
// retained to preserve the full legacy const surface (the next TUI phase, or a
// caller that drops the `Theme` indirection, may name them) — hence the allow.

/// Window/panel background — a deep purple-tinted black (`#191724`).
pub const BASE: Color = rose_pine_color(Role::Base);
/// A raised surface (status bar, chrome) (`#1f1d2e`).
pub const SURFACE0: Color = rose_pine_color(Role::Surface0);
/// A brighter surface (an off-flag, a subtle fill) (`#26233a`).
#[allow(dead_code)]
pub const SURFACE1: Color = rose_pine_color(Role::Surface1);
/// Muted foreground — borders, disabled text (`#6e6a86`).
#[allow(dead_code)]
pub const MUTED: Color = rose_pine_color(Role::Muted);
/// Secondary foreground (`#908caa`).
#[allow(dead_code)]
pub const SUBTEXT: Color = rose_pine_color(Role::Subtext);
/// Primary foreground — a lavender-white (`#e0def4`).
#[allow(dead_code)]
pub const TEXT: Color = rose_pine_color(Role::Text);
/// The accent — selection, titles, the brand. The cosmic violet (`#c4a7e7`).
pub const IRIS: Color = rose_pine_color(Role::Accent);
/// A pink accent — flags, warnings (`#eb6f92`).
pub const LOVE: Color = rose_pine_color(Role::Love);
/// A warm accent (a badge, a highlight) (`#f6c177`).
pub const GOLD: Color = rose_pine_color(Role::Gold);
/// A cool accent — static load, links (`#9ccfd8`).
pub const FOAM: Color = rose_pine_color(Role::Foam);
/// A soft rose (a secondary badge) (`#ebbcba`).
pub const ROSE: Color = rose_pine_color(Role::Rose);

// The sixteen legacy style helpers, now delegating to the default Theme.

/// Plain body text.
#[must_use]
pub fn text() -> Style {
    default_theme().text()
}

/// A muted secondary line.
#[must_use]
pub fn dim() -> Style {
    default_theme().dim()
}

/// The accent colour.
#[must_use]
pub fn accent() -> Style {
    default_theme().accent()
}

/// A panel / window title in the border.
#[must_use]
pub fn title() -> Style {
    default_theme().title()
}

/// The selection bar (a highlighted row).
#[must_use]
pub fn selection() -> Style {
    default_theme().selection()
}

/// A subtler selection for the main table.
#[must_use]
pub fn row_selection() -> Style {
    default_theme().row_selection()
}

/// A modal window's solid background.
#[must_use]
pub fn panel() -> Style {
    default_theme().panel()
}

/// A rounded panel border.
#[must_use]
pub fn border() -> Style {
    default_theme().border()
}

/// The top status bar.
#[must_use]
pub fn status_bar() -> Style {
    default_theme().status_bar()
}

/// A status-bar value (highlighted over the bar).
#[must_use]
pub fn status_value() -> Style {
    default_theme().status_value()
}

/// The table header row.
#[must_use]
pub fn header() -> Style {
    default_theme().header()
}

/// The colour of a load-type cell (`static` / `dynamic` / `none`).
#[must_use]
pub fn load(load: &str) -> Style {
    default_theme().load(load)
}

/// A set flag.
#[must_use]
pub fn flag_on() -> Style {
    default_theme().flag_on()
}

/// An unset flag.
#[must_use]
pub fn flag_off() -> Style {
    default_theme().flag_off()
}

/// A footer key label.
#[must_use]
pub fn key() -> Style {
    default_theme().key()
}

/// A footer key's description.
#[must_use]
pub fn key_desc() -> Style {
    default_theme().key_desc()
}

// --- glyph accessors (PROP-037 §2.2.2) -------------------------------------
// The tree's fold/dag/flag glyphs are read off the default theme's Glyphs set
// so call sites never hardcode an ASCII literal; threaded per-Theme in P3.

/// A folded-node marker (`▸` Tier ≥ 1, `+` Tier 0).
pub fn fold_collapsed() -> &'static str {
    default_theme().glyphs().fold_collapsed
}

/// An unfolded-node marker (`▾` Tier ≥ 1, `-` Tier 0).
pub fn fold_expanded() -> &'static str {
    default_theme().glyphs().fold_expanded
}

/// The DAG re-occurrence / cycle-guard marker (`↩` Tier ≥ 1, `*` Tier 0).
pub fn dag_dedup() -> &'static str {
    default_theme().glyphs().dag_dedup
}

/// A set-flag glyph (`●` Tier ≥ 1, `x` Tier 0). Distinct from the `flag_on`
/// *style* helper above — this is the glyph, that is the colour.
pub fn flag_on_glyph() -> &'static str {
    default_theme().glyphs().flag_on
}

/// An unset-flag glyph (`○` Tier ≥ 1, `.` Tier 0).
pub fn flag_off_glyph() -> &'static str {
    default_theme().glyphs().flag_off
}

/// The window/card close affordance (`✕` Tier ≥ 1, `x` Tier 0) — drawn top-right
/// on a window border (PROP-037 §2.2.4, §8). Distinct from the *style* helpers:
/// this is the glyph a component stamps into a border cell.
pub fn close_glyph() -> &'static str {
    default_theme().glyphs().close
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_theme_is_rose_pine_t3() {
        let t = default_theme();
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
    fn shim_style_fns_match_default_theme() {
        // The free fns are pure delegators.
        assert_eq!(text(), default_theme().text());
        assert_eq!(accent(), default_theme().accent());
        assert_eq!(selection(), default_theme().selection());
        assert_eq!(load("static"), default_theme().load("static"));
    }

    #[test]
    fn legacy_consts_match_rose_pine_table() {
        // R8: the eleven legacy consts are byte-identical to the Rosé Pine base
        // roles (the snapshot that pins the canonical-locked palette).
        assert_eq!(BASE, Color::Rgb(25, 23, 36));
        assert_eq!(SURFACE0, Color::Rgb(31, 29, 46));
        assert_eq!(SURFACE1, Color::Rgb(38, 35, 58));
        assert_eq!(MUTED, Color::Rgb(110, 106, 134));
        assert_eq!(SUBTEXT, Color::Rgb(144, 140, 170));
        assert_eq!(TEXT, Color::Rgb(224, 222, 244));
        assert_eq!(IRIS, Color::Rgb(196, 167, 231));
        assert_eq!(LOVE, Color::Rgb(235, 111, 146));
        assert_eq!(GOLD, Color::Rgb(246, 193, 119));
        assert_eq!(FOAM, Color::Rgb(156, 207, 216));
        assert_eq!(ROSE, Color::Rgb(235, 188, 186));
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
}
