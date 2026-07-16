//! F6 copy (PROP-037 §10): copy the current tree view to the system clipboard as
//! Markdown. The visible rows keep their tree connectors inside a fenced code
//! block so the structure survives a paste, and the per-package `T`/`C`/`S`
//! flags ride along. `arboard` is the clipboard backend (§10.5). PNG export and
//! the copy-format menu remain reserved (§12).

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-037#copy");

use super::menu::MenuState;
use super::state::{App, RowNode};

/// Render the current view as a Markdown document, plus the package count.
pub fn markdown(app: &App) -> (String, usize) {
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

/// Copy the current view to the clipboard as Markdown; returns a status line
/// for the footer flash (an error line if the clipboard is unavailable).
pub fn copy(app: &App) -> String {
    let (doc, count) = markdown(app);
    match write_clipboard(doc) {
        Ok(()) => format!(
            "\u{2713} copied {count} package{} as Markdown",
            if count == 1 { "" } else { "s" }
        ),
        Err(e) => format!("\u{2717} copy failed: {e}"),
    }
}

fn write_clipboard(text: String) -> Result<(), String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| format!("clipboard unavailable: {e}"))?;
    clipboard.set_text(text).map_err(|e| e.to_string())
}

/// Open the ComingSoon placeholder for PNG export (PROP-037 §10.4 `#copy-png`).
/// PNG export (a rasterized tree image) is **reserved** — selecting PNG opens the
/// [`ComingSoon`] modal until the rasterization (font + image crates) is built
/// (§12 non-goal-for-now). Routed through `app.menu` so the existing F-key menu
/// capture (`↑`/`↓`/`Enter`/`Esc`) drives it.
///
/// [`ComingSoon`]: super::ui::ComingSoon
#[allow(dead_code)] // wired when the copy-settings modal (§10.2) offers PNG as a format.
pub fn png_coming_soon(app: &mut App) {
    app.menu = Some(MenuState::coming_soon("PNG export"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::tree::model::{
        Boot, Condition, HOST_NAMESPACE, IndexLane, Load, LoadOrigin, LoadType, Package,
        PackageTree, Project, SCHEMA_VERSION,
    };

    fn app() -> App {
        let pkg = Package {
            id: "g/alpha".to_string(),
            group: "g".to_string(),
            name: "alpha".to_string(),
            kind: "flow".to_string(),
            version: "0.1.0".to_string(),
            content_hash: None,
            source: None,
            load: Load {
                load_type: LoadType::Static,
                transitive: true,
                declared: None,
                origin: LoadOrigin::Declared,
                in_static_md: true,
                in_index_md: false,
                boot_path: None,
            },
            condition: Condition::absent(),
            dependencies: Vec::new(),
        };
        let tree = PackageTree {
            schema_version: SCHEMA_VERSION,
            generated_at: None,
            tool_version: None,
            project: Project {
                root: "/tmp/x".to_string(),
                name: Some("demo".to_string()),
                is_workspace: false,
                host_namespace: HOST_NAMESPACE.to_string(),
            },
            roots: vec!["g/alpha".to_string()],
            packages: vec![pkg],
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
        };
        App::new(tree)
    }

    #[test]
    fn markdown_lists_the_packages_with_flags_in_a_code_block() {
        let (doc, count) = markdown(&app());
        assert_eq!(count, 1);
        assert!(doc.starts_with("# vibe tree — demo (1 packages)"));
        assert!(doc.contains("```"), "a fenced code block");
        assert!(doc.contains("g/alpha"), "the package id");
        assert!(doc.contains("static"), "the load type");
        assert!(doc.contains("[TS]"), "the transitive + STATIC.md flags");
    }
}
