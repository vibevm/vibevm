//! The linter against the law's own «before and after» pairs (STYLE.md
//! §9) and the Russian list (§10).
//!
//! The pairs are the fixtures because they are the law's own statement of
//! what it wants: the «before» of each pair is prose the law rejects, the
//! «after» is the prose it asks for, and a linter that cannot tell them
//! apart is not implementing this law. The «after» text is quoted with
//! the glossary links a real page would carry — §9 prints the paragraphs
//! as excerpts, and the link is what makes an excerpt a page.

use super::*;

use std::fs;
use std::path::PathBuf;

use crate::style::report::{Rule, Severity};

/// The glossary the fixture pages are judged against — three terms, all
/// of them words that mean something only in vibe.
const GLOSSARY: &str = "\
  <lock-file title=\"lock file\"><p>The record of which exact versions a project uses.</p></lock-file>\n  \
  <registry title=\"registry\"><p>A place packages are published to.</p></registry>\n  \
  <store title=\"store\"><p>The machine-wide cache of package content.</p></store>\n";

struct Package {
    dir: tempfile::TempDir,
}

impl Package {
    fn new(lang: &str, banned: &str) -> Package {
        let dir = tempfile::tempdir().expect("temp dir");
        fs::write(
            dir.path().join("vibe.toml"),
            format!(
                "[package]\nname = \"m\"\ngroup = \"org.demo\"\nkind = \"doc\"\n\
                 version = \"0.1.0\"\n\n[i18n]\ncanonical = \"{lang}\"\n"
            ),
        )
        .expect("manifest");
        fs::create_dir_all(dir.path().join(banned::STYLE_DIR)).expect("style dir");
        fs::write(banned::list_path(dir.path(), lang), banned).expect("list");
        Package { dir }
    }

    fn page(&self, rel: &str, body: &str) -> &Package {
        let path: PathBuf = self.dir.path().join("vibevm/vibespecs").join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("page dir");
        }
        fs::write(
            &path,
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                 <spec xmlns=\"https://vibevm.org/spec/1\">\n{body}</spec>\n"
            ),
        )
        .expect("page");
        self
    }

    fn glossary(&self) -> &Package {
        self.page(
            "glossary/index.xml",
            &format!("  <title id=\"root\">Glossary</title>\n{GLOSSARY}"),
        )
    }

    fn check(&self) -> Report {
        super::check(self.dir.path(), FULL_STYLE).expect("the check runs")
    }
}

/// The English list, cut to the entries the fixtures exercise. The real
/// list is package data and is read from the package under test; this one
/// keeps the fixture honest about which entry fired.
const EN: &str = "powerful\nflexible\nseamlessly\nstreamlines\nleverage\nat its core\n\
                  empowers\nnavigate the complexities\nrobust\n\
                  see the specification\nrefer to the spec\nas described in\n";

/// STYLE.md §9, the «before» of all four pairs, on one page.
const BAD_PAGE: &str = "  <title id=\"root\">The bad page</title>\n  \
  <p>The resolver materialises the closure declared by `[requires]` into `vibedeps/` once the \
  index feed is reconciled with the lock pins; unison versioning applies across the family to \
  the `-lang` and `-mcp` satellites, whereas `-docs` remains outside unison per PROP-028.</p>\n  \
  <what title=\"What it is\">\n    \
    <p>VibeVM is a powerful, flexible package manager that seamlessly streamlines how you \
    leverage specifications. At its core, it empowers you to navigate the complexities of \
    context engineering.</p>\n    \
    <p>The naming rules for groups are defined in PROP-003; see the specification for \
    details.</p>\n    \
    <p>Publishing involves making sure that you have built the package and that your token is \
    configured, after which the publish command can be run, and you should then verify the \
    result in the registry.</p>\n  \
  </what>\n";

/// STYLE.md §9, the «after» of all four pairs, on one page.
const GOOD_PAGE: &str = "  <title id=\"root\">Installing a dependency</title>\n  \
  <p>This page says what happens when you ask for a dependency, and when you want it.</p>\n  \
  <what title=\"What happens\">\n    \
    <p>When you run `vibe install`, vibe reads the packages your project asks for, finds the \
    exact versions named in your [lock file](../glossary/index.xml#lock-file), and copies them \
    into the `vibedeps/` folder. Packages that belong to one family move together: update the \
    language package, and its MCP companion updates to the same version. Documentation \
    packages are the exception. They keep their own version, so a fix in the manual never \
    forces a new release of the tool.</p>\n    \
    <p>VibeVM installs specifications the way npm installs libraries. You name a package, and \
    the text your agent reads at the start of a session is assembled from it.</p>\n    \
    <p>A group name is a reversed domain, like `org.vibevm.core`: lower-case, dots between the \
    parts, no part starting with a digit.</p>\n    \
    <rule ref=\"spec://org.vibevm.core/vibevm/common/PROP-057#STYLE-LINT\"/>\n  \
  </what>\n  \
  <by-hand title=\"By hand\">\n    \
    <list ordered=\"true\">\n      \
      <item>Build the package: `vibe package`.</item>\n      \
      <item>Check that the token file exists: `ls ~/.vibe/github.publish.token`.</item>\n      \
      <item>Publish: `vibe publish`.</item>\n      \
      <item>Open the package's page in the \
      [registry](../glossary/index.xml#registry) and check the version number.</item>\n    \
    </list>\n  \
  </by-hand>\n";

