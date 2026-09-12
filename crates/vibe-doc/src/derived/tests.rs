//! The record, and what it makes visible.

use std::fs;
use std::path::{Path, PathBuf};

use super::{DerivedEnv, RECORD, Verdict, check, read_record};

const COORDINATE: &str = "org.acme/manual";

fn env(repo: &Path) -> DerivedEnv {
    DerivedEnv {
        // No `cli-help` block in this package, so no binary is needed:
        // the schema and manifest generators are pure reads of the tree.
        binary: PathBuf::from("no-binary-needed"),
        repo_root: repo.to_path_buf(),
        coordinate: COORDINATE.into(),
        timeout_secs: 30,
    }
}

/// A package with one page carrying one `manifest-field` block.
fn package(tmp: &Path, field: &str) -> PathBuf {
    let pkg = tmp.join("pkg");
    let pages = pkg.join("vibevm/vibespecs");
    fs::create_dir_all(&pages).expect("mkdir");
    fs::write(
        pkg.join("vibe.toml"),
        "[package]\ngroup = \"org.acme\"\nname = \"manual\"\ntitle = \"A Manual\"\nabstract = \"Short.\"\n",
    )
    .expect("write manifest");
    fs::write(
        pages.join("card.xml"),
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
               <title id=\"root\">Card</title>\n  \
               <derived kind=\"manifest-field\" ref=\"{field}\"/>\n\
             </spec>\n"
        ),
    )
    .expect("write page");
    pkg
}

#[test]
fn a_first_build_reports_every_block_as_new_and_accept_records_it() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let pkg = package(tmp.path(), "title");
    let report = check(&pkg, &env(tmp.path()), false).expect("runs");
    assert_eq!(report.outcomes.len(), 1);
    assert_eq!(report.outcomes[0].verdict, Verdict::New);
    assert!(!report.ok(), "an unrecorded block is not agreement");
    assert!(!pkg.join(RECORD).exists(), "a check writes nothing");

    let accepted = check(&pkg, &env(tmp.path()), true).expect("accepts");
    assert!(accepted.ok());
    assert!(pkg.join(RECORD).is_file());

    let again = check(&pkg, &env(tmp.path()), false).expect("runs");
    assert_eq!(again.outcomes[0].verdict, Verdict::Same);
    assert!(again.ok());
}

#[test]
fn the_record_keeps_a_hash_and_a_size_and_never_the_text() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let pkg = package(tmp.path(), "title");
    check(&pkg, &env(tmp.path()), true).expect("accepts");
    let text = fs::read_to_string(pkg.join(RECORD)).expect("read");
    assert!(
        !text.contains("A Manual"),
        "the derived text must not reach the repository: {text}"
    );
    let record = read_record(&pkg).expect("parses");
    let row = record.derived.values().next().expect("one row");
    assert_eq!(row.bytes, "A Manual".len());
    assert_eq!(row.sha256.len(), 64);
}

#[test]
fn a_product_change_shows_up_as_a_changed_block_and_is_red() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let pkg = package(tmp.path(), "title");
    check(&pkg, &env(tmp.path()), true).expect("accepts");
    fs::write(
        pkg.join("vibe.toml"),
        "[package]\ngroup = \"org.acme\"\nname = \"manual\"\ntitle = \"A Renamed Manual\"\n",
    )
    .expect("rewrite manifest");
    let report = check(&pkg, &env(tmp.path()), false).expect("runs");
    assert_eq!(report.outcomes[0].verdict, Verdict::Changed);
    assert!(!report.ok());
    assert!(report.render().contains("CHANGED"), "{}", report.render());
}

#[test]
fn a_block_that_left_the_pages_is_reported_rather_than_forgotten() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let pkg = package(tmp.path(), "title");
    check(&pkg, &env(tmp.path()), true).expect("accepts");
    fs::write(
        pkg.join("vibevm/vibespecs/card.xml"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
           <title id=\"root\">Card</title>\n\
         </spec>\n",
    )
    .expect("rewrite page");
    let report = check(&pkg, &env(tmp.path()), false).expect("runs");
    assert_eq!(report.outcomes.len(), 1);
    assert_eq!(report.outcomes[0].verdict, Verdict::Vanished);
    assert!(report.render().contains("gone"), "{}", report.render());
}

#[test]
fn a_reference_the_generators_cannot_build_stops_the_check_by_name() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let pkg = package(tmp.path(), "package.nonesuch");
    let e = check(&pkg, &env(tmp.path()), false).expect_err("refused");
    assert!(
        e.to_string().contains("carries no `package.nonesuch`"),
        "{e}"
    );
}
