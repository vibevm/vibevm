//! The static ASCII tree renderer — the non-tty / `--plain` fallback
//! (PROP-036 §2.11 fallback).
//!
//! Phase 1 ships this deterministic renderer; the interactive TUI is Phase 2
//! (a clean seam is left in [`super::run`]). The DAG is walked from the
//! declared roots, each package shown once and a re-occurrence marked `(*)`
//! and not re-expanded — cycle-guarded on the `group/name` key
//! (PROP-036 §2.12).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-cli/PROP-036#tui");

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
        // A non-trivial admission (friend closure, override — PROP-050
        // ##VIBE-WHY) rides as a suffix; a plain root-edge arrival adds
        // nothing, so the ordinary tree stays as quiet as before.
        name: format!(
            "{prefix}{connector}{id}{marker}{}{}",
            pkg.bridge_suffix(),
            pkg.provenance_suffix
        ),
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

/// The single summary line `--quiet` prints — the plain header's project /
/// packages / roots facts compressed to one line, no tree, no column key
/// (help: "Reduce output to a single summary line").
pub fn summary_line(tree: &PackageTree) -> String {
    format!(
        "project: {}   packages: {}   roots: {}\n",
        tree.project.root,
        tree.packages.len(),
        tree.roots.len()
    )
}

/// The effective-load column label.
fn load_label(load: LoadType) -> &'static str {
    match load {
        LoadType::Static => "static",
        LoadType::Dynamic => "dynamic",
        LoadType::None => "none",
    }
}

