//! The scoping module's own tests: what the defaults observe, what the
//! project's `exclude` removes and reports, and what `[judging] exempt`
//! does and — the law that must never bend — does not do.
//!
//! File-backed submodule of [`super`] so every cell stays inside the
//! AI-Native file budget, the same split `cache/tests` already carries.
//! Nothing moved but the file: each assertion here is the one that stood
//! beside the code it tests.

use super::*;

// Layout note (PROP-052): these scaffolds pin the glob semantics on
// the live tree (`vibevm/vibespecs/`) that `DEFAULT_INCLUDES` names.
// They are this test module's own fixtures (the L2 "tests' own
// scaffolds" exemption), and they flip together with
// `DEFAULT_INCLUDES` — by hand, as at RELAYOUT-PLAN R4 — because
// progress-core is standalone by law (PROP-043 §2) and cannot import
// `vibe_core::layout` (`crates/vibe-core/src/layout.rs`).

#[test]
fn default_includes_observe_both_serialisations() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("vibevm/vibespecs")).expect("mkdir");
    std::fs::write(dir.path().join("vibevm/vibespecs/a.md"), "x").expect("write");
    std::fs::write(dir.path().join("vibevm/vibespecs/b.xml"), "x").expect("write");
    let files = observed_files(dir.path(), &ScopeConfig::default()).expect("enumerate");
    let names: Vec<String> = files.iter().map(|f| rel_str(f)).collect();
    assert!(
        names.contains(&"vibevm/vibespecs/a.md".to_string()),
        "{names:?}"
    );
    assert!(
        names.contains(&"vibevm/vibespecs/b.xml".to_string()),
        "{names:?}"
    );
}

/// A package slot as the campaign meets it: a licence, a derived index,
/// the authored cards beside it, and a doc whose name merely starts
/// like the licence.
fn package_tree() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let slot = dir.path().join("packages/x/v0.1.0");
    std::fs::create_dir_all(slot.join("spec/cards")).expect("mkdir");
    for rel in [
        "LICENSE.md",
        "spec/LICENSE-NOTES.md",
        "spec/cards/INDEX.md",
        "spec/cards/scaffold-a.md",
        "spec/cards/scaffold-b.md",
    ] {
        std::fs::write(slot.join(rel), "# T {#t}\n").expect("write");
    }
    dir
}

fn scan(dir: &tempfile::TempDir, exclude: &[&str]) -> (Vec<String>, ExcludeReport) {
    let cfg = ScopeConfig {
        include: vec!["packages/**/*.md".into()],
        exclude: exclude.iter().map(|s| s.to_string()).collect(),
        ..ScopeConfig::default()
    };
    let (files, report) = observed_files_reported(dir.path(), &cfg).expect("enumerate");
    (files.iter().map(|f| rel_str(f)).collect(), report)
}

#[test]
fn the_licence_leaves_by_name_and_the_notes_beside_it_stay() {
    let dir = package_tree();
    let (files, report) = scan(&dir, &[]);
    assert!(!files.contains(&"packages/x/v0.1.0/LICENSE.md".to_string()));
    assert!(files.contains(&"packages/x/v0.1.0/spec/LICENSE-NOTES.md".to_string()));
    // The name rule is not a config exclusion, so it reports nothing.
    assert_eq!(report.dropped, 0);
    assert!(report.stale.is_empty());
}

#[test]
fn a_config_exclude_drops_the_derived_index_and_keeps_the_cards() {
    let dir = package_tree();
    let (files, report) = scan(&dir, &["packages/**/spec/cards/INDEX.md"]);
    assert!(!files.contains(&"packages/x/v0.1.0/spec/cards/INDEX.md".to_string()));
    assert!(files.contains(&"packages/x/v0.1.0/spec/cards/scaffold-a.md".to_string()));
    assert!(files.contains(&"packages/x/v0.1.0/spec/cards/scaffold-b.md".to_string()));
    assert_eq!(report.dropped, 1);
    assert!(report.stale.is_empty());
}

#[test]
fn an_exclude_matching_nothing_names_itself_and_removes_nothing() {
    let dir = package_tree();
    let (files, report) = scan(&dir, &["packages/**/spec/cards/RETIRED.md"]);
    assert_eq!(files.len(), 4);
    assert_eq!(report.dropped, 0);
    assert_eq!(report.stale, vec!["packages/**/spec/cards/RETIRED.md"]);
}

#[test]
fn no_exclude_key_changes_nothing_and_says_nothing() {
    let dir = package_tree();
    let (files, report) = scan(&dir, &[]);
    let cfg = ScopeConfig {
        include: vec!["packages/**/*.md".into()],
        ..ScopeConfig::default()
    };
    let plain = observed_files(dir.path(), &cfg).expect("enumerate");
    assert_eq!(files.len(), plain.len());
    assert_eq!(report.dropped, 0);
    assert!(report.stale.is_empty());
}

#[test]
fn an_invalid_exclude_glob_is_an_error_naming_the_pattern() {
    let dir = package_tree();
    let cfg = ScopeConfig {
        include: vec!["packages/**/*.md".into()],
        exclude: vec!["packages/a**/x.md".into()],
        ..ScopeConfig::default()
    };
    let err = observed_files_reported(dir.path(), &cfg).expect_err("invalid glob");
    assert!(
        format!("{err:#}").contains("packages/a**/x.md"),
        "error must name the pattern: {err:#}"
    );
}

