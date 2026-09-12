use super::*;

fn args() -> DocCheckArgs {
    DocCheckArgs {
        examples: false,
        derived: false,
        citations: false,
        translations: false,
        media: false,
        coverage: false,
        style: false,
        min: vibe_doc::coverage::FULL_COVERAGE,
        accept: false,
        force: false,
        only: None,
        path: PathBuf::from("."),
        binary: None,
        sandbox: None,
        timeout: 300,
    }
}

#[test]
fn a_check_with_no_check_named_says_which_ones_exist() {
    let e = run_check(args(), DocEnv::default()).expect_err("refused");
    assert!(e.to_string().contains("--examples"), "{e}");
    assert!(e.to_string().contains("--translations"), "{e}");
    assert!(e.to_string().contains("--coverage"), "{e}");
    assert!(e.to_string().contains("--style"), "{e}");
    assert!(e.to_string().contains("PROP-057#PIPE-LIBRARY"), "{e}");
}

/// The linter gates by PAGE and shares `--min` with the coverage
/// gate: a hundred is the standing bar, and a lower one is for a
/// campaign still writing the pages.
#[test]
fn the_style_check_reaches_the_library_and_reports_the_bar() {
    let tmp = tempfile::tempdir().expect("temp dir");
    std::fs::write(
        tmp.path().join("vibe.toml"),
        "[package]\nname = \"lib-docs\"\ngroup = \"org.demo\"\nkind = \"doc\"\n",
    )
    .expect("write");
    let style = DocCheckArgs {
        style: true,
        path: tmp.path().to_path_buf(),
        ..args()
    };
    // No list for the package's language: the check refuses rather
    // than reporting the green nothing an empty list would print.
    let e = run_check(style, DocEnv::default()).expect_err("refused");
    assert!(e.to_string().contains("banned.en.txt"), "{e}");
    assert!(e.to_string().contains("STYLE-LINT"), "{e}");
}

/// The gate measures a corpus, and a run with no tree to read one
/// from has none. It refuses rather than reporting the green nothing
/// an empty obligation list would print — «zero promises found» and
/// «zero promises» are the same number and not the same fact.
#[test]
fn coverage_without_a_tree_refuses_instead_of_reporting_nothing() {
    let tmp = tempfile::tempdir().expect("temp dir");
    std::fs::write(
        tmp.path().join("vibe.toml"),
        "[package]\nname = \"lib-docs\"\ngroup = \"org.demo\"\nkind = \"doc\"\n",
    )
    .expect("write");
    let checked = DocCheckArgs {
        coverage: true,
        path: tmp.path().to_path_buf(),
        ..args()
    };
    let e = run_check(checked, DocEnv::default()).expect_err("refused");
    assert!(e.to_string().contains("OBS-COVERAGE-GATE"), "{e}");
    assert!(e.to_string().contains("facts.toml"), "{e}");
}

/// A package that adapts nothing passes `--translations` and says
/// why. The surface is thin on purpose: the verdict, the wording and
/// the exit code all come from the library, and this proves the flag
/// reaches it.
#[test]
fn the_translations_check_is_green_on_a_source_documentation() {
    let tmp = tempfile::tempdir().expect("temp dir");
    std::fs::write(
        tmp.path().join("vibe.toml"),
        "[package]\nname = \"lib-docs\"\ngroup = \"org.demo\"\nkind = \"doc\"\n",
    )
    .expect("write");
    let checked = DocCheckArgs {
        translations: true,
        path: tmp.path().to_path_buf(),
        ..args()
    };
    run_check(checked, DocEnv::default()).expect("nothing to mirror is a green state");
}

#[test]
fn the_real_home_is_the_relocation_variable_when_the_operator_set_one() {
    let picked = settings_home(
        &Some(OsString::from("/tmp/elsewhere")),
        &Some(OsString::from("/home/u")),
    );
    assert_eq!(picked, Some(PathBuf::from("/tmp/elsewhere")));
}

#[test]
fn without_a_relocation_the_guarded_home_is_dot_vibe_under_the_user() {
    let picked = settings_home(&None, &Some(OsString::from("/home/u")));
    assert_eq!(picked, Some(PathBuf::from("/home/u/.vibe")));
}

#[test]
fn with_no_home_at_all_the_tripwire_simply_has_no_home_half() {
    assert_eq!(settings_home(&None, &None), None);
}

/// The composition root NAMES the four citation sources; the library
/// discovers none of them. Without a working directory there is no
/// checkout — the local reader's own situation.
#[test]
fn without_a_working_directory_the_citation_world_has_no_checkout() {
    let world = spec_sources(&None, Some(Path::new("/home/u/.vibe")));
    assert!(!world.has_checkout());
}

/// A directory that carries no manifest answers to no coordinate,
/// which is a state, not a failure.
#[test]
fn a_directory_with_no_manifest_answers_to_no_coordinate() {
    let tmp = tempfile::tempdir().expect("temp dir");
    assert_eq!(self_coordinate(tmp.path()), (None, String::new()));
}

/// The project's own coordinate is what a `spec://` address must
/// carry to reach its authored specs.
#[test]
fn the_checkout_coordinate_is_read_from_the_project_table() {
    let tmp = tempfile::tempdir().expect("temp dir");
    std::fs::write(
        tmp.path().join("vibe.toml"),
        "[project]\nname = \"vibevm\"\ngroup = \"org.vibevm.core\"\n",
    )
    .expect("write");
    assert_eq!(
        self_coordinate(tmp.path()),
        (Some("org.vibevm.core".to_string()), "vibevm".to_string())
    );
}
