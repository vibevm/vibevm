//! Rendering tiers + colour projection (PROP-037 §2.2.3 `#rendering-tiers`).
//!
//! The TUI degrades through four tiers — **3** truecolour, **2** 256-colour,
//! **1** 16 ANSI, **0** dumb/mono — discovered once at launch by a **pure
//! function** over the environment ([`detect_tier`]). Degradation is a
//! **projection**: [`project_color`] takes the canonical [`Rgb`] for a role and
//! projects it onto the detected tier, so one [`Theme`](super::Theme) is built
//! and many projections flow from it — never bespoke per-tier rendering in a
//! component.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#rendering-tiers");

use ratatui_core::style::Color;
use specmark::spec;

use super::{Rgb, Role};

/// The four rendering tiers (PROP-037 §2.2.3), ordered low → high so `Ord`
/// gives "is this tier at least Tier N" for free.
///
/// `T0 < T1 < T2 < T3`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
    /// Dumb / `TERM=linux` / no Unicode: ANSI mono, ASCII frames.
    T0,
    /// 16 ANSI colours, role→ANSI mapping, rounded-or-square frames.
    T1,
    /// 256 colours: palette quantised to the 6×6×6 cube.
    T2,
    /// Truecolour: full RGB.
    T3,
}

impl Tier {
    /// Whether this tier renders full RGB (`Tier::T3` only).
    #[must_use]
    pub fn supports_truecolor(self) -> bool {
        self == Tier::T3
    }

    /// Whether this tier falls back to ASCII `+-|` frames (`Tier::T0` only).
    #[must_use]
    pub fn uses_ascii_frames(self) -> bool {
        self == Tier::T0
    }
}

/// Detect the rendering tier from the environment, **purely** (PROP-037
/// §2.2.3). `$COLORTERM` is consulted first, then `$TERM`; `crossterm` exposes
/// no colour-count API, so the TUI reads the env once at launch and feeds both
/// values in. The detected tier is overridable through the settings system.
///
/// The **default is Tier 3 (truecolour)** for anything not explicitly dumb:
/// every incumbent terminal (Warp, iTerm2, Windows Terminal, Alacritty, kitty,
/// GNOME…) renders truecolour, and several of them — especially on Windows —
/// do not advertise it via `TERM`/`COLORTERM` at all. Defaulting to Tier 3
/// makes the TUI colourful out of the box instead of degrading a modern
/// terminal to mono. The lower tiers are the **fallback** (the degradation
/// path), reached only when the environment explicitly advertises a lower
/// capability — a 256-colour `TERM`, or an explicitly dumb `TERM=linux`/`dumb`
/// — or when overridden via `vibe.tree.tier`.
///
/// # Examples
///
/// Every branch, doctested:
///
/// ```
/// use vibe_cli::commands::tree::tui::theme::tier::{Tier, detect_tier};
///
/// // COLORTERM wins outright → Tier 3.
/// assert_eq!(detect_tier(Some("truecolor"), Some("xterm")), Tier::T3);
/// assert_eq!(detect_tier(Some("24bit"), Some("dumb")), Tier::T3);
/// // CASE-INSENSITIVE.
/// assert_eq!(detect_tier(Some("TrueColor"), None), Tier::T3);
///
/// // 256-colour TERM (with no truecolour COLORTERM) → Tier 2.
/// assert_eq!(detect_tier(None, Some("xterm-256color")), Tier::T2);
/// assert_eq!(detect_tier(Some(""), Some("xterm-256color")), Tier::T2);
///
/// // Explicitly dumb → Tier 0 (the only path to mono).
/// assert_eq!(detect_tier(None, Some("linux")), Tier::T0);
/// assert_eq!(detect_tier(None, Some("dumb")), Tier::T0);
///
/// // Everything else — including an UNSET/empty TERM (a modern terminal that
/// // doesn't advertise via env, e.g. Warp on Windows) → Tier 3.
/// assert_eq!(detect_tier(None, None), Tier::T3);
/// assert_eq!(detect_tier(None, Some("")), Tier::T3);
/// assert_eq!(detect_tier(None, Some("xterm")), Tier::T3);
/// assert_eq!(detect_tier(None, Some("screen")), Tier::T3);
/// ```
#[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#rendering-tiers")]
#[must_use]
pub fn detect_tier(colorterm: Option<&str>, term: Option<&str>) -> Tier {
    // 1. COLORTERM truecolour wins → Tier 3 (case-insensitive).
    if let Some(ct) = colorterm {
        let lower = ct.to_ascii_lowercase();
        if lower == "truecolor" || lower == "24bit" {
            return Tier::T3;
        }
    }
    // 2. A 256-colour TERM → Tier 2.
    let term_str = term.unwrap_or("");
    if term_str.contains("256") {
        return Tier::T2;
    }
    // 3. Explicitly dumb terminals (the Linux VT, or `dumb`) → Tier 0 — the
    // only path to mono / ASCII frames. An empty or unset TERM is NOT dumb: it
    // is a modern terminal that did not advertise via env (Warp on Windows),
    // so it falls through to the Tier 3 default below.
    if term_str == "linux" || term_str == "dumb" {
        return Tier::T0;
    }
    // 4. Anything else — including an unset/empty TERM — assumes Tier 3
    // truecolour (the modern-terminal default; override via `vibe.tree.tier`).
    Tier::T3
}

