//! The coverage laws, on trees built here. A gate that only measures
//! one corpus measures a coincidence.

use std::fs;
use std::path::{Path, PathBuf};

use progress_core::parse::parse_document;

use super::*;

const SELF: &str = "com.example.docs/manual";

/// The specification the obligations live in, written in the same
/// spelling the corpus uses: a fact with a whole-unit marker that carries
/// `actionstage` and an audience.
const SPEC: &str = concat!(
    "## Documentation rules {#rules}\n\n",
    "@fact:TELL-USERS A user is told where the store is. ",
    "<status stage=\"spec\" state=\"done\" action=\"continue\" actionstage=\"doc\" audience=\"user\"/>\n\n",
    "@fact:TELL-AUTHORS An author is told what a card carries. ",
    "<status stage=\"spec\" state=\"done\" action=\"continue\" actionstage=\"doc\" audience=\"author\"/>\n\n",
    "@fact:TELL-NOBODY Promised to no one in particular. ",
    "<status stage=\"spec\" state=\"done\" action=\"continue\" actionstage=\"doc\"/>\n\n",
    "@fact:NOT-A-PROMISE Not a documentation promise at all. @status:spec/done\n",
);

/// A tree with one specification document and one documentation package
/// whose single page is `page_body`.
fn world(tmp: &Path, page_body: &str) -> (PathBuf, PathBuf) {
    let repo = tmp.join("repo");
    let spec_dir = repo.join("vibevm/vibespecs/common");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(
        repo.join("vibe.toml"),
        "[project]\nname = \"host\"\ngroup = \"com.example\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    fs::write(spec_dir.join("PROP-001-rules.md"), SPEC).unwrap();

    let doc_pkg = tmp.join("docs");
    let page_dir = doc_pkg.join("vibevm/vibespecs/model");
    fs::create_dir_all(&page_dir).unwrap();
    fs::write(page_dir.join("page.xml"), page_body).unwrap();
    (repo, doc_pkg)
}

fn page(audience: &str, body: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
           <title id=\"root\">Page</title>\n  \
           <status stage=\"doc\" state=\"work\" audience=\"{audience}\"/>\n{body}\
         </spec>\n"
    )
}

fn rule(anchor: &str) -> String {
    format!("  <rule ref=\"spec://com.example/host/common/PROP-001#{anchor}\"/>\n")
}

fn corpus(repo: &Path) -> progress_core::doc::ParsedDoc {
    let rel = "vibevm/vibespecs/common/PROP-001-rules.md";
    parse_document(rel, &fs::read_to_string(repo.join(rel)).unwrap())
}

fn run(repo: &Path, docs: &Path, min: u8) -> Report {
    let sources = SpecSources::for_checkout(repo, Some("com.example"), "host");
    let parsed = corpus(repo);
    let owed = obligations([&parsed]);
    check(docs, SELF, &sources, repo, owed, min).unwrap()
}

/// The whole gate in one test: a promise made to users is closed by a
/// page for users, and the report says so per audience.
#[test]
fn a_page_for_the_same_audience_closes_the_promise() {
    let tmp = tempfile::tempdir().unwrap();
    let (repo, docs) = world(tmp.path(), &page("user", &rule("TELL-USERS")));
    let report = run(&repo, &docs, 100);

    let users = report
        .per_audience
        .iter()
        .find(|t| t.audience == Audience::User)
        .unwrap();
    assert_eq!((users.covered, users.owed), (1, 1), "{:?}", report.gaps);
    let authors = report
        .per_audience
        .iter()
        .find(|t| t.audience == Audience::Author)
        .unwrap();
    assert_eq!((authors.covered, authors.owed), (0, 1));
    assert!(!report.ok(), "one promise of two is still open");
}

