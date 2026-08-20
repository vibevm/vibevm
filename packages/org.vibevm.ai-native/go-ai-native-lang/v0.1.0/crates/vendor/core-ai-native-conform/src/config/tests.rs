//! Tests for the v2 config surface (B-029 + B-034): the three-section
//! parse, the census-Q1 defaults, the nine loud tombstones, the unified
//! `ExemptEntry`, the Rust invariant (ported forward), the Go/TS unit
//! enumerators, the six refusals per language, and the vacuous/scope
//! helpers.

use super::*;
use std::collections::BTreeSet;
use std::path::Path;

/// Create `crates/<name>/Cargo.toml` — a crate dir the Rust enumerator
/// recognises under the default `crates/*` root.
fn crate_dir(root: &Path, name: &str) {
    let d = root.join("crates").join(name);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("Cargo.toml"), "[package]\n").unwrap();
}

/// Write `body` as `conform.toml` under `root` and return the
/// `Config::load` error chain (tombstone message, or parse context +
/// source). `{:?}` renders the full anyhow chain so a parse failure
/// exposes its underlying `unknown field` reason.
fn load_err(root: &Path, body: &str) -> String {
    std::fs::write(root.join("conform.toml"), body).unwrap();
    format!(
        "{:?}",
        Config::load(&root.join("conform.toml")).unwrap_err()
    )
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

// === topology / load / parse ===

#[test]
fn load_or_default_detects_topology() {
    let tmp = tempfile::tempdir().unwrap();
    // Single-crate layout → scan the root itself.
    let (cfg, origin) = Config::load_or_default(tmp.path()).unwrap();
    assert_eq!(origin, ConfigOrigin::Defaulted);
    assert_eq!(cfg.rust.roots, ["."]);
    // Workspace layout → scan crates/*.
    std::fs::create_dir_all(tmp.path().join("crates")).unwrap();
    let (cfg, _) = Config::load_or_default(tmp.path()).unwrap();
    assert_eq!(cfg.rust.roots, ["crates/*"]);
    // A real file wins and reports Loaded.
    std::fs::write(tmp.path().join("conform.toml"), "max_file_lines = 500\n").unwrap();
    let (cfg, origin) = Config::load_or_default(tmp.path()).unwrap();
    assert_eq!(origin, ConfigOrigin::Loaded);
    assert_eq!(cfg.max_file_lines, 500);
}

#[test]
fn v2_parses_all_three_sections() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("conform.toml"),
        concat!(
            "max_file_lines = 500\n",
            "[rust]\n",
            "roots = [\"crates/*\"]\n",
            "gated = [\"app\"]\n",
            "[[rust.exempt]]\nunit = \"helper\"\nreason = \"pre-adoption\"\n",
            "[go]\n",
            "roots = [\".\"]\n",
            "gated = [\"cmd/app\"]\n",
            "[[go.exempt]]\nunit = \"cmd/old\"\nreason = \"legacy\"\n",
            "[typescript]\n",
            "roots = [\"src\"]\n",
            "cells_dir = \"src/cells\"\n",
            "gated = [\"auth\"]\n",
            "[[typescript.exempt]]\nunit = \"legacy\"\nreason = \"todo\"\n",
        ),
    )
    .unwrap();
    let cfg = Config::load(&tmp.path().join("conform.toml")).unwrap();
    assert_eq!(cfg.max_file_lines, 500);
    assert_eq!(cfg.rust.gated, ["app"]);
    assert_eq!(cfg.rust.exempt[0].unit, "helper");
    assert_eq!(cfg.go.gated, ["cmd/app"]);
    assert_eq!(cfg.go.exempt[0].unit, "cmd/old");
    assert_eq!(cfg.typescript.gated, ["auth"]);
    assert_eq!(cfg.typescript.exempt[0].unit, "legacy");
    assert_eq!(cfg.typescript.cells_dir.as_deref(), Some("src/cells"));
    // No retired key leaked into the parsed value.
    assert!(cfg.gated_crates.is_none() && cfg.exempt.is_none() && cfg.roots.is_none());
}