/// Project a canonical [`Rgb`] onto the detected [`Tier`] (PROP-037 §2.2.3):
/// T3 keeps the truecolour; T2 quantises to the xterm 6×6×6 cube; T1 maps the
/// role to one of the 16 ANSI colours; T0 resets to the terminal default (mono).
/// `is_light` selects the ANSI fg/bg polarity for T1.
#[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#rendering-tiers")]
#[must_use]
pub fn project_color(rgb: Rgb, role: Role, tier: Tier, is_light: bool) -> Color {
    match tier {
        Tier::T3 => Color::Rgb(rgb.0, rgb.1, rgb.2),
        Tier::T2 => quantize_256(rgb),
        Tier::T1 => ansi_16(role, is_light),
        Tier::T0 => Color::Reset,
    }
}

/// The canonical xterm 6-level channel ramp (PROP-037 §2.2.3). A channel value
/// snaps to its nearest step; the colour indexes the 6×6×6 cube at
/// `16 + 36*r + 6*g + b`.
const CUBE_RAMP: [u8; 6] = [0, 95, 135, 175, 215, 255];

/// Quantise an [`Rgb`] to the nearest xterm 256-colour cube index. Nearest
/// channel wins (Euclidean-by-step is equivalent to per-channel nearest on a
/// monotonic ramp); the grayscale ladder at 232–255 is not used, matching the
/// "canonical 6-level ramp" decision (PROP-037 §2.2.3).
#[must_use]
pub(crate) fn quantize_256(rgb: Rgb) -> Color {
    let nearest = |c: u8| -> u8 {
        let mut best = 0u8;
        let mut best_dist = u32::MAX;
        for (i, &v) in CUBE_RAMP.iter().enumerate() {
            let d = (c as i32 - v as i32).unsigned_abs();
            if d < best_dist {
                best_dist = d;
                best = i as u8;
            }
        }
        best
    };
    let r = nearest(rgb.0);
    let g = nearest(rgb.1);
    let b = nearest(rgb.2);
    Color::Indexed(16 + 36 * r + 6 * g + b)
}

