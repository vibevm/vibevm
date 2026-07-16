//! The copy system (PROP-037 §10 `#copy`): the per-screen copy providers, the
//! F6 copy / Shift+F6 copy-settings flow, and the depth-2 file-destination
//! modal. Split into three files to stay under the 600-line budget:
//! - this file — the markdown providers ([`tree_markdown`] / [`card_markdown`]),
//!   the screen-aware [`copy`] (F6), the clipboard/file backends, and the
//!   confirm logic for both modals ([`confirm_settings`], [`confirm_file_dest_save`]);
//! - [`settings`] — [`CopySettings`] (format + destination radio groups);
//! - [`file_dest`] — [`FileDest`] (the path field + Save/Cancel buttons).
//!
//! ## Depth-2 cascade (boss ruling — not a full ModalStack)
//!
//! The 3-field captive modal surface on [`App`] (`modal_open` / `search` /
//! `menu`) is extended with **two new captive fields** — `copy_settings` and
//! `file_dest` — rather than a wide ModalStack refactor (deferred). The
//! depth-2 behaviour is a **fixed cascade order**: copy-settings is drawn over
//! the base; file-dest is drawn over copy-settings when present ([`super::render`]);
//! and file-dest captures input first when present, else copy-settings
//! ([`super::input`]). This satisfies §6 (a depth-2 stack) for this one flow
//! without the ModalStack migration.
//!
//! [`App`]: super::state::App

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#copy");

pub mod file_dest;
pub mod settings;

use ratatui_core::buffer::Buffer;
use ratatui_core::layout::Rect;
use specmark::spec;

pub use file_dest::FileDest;
pub use settings::{CopyDest, CopyFormat, CopySettings};

use super::menu::MenuState;
use super::state::{App, RowNode};

// --- per-screen copy providers (PROP-037 §10.1 `#copy-providers`) -----------

/// Render the tree view as a Markdown document, plus the package count
/// (PROP-037 §10.1/§10.3 `#copy-markdown`). The visible rows keep their tree
/// connectors inside a fenced code block so the structure survives a paste, and
/// the per-package `T`/`C`/`S` flags ride along. "What I see is what I copy"
/// (§10.1): in Tabs mode [`App::rows`] already holds only the active tab's
/// partition, so this serializes exactly that.
#[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#copy-markdown")]
pub fn tree_markdown(app: &App) -> (String, usize) {
    let mut count = 0usize;
    let mut body = String::new();
    for r in &app.rows {
        match r.node {
            RowNode::Package(_) | RowNode::Missing => {
                count += 1;
                let flags: String = [('T', r.transitive), ('C', r.condition), ('S', r.in_static)]
                    .into_iter()
                    .filter(|(_, on)| *on)
                    .map(|(c, _)| c)
                    .collect();
                let tail = if flags.is_empty() {
                    String::new()
                } else {
                    format!("  [{flags}]")
                };
                body.push_str(&format!("{}   {}{}\n", r.name.trim_end(), r.load, tail));
            }
            RowNode::Subheader | RowNode::Separator => {
                body.push_str(r.name.trim_end());
                body.push('\n');
            }
        }
    }
    let project = app
        .tree
        .project
        .name
        .as_deref()
        .unwrap_or(app.tree.project.host_namespace.as_str());
    let doc = format!("# vibe tree — {project} ({count} packages)\n\n```\n{body}```\n");
    (doc, count)
}

/// The open card's fields serialized as Markdown (PROP-037 §10.1/§10.3): the
/// selected row's id as the `#` heading, then each field as a `**header:**
/// value` block. Reuses the same [`Card`](super::ui::Card) the detail modal
/// builds ([`super::modal::detail_card`]) and serializes it, so the copy is
/// byte-faithful to what is on screen. `None` when no card is open (a
/// separator/subheader selection, or nothing selected).
#[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#copy-providers")]
pub fn card_markdown(app: &App) -> Option<String> {
    let card = super::modal::detail_card(app)?;
    let body = card.to_markdown();
    let heading = app
        .selected_row()
        .map(|r| r.id.as_str())
        .unwrap_or("detail");
    Some(format!("# {heading}\n\n{body}"))
}

