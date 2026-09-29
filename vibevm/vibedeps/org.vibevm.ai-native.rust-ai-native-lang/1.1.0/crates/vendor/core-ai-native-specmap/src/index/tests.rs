//! Index-builder unit tests, out-of-line per the file-length budget (the
//! `mdspec/tests.rs` pattern). Included via `#[cfg(test)] mod tests;`, so
//! `use super::*` is unchanged from the inline form.

use super::*;

/// A small synthetic tree — several anchored units across two docs plus
/// one tagged code item. Enough to exercise the inventory, the ordering
/// invariant, and the edge-from-code path without assuming any particular
/// host repository: specmap-core ships in the rust-ai-native package now,
/// so `CARGO_MANIFEST_DIR` is no longer a substantial spec tree.
fn synthetic_tree() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("vibevm/vibespecs/modules")).unwrap();
    std::fs::write(
        root.join("vibevm/vibespecs/A.md"),
        "## Alpha {#alpha}\n`prop r1`\n\nbody\n\n## Beta {#beta}\n\nbody\n",
    )
    .unwrap();
    std::fs::write(
        root.join("vibevm/vibespecs/modules/B.md"),
        "## Gamma {#gamma}\n`req r1`\n\nbody\n",
    )
    .unwrap();
    let src = root.join("crates/x/src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        src.join("lib.rs"),
        "#[spec(implements = \"spec://project/A#alpha\", r = 1)]\npub fn f() {}\n",
    )
    .unwrap();
    tmp
}

/// PROP-014 §2.5: determinism is a tested property — index twice, assert
/// byte-identical.
#[test]
fn index_is_deterministic() {
    let tmp = synthetic_tree();
    let a = to_canonical_bytes(&build(tmp.path(), &Config::default())).unwrap();
    let b = to_canonical_bytes(&build(tmp.path(), &Config::default())).unwrap();
    assert_eq!(a, b);
    assert!(a.ends_with('\n'));
}

#[test]
fn node_inventory_is_ordered_and_house_style() {
    let tmp = synthetic_tree();
    let map = build(tmp.path(), &Config::default());
    assert!(map.specUnits.len() >= 3, "got {}", map.specUnits.len());
    // Ordering invariant: (doc_path, line) non-decreasing.
    assert!(
        map.specUnits
            .windows(2)
            .all(|w| (&w[0].docPath, w[0].line) <= (&w[1].docPath, w[1].line))
    );
    // House-style URIs: no `spec/` prefix, no `.md`.
    assert!(
        map.specUnits
            .iter()
            .all(|u| !u.uri.starts_with("spec://project/spec/") && !u.uri.contains(".md#"))
    );
    // The tagged code item produced its edge into the spec unit.
    assert!(
        map.edges.iter().any(|e| e.uri == "spec://project/A#alpha"),
        "expected an edge into spec://project/A#alpha"
    );
}

/// End-to-end over a synthetic tree: suspects, dangling edges and
/// pin-ahead warnings all surface.
#[test]
fn suspects_dangling_and_pin_ahead_are_detected() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("vibevm/vibespecs")).unwrap();
    std::fs::write(
        root.join("vibevm/vibespecs/T.md"),
        "## The contract {#req-t}\n`req r2`\n\nIt MUST hold.\n",
    )
    .unwrap();
    let src_dir = root.join("crates/x/src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(
        src_dir.join("lib.rs"),
        r#"
#[spec(implements = "spec://project/T#req-t", r = 1)]
pub fn stale() {}

#[spec(implements = "spec://project/T#req-t", r = 2)]
pub fn current() {}

#[spec(implements = "spec://project/T#req-t", r = 3)]
pub fn ahead() {}

#[spec(implements = "spec://project/T#req-missing", r = 1)]
pub fn dangling() {}
"#,
    )
    .unwrap();

    let map = build(root, &Config::default());
    assert_eq!(map.specUnits.len(), 1);
    assert_eq!(map.specUnits[0].uri, "spec://project/T#req-t");
    assert_eq!(map.edges.len(), 4);
    assert_eq!(map.suspects.len(), 1, "exactly the r1 pin is suspect");
    assert_eq!(map.suspects[0].fromSymbol, "x::stale");
    assert_eq!(map.suspects[0].pinnedR, 1);
    assert_eq!(map.suspects[0].currentR, 2);
    let codes: Vec<&str> = map.warnings.iter().map(|w| w.code.as_str()).collect();
    assert!(codes.contains(&"dangling-edge"), "{codes:?}");
    assert!(codes.contains(&"pin-ahead-of-unit"), "{codes:?}");
}

