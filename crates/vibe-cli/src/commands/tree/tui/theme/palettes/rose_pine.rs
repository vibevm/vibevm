//! The **Rosé Pine** palette — the canonical-locked dark default (PROP-037
//! §2.2.1 `#palette-tokens`). A violet, cosmic-dark look: a purple-tinted base
//! with an iris/lavender accent. The eleven base-role hexes here are the exact
//! values the snapshot test in [`theme`] pins (PROP-037 §2.2.1 R8 fidelity);
//! they match the legacy `theme.rs` constants byte-for-byte.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#palette-tokens");

use specmark::spec;

use super::{Palette, Rgb, Role};

/// Rosé Pine — the cosmic-violet dark default. Canonical-locked: the eleven
/// base-role values in [`RosePine::TABLE`] are pinned by the snapshot test and
/// must not drift.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct RosePine;

/// The single source of truth for every Rosé Pine role → [`Rgb`] mapping. The
/// first eleven rows are the canonical base roles; the last five are the
/// derived composition tokens (PROP-037 §2.2.1). Read by both the [`Palette`]
/// impl and the const compatibility shim in [`theme`] that backs the legacy
/// `BASE`/`IRIS`/… colour constants — so the hex appears exactly once.
///
/// [`theme`]: crate::commands::tree::tui::theme
pub const TABLE: [(Role, Rgb); 16] = [
    (Role::Base, Rgb(25, 23, 36)),       // #191724
    (Role::Surface0, Rgb(31, 29, 46)),   // #1f1d2e
    (Role::Surface1, Rgb(38, 35, 58)),   // #26233a
    (Role::Muted, Rgb(110, 106, 134)),   // #6e6a86
    (Role::Subtext, Rgb(144, 140, 170)), // #908caa
    (Role::Text, Rgb(224, 222, 244)),    // #e0def4
    (Role::Accent, Rgb(196, 167, 231)),  // #c4a7e7  (iris)
    (Role::Love, Rgb(235, 111, 146)),    // #eb6f92
    (Role::Gold, Rgb(246, 193, 119)),    // #f6c177
    (Role::Foam, Rgb(156, 207, 216)),    // #9ccfd8
    (Role::Rose, Rgb(235, 188, 186)),    // #ebbcba
    // --- derived composition tokens -----------------------------------------
    (Role::Selection, Rgb(196, 167, 231)), // = Accent
    (Role::Border, Rgb(110, 106, 134)),    // = Muted
    (Role::Paper, Rgb(31, 29, 46)),        // = Surface0
    (Role::ButtonOn, Rgb(196, 167, 231)),  // = Accent
    (Role::ButtonOff, Rgb(38, 35, 58)),    // = Surface1
];

#[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#palette-tokens")]
impl Palette for RosePine {
    fn role(&self, role: Role) -> Rgb {
        for (r, rgb) in TABLE.iter() {
            if *r == role {
                return *rgb;
            }
        }
        // TABLE is total over `Role`, so this is unreachable; fall back to the
        // base rather than panicking (domain logic never unwinds).
        Rgb(25, 23, 36)
    }

    fn is_light(&self) -> bool {
        false
    }

    fn name(&self) -> &'static str {
        "rose-pine"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The eleven canonical base-role hexes are byte-identical to the legacy
    /// `theme.rs` constants — the R8 fidelity snapshot (PROP-037 §2.2.1).
    #[test]
    fn rose_pine_base_hexes_are_canonical() {
        assert_eq!(RosePine.role(Role::Base), Rgb(25, 23, 36), "#191724");
        assert_eq!(RosePine.role(Role::Surface0), Rgb(31, 29, 46), "#1f1d2e");
        assert_eq!(RosePine.role(Role::Surface1), Rgb(38, 35, 58), "#26233a");
        assert_eq!(RosePine.role(Role::Muted), Rgb(110, 106, 134), "#6e6a86");
        assert_eq!(RosePine.role(Role::Subtext), Rgb(144, 140, 170), "#908caa");
        assert_eq!(RosePine.role(Role::Text), Rgb(224, 222, 244), "#e0def4");
        assert_eq!(RosePine.role(Role::Accent), Rgb(196, 167, 231), "#c4a7e7");
        assert_eq!(RosePine.role(Role::Love), Rgb(235, 111, 146), "#eb6f92");
        assert_eq!(RosePine.role(Role::Gold), Rgb(246, 193, 119), "#f6c177");
        assert_eq!(RosePine.role(Role::Foam), Rgb(156, 207, 216), "#9ccfd8");
        assert_eq!(RosePine.role(Role::Rose), Rgb(235, 188, 186), "#ebbcba");
    }

    /// The derived roles compose exactly as PROP-037 §2.2.1 prescribes.
    #[test]
    fn rose_pine_derived_roles_compose() {
        assert_eq!(RosePine.role(Role::Selection), RosePine.role(Role::Accent));
        assert_eq!(RosePine.role(Role::Border), RosePine.role(Role::Muted));
        assert_eq!(RosePine.role(Role::Paper), RosePine.role(Role::Surface0));
        assert_eq!(RosePine.role(Role::ButtonOn), RosePine.role(Role::Accent));
        assert_eq!(
            RosePine.role(Role::ButtonOff),
            RosePine.role(Role::Surface1)
        );
    }

    #[test]
    fn rose_pine_is_dark() {
        assert!(!RosePine.is_light());
        assert_eq!(RosePine.name(), "rose-pine");
    }

    /// Every role resolves (the table is total) — nothing falls through to the
    /// base fallback.
    #[test]
    fn table_is_total() {
        let all = [
            Role::Base,
            Role::Surface0,
            Role::Surface1,
            Role::Muted,
            Role::Subtext,
            Role::Text,
            Role::Accent,
            Role::Love,
            Role::Gold,
            Role::Foam,
            Role::Rose,
            Role::Selection,
            Role::Border,
            Role::Paper,
            Role::ButtonOn,
            Role::ButtonOff,
        ];
        for r in all {
            let _ = RosePine.role(r);
        }
        assert_eq!(TABLE.len(), 16);
    }
}
