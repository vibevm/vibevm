//! The static ASCII tree renderer — the non-tty / `--plain` fallback
//! (PROP-036 §2.11 fallback).
//!
//! Phase 1 ships this deterministic renderer; the interactive TUI is Phase 2
//! (a clean seam is left in [`super::run`]). The DAG is walked from the
//! declared roots, each package shown once and a re-occurrence marked `(*)`
//! and not re-expanded — cycle-guarded on the `group/name` key
//! (PROP-036 §2.12).

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-036#tui");

use std::collections::{BTreeMap, BTreeSet};

use super::model::{LoadType, Package, PackageTree};

/// One rendered row: the drawn name cell plus the value + checkbox columns.
struct Row {
    name: String,
    load: &'static str,
    transitive: bool,
    condition: bool,
    static_md: bool,
}

/// Render the whole tree to a plain, ASCII-safe string (PROP-036 §2.11).
pub fn render(tree: &PackageTree) -> String {
    let by_id: BTreeMap<&str, &Package> =
        tree.packages.iter().map(|p| (p.id.as_str(), p)).collect();

    let mut rows: Vec<Row> = Vec::new();
    let mut expanded: BTreeSet<String> = BTreeSet::new();

    for (i, root) in tree.roots.iter().enumerate() {
        let last = i + 1 == tree.roots.len();
        walk(root, "", last, true, &by_id, &mut expanded, &mut rows);
    }

    // Any package not reached from a declared root (e.g. a drifted lock
    // root) is still shown, so the plain view never hides a package.
    let mut orphans: Vec<&Package> = tree
        .packages
        .iter()
        .filter(|p| !expanded.contains(&p.id))
        .collect();
    orphans.sort_by(|a, b| a.id.cmp(&b.id));

    let mut out = String::new();
    header(tree, &mut out);
    render_rows(&rows, &mut out);

    if !orphans.is_empty() {
        out.push_str("\nnot reached from a declared root:\n");
        let mut orphan_rows: Vec<Row> = Vec::new();
        for (i, p) in orphans.iter().enumerate() {
            let last = i + 1 == orphans.len();
            walk(
                &p.id,
                "",
                last,
                true,
                &by_id,
                &mut expanded,
                &mut orphan_rows,
            );
        }
        render_rows(&orphan_rows, &mut out);
    }

    out
}

/// Depth-first walk producing rows; `(*)`-marks and does not re-expand a
/// package already shown (PROP-036 §2.12).
fn walk(
    id: &str,
    prefix: &str,
    is_last: bool,
    is_root: bool,
    by_id: &BTreeMap<&str, &Package>,
    expanded: &mut BTreeSet<String>,
    rows: &mut Vec<Row>,
) {
    // A top-level root carries no branch glyph; every child gets a `├─`/`└─`
    // connector drawn on top of the accumulated vertical-bar prefix.
    let connector = if is_root {
        ""
    } else if is_last {
        "\u{2514}\u{2500} "
    } else {
        "\u{251c}\u{2500} "
    };

    let Some(pkg) = by_id.get(id) else {
        rows.push(Row {
            name: format!("{prefix}{connector}{id}  (not in lockfile)"),
            load: "?",
            transitive: false,
            condition: false,
            static_md: false,
        });
        return;
    };

    let repeated = expanded.contains(id);
    let marker = if repeated { " (*)" } else { "" };
    rows.push(Row {
        name: format!("{prefix}{connector}{id}{marker}"),
        load: load_label(pkg.load.load_type),
        transitive: pkg.load.transitive,
        condition: pkg.condition.present,
        static_md: pkg.load.in_static_md,
    });
    if repeated {
        return;
    }
    expanded.insert(id.to_string());

    // Children of a root start at column 0 (they own the connector); deeper
    // levels extend the parent's prefix with a vertical bar or blank gutter.
    let child_prefix = if is_root {
        String::new()
    } else if is_last {
        format!("{prefix}   ")
    } else {
        format!("{prefix}\u{2502}  ")
    };
    let deps = &pkg.dependencies;
    for (i, dep) in deps.iter().enumerate() {
        let last = i + 1 == deps.len();
        walk(dep, &child_prefix, last, false, by_id, expanded, rows);
    }
}