/// The Markdown document for the current screen — the card's fields when its
/// modal is open, the tree's markdown otherwise (PROP-037 §10.1 "what I see is
/// what I copy"). The single source for both the F6 clipboard copy and the
/// file-destination Save path.
fn screen_doc(app: &App) -> String {
    if app.modal_open
        && let Some(md) = card_markdown(app)
    {
        return md;
    }
    tree_markdown(app).0
}

// --- F6 copy to clipboard (PROP-037 §10.2 `#copy-flow`) --------------------

/// Copy the current screen to the clipboard as Markdown (PROP-037 §10.2). The
/// F6 entry point: the card's markdown when its modal is open, the tree's
/// markdown otherwise. Returns the footer flash line (a `\u{2713}` confirmation
/// or a `\u{2717}` error if the clipboard is unavailable).
#[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#copy-flow")]
pub fn copy(app: &App) -> String {
    if app.modal_open
        && let Some(md) = card_markdown(app)
    {
        return match write_clipboard(md) {
            Ok(()) => "\u{2713} copied the card as Markdown".to_string(),
            Err(e) => format!("\u{2717} copy failed: {e}"),
        };
    }
    let (doc, count) = tree_markdown(app);
    match write_clipboard(doc) {
        Ok(()) => format!(
            "\u{2713} copied {count} package{} as Markdown",
            if count == 1 { "" } else { "s" }
        ),
        Err(e) => format!("\u{2717} copy failed: {e}"),
    }
}

/// Write `text` to the system clipboard via `arboard` (PROP-037 §10.5).
fn write_clipboard(text: String) -> Result<(), String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| format!("clipboard unavailable: {e}"))?;
    clipboard.set_text(text).map_err(|e| e.to_string())
}

// --- copy-settings confirm (PROP-037 §10.2 `#copy-flow`) -------------------

/// Confirm the copy-settings modal (PROP-037 §10.2). Reads the format +
/// destination, then: PNG → the [`png_coming_soon`] ComingSoon placeholder
/// (§10.4); Markdown + clipboard → copy + flash + close; Markdown + file → push
/// the [`FileDest`] modal over it (the depth-2 cascade). A no-op when no
/// copy-settings modal is open.
#[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#copy-flow")]
pub fn confirm_settings(app: &mut App) {
    let Some(cs) = app.copy_settings.as_ref() else {
        return;
    };
    let format = cs.format_value();
    let dest = cs.dest_value();
    match (format, dest) {
        (CopyFormat::Png, _) => {
            // PNG is reserved (§10.4) — close settings and show the placeholder.
            app.copy_settings = None;
            png_coming_soon(app);
        }
        (CopyFormat::Markdown, CopyDest::Clipboard) => {
            app.copy_settings = None;
            app.flash = Some(copy(app));
        }
        (CopyFormat::Markdown, CopyDest::File) => {
            // Keep copy_settings open; push FileDest over it (depth-2 cascade).
            app.file_dest = Some(FileDest::new());
        }
    }
}

// --- file-destination confirm (PROP-037 §10.5 `#copy-dest`) ----------------

/// Save the screen's markdown to the file-dest path (PROP-037 §10.5). On
/// success: flash + close both file-dest and copy-settings (the depth-2 stack
/// unwinds). On error: flash the error and keep file-dest open so the user can
/// fix the path. A no-op when no file-dest modal is open.
#[spec(implements = "spec://vibevm/modules/vibe-cli/PROP-037#copy-dest")]
pub fn confirm_file_dest_save(app: &mut App) {
    let Some(path) = app.file_dest.as_ref().map(|fd| fd.path().to_string()) else {
        return;
    };
    let doc = screen_doc(app);
    match write_file(&path, &doc) {
        Ok(()) => {
            app.flash = Some(format!("\u{2713} saved to {path}"));
            app.file_dest = None;
            app.copy_settings = None;
        }
        Err(e) => {
            app.flash = Some(format!("\u{2717} save failed: {e}"));
        }
    }
}

/// Write `content` to `path` (PROP-037 §10.5). Writes to a sibling `.tmp` file
/// then renames over the destination for crash-safety; falls back to a direct
/// write when the rename cannot replace (e.g. a cross-volume or locked target
/// on Windows). The path is plain text for now (§10.5 — validation/picker is a
/// later REQ).
fn write_file(path: &str, content: &str) -> Result<(), String> {
    let tmp = format!("{path}.tmp");
    std::fs::write(&tmp, content).map_err(|e| format!("{path}: {e}"))?;
    match std::fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(_) => {
            // Rename failed (destination exists + locked, or cross-volume):
            // clean up the temp and fall back to a direct write.
            let _ = std::fs::remove_file(&tmp);
            std::fs::write(path, content).map_err(|e| format!("{path}: {e}"))
        }
    }
}