/// PROP-014 §7.1: an edge into an installed package's unit resolves
/// through `[[external_specs]]` — no dangling warning, suspects work —
/// while the external unit itself stays OUT of the serialised index.
#[test]
fn external_specs_resolve_edges_without_entering_the_index() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("vibevm/vibespecs")).unwrap();
    let ext = root.join("vibedeps/some-flow/0.3.0/spec/mechanisms");
    std::fs::create_dir_all(&ext).unwrap();
    std::fs::write(
        ext.join("ENGINE-X-v0.1.md"),
        "## Rules {#rules}\n`req r2`\n\nbody\n",
    )
    .unwrap();
    let src_dir = root.join("crates/x/src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(
        src_dir.join("lib.rs"),
        r#"
#[spec(implements = "spec://some-flow/mechanisms/ENGINE-X-v0.1#rules", r = 1)]
pub fn stale_pin_into_external() {}

#[spec(implements = "spec://some-flow/mechanisms/MISSING#rules", r = 1)]
pub fn dangling_even_with_externals() {}
"#,
    )
    .unwrap();
    let cfg = Config {
        external_specs: vec![crate::config::ExternalSpec {
            namespace: "some-flow".into(),
            root: "vibedeps/some-flow/0.3.0/spec".into(),
        }],
        ..Config::default()
    };
    let map = build(root, &cfg);
    // The external unit is resolution-only: not inventoried.
    assert!(
        map.specUnits.is_empty(),
        "external units leaked into the index: {}",
        map.specUnits.len()
    );
    // The resolved edge dangles no more — and its stale pin is a suspect.
    let codes: Vec<&str> = map.warnings.iter().map(|w| w.code.as_str()).collect();
    assert_eq!(
        codes.iter().filter(|c| **c == "dangling-edge").count(),
        1,
        "only the truly-missing target dangles: {codes:?}"
    );
    assert_eq!(map.suspects.len(), 1);
    assert_eq!(map.suspects[0].pinnedR, 1);
    assert_eq!(map.suspects[0].currentR, 2);
}

#[test]
fn drift_classification_reports_bumps_and_unbumped_hashes() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("vibevm/vibespecs")).unwrap();
    let src_dir = root.join("crates/x/src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(
        src_dir.join("lib.rs"),
        "#[spec(implements = \"spec://project/T#req-t\", r = 1)]\npub fn f() {}\n",
    )
    .unwrap();

    std::fs::write(
        root.join("vibevm/vibespecs/T.md"),
        "## C {#req-t}\n`req r1`\n\nIt MUST hold.\n",
    )
    .unwrap();
    let old = build(root, &Config::default());

    // (b) editorial change, no bump → unbumped-hash.
    std::fs::write(
        root.join("vibevm/vibespecs/T.md"),
        "## C {#req-t}\n`req r1`\n\nIt MUST always hold.\n",
    )
    .unwrap();
    let edited = build(root, &Config::default());
    let report = classify_drift(&old, &edited);
    assert!(
        report.iter().any(|l| l.starts_with("unbumped-hash:")),
        "{report:?}"
    );

    // (a) semantic change + bump → revision bump + suspect listing.
    std::fs::write(
        root.join("vibevm/vibespecs/T.md"),
        "## C {#req-t}\n`req r2`\n\nIt MUST hold, monotonically.\n",
    )
    .unwrap();
    let bumped = build(root, &Config::default());
    let report = classify_drift(&old, &bumped);
    assert!(
        report.iter().any(|l| l.starts_with("revision bump:")),
        "{report:?}"
    );
    assert!(
        report.iter().any(|l| l.contains("now SUSPECT")),
        "{report:?}"
    );
    assert_eq!(bumped.suspects.len(), 1);
}

/// Render a warning slice one-per-line (the generated `Warning` has no
/// `Debug`, so tests format it themselves).
fn warn_lines(w: &[Warning]) -> String {
    w.iter()
        .map(|x| format!("{}:{} [{}] {}", x.file, x.line, x.code, x.message))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Helper: a spec doc with `n` anchored units `a1..aN`, each `req r1`.
fn spec_doc_with_units(n: usize) -> String {
    let mut s = String::new();
    for k in 1..=n {
        s.push_str(&format!("## a{k} {{#a{k}}}\n`req r1`\n\nbody\n\n"));
    }
    s
}

/// §3.4: at the start threshold 3, an item reaching 3 distinct targets
/// fires (inclusive boundary); one with 2 stays silent.
#[test]
fn overloaded_item_fires_inclusively_and_is_silent_below() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("vibevm/vibespecs")).unwrap();
    std::fs::write(root.join("vibevm/vibespecs/T.md"), spec_doc_with_units(4)).unwrap();
    let src = root.join("crates/x/src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        src.join("lib.rs"),
        "#[spec(implements = \"spec://project/T#a1\", r = 1)]\n\
         #[spec(implements = \"spec://project/T#a2\", r = 1)]\n\
         #[spec(implements = \"spec://project/T#a3\", r = 1)]\n\
         pub fn overloaded() {}\n\
         #[spec(implements = \"spec://project/T#a1\", r = 1)]\n\
         #[spec(implements = \"spec://project/T#a2\", r = 1)]\n\
         pub fn ok() {}\n",
    )
    .unwrap();
    let map = build(root, &Config::default());
    let ov: Vec<&Warning> = map
        .warnings
        .iter()
        .filter(|w| w.code == "overloaded-item")
        .collect();
    assert_eq!(
        ov.len(),
        1,
        "exactly the 3-target item fires:\n{}",
        warn_lines(&map.warnings)
    );
    assert!(ov[0].message.contains("x::overloaded"), "{}", ov[0].message);
    assert!(ov[0].message.contains("3 distinct"), "{}", ov[0].message);
}

