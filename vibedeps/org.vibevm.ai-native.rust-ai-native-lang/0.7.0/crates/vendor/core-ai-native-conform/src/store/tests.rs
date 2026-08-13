//! Store tests — the single-crate `.`-root naming pin (B-029) and the
//! B-059 exclude-substrings pins (one key, one meaning; dead = loud).

use super::*;
use crate::config::Config;

struct NullFrontend;
impl Frontend for NullFrontend {
    fn id(&self) -> &'static str {
        "null"
    }
    fn version(&self) -> &'static str {
        "0"
    }
    fn extract(&self, _f: &str, _c: &str, _m: &str, _t: &str) -> Vec<Fact> {
        Vec::new()
    }
}

/// A `.` root attributes its files to the project directory's own
/// basename — the scanner half of the single-crate fix (the validator
/// half is pinned in `config.rs`). Before the shared `crate_dir_name`
/// derivation this came out as the empty string, so every crate-keyed
/// rule silently skipped the whole tree.
#[test]
fn dot_root_names_the_project_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src").join("lib.rs"), "pub fn f() {}\n").unwrap();
    let cfg: Config = toml::from_str("[rust]\nroots = [\".\"]\n").unwrap();

    let store = Store::for_rust(root, &cfg);
    let mut log = ExtractionLog::default();
    let facts = store
        .extract_workspace(root, &NullFrontend, &mut log)
        .unwrap();

    let expected = root.file_name().unwrap().to_string_lossy().into_owned();
    assert_eq!(facts.len(), 1, "one source file scanned");
    assert_eq!(facts[0].crate_name, expected);
}

// ---- B-059: `exclude_substrings` — one key, one meaning; dead = loud ----

/// Half 1: an exclude written in the repo-relative space (the space a
/// finding's address lives in) filters the file. Before the fix the check
/// read only the in-crate path (`src/lib.rs`), where `crates/foo/` can
/// never appear — the entry matched nothing and said nothing.
#[test]
fn exclude_matches_repo_relative_path() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("crates/foo/src")).unwrap();
    std::fs::write(root.join("crates/foo/src/lib.rs"), "pub fn f() {}\n").unwrap();
    let cfg: Config =
        toml::from_str("[rust]\nroots=[\"crates/*\"]\nexclude_substrings=[\"crates/foo/\"]\n")
            .unwrap();
    let store = Store::for_rust(root, &cfg);
    let mut log = ExtractionLog::default();
    let facts = store
        .extract_workspace(root, &NullFrontend, &mut log)
        .unwrap();
    assert!(
        facts.is_empty(),
        "repo-relative exclude must filter the file"
    );
    assert!(log.dead_excludes.is_empty(), "a matching entry is not dead");
}

/// Half 2: an exclude substring that matches no file in the whole pass is
/// surfaced — named in the log (and on stderr), no longer silent.
#[test]
fn dead_exclude_is_announced() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("crates/foo/src")).unwrap();
    std::fs::write(root.join("crates/foo/src/lib.rs"), "pub fn f() {}\n").unwrap();
    let cfg: Config = toml::from_str(
        "[rust]\nroots=[\"crates/*\"]\nexclude_substrings=[\"crates/foo/\",\"no-such-path/\"]\n",
    )
    .unwrap();
    let store = Store::for_rust(root, &cfg);
    let mut log = ExtractionLog::default();
    store
        .extract_workspace(root, &NullFrontend, &mut log)
        .unwrap();
    assert_eq!(log.dead_excludes, vec!["no-such-path/".to_string()]);
}

/// Compat: an exclude written in the in-crate space (the old reading)
/// still filters — half 1 adds the repo-relative space, it does not retire
/// the in-crate one. The string is our own real `"/generated/"`; the
/// generated file is dropped either way, the crate root kept.
#[test]
fn in_crate_exclude_still_filters() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("crates/foo/src/generated")).unwrap();
    std::fs::write(
        root.join("crates/foo/src/generated/x.rs"),
        "pub fn g() {}\n",
    )
    .unwrap();
    std::fs::write(root.join("crates/foo/src/lib.rs"), "pub fn f() {}\n").unwrap();
    let cfg: Config =
        toml::from_str("[rust]\nroots=[\"crates/*\"]\nexclude_substrings=[\"/generated/\"]\n")
            .unwrap();
    let store = Store::for_rust(root, &cfg);
    let mut log = ExtractionLog::default();
    let facts = store
        .extract_workspace(root, &NullFrontend, &mut log)
        .unwrap();
    let files: Vec<&str> = facts.iter().map(|f| f.file.as_str()).collect();
    assert_eq!(files, vec!["crates/foo/src/lib.rs"]);
    assert!(log.dead_excludes.is_empty());
}