#[test]
fn defaults_match_census_q1() {
    let r = RustConfig::default();
    assert_eq!(r.roots, ["crates/*"]);
    assert!(r.skip_dirs.is_empty());
    assert_eq!(r.exclude_substrings, ["/generated/"]);
    assert!(r.gated.is_empty());
    assert!(r.exempt.is_empty());
    assert!(r.gated_pub_doctest.is_empty());
    assert!(r.audit_crates.is_empty());
    assert!(r.env_roots.is_empty());
    assert!(r.registry_file.is_none());
    assert!(r.registry_gated_crate.is_none());
    assert_eq!(Config::default().max_file_lines, 600);
    let g = GoConfig::default();
    assert_eq!(g.roots, ["."]);
    assert!(g.skip_dirs.is_empty());
    assert!(g.gated.is_empty() && g.exempt.is_empty());
    let t = TsConfig::default();
    assert_eq!(t.roots, ["src"]);
    assert!(t.skip_dirs.is_empty());
    assert_eq!(t.seam, "index");
    assert!(t.gated.is_empty() && t.exempt.is_empty());
}

#[test]
fn skip_dirs_are_uniform_per_language_policy() {
    let cfg: Config = toml::from_str(
        "[rust]\nskip_dirs=[\"rust-cache\"]\n\
         [typescript]\nskip_dirs=[\"ts-cache\"]\n\
         [go]\nskip_dirs=[\"go-cache\"]\n",
    )
    .unwrap();
    assert_eq!(cfg.rust.skip_dirs, ["rust-cache"]);
    assert_eq!(cfg.typescript.skip_dirs, ["ts-cache"]);
    assert_eq!(cfg.go.skip_dirs, ["go-cache"]);
}

#[test]
fn rust_floor_disable_parses_step_and_reason() {
    // B-049: `[[rust.floor_disable]]` mirrors the Go/TS slot EXACTLY —
    // a `{step, reason}` table, each disablement carrying its reason so
    // the Rust floor can print it (enforced in its own lane, next slice).
    // A flat `["step"]` list is deliberately NOT accepted: parity means
    // Rust disables a step with a recorded reason, never bare.
    let cfg: Config = toml::from_str(
        "[[rust.floor_disable]]\nstep = \"clippy\"\nreason = \"pinned toolchain lints churn\"\n",
    )
    .unwrap();
    assert_eq!(cfg.rust.floor_disable.len(), 1);
    assert_eq!(cfg.rust.floor_disable[0].step, "clippy");
    assert_eq!(
        cfg.rust.floor_disable[0].reason,
        "pinned toolchain lints churn"
    );
    // Default is empty.
    assert!(RustConfig::default().floor_disable.is_empty());
}

// === tombstones — each retired key names its own move ===

#[test]
fn tombstone_each_retired_key_names_its_move() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    assert!(load_err(root, "roots = [\"crates/*\"]\n").contains("`roots` has moved"));
    let e = load_err(root, "exclude_substrings = [\"/x/\"]\n");
    assert!(
        e.contains("`exclude_substrings` has moved") && e.contains("[rust]"),
        "{e}"
    );
    let e = load_err(root, "gated_crates = [\"app\"]\n");
    assert!(
        e.contains("`gated_crates` has moved") && e.contains("`gated` under `[rust]`"),
        "{e}"
    );
    assert!(
        load_err(root, "gated_pub_doctest = [\"app\"]\n").contains("`gated_pub_doctest` has moved")
    );
    assert!(load_err(root, "audit_crates = [\"app\"]\n").contains("`audit_crates` has moved"));
    assert!(load_err(root, "env_roots = [\"x\"]\n").contains("`env_roots` has moved"));
    assert!(load_err(root, "registry_file = \"x\"\n").contains("`registry_file` has moved"));
    assert!(
        load_err(root, "registry_gated_crate = \"x\"\n")
            .contains("`registry_gated_crate` has moved")
    );
    let e = load_err(root, "[[exempt]]\ncrate = \"x\"\nreason = \"r\"\n");
    assert!(
        e.contains("`[[exempt]]` has moved")
            && e.contains("[[rust.exempt]]")
            && e.contains("`crate` → `unit`"),
        "{e}"
    );
    // An unrelated stray key is STILL serde's generic deny, never a tombstone.
    let e = load_err(root, "frobnicate = 1\n");
    assert!(e.contains("unknown field"), "{e}");
}

#[test]
fn exempt_entry_uses_unit_not_crate() {
    let e: ExemptEntry = toml::from_str("unit = \"app\"\nreason = \"stub\"\n").unwrap();
    assert_eq!(e.unit, "app");
    assert_eq!(e.reason, "stub");
    // The retired `crate` spelling is a foreign key now → deny.
    let err = toml::from_str::<ExemptEntry>("crate = \"app\"\nreason = \"stub\"\n").unwrap_err();
    assert!(err.to_string().contains("unknown field `crate`"), "{err}");
}