#[test]
fn the_law_s_own_bad_page_is_caught_on_every_pair() {
    let package = Package::new("en", EN);
    package.glossary().page("model/bad.xml", BAD_PAGE);
    let report = package.check();
    let rules: Vec<Rule> = report.findings.iter().map(|f| f.rule).collect();

    assert!(rules.contains(&Rule::Banned), "the tics: {rules:?}");
    assert!(rules.contains(&Rule::Deferral), "the deferral: {rules:?}");
    assert!(
        rules.contains(&Rule::SentenceLength),
        "the density trap and the procedure in prose: {rules:?}"
    );
    assert!(
        rules.contains(&Rule::TermBeforeIntroduction),
        "the terms nobody introduced: {rules:?}"
    );
    let bad = report
        .pages
        .iter()
        .find(|p| p.page == "model/bad.xml")
        .expect("the page was scored");
    assert!(!bad.clean(), "{bad:?}");
    assert!(!report.ok());
}

#[test]
fn the_law_s_own_good_page_is_clean() {
    let package = Package::new("en", EN);
    package.glossary().page("model/good.xml", GOOD_PAGE);
    let report = package.check();
    let errors: Vec<&Finding> = report
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "{errors:#?}");
    assert!(report.ok(), "{}", report.render());
}

/// §10: the adaptation brings its own list, and the check reads the one
/// the package is written in.
#[test]
fn the_russian_list_judges_the_russian_page() {
    const RU: &str = "является\nданный\nпозволяет\nа также\nстоит отметить\n";
    let package = Package::new("ru", RU);
    package.page(
        "model/ru.xml",
        "  <title id=\"root\">Файл блокировки</title>\n  \
         <p>Эта страница объясняет, что происходит при установке.</p>\n  \
         <s title=\"Как это работает\">\n    \
           <p>Данный файл является записью версий, а также позволяет собрать дерево. \
           Стоит отметить, что он обновляется сам.</p>\n  \
         </s>\n",
    );
    let report = package.check();
    assert_eq!(report.lang, "ru");
    let phrases: Vec<&str> = report
        .findings
        .iter()
        .filter(|f| f.rule == Rule::Banned)
        .map(|f| f.message.as_str())
        .collect();
    assert_eq!(phrases.len(), 5, "{phrases:?}");
    // Cyrillic is prose, not an emoji, and « » is the manual's own
    // typography.
    assert!(
        !report.findings.iter().any(|f| f.rule == Rule::Sign),
        "{:#?}",
        report.findings
    );
}

/// The bar is the share of clean pages, exactly as `--coverage` uses
/// `--min`: a hundred is the standing bar, and a lower one is for a
/// campaign that is still writing the pages.
#[test]
fn a_lower_bar_lets_an_unfinished_corpus_run() {
    let package = Package::new("en", EN);
    package
        .glossary()
        .page("model/bad.xml", BAD_PAGE)
        .page("model/good.xml", GOOD_PAGE);
    let strict = package.check();
    assert!(!strict.ok());
    // Two of the three pages carry no error: the good one and the
    // glossary it is judged against.
    assert_eq!(strict.percent(), 66);
    let lenient = super::check(package.dir.path(), 66).expect("runs");
    assert!(lenient.ok());
}

/// A page that opens with a prompt is a task page, and a task is checked
/// by its asserts.
#[test]
fn a_prompt_without_an_assert_on_a_scenario_page_is_an_error() {
    let package = Package::new("en", EN);
    package.glossary().page(
        "howto/do-it.xml",
        "  <title id=\"root\">Do it</title>\n  \
         <p>This page does the thing you want, and says what you get.</p>\n  \
         <prompt id=\"do-it\" assert=\"none\">Do the thing in this folder.</prompt>\n",
    );
    let report = package.check();
    let found: Vec<&Finding> = report
        .findings
        .iter()
        .filter(|f| f.rule == Rule::PromptWithoutAssert)
        .collect();
    assert_eq!(found.len(), 1, "{:#?}", report.findings);
    assert_eq!(found[0].severity, Severity::Error);
    assert_eq!(found[0].block, "p02");
}

/// An illustrative prompt lower down an explanation page is what
/// `assert="none"` is for, and it is left alone.
#[test]
fn an_illustrative_prompt_on_an_explanation_page_is_left_alone() {
    let package = Package::new("en", EN);
    package.glossary().page(
        "model/idea.xml",
        "  <title id=\"root\">An idea</title>\n  \
         <p>This page explains an idea, and when you want it.</p>\n  \
         <s title=\"Asking for it\">\n    \
           <prompt id=\"ask\" assert=\"none\">Ask for the thing.</prompt>\n  \
         </s>\n",
    );
    assert!(
        !package
            .check()
            .findings
            .iter()
            .any(|f| f.rule == Rule::PromptWithoutAssert)
    );
}

/// An empty list would report every page clean, which is the wrong kind
/// of green.
#[test]
fn a_package_whose_language_has_no_list_is_refused() {
    let dir = tempfile::tempdir().expect("temp dir");
    fs::write(
        dir.path().join("vibe.toml"),
        "[package]\nname = \"m\"\n\n[i18n]\ncanonical = \"fr\"\n",
    )
    .expect("manifest");
    let e = super::check(dir.path(), FULL_STYLE).expect_err("refused");
    assert!(e.to_string().contains("banned.fr.txt"), "{e}");
}
