//! The detail modal: `Enter` opens the detail **Card** (PROP-037 §8
//! `#detail-card`) centred over the tree, showing the selected row's full
//! detail as a labelled vertical form — bold headers, wrapped values, a `✕`
//! close affordance; `Esc`/`✕` close it (PROP-036 §2.11). Rendered last in the
//! draw pass so it sits on top.
//!
//! The Card component (PROP-037 §2.9 `#card`, in [`super::ui::card`]) owns the
//! frame, the panel ground, the close glyph, the bold-header + wrapped-value
//! layout, and the content-based sizing. This module is only the bridge from
//! the TUI's selected row to a populated [`Card`]: it decides *what* fields the
//! card carries (the PROP-036 §2.11 package detail), the card decides *how*
//! they look.

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-036#tui");

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use ratatui_core::text::Line;

use super::super::model::{
    Condition, ConditionKind, DeclaredLink, LoadOrigin, LoadType, Package, Source, SourceKind,
};
use super::state::{App, RowNode};
use super::theme::Theme;
use super::ui::Card;

/// Draw the detail modal centred over `area`: build a [`Card`] for the selected
/// row and let it render itself (PROP-037 §8). A no-op when the selection
/// carries no detail (separator / subheader / nothing selected).
pub fn draw(area: Rect, buf: &mut Buffer, app: &App) {
    if area.width < 8 || area.height < 5 {
        return;
    }
    let Some(card) = detail_card(app) else {
        return;
    };
    card.render(area, buf, &app.theme);
}

/// Build the detail [`Card`] for the selected row, if any (PROP-036 §2.11).
fn detail_card(app: &App) -> Option<Card> {
    let row = app.selected_row()?;
    match row.node {
        // Label rows carry no detail; `open_modal` never opens them, and a
        // `None` here keeps the modal closed if one ever slips through.
        RowNode::Separator | RowNode::Subheader => None,
        RowNode::Missing => {
            let mut card = Card::new(Line::styled(" detail ", app.theme.title()));
            card.push("id", &row.id);
            card.push("status", "not in the lockfile");
            Some(card)
        }
        RowNode::Package(i) => app
            .tree
            .packages
            .get(i)
            .map(|p| package_card(p, &app.theme)),
    }
}

/// The full package detail, one labelled field per PROP-036 §2.11 entry. The
/// package id is the card's border title (the prominent heading); the labelled
/// rows below carry the rest of the field set.
fn package_card(p: &Package, theme: &Theme) -> Card {
    let mut card = Card::new(Line::styled(format!(" {} ", p.id), theme.title()));
    card.push("group", &p.group);
    card.push("name", &p.name);
    card.push("version", &p.version);
    card.push("kind", &p.kind);
    card.push("load type", load_type_label(p.load.load_type));
    card.push(
        "declared link",
        p.load.declared.map(declared_label).unwrap_or("(none)"),
    );
    card.push(
        "transitive",
        format!(
            "{}   (origin: {})",
            p.load.transitive,
            origin_label(p.load.origin)
        ),
    );
    card.push("condition", condition_value(&p.condition));
    card.push("in STATIC.md", p.load.in_static_md);
    card.push("in INDEX.md", p.load.in_index_md);
    card.push("source", source_value(p.source.as_ref()));
    card.push(
        "content hash",
        p.content_hash.as_deref().unwrap_or("(none)"),
    );
    card.push("boot path", p.load.boot_path.as_deref().unwrap_or("(none)"));
    card.push("dependencies", deps_value(&p.dependencies));
    card
}

/// The `when` condition value, with the full raw text and the parsed kind
/// (PROP-036 §2.5).
fn condition_value(c: &Condition) -> String {
    if !c.present {
        return "(none)".to_string();
    }
    let raw = c.raw.as_deref().unwrap_or("");
    let kind = c.kind.map(condition_kind_label).unwrap_or("");
    let value = c.value.as_deref().unwrap_or("");
    if kind.is_empty() {
        raw.to_string()
    } else {
        format!("{raw}   ({kind} = {value})")
    }
}

/// The source provenance value (PROP-036 §2.7 `source`): kind + url on the
/// first line, indented `ref:`/`commit:` lines below when present.
fn source_value(source: Option<&Source>) -> String {
    match source {
        None => "(none)".to_string(),
        Some(s) => {
            let kind = s.kind.map(source_kind_label).unwrap_or("(unknown)");
            let url = s.url.as_deref().unwrap_or("");
            let mut out = format!("{kind}   {url}");
            if let Some(git_ref) = &s.git_ref {
                out.push_str(&format!("\nref: {git_ref}"));
            }
            if let Some(commit) = &s.commit {
                out.push_str(&format!("\ncommit: {commit}"));
            }
            out
        }
    }
}

/// The dependency list value: the count on the first line, each edge on its own
/// indented line below (so a long dependency list wraps instead of running off
/// the right edge).
fn deps_value(deps: &[String]) -> String {
    let mut out = format!("{}", deps.len());
    for dep in deps {
        out.push_str(&format!("\n  {dep}"));
    }
    out
}

