//! Tests for the fragment view (V7-FRAGMENT-DRIFT). Kept in a sibling file so
//! the capability's implementation stays under the per-file budget; the five
//! §5 cases plus the module-path mirror live here.

use super::*;
use crate::fragment;
use specmap_core::config::Config;
use specmap_core::generated::specmap::CodeItem;
use std::fs;
use std::path::Path;

/// A coordinate used throughout: group `org.demo`, name `demo`.
const COORD: &str = "org.demo/demo";
const URI: &str = "spec://org.demo/demo/D#req-r";

/// An own-tree fixture with one tagged, multi-line item `x::f`. The map is
/// built fresh (the posture `fragment` takes for the own tree), so the item
/// carries a real fingerprint and end line.
fn own_tree() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fs::write(
        root.join("specmap.toml"),
        "namespace = \"demo\"\nscan_roots = [\"crates/*\"]\nspec_roots = [\"spec\"]\n",
    )
    .unwrap();
    fs::create_dir_all(root.join("spec")).unwrap();
    fs::write(
        root.join("spec/D.md"),
        "## The rule {#req-r}\n`req r1`\n\nIt MUST hold.\n",
    )
    .unwrap();
    let src = root.join("crates/x/src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("lib.rs"),
        "#[spec(implements = \"spec://demo/D#req-r\", r = 1)]\npub fn f(x: u32) -> u32 {\n    x + 1\n}\n",
    )
    .unwrap();
    tmp
}

/// An installed-package slot under `<root>/vibedeps/flow-demo/0.1.0/` that
/// carries a real schema-3 map. The carried map is the *checkpoint* a body
/// edit drifts against. Mirrors `foreign::tests::slot_with_map`, kept local so
/// the fragment tests are self-contained.
fn slot_with_map(root: &Path) -> std::path::PathBuf {
    let slot = root.join("vibedeps/flow-demo/0.1.0");
    fs::create_dir_all(&slot).unwrap();
    fs::write(
        slot.join("vibe.toml"),
        "[package]\ngroup = \"org.demo\"\nname = \"demo\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(
        slot.join("specmap.toml"),
        format!("namespace = \"{COORD}\"\nscan_roots = [\"crates/*\"]\nspec_roots = [\"spec\"]\n"),
    )
    .unwrap();
    fs::create_dir_all(slot.join("spec")).unwrap();
    fs::write(
        slot.join("spec/D.md"),
        "## The rule {#req-r}\n`req r1`\n\nIt MUST hold.\n",
    )
    .unwrap();
    let src = slot.join("crates/x/src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("lib.rs"),
        "#[spec(implements = \"spec://org.demo/demo/D#req-r\", r = 1)]\n\
         pub fn f(x: u32) -> u32 {\n    x + 1\n}\n",
    )
    .unwrap();
    let cfg = Config::load(&slot).unwrap().unwrap_or_default();
    let map = specmap_core::index::build(&slot, &cfg);
    fs::write(
        slot.join("package.specmap.json"),
        specmap_core::index::to_canonical_bytes(&map).unwrap(),
    )
    .unwrap();
    slot
}

/// §5 (1): the own-tree fragment is returned and the fingerprint matches — a
/// Same verdict that proves the re-scan seam reproduces the build's hash.
#[test]
fn own_tree_fragment_matches_its_fingerprint() {
    let tmp = own_tree();
    let root = tmp.path();
    let Fragment::Text(text) = fragment(root, "x::f", false).unwrap() else {
        panic!("expected the text form");
    };
    assert!(text.contains("pub fn f"), "the body is shown: {text}");
    assert!(text.contains("x + 1"), "the body is shown: {text}");
    assert!(
        text.contains("fingerprint ok"),
        "fresh build => Same: {text}"
    );
    // The JSON form carries the verdict structurally.
    let Fragment::Json(v) = fragment(root, "x::f", true).unwrap() else {
        panic!("expected the json form");
    };
    assert_eq!(v["drift"]["verdict"], "same");
    assert!(
        v["drift"]["recorded"]
            .as_str()
            .unwrap()
            .starts_with("tok1:")
    );
}

/// §5 (2): an installed package's fragment is returned, from the carried map
/// and the slot's sources, with a Same verdict.
#[test]
fn foreign_fragment_is_returned_from_the_carried_map() {
    let tmp = tempfile::tempdir().unwrap();
    slot_with_map(tmp.path());
    let Fragment::Text(text) = fragment(tmp.path(), URI, false).unwrap() else {
        panic!("expected the text form");
    };
    assert!(text.contains("pub fn f"), "the body is shown: {text}");
    assert!(
        text.contains("source read from the installed package"),
        "carried-map provenance: {text}"
    );
    assert!(text.contains("fingerprint ok"), "in sync => Same: {text}");
}