// === Rust invariant (ported forward to the [rust] shape) ===

#[test]
fn tree_invariant_catches_each_violation_class() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    crate_dir(root, "app");
    crate_dir(root, "helper");

    // Unclassified on-disk crate.
    let cfg: Config = toml::from_str("[rust]\ngated = [\"app\"]\n").unwrap();
    let err = cfg.validate_against_tree(root).unwrap_err().to_string();
    assert!(
        err.contains("`helper` is neither gated nor exempt"),
        "{err}"
    );
    assert!(err.contains("crate"), "{err}");

    // Phantom listed crate.
    let cfg: Config = toml::from_str("[rust]\ngated = [\"app\", \"helper\", \"ghost\"]\n").unwrap();
    let err = cfg.validate_against_tree(root).unwrap_err().to_string();
    assert!(err.contains("`ghost` is listed"), "{err}");

    // Both gated and exempt.
    let cfg: Config = toml::from_str(
        "[rust]\ngated = [\"app\", \"helper\"]\n\
         [[rust.exempt]]\nunit = \"app\"\nreason = \"x\"\n",
    )
    .unwrap();
    let err = cfg.validate_against_tree(root).unwrap_err().to_string();
    assert!(err.contains("both gated and exempt"), "{err}");

    // Empty reason.
    let cfg: Config = toml::from_str(
        "[rust]\ngated = [\"app\"]\n[[rust.exempt]]\nunit = \"helper\"\nreason = \"  \"\n",
    )
    .unwrap();
    let err = cfg.validate_against_tree(root).unwrap_err().to_string();
    assert!(err.contains("without a recorded reason"), "{err}");

    // A literal root (tooling crate outside crates/) satisfies the
    // listed-name check without a crates/ directory.
    std::fs::create_dir_all(root.join("tooling")).unwrap();
    let cfg: Config = toml::from_str(
        "[rust]\nroots = [\"crates/*\", \"tooling\"]\n\
         gated = [\"app\", \"helper\"]\n\
         [[rust.exempt]]\nunit = \"tooling\"\nreason = \"dev tooling\"\n",
    )
    .unwrap();
    cfg.validate_against_tree(root).unwrap();
}

/// A bare single-crate layout (`roots = ["."]`) gates or exempts the
/// crate under the name the scanner attributes its files to.
#[test]
fn dot_root_names_the_project_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::write(root.join("Cargo.toml"), "[package]\n").unwrap();
    std::fs::create_dir_all(root.join("src")).unwrap();
    let name = root.file_name().unwrap().to_string_lossy().into_owned();

    let gated: Config =
        toml::from_str(&format!("[rust]\nroots = [\".\"]\ngated = [\"{name}\"]\n")).unwrap();
    gated.validate_against_tree(root).unwrap();

    let exempt: Config = toml::from_str(&format!(
        "[rust]\nroots = [\".\"]\n[[rust.exempt]]\nunit = \"{name}\"\nreason = \"pre-adoption\"\n"
    ))
    .unwrap();
    exempt.validate_against_tree(root).unwrap();

    let ghost: Config = toml::from_str("[rust]\nroots = [\".\"]\ngated = [\"ghost\"]\n").unwrap();
    let err = ghost.validate_against_tree(root).unwrap_err().to_string();
    assert!(err.contains("`ghost` is listed"), "{err}");
}

// === Go / TS unit enumerators ===

#[test]
fn go_units_walks_nested_packages_and_respects_excludes() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let go = |rel: &str| {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, "package x\n").unwrap();
    };
    go("a/x.go");
    go("b/c/y.go");
    go("a/skip/d.go"); // excluded substring "/skip/" → dir a/skip has no scanned go
    go("vendor/v.go"); // GO_SKIP_DIRS
    let cfg: GoConfig =
        toml::from_str("roots = [\".\"]\nexclude_substrings = [\"/skip/\"]\n").unwrap();
    assert_eq!(go_units(root, &cfg), set(&["a", "b/c"]));

    // A literal root enumerates relative to itself, not the repo root.
    std::fs::create_dir_all(root.join("internal/cells/alpha")).unwrap();
    std::fs::write(root.join("internal/cells/alpha/a.go"), "package alpha\n").unwrap();
    let cfg: GoConfig = toml::from_str("roots = [\"internal/cells\"]\n").unwrap();
    assert_eq!(go_units(root, &cfg), set(&["alpha"]));

    // Empty roots → no units.
    let cfg: GoConfig = toml::from_str("roots = []\n").unwrap();
    assert!(go_units(root, &cfg).is_empty());
}