/// Map a [`Role`] to one of the 16 ANSI colours for Tier 1 (PROP-037 §2.2.3).
///
/// The mapping (documented, deterministic):
/// - `Accent`/`Selection`/`ButtonOn` → `Magenta` (the violet/mauve brand),
/// - `Love` → `Red`,
/// - `Gold` → `Yellow`,
/// - `Foam` → `Green`,
/// - `Rose` → `LightMagenta`,
/// - `Muted`/`Border` → `DarkGray`,
/// - `Subtext`/`ButtonOff` → `Gray`,
/// - `Text` → `White` (dark) / `Black` (light),
/// - `Base`/`Surface0`/`Surface1`/`Paper` → `Black` (dark) / `White` (light).
#[must_use]
pub(crate) fn ansi_16(role: Role, is_light: bool) -> Color {
    // The foreground/background polarity pair: dark themes want light text on
    // dark base; light themes invert.
    let text = if is_light { Color::Black } else { Color::White };
    let base = if is_light { Color::White } else { Color::Black };
    match role {
        Role::Accent | Role::Selection | Role::ButtonOn => Color::Magenta,
        Role::Love => Color::Red,
        Role::Gold => Color::Yellow,
        Role::Foam => Color::Green,
        Role::Rose => Color::LightMagenta,
        Role::Muted | Role::Border => Color::DarkGray,
        Role::Subtext | Role::ButtonOff => Color::Gray,
        Role::Text => text,
        Role::Base | Role::Surface0 | Role::Surface1 | Role::Paper => base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_truecolor_branch() {
        assert_eq!(detect_tier(Some("truecolor"), Some("xterm")), Tier::T3);
        assert_eq!(detect_tier(Some("24bit"), Some("dumb")), Tier::T3);
        assert_eq!(detect_tier(Some("TrueColor"), None), Tier::T3);
        assert_eq!(detect_tier(Some("24BIT"), Some("linux")), Tier::T3);
    }

    #[test]
    fn detect_256_branch() {
        assert_eq!(detect_tier(None, Some("xterm-256color")), Tier::T2);
        assert_eq!(detect_tier(Some(""), Some("xterm-256color")), Tier::T2);
        assert_eq!(detect_tier(None, Some("screen-256color")), Tier::T2);
    }

    #[test]
    fn detect_dumb_branch() {
        // Only explicitly dumb terminals → Tier 0.
        assert_eq!(detect_tier(None, Some("linux")), Tier::T0);
        assert_eq!(detect_tier(None, Some("dumb")), Tier::T0);
    }

    #[test]
    fn detect_default_t3_branch() {
        // Anything else — including unset/empty TERM (a modern terminal that
        // didn't advertise via env) → Tier 3, the modern-terminal default.
        assert_eq!(detect_tier(None, Some("xterm")), Tier::T3);
        assert_eq!(detect_tier(None, Some("screen")), Tier::T3);
        assert_eq!(detect_tier(None, Some("rxvt-unicode")), Tier::T3);
        assert_eq!(detect_tier(None, None), Tier::T3);
        assert_eq!(detect_tier(None, Some("")), Tier::T3);
    }

    #[test]
    fn colorterm_beats_term() {
        // COLORTERM=truecolor with a dumb TERM still yields T3.
        assert_eq!(detect_tier(Some("truecolor"), Some("dumb")), Tier::T3);
    }

    #[test]
    fn tier_ordering_and_methods() {
        assert!(Tier::T0 < Tier::T1);
        assert!(Tier::T1 < Tier::T2);
        assert!(Tier::T2 < Tier::T3);
        assert!(Tier::T3.supports_truecolor());
        assert!(!Tier::T2.supports_truecolor());
        assert!(Tier::T0.uses_ascii_frames());
        assert!(!Tier::T1.uses_ascii_frames());
    }

    #[test]
    fn quantize_snaps_to_cube_corners() {
        // Pure black / white / a primary each snap to the cube corners.
        assert_eq!(quantize_256(Rgb(0, 0, 0)), Color::Indexed(16)); // 16 + 0
        assert_eq!(quantize_256(Rgb(255, 255, 255)), Color::Indexed(231)); // 16+36*5+6*5+5
        assert_eq!(quantize_256(Rgb(255, 0, 0)), Color::Indexed(196)); // 16+36*5 = 196
        assert_eq!(quantize_256(Rgb(0, 255, 0)), Color::Indexed(46)); // 16+6*5 = 46
        assert_eq!(quantize_256(Rgb(0, 0, 255)), Color::Indexed(21)); // 16+5 = 21
    }

    #[test]
    fn project_color_per_tier() {
        let rose_accent = Rgb(196, 167, 231); // Rosé Pine iris
        assert_eq!(
            project_color(rose_accent, Role::Accent, Tier::T3, false),
            Color::Rgb(196, 167, 231)
        );
        // T2 → some Indexed cube cell (not Reset, not Rgb).
        assert!(matches!(
            project_color(rose_accent, Role::Accent, Tier::T2, false),
            Color::Indexed(_)
        ));
        // T1 → the ANSI mapping (accent = Magenta).
        assert_eq!(
            project_color(rose_accent, Role::Accent, Tier::T1, false),
            Color::Magenta
        );
        assert_eq!(
            project_color(rose_accent, Role::Love, Tier::T1, false),
            Color::Red
        );
        // T0 → Reset (mono).
        assert_eq!(
            project_color(rose_accent, Role::Accent, Tier::T0, false),
            Color::Reset
        );
    }

    #[test]
    fn ansi_polarity_inverts_on_light() {
        assert_eq!(ansi_16(Role::Text, false), Color::White);
        assert_eq!(ansi_16(Role::Text, true), Color::Black);
        assert_eq!(ansi_16(Role::Base, false), Color::Black);
        assert_eq!(ansi_16(Role::Base, true), Color::White);
        // Non-polar roles are light-invariant.
        assert_eq!(ansi_16(Role::Love, true), Color::Red);
        assert_eq!(ansi_16(Role::Foam, true), Color::Green);
    }
}
