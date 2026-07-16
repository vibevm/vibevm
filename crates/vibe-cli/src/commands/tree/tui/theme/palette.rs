//! The semantic colour vocabulary (PROP-037 §2.2.1 `#palette-tokens`).
//!
//! Colour reaches a component only through a [`Palette`] — a data-driven
//! mapping from a fixed set of **semantic role tokens** ([`Role`]) to an exact
//! [`Rgb`]. No component names a colour literal; it names a role, and the
//! active palette resolves the role to a value. This is the indirection that
//! lets one restyle the whole TUI by swapping the palette (PROP-037 §1.4 — the
//! theme is the TUI's "CSS").

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#palette-tokens");

/// The sixteen semantic role tokens (PROP-037 §2.2.1). A component names a
/// role; the active [`Palette`] resolves it to a colour. The first eleven are
/// the "base" roles (the palette's identity); the last five are **derived**
/// composition tokens (`Selection` = accent ground + base text, `Border` =
/// muted, `Paper` = surface0, `ButtonOn`/`ButtonOff` = accent/surface1) a
/// palette may override but usually maps to a base role.
///
/// `#[repr(u8)]` keeps the discriminants small and lets the const compatibility
/// shim in [`crate::commands::tree::tui::theme`] look a role up in a `const`
/// table by discriminant.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Role {
    /// Window/panel background.
    Base,
    /// A raised surface (status bar, chrome).
    Surface0,
    /// A brighter surface (an off flag, a subtle fill).
    Surface1,
    /// Muted foreground — borders, disabled text.
    Muted,
    /// Secondary foreground.
    Subtext,
    /// Primary foreground.
    Text,
    /// The accent — selection, titles, the brand.
    Accent,
    /// A pink accent — flags, warnings.
    Love,
    /// A warm accent — a badge, a highlight.
    Gold,
    /// A cool accent — static load, links.
    Foam,
    /// A soft rose — a secondary badge.
    Rose,
    // --- derived composition tokens -------------------------------------
    /// The selection bar ground (composed from the accent).
    Selection,
    /// A panel border stroke (usually `Muted`).
    Border,
    /// A "paper" detail-card panel (usually `Surface0`).
    Paper,
    /// An enabled button (usually `Accent`).
    ButtonOn,
    /// A disabled button (usually `Surface1`).
    ButtonOff,
}

/// A canonical `sRGB` colour value — the single carrier a [`Palette`] emits.
///
/// Stored as three `u8` channels so a palette's identity is terminal-agnostic;
/// [`Rgb::to_color`] lifts it to a ratatui [`Color::Rgb`] for Tier 3 rendering,
/// and [`crate::commands::tree::tui::theme::project_color`] projects it down to
/// 256 / 16 / mono for the lower tiers (PROP-037 §2.2.3).
///
/// [`Color::Rgb`]: ratatui_core::style::Color::Rgb
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// const constructor — the only way to build a palette table at const time.
    #[must_use]
    pub const fn from_hex(r: u8, g: u8, b: u8) -> Self {
        Self(r, g, b)
    }

    /// Lift to a ratatui truecolour (Tier 3).
    #[must_use]
    pub fn to_color(self) -> ratatui_core::style::Color {
        ratatui_core::style::Color::Rgb(self.0, self.1, self.2)
    }
}

/// A complete semantic-role → colour mapping (PROP-037 §2.2.1).
///
/// A palette carries the eleven base roles plus values for the five derived
/// roles, an `is_light` flag (so the TUI can invert selection/paper rendering
/// for a light theme), and a display `name`. Five implementations ship — see
/// [`crate::commands::tree::tui::theme::palettes`].
///
/// # Examples
///
/// A tiny anonymous palette over two roles:
///
/// ```
/// use vibe_cli::commands::tree::tui::theme::palette::{Palette, Rgb, Role};
///
/// struct Mono;
/// impl Palette for Mono {
///     fn role(&self, r: Role) -> Rgb {
///         match r {
///             Role::Base | Role::Text => Rgb(240, 240, 240),
///             _ => Rgb(20, 20, 20),
///         }
///     }
///     fn is_light(&self) -> bool { true }
///     fn name(&self) -> &'static str { "mono" }
/// }
///
/// let p = Mono;
/// assert_eq!(p.role(Role::Base), Rgb(240, 240, 240));
/// assert!(p.is_light());
/// assert_eq!(p.name(), "mono");
/// ```
pub trait Palette: Send + Sync {
    /// Resolve a semantic [`Role`] to its exact [`Rgb`] in this palette.
    fn role(&self, role: Role) -> Rgb;
    /// Whether this is a light-background palette (drives selection/paper
    /// inversion, PROP-037 §2.2.1).
    fn is_light(&self) -> bool;
    /// The palette's display name (e.g. `"rose-pine"`).
    fn name(&self) -> &'static str;
}