#[test]
fn ts_units_lists_cell_subdirs_and_none_is_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    for cell in ["foo", "bar"] {
        let d = root.join("src/cells").join(cell);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("index.ts"), "export {}\n").unwrap();
    }
    let cfg: TsConfig = toml::from_str("cells_dir = \"src/cells\"\n").unwrap();
    assert_eq!(ts_units(root, &cfg), set(&["foo", "bar"]));

    // cells_dir = None → empty set (no cells configured, vacuously green).
    assert!(ts_units(root, &TsConfig::default()).is_empty());
    // A missing cells_dir directory → empty set, not an error.
    let cfg: TsConfig = toml::from_str("cells_dir = \"does/not/exist\"\n").unwrap();
    assert!(ts_units(root, &cfg).is_empty());
}

// === six refusals per language ===

fn go_tree(root: &Path) {
    std::fs::create_dir_all(root.join("a")).unwrap();
    std::fs::write(root.join("a/x.go"), "package a\n").unwrap();
}

fn ts_tree(root: &Path) {
    let d = root.join("cells/a");
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("index.ts"), "export {}\n").unwrap();
}

#[test]
fn go_validator_six_refusals_say_package() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    go_tree(root);

    let cfg: Config = toml::from_str("[go]\nroots = [\".\"]\ngated = [\"a\", \"a\"]\n").unwrap();
    let err = cfg.validate_go_against_tree(root).unwrap_err().to_string();
    assert!(
        err.contains("duplicate package") && err.contains("[go]"),
        "{err}"
    );

    let cfg: Config = toml::from_str(
        "[go]\nroots = [\".\"]\n\
         [[go.exempt]]\nunit = \"a\"\nreason = \"r\"\n\
         [[go.exempt]]\nunit = \"a\"\nreason = \"r\"\n",
    )
    .unwrap();
    let err = cfg.validate_go_against_tree(root).unwrap_err().to_string();
    assert!(
        err.contains("duplicate package") && err.contains("[[go.exempt]]"),
        "{err}"
    );

    let cfg: Config = toml::from_str(
        "[go]\nroots = [\".\"]\ngated = [\"a\"]\n[[go.exempt]]\nunit = \"a\"\nreason = \"r\"\n",
    )
    .unwrap();
    let err = cfg.validate_go_against_tree(root).unwrap_err().to_string();
    assert!(err.contains("packages both gated and exempt"), "{err}");

    let cfg: Config =
        toml::from_str("[go]\nroots = [\".\"]\n[[go.exempt]]\nunit = \"a\"\nreason = \"  \"\n")
            .unwrap();
    let err = cfg.validate_go_against_tree(root).unwrap_err().to_string();
    assert!(err.contains("without a recorded reason"), "{err}");

    let cfg: Config = toml::from_str("[go]\nroots = [\".\"]\n").unwrap();
    let err = cfg.validate_go_against_tree(root).unwrap_err().to_string();
    assert!(
        err.contains("package `a` is neither gated nor exempt"),
        "{err}"
    );

    let cfg: Config =
        toml::from_str("[go]\nroots = [\".\"]\ngated = [\"a\", \"ghost\"]\n").unwrap();
    let err = cfg.validate_go_against_tree(root).unwrap_err().to_string();
    assert!(
        err.contains("`ghost` is listed") && err.contains("no package directory"),
        "{err}"
    );
}

#[test]
fn ts_validator_six_refusals_say_cell() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    ts_tree(root);

    let cfg: Config =
        toml::from_str("[typescript]\ncells_dir = \"cells\"\ngated = [\"a\", \"a\"]\n").unwrap();
    let err = cfg
        .validate_typescript_against_tree(root)
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("duplicate cell") && err.contains("[typescript]"),
        "{err}"
    );

    let cfg: Config = toml::from_str(
        "[typescript]\ncells_dir = \"cells\"\n\
         [[typescript.exempt]]\nunit = \"a\"\nreason = \"r\"\n\
         [[typescript.exempt]]\nunit = \"a\"\nreason = \"r\"\n",
    )
    .unwrap();
    let err = cfg
        .validate_typescript_against_tree(root)
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("duplicate cell") && err.contains("[[typescript.exempt]]"),
        "{err}"
    );

    let cfg: Config = toml::from_str(
        "[typescript]\ncells_dir = \"cells\"\ngated = [\"a\"]\n\
         [[typescript.exempt]]\nunit = \"a\"\nreason = \"r\"\n",
    )
    .unwrap();
    let err = cfg
        .validate_typescript_against_tree(root)
        .unwrap_err()
        .to_string();
    assert!(err.contains("cells both gated and exempt"), "{err}");

    let cfg: Config = toml::from_str(
        "[typescript]\ncells_dir = \"cells\"\n[[typescript.exempt]]\nunit = \"a\"\nreason = \"  \"\n",
    )
    .unwrap();
    let err = cfg
        .validate_typescript_against_tree(root)
        .unwrap_err()
        .to_string();
    assert!(err.contains("without a recorded reason"), "{err}");

    let cfg: Config = toml::from_str("[typescript]\ncells_dir = \"cells\"\n").unwrap();
    let err = cfg
        .validate_typescript_against_tree(root)
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("cell `a` is neither gated nor exempt"),
        "{err}"
    );

    let cfg: Config =
        toml::from_str("[typescript]\ncells_dir = \"cells\"\ngated = [\"a\", \"ghost\"]\n")
            .unwrap();
    let err = cfg
        .validate_typescript_against_tree(root)
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("`ghost` is listed") && err.contains("no cell directory"),
        "{err}"
    );
}