// --- PNG export → ComingSoon (PROP-037 §10.4 `#copy-png`) ------------------

/// Open the ComingSoon placeholder for PNG export (PROP-037 §10.4). PNG export
/// (a rasterized tree image) is **reserved** — selecting PNG in the
/// copy-settings modal routes here until the rasterization (font + image
/// crates) is built (§12 non-goal-for-now). Routed through `app.menu` so the
/// existing F-key menu capture (`↑`/`↓`/`Enter`/`Esc`) drives it.
pub fn png_coming_soon(app: &mut App) {
    app.menu = Some(MenuState::coming_soon("PNG export"));
}

// --- render wrappers (drawn by `super::render` in the depth-2 order) --------

/// Draw the copy-settings modal centred over `area`, if open. A no-op
/// otherwise (PROP-037 §10.2).
pub(super) fn render_settings(area: Rect, buf: &mut Buffer, app: &App) {
    if let Some(cs) = app.copy_settings.as_ref() {
        cs.render(area, buf, &app.theme);
    }
}

/// Draw the file-destination modal centred over `area`, if open. A no-op
/// otherwise (PROP-037 §10.5). Drawn over copy-settings by [`super::render`].
pub(super) fn render_file_dest(area: Rect, buf: &mut Buffer, app: &App) {
    if let Some(fd) = app.file_dest.as_ref() {
        fd.render(area, buf, &app.theme);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::tree::model::{
        Boot, Condition, HOST_NAMESPACE, IndexLane, Load, LoadOrigin, LoadType, Package,
        PackageTree, Project, SCHEMA_VERSION,
    };
    use crate::commands::tree::tui::state::DisplayMode;

    fn pkg_static(id: &str) -> Package {
        let (group, name) = id.split_once('/').unwrap_or(("g", id));
        Package {
            id: id.to_string(),
            group: group.to_string(),
            name: name.to_string(),
            kind: "flow".to_string(),
            version: "0.1.0".to_string(),
            content_hash: None,
            source: None,
            load: Load {
                load_type: LoadType::Static,
                transitive: false,
                declared: None,
                origin: LoadOrigin::Declared,
                in_static_md: true,
                in_index_md: false,
                boot_path: None,
            },
            condition: Condition::absent(),
            dependencies: Vec::new(),
        }
    }

    fn pkg_none(id: &str) -> Package {
        let (group, name) = id.split_once('/').unwrap_or(("g", id));
        Package {
            id: id.to_string(),
            group: group.to_string(),
            name: name.to_string(),
            kind: "flow".to_string(),
            version: "0.1.0".to_string(),
            content_hash: None,
            source: None,
            load: Load {
                load_type: LoadType::None,
                transitive: false,
                declared: None,
                origin: LoadOrigin::None,
                in_static_md: false,
                in_index_md: false,
                boot_path: None,
            },
            condition: Condition::absent(),
            dependencies: Vec::new(),
        }
    }

    fn tree(packages: Vec<Package>, roots: &[&str]) -> PackageTree {
        PackageTree {
            schema_version: SCHEMA_VERSION,
            generated_at: None,
            tool_version: None,
            project: Project {
                root: "/tmp/x".to_string(),
                name: Some("demo".to_string()),
                is_workspace: false,
                host_namespace: HOST_NAMESPACE.to_string(),
            },
            roots: roots.iter().map(|s| s.to_string()).collect(),
            packages,
            boot: Boot {
                static_md: None,
                index_md: IndexLane {
                    present: false,
                    path: "spec/boot/INDEX.md".to_string(),
                    static_pointer: None,
                    entries: Vec::new(),
                },
            },
            in_place_specs: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    #[test]
    fn tree_markdown_lists_the_packages_with_flags_in_a_code_block() {
        let app = App::new(tree(vec![pkg_static("g/alpha")], &["g/alpha"]));
        let (doc, count) = tree_markdown(&app);
        assert_eq!(count, 1);
        assert!(doc.starts_with("# vibe tree — demo (1 packages)"));
        assert!(doc.contains("```"), "a fenced code block");
        assert!(doc.contains("g/alpha"), "the package id");
        assert!(doc.contains("static"), "the load type");
        assert!(doc.contains("[S]"), "the STATIC.md flag");
    }

    #[test]
    fn tree_markdown_honours_the_active_tab_in_tabs_mode() {
        // Two packages in different partitions: `g/static` is static-load,
        // `g/ghost` is none-load (lands in the no-boot tab). Both are declared
        // roots so neither becomes an orphan (the flatten orphan pass keys off
        // declared roots, not the partition filter — see `modes::tests`).
        let mut app = App::new(tree(
            vec![pkg_static("g/static"), pkg_none("g/ghost")],
            &["g/static", "g/ghost"],
        ));
        app.display_mode = DisplayMode::Tabs;
        app.rebuild();
        // Tab 0 (static-first) holds the static partition.
        app.tab = 0;
        app.rebuild();
        let (doc0, _) = tree_markdown(&app);
        assert!(
            doc0.contains("g/static"),
            "the static tab shows the static package"
        );
        assert!(
            !doc0.contains("g/ghost"),
            "the static tab does NOT show the none-load package"
        );
        // Move to the none/no-boot tab (index 2: static, dynamic, none).
        app.tab = 2;
        app.rebuild();
        let (doc2, _) = tree_markdown(&app);
        assert!(
            doc2.contains("g/ghost"),
            "the no-boot tab shows the none-load package"
        );
        assert!(
            !doc2.contains("g/static"),
            "the no-boot tab does NOT show the static package"
        );
    }

    #[test]
    fn card_markdown_serializes_the_open_card_fields() {
        let mut app = App::new(tree(vec![pkg_static("g/alpha")], &["g/alpha"]));
        app.table.select(Some(0));
        app.modal_open = true;
        let md = card_markdown(&app).expect("the card is open");
        assert!(
            md.starts_with("# g/alpha\n"),
            "the package id is the heading"
        );
        // The same field set the detail modal renders (PROP-036 §2.11).
        for header in ["**group:**", "**name:**", "**version:**", "**kind:**"] {
            assert!(
                md.contains(header),
                "the card markdown carries the {header} field"
            );
        }
        assert!(md.contains("g/alpha"), "the group/name value rides along");
    }

    #[test]
    fn card_markdown_is_none_when_no_card_is_open() {
        // A separator selection carries no card.
        let mut app = App::new(tree(vec![pkg_static("g/alpha")], &["g/alpha"]));
        // Force a non-package selection by deselecting.
        app.table.select(None);
        assert!(card_markdown(&app).is_none());
    }

    #[test]
    fn confirm_settings_markdown_file_pushes_file_dest() {
        let mut app = App::new(tree(vec![pkg_static("g/alpha")], &["g/alpha"]));
        let mut cs = CopySettings::new();
        // Set format=Markdown (already), dest=File.
        cs.focus_next(); // focus → destination
        cs.select_down(); // Clipboard → File
        cs.focus_next(); // back to format (arbitrary)
        app.copy_settings = Some(cs);
        confirm_settings(&mut app);
        assert!(
            app.copy_settings.is_some(),
            "copy-settings stays open (depth-2 base)"
        );
        assert!(app.file_dest.is_some(), "file-dest is pushed over it");
    }

    #[test]
    fn confirm_settings_png_routes_to_coming_soon() {
        let mut app = App::new(tree(vec![pkg_static("g/alpha")], &["g/alpha"]));
        let mut cs = CopySettings::new();
        cs.select_down(); // Markdown → PNG (format group focused)
        app.copy_settings = Some(cs);
        confirm_settings(&mut app);
        assert!(app.copy_settings.is_none(), "copy-settings closes");
        assert!(app.menu.is_some(), "the ComingSoon menu opens");
    }

    #[test]
    fn confirm_settings_markdown_clipboard_copies_and_closes() {
        let mut app = App::new(tree(vec![pkg_static("g/alpha")], &["g/alpha"]));
        app.copy_settings = Some(CopySettings::new()); // Markdown + Clipboard
        confirm_settings(&mut app);
        assert!(app.copy_settings.is_none(), "copy-settings closes");
        assert!(app.flash.is_some(), "a flash confirmation is set");
    }
}
