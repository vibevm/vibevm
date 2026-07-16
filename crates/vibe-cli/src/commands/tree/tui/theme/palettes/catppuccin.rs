//! The four **Catppuccin** palettes (PROP-037 §2.2.1 `#palette-tokens`): Mocha,
//! Macchiato, and Frappé are dark; Latte is the light variant. The hex values
//! are the canonical Catppuccin colours (`accent`←mauve, `love`←red,
//! `gold`←yellow, `foam`←teal, `rose`←pink, `muted`←overlay0, `subtext`←
//! subtext0). Each is a unit struct over a `const TABLE` consulted by its
//! [`Palette`] impl.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#palette-tokens");

use specmark::spec;

use super::{Palette, Rgb, Role};

/// Catppuccin **Mocha** — the darkest variant.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Mocha;
/// Catppuccin **Macchiato**.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Macchiato;
/// Catppuccin **Frappé**.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Frappe;
/// Catppuccin **Latte** — the light variant (`is_light = true`).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Latte;

const MOCHA: [(Role, Rgb); 16] = [
    (Role::Base, Rgb(30, 30, 46)),       // #1e1e2e
    (Role::Surface0, Rgb(49, 50, 68)),   // #313244
    (Role::Surface1, Rgb(69, 71, 90)),   // #45475a
    (Role::Muted, Rgb(108, 112, 134)),   // #6c7086
    (Role::Subtext, Rgb(166, 173, 200)), // #a6adc8
    (Role::Text, Rgb(205, 214, 244)),    // #cdd6f4
    (Role::Accent, Rgb(203, 166, 247)),  // #cba6f7  (mauve)
    (Role::Love, Rgb(243, 139, 168)),    // #f38ba8  (red)
    (Role::Gold, Rgb(249, 226, 175)),    // #f9e2af  (yellow)
    (Role::Foam, Rgb(148, 226, 213)),    // #94e2d5  (teal)
    (Role::Rose, Rgb(245, 194, 231)),    // #f5c2e7  (pink)
    (Role::Selection, Rgb(203, 166, 247)),
    (Role::Border, Rgb(108, 112, 134)),
    (Role::Paper, Rgb(49, 50, 68)),
    (Role::ButtonOn, Rgb(203, 166, 247)),
    (Role::ButtonOff, Rgb(69, 71, 90)),
];

const MACCHIATO: [(Role, Rgb); 16] = [
    (Role::Base, Rgb(36, 39, 58)),       // #24273a
    (Role::Surface0, Rgb(54, 58, 79)),   // #363a4f
    (Role::Surface1, Rgb(73, 77, 100)),  // #494d64
    (Role::Muted, Rgb(110, 115, 141)),   // #6e738d
    (Role::Subtext, Rgb(165, 173, 203)), // #a5adcb
    (Role::Text, Rgb(202, 211, 245)),    // #cad3f5
    (Role::Accent, Rgb(198, 160, 246)),  // #c6a0f6
    (Role::Love, Rgb(237, 135, 150)),    // #ed8796
    (Role::Gold, Rgb(238, 212, 159)),    // #eed49f
    (Role::Foam, Rgb(139, 213, 202)),    // #8bd5ca
    (Role::Rose, Rgb(245, 189, 230)),    // #f5bde6
    (Role::Selection, Rgb(198, 160, 246)),
    (Role::Border, Rgb(110, 115, 141)),
    (Role::Paper, Rgb(54, 58, 79)),
    (Role::ButtonOn, Rgb(198, 160, 246)),
    (Role::ButtonOff, Rgb(73, 77, 100)),
];

const FRAPPE: [(Role, Rgb); 16] = [
    (Role::Base, Rgb(48, 52, 70)),       // #303446
    (Role::Surface0, Rgb(65, 69, 89)),   // #414559
    (Role::Surface1, Rgb(81, 87, 109)),  // #51576d
    (Role::Muted, Rgb(115, 121, 148)),   // #737994
    (Role::Subtext, Rgb(165, 173, 206)), // #a5adce
    (Role::Text, Rgb(198, 208, 245)),    // #c6d0f5
    (Role::Accent, Rgb(202, 158, 230)),  // #ca9ee6
    (Role::Love, Rgb(231, 130, 132)),    // #e78284
    (Role::Gold, Rgb(229, 200, 144)),    // #e5c890
    (Role::Foam, Rgb(129, 200, 190)),    // #81c8be
    (Role::Rose, Rgb(244, 184, 228)),    // #f4b8e4
    (Role::Selection, Rgb(202, 158, 230)),
    (Role::Border, Rgb(115, 121, 148)),
    (Role::Paper, Rgb(65, 69, 89)),
    (Role::ButtonOn, Rgb(202, 158, 230)),
    (Role::ButtonOff, Rgb(81, 87, 109)),
];