// === vacuous-green + empty-scope helpers ===

#[test]
fn vacuous_and_scope_helpers() {
    use crate::SourceFacts;

    // Rust vacuous (behavioural stability: a gated crate with zero
    // scanned crate_names is returned).
    let cfg: Config = toml::from_str("[rust]\ngated = [\"app\", \"helper\"]\n").unwrap();
    let facts = vec![SourceFacts {
        file: "crates/app/src/lib.rs".into(),
        crate_name: "app".into(),
        facts: vec![],
    }];
    assert_eq!(cfg.vacuously_gated(&facts), vec!["helper".to_string()]);

    // Go vacuous: the enumerator's spelling drives the comparison.
    let scanned = set(&["a"]);
    let gated = vec!["a".to_string(), "b".to_string()];
    assert_eq!(go_vacuously_gated(&gated, &scanned), vec!["b".to_string()]);

    // Go scope: non-empty roots + zero units warns; populated or absent
    // scope does not.
    let go_cfg: GoConfig = toml::from_str("roots = [\".\"]\n").unwrap();
    assert!(!go_scope_warnings(&BTreeSet::new(), &go_cfg).is_empty());
    assert!(go_scope_warnings(&set(&["a"]), &go_cfg).is_empty());
    let go_empty_roots: GoConfig = toml::from_str("roots = []\n").unwrap();
    assert!(go_scope_warnings(&BTreeSet::new(), &go_empty_roots).is_empty());

    // TS vacuous + scope.
    assert_eq!(
        ts_vacuously_gated(&["x".to_string()], &BTreeSet::new()),
        vec!["x".to_string()]
    );
    let ts_cfg: TsConfig = toml::from_str("cells_dir = \"src/cells\"\n").unwrap();
    assert!(!ts_scope_warnings(&BTreeSet::new(), &ts_cfg).is_empty());
    assert!(ts_scope_warnings(&BTreeSet::new(), &TsConfig::default()).is_empty());

    // Rust scope warning: glob roots resolving nowhere warn; a literal
    // `.` root is a crate (not a glob unit) and must NOT warn — the
    // single-crate default layout.
    let tmp = tempfile::tempdir().unwrap();
    let rust_cfg: RustConfig = toml::from_str("roots = [\"nowhere/*\"]\n").unwrap();
    assert!(!rust_scope_warnings(tmp.path(), &rust_cfg).is_empty());
    let literal_cfg: RustConfig = toml::from_str("roots = [\".\"]\n").unwrap();
    std::fs::write(tmp.path().join("Cargo.toml"), "[package]\n").unwrap();
    assert!(rust_scope_warnings(tmp.path(), &literal_cfg).is_empty());
    crate_dir(tmp.path(), "app");
    let glob_cfg: RustConfig = toml::from_str("roots = [\"crates/*\"]\n").unwrap();
    assert!(rust_scope_warnings(tmp.path(), &glob_cfg).is_empty());
}

// === Go default excludes (ported, unchanged) ===

#[test]
fn go_default_excludes_fixtures() {
    let excludes = GoConfig::default().exclude_substrings;
    assert!(
        excludes.iter().any(|s| s == "/fixtures/"),
        "Go default exclude_substrings must contain `/fixtures/`: {excludes:?}"
    );
    assert!(excludes.iter().any(|s| s == "/testdata/"));
    assert!(excludes.iter().any(|s| s == "/vendor/"));
}