fn load_type_label(t: LoadType) -> &'static str {
    match t {
        LoadType::Static => "static",
        LoadType::Dynamic => "dynamic",
        LoadType::None => "none",
    }
}

fn origin_label(o: LoadOrigin) -> &'static str {
    match o {
        LoadOrigin::Declared => "declared",
        LoadOrigin::Suggested => "suggested",
        LoadOrigin::Default => "default",
        LoadOrigin::StaticTransitive => "static-transitive",
        LoadOrigin::WhenForced => "when-forced",
        LoadOrigin::None => "none",
    }
}

fn declared_label(d: DeclaredLink) -> &'static str {
    match d {
        DeclaredLink::Static => "static",
        DeclaredLink::Dynamic => "dynamic",
        DeclaredLink::StaticTransitive => "static-transitive",
        DeclaredLink::StaticHard => "static-hard",
    }
}

fn source_kind_label(k: SourceKind) -> &'static str {
    match k {
        SourceKind::Registry => "registry",
        SourceKind::Git => "git",
        SourceKind::Override => "override",
        SourceKind::Path => "path",
        SourceKind::Embedded => "embedded",
    }
}

fn condition_kind_label(k: ConditionKind) -> &'static str {
    match k {
        ConditionKind::Os => "os",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::tree::model::*;

    /// Build a representative package covering every field the card surfaces.
    fn fixture_pkg() -> Package {
        Package {
            id: "org.demo/widget".to_string(),
            group: "org.demo".to_string(),
            name: "widget".to_string(),
            kind: "flow".to_string(),
            version: "1.2.3".to_string(),
            content_hash: Some("abc0123456789".to_string()),
            source: Some(Source {
                kind: Some(SourceKind::Git),
                url: Some("https://example.invalid/widget".to_string()),
                git_ref: Some("main".to_string()),
                commit: Some("deadbeef".to_string()),
            }),
            load: Load {
                load_type: LoadType::Static,
                transitive: true,
                declared: Some(DeclaredLink::Static),
                origin: LoadOrigin::Declared,
                in_static_md: true,
                in_index_md: false,
                boot_path: Some("spec/boot/widget.md".to_string()),
            },
            condition: Condition {
                present: true,
                raw: Some("os == windows".to_string()),
                kind: Some(ConditionKind::Os),
                value: Some("windows".to_string()),
            },
            dependencies: vec!["org.demo/dep-a".to_string(), "org.demo/dep-b".to_string()],
        }
    }

    /// `package_card` surfaces the PROP-036 §2.11 field set: every expected
    /// header is present, and the variable-length source / dependencies values
    /// carry their detail (PROP-037 §8).
    #[test]
    fn package_card_carries_the_expected_field_set() {
        let theme = Theme::default();
        let card = package_card(&fixture_pkg(), &theme);
        let headers: Vec<&str> = card.rows().iter().map(|r| r.header.as_str()).collect();
        for expected in [
            "group",
            "name",
            "version",
            "kind",
            "load type",
            "declared link",
            "transitive",
            "condition",
            "in STATIC.md",
            "in INDEX.md",
            "source",
            "content hash",
            "boot path",
            "dependencies",
        ] {
            assert!(
                headers.contains(&expected),
                "card is missing field {expected:?}"
            );
        }
    }

    /// The source value folds kind + url + ref + commit into one wrapped value;
    /// the dependencies value lists the count then each edge on its own line.
    #[test]
    fn package_card_folds_source_and_dependencies_values() {
        let theme = Theme::default();
        let card = package_card(&fixture_pkg(), &theme);
        let rows: std::collections::HashMap<&str, &str> = card
            .rows()
            .iter()
            .map(|r| (r.header.as_str(), r.value.as_str()))
            .collect();
        let source = rows["source"];
        assert!(source.contains("git"));
        assert!(source.contains("https://example.invalid/widget"));
        assert!(source.contains("ref: main"));
        assert!(source.contains("commit: deadbeef"));

        let deps = rows["dependencies"];
        assert!(deps.starts_with("2"));
        assert!(deps.contains("org.demo/dep-a"));
        assert!(deps.contains("org.demo/dep-b"));
    }

    /// A missing-row card carries the id + status fields only.
    #[test]
    fn missing_row_card_has_id_and_status() {
        let theme = Theme::default();
        let mut card = Card::new(Line::styled(" detail ", theme.title()));
        card.push("id", "org.demo/ghost");
        card.push("status", "not in the lockfile");
        let headers: Vec<&str> = card.rows().iter().map(|r| r.header.as_str()).collect();
        assert_eq!(headers, ["id", "status"]);
    }

    /// `condition_value` formats a present condition with its parsed kind, and a
    /// `(none)` for an absent one.
    #[test]
    fn condition_value_formats_present_and_absent() {
        let present = Condition {
            present: true,
            raw: Some("os == windows".to_string()),
            kind: Some(ConditionKind::Os),
            value: Some("windows".to_string()),
        };
        assert_eq!(condition_value(&present), "os == windows   (os = windows)");

        let absent = Condition::absent();
        assert_eq!(condition_value(&absent), "(none)");
    }
}