/// The effective-load column label.
fn load_label(load: LoadType) -> &'static str {
    match load {
        LoadType::Static => "static",
        LoadType::Dynamic => "dynamic",
        LoadType::None => "none",
    }
}

/// The status header: the column key and the `STATIC.md` size indicator
/// (PROP-036 §2.6).
fn header(tree: &PackageTree, out: &mut String) {
    out.push_str(&format!("project: {}\n", tree.project.root));
    if let Some(lane) = &tree.boot.static_md {
        out.push_str(&format!(
            "STATIC.md: {} bytes, {} lines, {} contribution(s)\n",
            lane.bytes,
            lane.lines,
            lane.contributions.len()
        ));
    } else {
        out.push_str("STATIC.md: (none)\n");
    }
    out.push_str(&format!(
        "packages: {}   roots: {}\n",
        tree.packages.len(),
        tree.roots.len()
    ));
    out.push_str("columns: load  T=transitive  C=condition  S=in STATIC.md\n\n");
}

/// Format the collected rows with an aligned name column.
fn render_rows(rows: &[Row], out: &mut String) {
    let name_width = rows
        .iter()
        .map(|r| display_width(&r.name))
        .max()
        .unwrap_or(0);
    for r in rows {
        let pad = name_width.saturating_sub(display_width(&r.name));
        out.push_str(&r.name);
        for _ in 0..pad {
            out.push(' ');
        }
        out.push_str(&format!(
            "  {:<7}  {}  {}  {}\n",
            r.load,
            checkbox(r.transitive),
            checkbox(r.condition),
            checkbox(r.static_md),
        ));
    }
}

/// A single-character checkbox cell.
fn checkbox(on: bool) -> char {
    if on { 'x' } else { '.' }
}

/// Character width of a name cell — `char` count, so the box-drawing glyphs
/// (each one `char`) align the same as ASCII.
fn display_width(s: &str) -> usize {
    s.chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::tree::model::*;

    fn pkg(id: &str, load: LoadType, transitive: bool, in_static: bool, deps: &[&str]) -> Package {
        let (group, name) = id.split_once('/').unwrap();
        Package {
            id: id.to_string(),
            group: group.to_string(),
            name: name.to_string(),
            kind: "flow".to_string(),
            version: "0.1.0".to_string(),
            content_hash: None,
            source: None,
            load: Load {
                load_type: load,
                transitive,
                declared: None,
                origin: LoadOrigin::Default,
                in_static_md: in_static,
                in_index_md: false,
                boot_path: None,
            },
            condition: Condition::absent(),
            dependencies: deps.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn tree(packages: Vec<Package>, roots: &[&str]) -> PackageTree {
        PackageTree {
            schema_version: SCHEMA_VERSION,
            generated_at: None,
            tool_version: None,
            project: Project {
                root: "/tmp/x".to_string(),
                name: Some("x".to_string()),
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
    fn renders_a_diamond_once_with_a_reoccurrence_marker() {
        // a -> b, a -> c, b -> d, c -> d : `d` shown twice, second `(*)`.
        let packages = vec![
            pkg("g/a", LoadType::None, false, false, &["g/b", "g/c"]),
            pkg("g/b", LoadType::Static, true, true, &["g/d"]),
            pkg("g/c", LoadType::Dynamic, false, false, &["g/d"]),
            pkg("g/d", LoadType::Static, true, true, &[]),
        ];
        let out = render(&tree(packages, &["g/a"]));
        assert!(out.contains("g/d"));
        assert!(out.contains("(*)"), "a re-reached node is marked:\n{out}");
        // The static, transitive, in-STATIC.md node shows all three flags.
        assert!(out.contains("static"));
    }

    #[test]
    fn cycle_does_not_recurse_forever() {
        // a -> b -> a : the back-edge is marked, not re-expanded.
        let packages = vec![
            pkg("g/a", LoadType::None, false, false, &["g/b"]),
            pkg("g/b", LoadType::None, false, false, &["g/a"]),
        ];
        let out = render(&tree(packages, &["g/a"]));
        assert!(out.contains("(*)"));
    }
}