/// §3.4: two edges of different verbs into the SAME target count as one
/// connection, not two — so a dual-verb single target does not fire.
#[test]
fn two_verbs_into_one_target_count_as_one_connection() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("vibevm/vibespecs")).unwrap();
    std::fs::write(root.join("vibevm/vibespecs/T.md"), spec_doc_with_units(1)).unwrap();
    let src = root.join("crates/x/src");
    std::fs::create_dir_all(&src).unwrap();
    // implements + verifies the one point → two edges, one connection.
    std::fs::write(
        src.join("lib.rs"),
        "#[spec(implements = \"spec://project/T#a1\", r = 1)]\n\
         #[verifies(\"spec://project/T#a1\", r = 1)]\n\
         pub fn dual() {}\n",
    )
    .unwrap();
    let map = build(root, &Config::default());
    let n = map
        .warnings
        .iter()
        .filter(|w| w.code == "overloaded-item")
        .count();
    assert_eq!(
        n,
        0,
        "one distinct target must not fire at threshold 3:\n{}",
        warn_lines(&map.warnings)
    );
}

/// §3.1: `0` disables the check — a heavily-linked item stays silent.
#[test]
fn overloaded_item_disabled_at_zero() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("vibevm/vibespecs")).unwrap();
    std::fs::write(root.join("vibevm/vibespecs/T.md"), spec_doc_with_units(5)).unwrap();
    let src = root.join("crates/x/src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        src.join("lib.rs"),
        "#[spec(implements = \"spec://project/T#a1\", r = 1)]\n\
         #[spec(implements = \"spec://project/T#a2\", r = 1)]\n\
         #[spec(implements = \"spec://project/T#a3\", r = 1)]\n\
         #[spec(implements = \"spec://project/T#a4\", r = 1)]\n\
         #[spec(implements = \"spec://project/T#a5\", r = 1)]\n\
         pub fn big() {}\n",
    )
    .unwrap();
    let cfg = Config {
        max_connections_per_item: 0,
        ..Config::default()
    };
    let map = build(root, &cfg);
    let n = map
        .warnings
        .iter()
        .filter(|w| w.code == "overloaded-item")
        .count();
    assert_eq!(n, 0, "threshold 0 disables the check");
}

/// §4/§5: two overloaded items produce a byte-identical index across
/// builds, and their order follows the global (file, line) warning sort.
#[test]
fn overloaded_warnings_are_deterministically_ordered() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("vibevm/vibespecs")).unwrap();
    std::fs::write(root.join("vibevm/vibespecs/T.md"), spec_doc_with_units(6)).unwrap();
    let src = root.join("crates/x/src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        src.join("lib.rs"),
        "#[spec(implements = \"spec://project/T#a1\", r = 1)]\n\
         #[spec(implements = \"spec://project/T#a2\", r = 1)]\n\
         #[spec(implements = \"spec://project/T#a3\", r = 1)]\n\
         pub fn first() {}\n\
         #[spec(implements = \"spec://project/T#a4\", r = 1)]\n\
         #[spec(implements = \"spec://project/T#a5\", r = 1)]\n\
         #[spec(implements = \"spec://project/T#a6\", r = 1)]\n\
         pub fn second() {}\n",
    )
    .unwrap();
    let a = build(root, &Config::default());
    let b = build(root, &Config::default());
    // Byte-identical ⇒ the warning order is deterministic.
    assert_eq!(
        to_canonical_bytes(&a).unwrap(),
        to_canonical_bytes(&b).unwrap()
    );
    let ov: Vec<&Warning> = a
        .warnings
        .iter()
        .filter(|w| w.code == "overloaded-item")
        .collect();
    assert_eq!(
        ov.len(),
        2,
        "both items are overloaded:\n{}",
        warn_lines(&a.warnings)
    );
    // Same file ⇒ ordered by line: first before second.
    assert!(ov[0].line < ov[1].line, "sorted by (file, line)");
    assert!(ov[0].message.contains("x::first"));
    assert!(ov[1].message.contains("x::second"));
}

/// Refinement #3 (composition): the default scanner set — Rust plus JTD —
/// is byte-stable against the Rust-only scan when `schema_roots` is empty.
/// The JTD scanner walks zero roots and contributes nothing, so the
/// committed `specmap.json` is identical, proved by byte comparison (not by
/// assertion). This is the regression gate: a project with no schema roots
/// changes by not one byte through the seam.
#[test]
fn empty_schema_roots_is_byte_stable_against_rust_only() {
    let tmp = synthetic_tree();
    let cfg = Config::default(); // schema_roots empty
    let rust_only = build_with_scanner(tmp.path(), &cfg, &crate::scanner::RustScanner);
    let default = build_with_scanner(tmp.path(), &cfg, &crate::scanner::DefaultScanner::new());
    assert_eq!(
        to_canonical_bytes(&rust_only).unwrap(),
        to_canonical_bytes(&default).unwrap(),
        "empty schema_roots must reproduce the Rust-only index byte for byte"
    );
}