/// The status header: the column key and the static-lane size indicator
/// (PROP-036 §2.6). The lane is named by the file this project actually uses —
/// `STATIC.xml` on an XML spec target, `STATIC.md` otherwise — which the model
/// carries (`boot.static_lane_name`); the renderer never re-derives it.
fn header(tree: &PackageTree, out: &mut String) {
    let lane_name = tree.boot.static_lane_name.as_str();
    out.push_str(&format!("project: {}\n", tree.project.root));
    if let Some(lane) = &tree.boot.static_md {
        out.push_str(&format!(
            "{lane_name}: {} bytes, {} lines, {} contribution(s)\n",
            lane.bytes,
            lane.lines,
            lane.contributions.len()
        ));
    } else {
        out.push_str(&format!("{lane_name}: (none)\n"));
    }
    out.push_str(&format!(
        "packages: {}   roots: {}\n",
        tree.packages.len(),
        tree.roots.len()
    ));
    out.push_str(&format!(
        "columns: load  T=transitive  C=condition  S=in {lane_name}\n\n"
    ));
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
            bridge: false,
            authors: Vec::new(),
            upstream: None,
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
            provenance_suffix: String::new(),
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
                self_coord: "org.vibevm.core/vibevm".to_string(),
            },
            roots: roots.iter().map(|s| s.to_string()).collect(),
            packages,
            boot: Boot {
                static_lane_name: vibe_core::layout::STATIC_MD.to_string(),
                static_md: None,
                index_md: IndexLane {
                    present: false,
                    path: vibe_core::machine_json_path(&vibe_core::layout::current_boot_index()),
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

    #[test]
    fn bridge_row_has_a_distinct_upstream_suffix() {
        let mut bridge = pkg("org.bridge/spec-kit", LoadType::None, false, false, &[]);
        bridge.bridge = true;
        bridge.upstream = Some(Upstream {
            describes: Some("pkg:github/github/spec-kit@v1.0.6".to_string()),
            sources: Vec::new(),
        });
        let out = render(&tree(vec![bridge], &["org.bridge/spec-kit"]));
        assert!(
            out.contains("org.bridge/spec-kit [bridge -> pkg:github/github/spec-kit@v1.0.6]"),
            "bridge role and upstream identity are visible:\n{out}"
        );
    }

    #[test]
    fn summary_line_is_one_line_of_header_facts_no_tree() {
        // The diamond fixture: 4 packages, 1 declared root.
        let packages = vec![
            pkg("g/a", LoadType::None, false, false, &["g/b", "g/c"]),
            pkg("g/b", LoadType::Static, true, true, &["g/d"]),
            pkg("g/c", LoadType::Dynamic, false, false, &["g/d"]),
            pkg("g/d", LoadType::Static, true, true, &[]),
        ];
        let line = summary_line(&tree(packages, &["g/a"]));
        assert_eq!(line.matches('\n').count(), 1, "exactly one line: {line:?}");
        assert!(line.contains("project: /tmp/x"), "{line:?}");
        assert!(line.contains("packages: 4"), "{line:?}");
        assert!(line.contains("roots: 1"), "{line:?}");
        // No tree glyphs, no column key, no package rows — the summary is not
        // the tree.
        assert!(!line.contains('├') && !line.contains('└'), "{line:?}");
        assert!(!line.contains("g/a"), "{line:?}");
        assert!(!line.contains("columns:"), "{line:?}");
    }

    /// The header lines of a render, up to and including the blank separator —
    /// the part the lane name governs.
    fn header_of(out: &str) -> String {
        let mut header = String::new();
        for line in out.lines() {
            header.push_str(line);
            header.push('\n');
            if line.is_empty() {
                break;
            }
        }
        header
    }

    /// A Markdown-lane project's header is what it has always been, byte for
    /// byte: the lane-name threading must not move a single character of the
    /// default case (every committed golden depends on it).
    #[test]
    fn a_markdown_lane_header_is_byte_identical_to_the_legacy_text() {
        let out = render(&tree(
            vec![pkg("g/a", LoadType::None, false, false, &[])],
            &["g/a"],
        ));
        assert_eq!(
            header_of(&out),
            "project: /tmp/x\n\
             STATIC.md: (none)\n\
             packages: 1   roots: 1\n\
             columns: load  T=transitive  C=condition  S=in STATIC.md\n\n"
        );
    }

    /// An XML-target project generates `STATIC.xml`, so both places the header
    /// names the lane — the size line and the column legend — say `STATIC.xml`
    /// (PROP-045 ##STATIC-FOLLOWS-THE-TARGET). Nothing else in the header moves.
    #[test]
    fn an_xml_lane_is_named_in_the_size_line_and_the_legend() {
        let mut t = tree(
            vec![pkg("g/a", LoadType::None, false, false, &[])],
            &["g/a"],
        );
        t.boot.static_lane_name = vibe_core::layout::STATIC_XML.to_string();
        let out = render(&t);
        assert_eq!(
            header_of(&out),
            "project: /tmp/x\n\
             STATIC.xml: (none)\n\
             packages: 1   roots: 1\n\
             columns: load  T=transitive  C=condition  S=in STATIC.xml\n\n"
        );
        assert!(
            !out.contains("STATIC.md"),
            "the Markdown spelling must not survive anywhere:\n{out}"
        );
    }

    /// The observed defect: a committed `STATIC.xml` was reported under the
    /// Markdown name together with its real byte/line/contribution counts. The
    /// counts stay; only the name follows the file.
    #[test]
    fn a_present_xml_lane_reports_its_size_under_its_own_name() {
        let mut t = tree(
            vec![pkg("g/a", LoadType::Static, false, true, &[])],
            &["g/a"],
        );
        t.boot.static_lane_name = vibe_core::layout::STATIC_XML.to_string();
        t.boot.static_md = Some(StaticLane {
            present: true,
            path: vibe_core::machine_json_path(&vibe_core::layout::current_boot_static_xml()),
            bytes: 37217,
            lines: 300,
            contributions: Vec::new(),
        });
        let out = render(&t);
        assert!(
            out.contains("STATIC.xml: 37217 bytes, 300 lines, 0 contribution(s)\n"),
            "{out}"
        );
        assert!(!out.contains("STATIC.md"), "{out}");
    }
}