const LATTE: [(Role, Rgb); 16] = [
    (Role::Base, Rgb(239, 241, 245)),     // #eff1f5
    (Role::Surface0, Rgb(204, 208, 218)), // #ccd0da
    (Role::Surface1, Rgb(188, 192, 204)), // #bcc0cc
    (Role::Muted, Rgb(156, 160, 176)),    // #9ca0b0
    (Role::Subtext, Rgb(108, 111, 133)),  // #6c6f85
    (Role::Text, Rgb(76, 79, 105)),       // #4c4f69
    (Role::Accent, Rgb(136, 57, 239)),    // #8839ef  (mauve)
    (Role::Love, Rgb(210, 15, 57)),       // #d20f39  (red)
    (Role::Gold, Rgb(223, 142, 29)),      // #df8e1d  (yellow)
    (Role::Foam, Rgb(23, 146, 153)),      // #179299  (teal)
    (Role::Rose, Rgb(234, 118, 203)),     // #ea76cb  (pink)
    (Role::Selection, Rgb(136, 57, 239)),
    (Role::Border, Rgb(156, 160, 176)),
    (Role::Paper, Rgb(204, 208, 218)),
    (Role::ButtonOn, Rgb(136, 57, 239)),
    (Role::ButtonOff, Rgb(188, 192, 204)),
];

/// Look up a role in a Catppuccin table. Total over `Role`; falls back to the
/// base rather than panicking (domain logic never unwinds).
fn lookup(table: &[(Role, Rgb); 16], role: Role) -> Rgb {
    for (r, rgb) in table.iter() {
        if *r == role {
            return *rgb;
        }
    }
    table[0].1
}

#[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#palette-tokens")]
impl Palette for Mocha {
    fn role(&self, role: Role) -> Rgb {
        lookup(&MOCHA, role)
    }
    fn is_light(&self) -> bool {
        false
    }
    fn name(&self) -> &'static str {
        "catppuccin-mocha"
    }
}

#[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#palette-tokens")]
impl Palette for Macchiato {
    fn role(&self, role: Role) -> Rgb {
        lookup(&MACCHIATO, role)
    }
    fn is_light(&self) -> bool {
        false
    }
    fn name(&self) -> &'static str {
        "catppuccin-macchiato"
    }
}

#[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#palette-tokens")]
impl Palette for Frappe {
    fn role(&self, role: Role) -> Rgb {
        lookup(&FRAPPE, role)
    }
    fn is_light(&self) -> bool {
        false
    }
    fn name(&self) -> &'static str {
        "catppuccin-frappe"
    }
}

#[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#palette-tokens")]
impl Palette for Latte {
    fn role(&self, role: Role) -> Rgb {
        lookup(&LATTE, role)
    }
    fn is_light(&self) -> bool {
        true
    }
    fn name(&self) -> &'static str {
        "catppuccin-latte"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mocha_base_hexes() {
        assert_eq!(Mocha.role(Role::Base), Rgb(30, 30, 46));
        assert_eq!(Mocha.role(Role::Accent), Rgb(203, 166, 247));
        assert_eq!(Mocha.role(Role::Love), Rgb(243, 139, 168));
        assert!(!Mocha.is_light());
    }

    #[test]
    fn macchiato_base_hexes() {
        assert_eq!(Macchiato.role(Role::Base), Rgb(36, 39, 58));
        assert_eq!(Macchiato.role(Role::Accent), Rgb(198, 160, 246));
        assert!(!Macchiato.is_light());
    }

    #[test]
    fn frappe_base_hexes() {
        assert_eq!(Frappe.role(Role::Base), Rgb(48, 52, 70));
        assert_eq!(Frappe.role(Role::Foam), Rgb(129, 200, 190));
        assert!(!Frappe.is_light());
    }

    #[test]
    fn latte_is_light_and_canonical() {
        assert!(Latte.is_light());
        assert_eq!(Latte.role(Role::Base), Rgb(239, 241, 245));
        assert_eq!(Latte.role(Role::Text), Rgb(76, 79, 105));
        assert_eq!(Latte.role(Role::Accent), Rgb(136, 57, 239));
    }

    #[test]
    fn catppuccin_tables_are_total() {
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
            for table in [MOCHA, MACCHIATO, FRAPPE, LATTE] {
                assert!(table.iter().any(|(rr, _)| *rr == r), "role missing");
                let _ = lookup(&table, r);
            }
        }
    }
}
