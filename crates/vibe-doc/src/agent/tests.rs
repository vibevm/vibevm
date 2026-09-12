//! Reading one page by address: what comes back, in which language, and
//! what a bad address gets told.

use std::fs;
use std::path::{Path, PathBuf};

use super::*;

const PAGE: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<spec xmlns=\"https://vibevm.org/spec/1\">\n  \
  <title id=\"root\">Boot lane</title>\n  \
  <p>The boot lane is what a session reads first.</p>\n  \
  <rule ref=\"spec://com.example/host/common/PROP-001#A-RULE\"/>\n\
</spec>\n";

const ADAPTED: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<spec xmlns=\"https://vibevm.org/spec/1\">\n  \
  <title id=\"root\">Загрузочная полоса</title>\n  \
  <p>Загрузочная полоса — это то, что сессия читает первым.</p>\n  \
  <rule ref=\"spec://com.example/host/common/PROP-001#A-RULE\"/>\n\
</spec>\n";

/// A checkout whose in-tree registry holds a documentation package and,
/// optionally, its Russian adaptation.
fn world(tmp: &Path, with_adaptation: bool) -> PathBuf {
    let repo = tmp.join("repo");
    let spec_dir = repo.join("vibevm/vibespecs/common");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(
        repo.join("vibe.toml"),
        "[project]\nname = \"host\"\ngroup = \"com.example\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    fs::write(
        spec_dir.join("PROP-001-the-thing.xml"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
           <title id=\"root\">PROP-001</title>\n  \
           <p><A-RULE fact=\"true\" status=\"spec/done\">The rule says a thing.</A-RULE></p>\n\
         </spec>\n",
    )
    .unwrap();

    write_package(&repo, "thing-docs", "en", PAGE);
    if with_adaptation {
        write_package(&repo, "thing-docs-ru", "ru", ADAPTED);
    }
    repo
}

fn write_package(repo: &Path, name: &str, lang: &str, page: &str) {
    let dir = repo.join(format!("vibevm/vibepacks/com.example/{name}/v0.1.0"));
    fs::create_dir_all(dir.join("vibevm/vibespecs/model")).unwrap();
    fs::write(
        dir.join("vibe.toml"),
        format!(
            "[package]\nname = \"{name}\"\ngroup = \"com.example\"\nkind = \"doc\"\n\
             version = \"0.1.0\"\ntitle = \"Thing\"\nabstract = \"What it covers.\"\n\
             [i18n]\ncanonical = \"{lang}\"\n\
             [[documents]]\npackage = \"com.example/thing\"\nversion = \"^1.0\"\n"
        ),
    )
    .unwrap();
    fs::write(dir.join("vibevm/vibespecs/model/boot-lane.xml"), page).unwrap();
}

fn sources(repo: &Path) -> SpecSources {
    SpecSources::for_checkout(repo, Some("com.example"), "host")
}

const ADDRESS: &str = "spec://com.example/thing-docs/model/boot-lane";

/// The whole tool in one case: an address in, the Markdown projection
/// out, with the cited rule's CURRENT text substituted in and the block
/// numbers a citation uses.
#[test]
fn an_address_returns_the_page_with_its_rules_resolved() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = world(tmp.path(), false);
    let page = read_page(ADDRESS, Format::Md, None, &sources(&repo)).unwrap();

    assert_eq!(page.lang, "en");
    assert_eq!(page.source, Source::InTree);
    assert!(page.text.contains("[p01]"), "{}", page.text);
    assert!(
        page.text.contains("The rule says a thing."),
        "the Markdown carries the rule's text: {}",
        page.text
    );
}

/// The XML projection keeps the ADDRESS instead — which is what an agent
/// that resolves its own citations wants, and cheaper by every rule it
/// does not read.
#[test]
fn the_xml_projection_keeps_the_address_instead_of_the_text() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = world(tmp.path(), false);
    let page = read_page(ADDRESS, Format::Xml, None, &sources(&repo)).unwrap();
    assert!(page.text.contains("PROP-001#A-RULE"), "{}", page.text);
    assert!(
        !page.text.contains("The rule says a thing."),
        "{}",
        page.text
    );
}

/// A language names the adaptation published beside the source, by the
/// naming convention and not by a field.
#[test]
fn a_language_reaches_the_adaptation_published_beside_the_source() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = world(tmp.path(), true);
    let page = read_page(ADDRESS, Format::Md, Some("ru"), &sources(&repo)).unwrap();
    assert_eq!(page.lang, "ru");
    assert!(page.address.contains("thing-docs-ru"), "{}", page.address);
    assert!(page.text.contains("Загрузочная"), "{}", page.text);
}

/// An adaptation this machine does not hold is a fallback, not a
/// failure: the source answers and the reply says which language it is.
/// A reader who asked for Russian and got nothing is worse served than
/// one who got English and was told so.
#[test]
fn an_absent_adaptation_falls_back_and_says_which_language_came_back() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = world(tmp.path(), false);
    let page = read_page(ADDRESS, Format::Md, Some("ru"), &sources(&repo)).unwrap();
    assert_eq!(page.lang, "en");
    assert_eq!(page.address, ADDRESS);
}

/// Asking a translation for its own language does not name
/// `<name>-ru-ru`.
#[test]
fn an_adaptation_asked_for_its_own_language_is_left_alone() {
    assert_eq!(adaptation_of("spec://g/n-ru/p", "ru"), None);
    assert_eq!(
        adaptation_of("spec://g/n/p", "ru").as_deref(),
        Some("spec://g/n-ru/p")
    );
    // A pinned version travels with the name and the tag goes before it.
    assert_eq!(
        adaptation_of("spec://g/n@1.2.3/p", "ru").as_deref(),
        Some("spec://g/n-ru@1.2.3/p")
    );
}

/// An anchor is accepted and ignored: this returns a page, and a
/// fragment that looked like a selector would promise something nothing
/// here does.
#[test]
fn an_anchor_is_accepted_and_the_whole_page_comes_back() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = world(tmp.path(), false);
    let with = read_page(&format!("{ADDRESS}#p01"), Format::Md, None, &sources(&repo)).unwrap();
    let without = read_page(ADDRESS, Format::Md, None, &sources(&repo)).unwrap();
    assert_eq!(with.text, without.text);
}

/// An address no source holds names where it looked, so the reply is
/// actionable rather than «not found».
#[test]
fn an_unreachable_package_says_where_it_was_looked_for() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = world(tmp.path(), false);
    let e = read_page(
        "spec://com.example/nothing-docs/model/boot-lane",
        Format::Md,
        None,
        &sources(&repo),
    )
    .expect_err("no source holds it");
    assert!(e.to_string().contains("no source holds"), "{e}");
    assert!(e.to_string().contains("in-tree"), "{e}");
}
