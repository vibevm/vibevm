use std::ffi::OsString;

use super::*;

fn args() -> DocCheckArgs {
    DocCheckArgs {
        examples: false,
        derived: false,
        citations: false,
        translations: false,
        chapters: false,
        json: false,
        media: false,
        coverage: false,
        style: false,
        prompts: false,
        runner: None,
        sample: None,
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
    assert!(e.to_string().contains("--prompts"), "{e}");
    assert!(e.to_string().contains("--chapters"), "{e}");
    assert!(e.to_string().contains("PROP-057#PIPE-LIBRARY"), "{e}");
}

/// The learning-path measurement does not change the exit code, however
/// many links point ahead — the norm's own ruling
/// (`##NAV-CHAPTERS-CHECKED`), because an orientation page points ahead on
/// purpose and a gate here would teach an author to stop writing it.
///
/// The package below declares a two-chapter path and the first page links
/// into the second chapter, so the measurement is not empty; the run still
/// succeeds. It is the one block of `run_check` that never bails, and that
/// is what this pins.
#[test]
fn the_learning_path_measurement_reports_and_never_fails() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let pages = tmp.path().join("vibevm/vibespecs");
    for (document, text) in [
        (
            "start/index",
            "later comes [the model](../model/two-trees.xml)",
        ),
        ("model/two-trees", "one paragraph"),
    ] {
        let path = pages.join(format!("{document}.xml"));
        std::fs::create_dir_all(path.parent().expect("a folder")).expect("mkdir");
        std::fs::write(
            path,
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                 <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
                   <title id=\"root\">T</title>\n  <p>{text}</p>\n\
                 </spec>\n"
            ),
        )
        .expect("write");
    }
    std::fs::write(
        tmp.path().join("vibe.toml"),
        "[package]\nname = \"lib-docs\"\ngroup = \"org.demo\"\nkind = \"doc\"\n\
         version = \"0.1.0\"\ntitle = \"t\"\nabstract = \"a\"\n\n\
         [[documents]]\npackage = \"org.demo/lib\"\nversion = \"^1.0\"\n\n\
         [navigation]\npinned = []\n\n\
         [[navigation.chapter]]\nid = \"start\"\ntitle = \"Getting started\"\n\
         pages = [\"start/index\"]\n\n\
         [[navigation.chapter]]\nid = \"model\"\ntitle = \"How it works\"\n\
         pages = [\"model/two-trees\"]\n",
    )
    .expect("write");

    for json in [false, true] {
        let checked = DocCheckArgs {
            chapters: true,
            json,
            path: tmp.path().to_path_buf(),
            ..args()
        };
        run_check(checked, DocEnv::default())
            .expect("a link pointing ahead is measured, never gated");
    }

    // And the measurement itself is the one the library took, so the
    // surface stays thin: one pair, counted once.
    let report = vibe_doc::chapters::check(tmp.path()).expect("the library measures it");
    assert_eq!(report.forward_link_count, 1);
    assert_eq!(report.forward_links[0].page, "start/index");
    assert_eq!(report.forward_links[0].target, "model/two-trees");
}

/// The prompt check calls a real agent, so a run with no agent to call
/// refuses rather than printing the green nothing «0 prompts run» would
/// be.
#[test]
fn the_prompt_check_reaches_the_library_and_asks_for_an_agent() {
    let tmp = tempfile::tempdir().expect("temp dir");
    std::fs::write(
        tmp.path().join("vibe.toml"),
        "[package]\nname = \"lib-docs\"\ngroup = \"org.demo\"\nkind = \"doc\"\n",
    )
    .expect("write");
    std::fs::write(
        tmp.path().join("prompts.toml"),
        "schema = 1\n\n[doc.prompts]\nfixture = \"empty\"\n",
    )
    .expect("write");
    let checked = DocCheckArgs {
        prompts: true,
        path: tmp.path().to_path_buf(),
        ..args()
    };
    let e = run_check(checked, DocEnv::default()).expect_err("refused");
    assert!(e.to_string().contains("--runner"), "{e}");
    assert!(e.to_string().contains("STYLE-PROMPT-FIRST"), "{e}");
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
