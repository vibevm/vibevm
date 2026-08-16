//! Unit tests for the layout rules — split out of `mod.rs` together with
//! the code they exercise, by the same `#[path]` idiom `postproc.rs` and
//! `vocabulary.rs` use, so the driver keeps no tests of its own and
//! neither file sits against the 600-line budget. One test per behaviour
//! the layout owes its callers: the nested, sorted schema scan; the
//! non-schema skips; the path-mirroring rule; the refusal of a segment
//! that cannot be a module name; and the every-level `mod.rs` tree.

use super::*;
use tempfile::tempdir;

/// Nested schemas must be found at any depth, and the result must be
/// sorted — the walk order a filesystem gives is not guaranteed.
#[test]
fn schemas_under_finds_nested_schemas_in_sorted_order() -> Result<()> {
    let dir = tempdir()?;
    std::fs::write(dir.path().join("a.jtd.json"), "{}")?;
    std::fs::create_dir_all(dir.path().join("sub").join("deep"))?;
    std::fs::write(dir.path().join("sub").join("deep").join("b.jtd.json"), "{}")?;

    let found = schemas_under(dir.path())?;
    assert_eq!(
        found,
        vec![
            dir.path().join("a.jtd.json"),
            dir.path().join("sub").join("deep").join("b.jtd.json"),
        ]
    );
    Ok(())
}

/// Only `*.jtd.json` FILES count: other extensions, backup tails, and a
/// directory merely named like a schema must not be picked up.
#[test]
fn schemas_under_skips_non_schema_entries() -> Result<()> {
    let dir = tempdir()?;
    std::fs::write(dir.path().join("x.json"), "{}")?;
    std::fs::write(dir.path().join("y.jtd.json.bak"), "{}")?;
    std::fs::create_dir_all(dir.path().join("z.jtd.json"))?;

    assert_eq!(schemas_under(dir.path())?, Vec::<PathBuf>::new());
    Ok(())
}

/// The output path mirrors the schema's path relative to its home:
/// root-level schemas keep today's flat layout, nested ones carry
/// their directory (the epoch — PROP-044 §4.6) into the module path.
#[test]
fn schema_module_dir_mirrors_schema_path() -> Result<()> {
    let (root, out) = (Path::new("schemas"), Path::new("generated"));
    assert_eq!(
        schema_module_dir(root, out, &root.join("init_report.jtd.json"))?,
        out.join("init_report")
    );
    assert_eq!(
        schema_module_dir(root, out, &root.join("journal").join("journal.jtd.json"))?,
        out.join("journal").join("journal")
    );
    assert_eq!(
        schema_module_dir(
            root,
            out,
            &root.join("index").join("e1").join("entry.jtd.json")
        )?,
        out.join("index").join("e1").join("entry")
    );
    Ok(())
}

/// Measured before this guard existed: `by-cap.jtd.json` emitted
/// `pub mod by-cap;`, codegen exited 0, and `vibe-wire` failed to parse.
/// The refusal now names the schema, the segment and the rename.
#[test]
fn schema_module_dir_refuses_a_segment_that_is_not_a_module_name() {
    let (root, out) = (Path::new("schemas"), Path::new("generated"));

    let err = schema_module_dir(root, out, &root.join("by-cap.jtd.json"))
        .expect_err("a hyphenated file name cannot be a module");
    let msg = err.to_string();
    assert!(msg.contains("by-cap"), "names the segment: {msg}");
    assert!(msg.contains("by_cap"), "names the fix: {msg}");

    let err = schema_module_dir(
        root,
        out,
        &root.join("index").join("e-1").join("entry.jtd.json"),
    )
    .expect_err("a hyphenated DIRECTORY is just as fatal as a file name");
    assert!(err.to_string().contains("e-1"), "names the directory");

    let err = schema_module_dir(root, out, &root.join("type.jtd.json"))
        .expect_err("a Rust keyword cannot be a module either");
    assert!(
        err.to_string().contains("keyword"),
        "says why, not just that"
    );

    // The legal shapes the six real schemas use stay legal.
    for good in ["by_cap", "by_purl", "entry", "repomd", "journal", "_x9"] {
        schema_module_dir(root, out, &root.join(format!("{good}.jtd.json")))
            .unwrap_or_else(|e| panic!("{good} must be accepted: {e}"));
    }
}

/// The `mod.rs` tree registers every directory from the output root down
/// to each leaf — intermediates included — each with its direct children
/// sorted, so `format_id` stays a child of the top level only.
#[test]
fn module_tree_registers_every_level_with_sorted_children() {
    let out = Path::new("generated");
    let leaves = vec![
        out.join("format_id"),
        out.join("init_report"),
        out.join("index").join("e1").join("entry"),
        out.join("index").join("e1").join("by_name"),
        out.join("journal").join("e1").join("journal"),
    ];
    let tree = module_tree(out, &leaves);

    let children_of = |dir: &Path| -> Vec<String> {
        tree.get(dir)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    };
    assert_eq!(
        children_of(out),
        vec!["format_id", "index", "init_report", "journal"]
    );
    assert_eq!(children_of(&out.join("index")), vec!["e1"]);
    assert_eq!(
        children_of(&out.join("index").join("e1")),
        vec!["by_name", "entry"]
    );
    assert_eq!(
        children_of(&out.join("journal").join("e1")),
        vec!["journal"]
    );
    // Exactly the directories on some root→leaf path carry a `mod.rs`.
    assert_eq!(tree.len(), 5);
}
