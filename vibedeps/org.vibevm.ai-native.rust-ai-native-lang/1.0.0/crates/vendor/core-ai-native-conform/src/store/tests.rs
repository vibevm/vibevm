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

fn write_source(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}

fn fact_files(facts: &[SourceFacts]) -> Vec<&str> {
    facts.iter().map(|facts| facts.file.as_str()).collect()
}

/// TypeScript keeps its ecosystem-wide `node_modules` rule while the
/// consumer adds an unrelated project directory through policy.
#[test]
fn typescript_built_in_and_policy_skip_dirs_compose() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write_source(root, "src/keep.ts", "export const keep = 1;\n");
    write_source(
        root,
        "src/node_modules/dependency.ts",
        "export const dependency = 1;\n",
    );
    write_source(
        root,
        "src/project-cache/generated.ts",
        "export const generated = 1;\n",
    );
    let cfg: Config = toml::from_str(
        "[typescript]\nroots=[\"src\"]\nskip_dirs=[\"project-cache\"]\n\
         exclude_substrings=[]\n",
    )
    .unwrap();

    let mut log = ExtractionLog::default();
    let facts = Store::for_typescript(root, &cfg)
        .extract_typescript(root, &NullFrontend, &mut log)
        .unwrap();
    assert_eq!(fact_files(&facts), ["src/keep.ts"]);
}

/// Go keeps its ecosystem-wide `vendor` rule while the consumer adds an
/// unrelated project directory through policy.
#[test]
fn go_built_in_and_policy_skip_dirs_compose() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write_source(root, "keep.go", "package keep\n");
    write_source(root, "vendor/dependency.go", "package dependency\n");
    write_source(root, "project-cache/generated.go", "package generated\n");
    let cfg: Config = toml::from_str(
        "[go]\nroots=[\".\"]\nskip_dirs=[\"project-cache\"]\n\
         exclude_substrings=[]\n",
    )
    .unwrap();

    let mut log = ExtractionLog::default();
    let facts = Store::for_go(root, &cfg)
        .extract_go(root, &NullFrontend, &mut log)
        .unwrap();
    assert_eq!(fact_files(&facts), ["keep.go"]);
}

/// Red proof for B-064 on the TypeScript walk: without consumer policy
/// `vibedeps` is ordinary source; naming it in policy prunes the tree.
#[test]
fn typescript_vibedeps_is_consumer_policy_not_engine_knowledge() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write_source(root, "src/vibedeps/probe.ts", "export const probe = 1;\n");

    let unconfigured: Config =
        toml::from_str("[typescript]\nroots=[\"src\"]\nskip_dirs=[]\nexclude_substrings=[]\n")
            .unwrap();
    let mut log = ExtractionLog::default();
    let scanned = Store::for_typescript(root, &unconfigured)
        .extract_typescript(root, &NullFrontend, &mut log)
        .unwrap();
    assert_eq!(fact_files(&scanned), ["src/vibedeps/probe.ts"]);

    let configured: Config = toml::from_str(
        "[typescript]\nroots=[\"src\"]\nskip_dirs=[\"vibedeps\"]\n\
         exclude_substrings=[]\n",
    )
    .unwrap();
    let mut log = ExtractionLog::default();
    let skipped = Store::for_typescript(root, &configured)
        .extract_typescript(root, &NullFrontend, &mut log)
        .unwrap();
    assert!(skipped.is_empty(), "consumer policy must skip vibedeps");
}

/// The same red proof at the second former hardcode site, the Go walk.
#[test]
fn go_vibedeps_is_consumer_policy_not_engine_knowledge() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write_source(root, "vibedeps/probe.go", "package probe\n");

    let unconfigured: Config =
        toml::from_str("[go]\nroots=[\".\"]\nskip_dirs=[]\nexclude_substrings=[]\n").unwrap();
    let mut log = ExtractionLog::default();
    let scanned = Store::for_go(root, &unconfigured)
        .extract_go(root, &NullFrontend, &mut log)
        .unwrap();
    assert_eq!(fact_files(&scanned), ["vibedeps/probe.go"]);

    let configured: Config =
        toml::from_str("[go]\nroots=[\".\"]\nskip_dirs=[\"vibedeps\"]\nexclude_substrings=[]\n")
            .unwrap();
    let mut log = ExtractionLog::default();
    let skipped = Store::for_go(root, &configured)
        .extract_go(root, &NullFrontend, &mut log)
        .unwrap();
    assert!(skipped.is_empty(), "consumer policy must skip vibedeps");
}