/// The audience is half the question. A page that cites the rule but is
/// written for somebody else has not told the people it was promised to,
/// and the report says which pages DO cite it so the repair is one line.
#[test]
fn a_page_for_another_audience_does_not_close_the_promise() {
    let tmp = tempfile::tempdir().unwrap();
    let (repo, docs) = world(tmp.path(), &page("author", &rule("TELL-USERS")));
    let report = run(&repo, &docs, 100);

    let gap = report
        .gaps
        .iter()
        .find(|g| g.audience == Audience::User)
        .expect("the user promise is open");
    assert_eq!(gap.reason, GapReason::WrongAudience);
    assert_eq!(gap.cited_by, vec!["model/page.xml".to_string()]);
    assert!(report.render().contains("cited, but by no page"));
}

/// An address nothing cites reports the other reason, and the two are
/// not the same repair.
#[test]
fn an_uncited_promise_says_that_nothing_cites_it() {
    let tmp = tempfile::tempdir().unwrap();
    let (repo, docs) = world(tmp.path(), &page("user", ""));
    let report = run(&repo, &docs, 100);

    assert!(report.gaps.iter().all(|g| g.reason == GapReason::Uncited));
    assert!(report.gaps.iter().all(|g| g.cited_by.is_empty()));
}

/// `##OBS-COVERAGE-GATE` binds a promise to the people it was made to.
/// A `actionstage="doc"` marker with no audience is the vocabulary's
/// silent default, not a decision, and the gate does not invent an
/// obligation out of it. A fact with no documentation stage is not one
/// either.
#[test]
fn only_a_doc_marker_that_names_an_audience_is_an_obligation() {
    let tmp = tempfile::tempdir().unwrap();
    let (repo, _) = world(tmp.path(), &page("user", ""));
    let parsed = corpus(&repo);
    let owed = obligations([&parsed]);

    let anchors: Vec<&str> = owed.iter().map(|o| o.anchor.as_str()).collect();
    assert_eq!(anchors, vec!["TELL-USERS", "TELL-AUTHORS"]);
    assert_eq!(
        owed[0].address,
        "vibevm/vibespecs/common/PROP-001-rules.md#TELL-USERS"
    );
}

/// The threshold is a flag for a campaign that is still writing, and it
/// moves the verdict without moving the numbers.
#[test]
fn the_minimum_moves_the_verdict_and_not_the_measurement() {
    let tmp = tempfile::tempdir().unwrap();
    let (repo, docs) = world(tmp.path(), &page("user", &rule("TELL-USERS")));

    let strict = run(&repo, &docs, 100);
    let lenient = run(&repo, &docs, 50);
    assert_eq!(strict.percent(), lenient.percent());
    assert!(!strict.ok());
    assert!(lenient.ok());
}

/// A page the pivot refuses cites nothing anybody can read, and unknown
/// is not covered — the run is red whatever the percentage says.
#[test]
fn an_unreadable_page_is_never_a_pass() {
    let tmp = tempfile::tempdir().unwrap();
    let body = format!("{}{}", rule("TELL-USERS"), rule("TELL-AUTHORS"));
    let (repo, docs) = world(tmp.path(), &page("user,author", &body));
    fs::write(
        docs.join("vibevm/vibespecs/model/broken.xml"),
        "<?xml version=\"1.0\"?>\n<spec xmlns=\"https://vibevm.org/spec/1\"><nope/></spec>\n",
    )
    .unwrap();

    let report = run(&repo, &docs, 100);
    assert_eq!(report.percent(), 100);
    assert!(!report.ok(), "an unreadable page is reported, never passed");
    assert!(report.render().contains("unreadable model/broken.xml"));
}

/// A tree that promised nothing is complete. The alternative is a
/// division by zero dressed up as a failure.
#[test]
fn a_corpus_that_promises_nothing_is_fully_covered() {
    let tmp = tempfile::tempdir().unwrap();
    let (repo, docs) = world(tmp.path(), &page("user", ""));
    let sources = SpecSources::for_checkout(&repo, Some("com.example"), "host");
    let report = check(&docs, SELF, &sources, &repo, Vec::new(), 100).unwrap();
    assert_eq!(report.percent(), 100);
    assert!(report.ok());
}
