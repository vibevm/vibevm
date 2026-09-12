//! The runner end to end, over a fixture whose command needs no product
//! build: `cat` is the runner's own, so the whole pipeline — page, collect,
//! fixture recipe, sandbox, capture, normalisation, comparison, accept —
//! is exercised without a compiled binary in the way.

use std::fs;
use std::path::{Path, PathBuf};

use super::{Options, check, slug};
use crate::examples::sandbox::RunnerEnv;

fn env(tmp: &Path) -> RunnerEnv {
    RunnerEnv {
        binary: PathBuf::from("no-binary-needed-for-cat"),
        sandbox_root: tmp.join("sand"),
        repo_root: None,
        user_home: None,
        settings_home: None,
        cargo: PathBuf::from("cargo"),
        timeout_secs: 30,
    }
}

/// A package with one fixture (`greeting`) that lays down one file, and
/// one page whose example `cat`s it.
fn package(tmp: &Path, expect: &str) -> PathBuf {
    let pkg = tmp.join("pkg");
    let fixture = pkg.join("examples/greeting");
    fs::create_dir_all(fixture.join("tree/work")).expect("mkdir");
    fs::write(fixture.join("example.toml"), "schema = 1\n").expect("write recipe");
    fs::write(
        fixture.join("tree/work/note.txt"),
        "hello from the fixture\n",
    )
    .expect("write file");

    let pages = pkg.join("vibevm/vibespecs/start");
    fs::create_dir_all(&pages).expect("mkdir");
    fs::write(
        pages.join("p.xml"),
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
               <title id=\"root\">P</title>\n  \
               <example id=\"note\" fixture=\"greeting\">\n    \
                 <run>cat note.txt</run>\n    \
                 <expect>{expect}</expect>\n  \
               </example>\n\
             </spec>\n"
        ),
    )
    .expect("write page");
    pkg
}

#[test]
fn a_documented_command_that_matches_its_golden_is_green() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let pkg = package(tmp.path(), "hello from the fixture");
    let report = check(&pkg, &env(tmp.path()), &Options::default()).expect("runs");
    assert!(report.ok(), "{}", report.render());
    assert_eq!(report.counts().matched, 1);
}

#[test]
fn a_golden_that_lies_is_red_and_the_diff_says_where() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let pkg = package(tmp.path(), "goodbye from the fixture");
    let report = check(&pkg, &env(tmp.path()), &Options::default()).expect("runs");
    assert!(!report.ok());
    assert_eq!(report.counts().differ, 1);
    let text = report.render();
    assert!(text.contains("- goodbye from the fixture"), "{text}");
    assert!(text.contains("+ hello from the fixture"), "{text}");
}

#[test]
fn accept_fills_an_empty_golden_on_the_page_itself() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let pkg = package(tmp.path(), "");
    let opts = Options {
        accept: true,
        ..Options::default()
    };
    let report = check(&pkg, &env(tmp.path()), &opts).expect("runs");
    assert_eq!(report.counts().captured, 1, "{}", report.render());
    let page = fs::read_to_string(pkg.join("vibevm/vibespecs/start/p.xml")).expect("read");
    assert!(
        page.contains("<expect>hello from the fixture</expect>"),
        "{page}"
    );

    // And the second run, with no flags at all, is now green.
    let again = check(&pkg, &env(tmp.path()), &Options::default()).expect("runs");
    assert!(again.ok(), "{}", again.render());
}

#[test]
fn a_deferred_example_is_skipped_with_its_reason_and_never_red() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let pkg = package(tmp.path(), "this golden is wrong on purpose");
    fs::write(
        pkg.join("examples/deferred.toml"),
        "schema = 1\n\n[[deferred]]\npage = \"start/p.xml\"\nid = \"note\"\n\
         captured = \"release\"\nreason = \"captured from the release distribution\"\n",
    )
    .expect("write");
    let report = check(&pkg, &env(tmp.path()), &Options::default()).expect("runs");
    assert!(report.ok(), "{}", report.render());
    assert_eq!(report.counts().skipped, 1);
    assert!(
        report
            .render()
            .contains("captured from the release distribution")
    );
}

#[test]
fn a_named_fixture_with_nothing_behind_it_fails_that_example_and_no_other() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let pkg = package(tmp.path(), "hello from the fixture");
    let pages = pkg.join("vibevm/vibespecs/start");
    fs::write(
        pages.join("q.xml"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
           <title id=\"root\">Q</title>\n  \
           <example id=\"ghost\" fixture=\"nowhere\">\n    \
             <run>cat note.txt</run>\n    \
             <expect>x</expect>\n  \
           </example>\n\
         </spec>\n",
    )
    .expect("write");
    let report = check(&pkg, &env(tmp.path()), &Options::default()).expect("runs");
    let counts = report.counts();
    assert_eq!(counts.failed, 1, "{}", report.render());
    assert_eq!(counts.matched, 1, "the healthy example still ran");
}

#[test]
fn the_filter_selects_by_page_and_id() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let pkg = package(tmp.path(), "hello from the fixture");
    let opts = Options {
        only: Some("nothing-matches-this".into()),
        ..Options::default()
    };
    let report = check(&pkg, &env(tmp.path()), &opts).expect("runs");
    assert!(report.outcomes.is_empty());
}

#[test]
fn a_run_directory_name_stays_short_because_windows_counts_every_character() {
    let name = slug("lifecycle/build-package-deploy.xml", "deploy-plan");
    assert!(name.len() <= 40, "{name}");
    assert!(name.starts_with("build-package-deploy-deploy-plan"));
    assert!(!name.contains('/') && !name.contains('.'));
}
