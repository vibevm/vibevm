//! The five shipped palettes (PROP-037 §2.2.1 `#palette-tokens`) and their one
//! registration point. [`RosePine`] is the canonical-locked dark default;
//! [`catppuccin`] contributes the four Catppuccin variants. [`resolve`] is the
//! single place a [`PaletteName`] becomes a boxed [`Palette`] — adding a
//! palette means adding a variant plus a match arm here, nothing else.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#palette-tokens");

use specmark::spec;

pub use catppuccin::{Frappe, Latte, Macchiato, Mocha};
pub use rose_pine::RosePine;

use super::{Palette, Rgb, Role};

mod catppuccin;
/// Visible to the parent so the legacy colour-const shim can read [`rose_pine::TABLE`]
/// at `const` time (the single source of the canonical-locked hexes).
pub(super) mod rose_pine;

/// The enumerable set of shipped palettes (PROP-037 §2.2.1). The active palette
/// is a Model field, persisted through the settings system and overridable at
/// the CLI/env; this enum is its serialisable identity.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PaletteName {
    /// Rosé Pine — the canonical-locked cosmic-violet dark default.
    RosePine,
    /// Catppuccin Mocha (dark).
    Mocha,
    /// Catppuccin Macchiato (dark).
    Macchiato,
    /// Catppuccin Frappé (dark).
    Frappe,
    /// Catppuccin Latte (light).
    Latte,
}

impl PaletteName {
    /// The display name of the palette this variant resolves to.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            PaletteName::RosePine => "rose-pine",
            PaletteName::Mocha => "catppuccin-mocha",
            PaletteName::Macchiato => "catppuccin-macchiato",
            PaletteName::Frappe => "catppuccin-frappe",
            PaletteName::Latte => "catppuccin-latte",
        }
    }
}

/// The single registration point: turn a [`PaletteName`] into a boxed,
/// dyn-dispatchable [`Palette`]. This is the only match a palette identity
/// flows through — the [`Theme`](super::Theme) constructor calls it, and a
/// future settings-driven palette switch calls nothing else.
///
/// [`Theme`](super::Theme): crate::commands::tree::tui::theme::Theme
#[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#palette-tokens")]
#[must_use]
pub fn resolve(name: PaletteName) -> Box<dyn Palette> {
    match name {
        PaletteName::RosePine => Box::new(RosePine),
        PaletteName::Mocha => Box::new(Mocha),
        PaletteName::Macchiato => Box::new(Macchiato),
        PaletteName::Frappe => Box::new(Frappe),
        PaletteName::Latte => Box::new(Latte),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_returns_each_palette() {
        for name in [
            PaletteName::RosePine,
            PaletteName::Mocha,
            PaletteName::Macchiato,
            PaletteName::Frappe,
            PaletteName::Latte,
        ] {
            let p = resolve(name);
            assert_eq!(p.name(), name.label());
            // Smoke: every role resolves on every palette.
            for r in [
                Role::Base,
                Role::Text,
                Role::Accent,
                Role::Selection,
                Role::ButtonOff,
            ] {
                let _ = p.role(r);
            }
        }
    }

    #[test]
    fn only_latte_is_light() {
        assert!(!resolve(PaletteName::RosePine).is_light());
        assert!(!resolve(PaletteName::Mocha).is_light());
        assert!(!resolve(PaletteName::Macchiato).is_light());
        assert!(!resolve(PaletteName::Frappe).is_light());
        assert!(resolve(PaletteName::Latte).is_light());
    }

    #[test]
    fn labels_are_unique_and_stable() {
        let labels: Vec<&str> = vec![
            PaletteName::RosePine.label(),
            PaletteName::Mocha.label(),
            PaletteName::Macchiato.label(),
            PaletteName::Frappe.label(),
            PaletteName::Latte.label(),
        ];
        let mut sorted = labels.clone();
        sorted.sort();
        let mut deduped = sorted;
        deduped.dedup();
        assert_eq!(deduped.len(), labels.len(), "palette labels are unique");
        // Rgb::from_hex is a const constructor round-trip.
        assert_eq!(Rgb::from_hex(1, 2, 3), Rgb(1, 2, 3));
    }
}
