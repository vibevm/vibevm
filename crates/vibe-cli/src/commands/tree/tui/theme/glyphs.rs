//! The glyph vocabulary (PROP-037 §2.2.2 `#glyph-vocabulary`).
//!
//! Every glyph the TUI draws is a field on [`Glyphs`], never a hardcoded string
//! at a call site. [`Glyphs::rich`] is the Tier ≥ 1 Unicode vocabulary — the
//! fold indicator is `▾`/`▸` (not `+`/`-`), the DAG re-occurrence marker is
//! `↩` (not `(*)`), the on/off flags are `●`/`○` (not `x`/`.`), the frame stays
//! rounded `╭╮╰╯`, and the bar indicator uses block elements `▁▂▃▄▅▆▇█`.
//! [`Glyphs::ascii`] is the Tier 0 fallback (`+`/`-`/`*`/`#`/`x`/`.`). The
//! primary UI never uses those ASCII characters as *semantic* glyphs (PROP-037
//! §2.2.2).

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#glyph-vocabulary");

/// The complete glyph set the TUI renders with. Built once for a given tier by
/// [`Glyphs::rich`] (Tier ≥ 1) or [`Glyphs::ascii`] (Tier 0); the active
/// [`Theme`](crate::commands::tree::tui::theme::Theme) owns one and hands
/// `&Glyphs` to every component that draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Glyphs {
    /// Tree vertical connector (`│` rich / `|` ascii).
    pub tree_vertical: &'static str,
    /// Tree branch connector (`├` rich / `+` ascii).
    pub tree_branch: &'static str,
    /// Tree corner connector (`└` rich / `+` ascii).
    pub tree_corner: &'static str,
    /// Tree horizontal connector (`─` rich / `-` ascii).
    pub tree_horizontal: &'static str,
    /// A node currently unfolded (`▾` rich / `-` ascii).
    pub fold_expanded: &'static str,
    /// A node currently folded (`▸` rich / `+` ascii).
    pub fold_collapsed: &'static str,
    /// The DAG re-occurrence / cycle-guard marker (`↩` rich / `*` ascii).
    pub dag_dedup: &'static str,
    /// A set flag (`●` rich / `x` ascii).
    pub flag_on: &'static str,
    /// An unset flag (`○` rich / `.` ascii).
    pub flag_off: &'static str,
    /// Top-left frame corner (`╭` rich / `+` ascii).
    pub corner_tl: &'static str,
    /// Top-right frame corner (`╮` rich / `+` ascii).
    pub corner_tr: &'static str,
    /// Bottom-left frame corner (`╰` rich / `+` ascii).
    pub corner_bl: &'static str,
    /// Bottom-right frame corner (`╯` rich / `+` ascii).
    pub corner_br: &'static str,
    /// The window close affordance (`✕` rich / `x` ascii).
    pub close: &'static str,
    /// A horizontal separator (`─` rich / `-` ascii).
    pub separator: &'static str,
    /// Scroll-up affordance (`▲` rich / `^` ascii).
    pub scroll_up: &'static str,
    /// Scroll-down affordance (`▼` rich / `v` ascii).
    pub scroll_down: &'static str,
    /// The block-element load/activity bar, low → high.
    pub bar: [&'static str; 8],
}

impl Glyphs {
    /// The Tier ≥ 1 Unicode vocabulary (PROP-037 §2.2.2).
    #[must_use]
    pub fn rich() -> Self {
        Self {
            tree_vertical: "│",
            tree_branch: "├",
            tree_corner: "└",
            tree_horizontal: "─",
            fold_expanded: "▾",
            fold_collapsed: "▸",
            dag_dedup: "↩",
            flag_on: "●",
            flag_off: "○",
            corner_tl: "╭",
            corner_tr: "╮",
            corner_bl: "╰",
            corner_br: "╯",
            close: "✕",
            separator: "─",
            scroll_up: "▲",
            scroll_down: "▼",
            bar: ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"],
        }
    }

    /// The Tier 0 ASCII fallback (`TERM=linux`, dumb, no Unicode) — PROP-037
    /// §2.2.3. No Unicode in any field.
    #[must_use]
    pub fn ascii() -> Self {
        Self {
            tree_vertical: "|",
            tree_branch: "+",
            tree_corner: "+",
            tree_horizontal: "-",
            fold_expanded: "-",
            fold_collapsed: "+",
            dag_dedup: "*",
            flag_on: "x",
            flag_off: ".",
            corner_tl: "+",
            corner_tr: "+",
            corner_bl: "+",
            corner_br: "+",
            close: "x",
            separator: "-",
            scroll_up: "^",
            scroll_down: "v",
            bar: ["#"; 8],
        }
    }
}

/// The four frame corners for a window, rounded (`╭╮╰╯`) or square
/// (`┌┐└┘`) — both still Unicode, for the Tier ≥ 1 rounded-vs-square frame
/// choice (PROP-037 §2.2.4). Tier 0 builds its [`Glyphs`] with `+` corners
/// directly via [`Glyphs::ascii`].
#[must_use]
pub fn corners(rounded: bool) -> [&'static str; 4] {
    if rounded {
        ["╭", "╮", "╰", "╯"]
    } else {
        ["┌", "┐", "└", "┘"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rich_uses_unicode_vocabulary() {
        let g = Glyphs::rich();
        assert_eq!(g.fold_expanded, "▾");
        assert_eq!(g.fold_collapsed, "▸");
        assert_eq!(g.dag_dedup, "↩");
        assert_eq!(g.flag_on, "●");
        assert_eq!(g.flag_off, "○");
        assert_eq!(g.close, "✕");
        assert_eq!(g.bar[7], "█");
        assert_eq!(g.bar.len(), 8);
    }

    #[test]
    fn ascii_is_ascii_only() {
        let g = Glyphs::ascii();
        let all = [
            g.tree_vertical,
            g.tree_branch,
            g.tree_corner,
            g.tree_horizontal,
            g.fold_expanded,
            g.fold_collapsed,
            g.dag_dedup,
            g.flag_on,
            g.flag_off,
            g.corner_tl,
            g.corner_tr,
            g.corner_bl,
            g.corner_br,
            g.close,
            g.separator,
            g.scroll_up,
            g.scroll_down,
        ];
        for s in all {
            assert!(s.is_ascii(), "{s:?} must be ASCII at Tier 0");
        }
        assert_eq!(g.bar[0], "#");
    }

    #[test]
    fn corners_rounded_vs_square() {
        assert_eq!(corners(true), ["╭", "╮", "╰", "╯"]);
        assert_eq!(corners(false), ["┌", "┐", "└", "┘"]);
    }
}