/// §5 (3, the main test): a body edited after the map was built is noticed —
/// both fingerprints shown, the text still returned.
#[test]
fn edited_body_is_drift_and_the_text_is_still_returned() {
    let tmp = tempfile::tempdir().unwrap();
    let slot = slot_with_map(tmp.path());
    let lib = slot.join("crates/x/src/lib.rs");

    // Before the edit: Same.
    let Fragment::Text(before) = fragment(tmp.path(), URI, false).unwrap() else {
        panic!("expected the text form");
    };
    assert!(before.contains("fingerprint ok"), "{before}");

    // Edit the body in the installed slot — the carried map (the checkpoint)
    // is NOT rebuilt, so its recorded fingerprint now differs.
    fs::write(
        &lib,
        "#[spec(implements = \"spec://org.demo/demo/D#req-r\", r = 1)]\n\
         pub fn f(x: u32) -> u32 {\n    x + 2\n}\n",
    )
    .unwrap();

    let Fragment::Text(after) = fragment(tmp.path(), URI, false).unwrap() else {
        panic!("expected the text form");
    };
    assert!(
        after.contains("DRIFT:"),
        "the drift is noticed and said aloud: {after}"
    );
    assert!(
        after.contains("recorded: tok1:") && after.contains("current:  tok1:"),
        "both fingerprints are shown: {after}"
    );
    // The text is still returned — the current body, with the edit.
    assert!(
        after.contains("x + 2"),
        "the current source is shown: {after}"
    );
}

/// §5 (4): an element with no recorded fingerprint is shown without
/// verification, and that is said aloud (not invented, not silent).
#[test]
fn no_fingerprint_is_shown_without_verification() {
    let tmp = own_tree();
    let root = tmp.path();
    // Hand-craft a map whose one item carries a span but no fingerprint — the
    // shape a non-Rust scanner (Go/TypeScript) produces. Run the shared
    // analyser directly so the public fresh-build path (which always mints a
    // fingerprint) does not mask the case.
    let item = CodeItem {
        symbol: "x::f".to_string(),
        itemKind: "fn".to_string(),
        crateName: "x".to_string(),
        file: "crates/x/src/lib.rs".to_string(),
        line: 1,
        endLine: None,
        fingerprint: None,
    };
    let text = fs::read_to_string(root.join(&item.file)).unwrap();
    let a = analyse(&item, &text).unwrap();
    let rendered = render_text(&a, &item, Source::Fresh);
    assert!(
        rendered.contains("shown without verification"),
        "the missing fingerprint is said aloud: {rendered}"
    );
    assert!(
        rendered.contains("pub fn f"),
        "the body is still shown: {rendered}"
    );
}

/// §5 (5): a range that starts past the end of a shortened file is a clear
/// message, not a panic and not silence. The carried map (the checkpoint)
/// still records the element; the installed source has since shrunk below its
/// start line — so resolution succeeds and the range check fires.
#[test]
fn range_past_end_of_file_is_a_clear_message() {
    let tmp = tempfile::tempdir().unwrap();
    let slot = slot_with_map(tmp.path());
    // The carried map records `x::f` starting at line 1; shrink the installed
    // source to nothing so the recorded start is past the end.
    fs::write(slot.join("crates/x/src/lib.rs"), "").unwrap();
    let err = fragment(tmp.path(), URI, false).expect_err("shortened file => clear error");
    let msg = format!("{err}");
    assert!(
        msg.contains("shorter than when the map was built"),
        "a clear message, not a panic: {msg}"
    );
}

/// `module_path_of` mirrors the scanner's rule on the standard layout.
#[test]
fn module_path_mirrors_the_scanner() {
    assert_eq!(module_path_of("x", "crates/x/src/lib.rs").unwrap(), "x");
    assert_eq!(
        module_path_of("vibe-trace", "crates/vibe-trace/src/fragment.rs").unwrap(),
        "vibe_trace::fragment"
    );
    assert_eq!(
        module_path_of("vibe-workspace", "crates/vibe-workspace/src/lib.rs").unwrap(),
        "vibe_workspace"
    );
    assert_eq!(
        module_path_of("x", "crates/x/tests/cli_e2e.rs").unwrap(),
        "x::tests::cli_e2e"
    );
}