/// The whole point of the key, and the one law that must never bend:
/// an exemption changes what is JUDGED, never what is OBSERVED. An
/// exempt page is in the corpus, parsed, checked and mapped exactly as
/// before — `exclude` is the key that removes a file, and this is not
/// that key (PROP-057 `##OBS-NOT-JUDGED`).
#[test]
fn a_judging_exemption_leaves_the_corpus_untouched() {
    let dir = package_tree();
    let cfg = ScopeConfig {
        include: vec!["packages/**/*.md".into()],
        judging: JudgingSection {
            exempt: vec!["packages/x/**".into()],
        },
        ..ScopeConfig::default()
    };
    let (files, report) = observed_files_reported(dir.path(), &cfg).expect("enumerate");
    let names: Vec<String> = files.iter().map(|f| rel_str(f)).collect();
    assert!(
        names.contains(&"packages/x/v0.1.0/spec/cards/scaffold-a.md".to_string()),
        "{names:?}"
    );
    assert_eq!(report.dropped, 0, "an exemption drops nothing");

    let exemption = cfg.judging_exemption().expect("compile");
    assert!(exemption.covers(Path::new("packages/x/v0.1.0/spec/cards/scaffold-a.md")));
    assert!(!exemption.covers(Path::new("packages/y/v0.1.0/spec/cards/scaffold-a.md")));
}

#[test]
fn no_judging_key_exempts_nothing() {
    let cfg = ScopeConfig::default();
    let exemption = cfg.judging_exemption().expect("compile");
    assert!(exemption.is_empty());
    assert!(!exemption.covers(Path::new("vibevm/vibepacks/g/n/v0.1.0/a.xml")));
}

#[test]
fn an_invalid_judging_exempt_glob_is_an_error_naming_the_pattern() {
    let err =
        JudgingExemption::compile(&["packages/a**/x.md".to_string()]).expect_err("invalid glob");
    assert!(
        format!("{err:#}").contains("packages/a**/x.md"),
        "error must name the pattern: {err:#}"
    );
}

/// The glob dialect of the key, written down where a second reader can
/// be held to it. The debt is counted today by a stopgap script
/// outside this crate (PROP-047 `##DEBT-MUST-BE-ASKABLE`), and a
/// pattern that meant one thing to the script and another to the verb
/// that replaces it would move the debt silently on the day of the
/// swap.
///
/// It is the dialect `exclude` has had since DRIFT-024, deliberately:
/// one configuration file must not hold two readings of `*`. `**`
/// stands for zero or more whole components, and — the part that
/// surprises — a plain `*` crosses separators too, because the crate's
/// default `MatchOptions` do not require a literal separator. So a
/// pattern is at least as wide as it looks and never narrower, which
/// is the safe direction for an exemption to err only under review:
/// every pattern here is enumerated and reviewed, never a wildcard.
#[test]
fn the_exempt_globs_speak_the_dialect_the_exclude_key_speaks() {
    let exemption = JudgingExemption::compile(&[
        "a/**/c".to_string(),
        "b/**".to_string(),
        "d/*/f".to_string(),
    ])
    .expect("compile");
    assert!(exemption.covers(Path::new("a/c")), "zero components");
    assert!(exemption.covers(Path::new("a/x/y/c")), "many components");
    assert!(!exemption.covers(Path::new("a/x/cx")));
    assert!(exemption.covers(Path::new("b/x/y.xml")));
    assert!(!exemption.covers(Path::new("bx/y.xml")), "a component ends");
    assert!(exemption.covers(Path::new("d/e/f")));
    assert!(
        exemption.covers(Path::new("d/e/x/f")),
        "a plain `*` crosses separators under the default MatchOptions"
    );
}

#[test]
fn the_judging_table_is_read_from_facts_toml() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("facts.toml"),
        "schema = 1\ninclude = [\"a/**/*.xml\"]\n\n\
         [judging]\nexempt = [\"a/docs/**\"]\n",
    )
    .expect("write");
    let cfg = load_config(dir.path()).expect("load");
    let exemption = cfg.judging_exemption().expect("compile");
    assert!(exemption.covers(Path::new("a/docs/page.xml")));
    assert!(!exemption.covers(Path::new("a/specs/page.xml")));
}

#[test]
fn facts_toml_wins_over_the_legacy_progress_spelling() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("facts.toml"),
        "schema = 1\ninclude = [\"facts/**/*.md\"]\n",
    )
    .expect("write facts");
    std::fs::write(
        dir.path().join("progress.toml"),
        "schema = 1\ninclude = [\"legacy/**/*.md\"]\n",
    )
    .expect("write legacy");
    let cfg = load_config(dir.path()).expect("load");
    assert_eq!(cfg.include, vec!["facts/**/*.md"]);
}

#[test]
fn an_absent_exclude_key_parses_to_an_empty_list() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("progress.toml"),
        "schema = 1\ninclude = [\"spec/**/*.md\"]\n",
    )
    .expect("write");
    let cfg = load_config(dir.path()).expect("load");
    assert!(cfg.exclude.is_empty());

    std::fs::write(
        dir.path().join("progress.toml"),
        "schema = 1\ninclude = [\"spec/**/*.md\"]\nexclude = [\"spec/gen/**/*.md\"]\n",
    )
    .expect("write");
    let cfg = load_config(dir.path()).expect("load");
    assert_eq!(cfg.exclude, vec!["spec/gen/**/*.md"]);
}
