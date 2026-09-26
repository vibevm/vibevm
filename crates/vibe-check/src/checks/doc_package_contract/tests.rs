//! Unit tests for [`super`], out-of-line per the file-length budget.

use std::fs;
use std::path::Path;

use tempfile::tempdir;

use crate::test_support::opts;
use crate::{CheckId, CheckReport, Severity, check_project};

/// A complete, legal `doc` package — the shape every negative case here
/// varies by exactly one line. `title` and `abstract` are present
/// because without them the manifest does not parse at all (A2.4), and a
/// cell that never runs proves nothing.
fn write_doc_package(root: &Path, extra: &str) {
    fs::write(
        root.join("vibe.toml"),
        format!(
            r#"[package]
group = "org.vibevm.core"
name = "vibevm-docs"
kind = "doc"
version = "0.1.0"
title = "VibeVM Manual"
abstract = "What it covers, for whom, what it assumes known, what it leaves out."

[[documents]]
package = "org.vibevm.core/vibevm"
version = "^1.0"
{extra}"#
        ),
    )
    .unwrap();
    fs::write(root.join("README.md"), "# VibeVM Manual\n").unwrap();
}

fn findings(root: &Path) -> CheckReport {
    check_project(root, &opts())
}

fn contract_findings(report: &CheckReport) -> Vec<&crate::Finding> {
    report
        .findings
        .iter()
        .filter(|f| f.check == CheckId::DocPackageContract)
        .collect()
}

#[test]
fn a_complete_doc_package_passes_clean() {
    let project = tempdir().unwrap();
    write_doc_package(project.path(), "");
    let report = findings(project.path());
    assert!(
        contract_findings(&report).is_empty(),
        "a complete doc package owes nothing: {:?}",
        report.findings
    );
}

/// Documentation is read, not run. Each forbidden section is its own
/// finding, so an author who declared two is told about both rather than
/// led through one recheck per section.
#[test]
fn every_executable_section_is_its_own_error() {
    let cases = [
        (
            "[boot_snippet]\nsource = \"boot/10-docs.md\"\n",
            "[boot_snippet]",
        ),
        (
            "[[binary]]\nname = \"reader\"\ncrate = \"crates/reader\"\n",
            "[[binary]]",
        ),
    ];
    for (toml, section) in cases {
        let project = tempdir().unwrap();
        write_doc_package(project.path(), toml);
        let report = findings(project.path());
        let hit = contract_findings(&report)
            .into_iter()
            .find(|f| f.severity == Severity::Error && f.message.contains(section));
        assert!(
            hit.is_some(),
            "`{section}` in a doc package must be an error: {:?}",
            report.findings
        );
        assert!(
            hit.unwrap().message.contains("KIND-DOC-MUST-NOT-EXECUTE"),
            "the refusal cites the rule it enforces"
        );
    }

    // Two at once: two findings, not one.
    let project = tempdir().unwrap();
    write_doc_package(
        project.path(),
        "[boot_snippet]\nsource = \"boot/10-docs.md\"\n\n\
         [[binary]]\nname = \"reader\"\ncrate = \"crates/reader\"\n",
    );
    let report = findings(project.path());
    assert_eq!(
        contract_findings(&report)
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .count(),
        2,
        "each forbidden section speaks for itself: {:?}",
        report.findings
    );
}

/// The third forbidden section never reaches this cell, and that is the
/// correct outcome rather than a gap: `[[mcp_server]]` outside an `mcp`
/// package is refused by the manifest grammar itself (PROP-027), so a
/// `doc` package declaring one does not parse. The cell keeps its branch
/// — the rule belongs to `##KIND-DOC-MUST-NOT-EXECUTE` whatever else
/// enforces it — and this test pins where the refusal actually comes
/// from, so a future loosening of PROP-027 is noticed here.
#[test]
fn an_mcp_server_is_refused_before_this_cell_ever_sees_it() {
    let project = tempdir().unwrap();
    write_doc_package(
        project.path(),
        "[[binary]]\nname = \"docs\"\ncrate = \"crates/docs\"\n\n\
         [[mcp_server]]\nname = \"docs\"\nbinary = \"docs\"\n",
    );
    let report = findings(project.path());
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.check == CheckId::ManifestValidity
                && f.severity == Severity::Error
                && f.message
                    .contains("[[mcp_server]] is legal only in `mcp`-kind")),
        "the manifest grammar refuses it first: {:?}",
        report.findings
    );
}

#[test]
fn a_doc_package_without_a_readme_is_an_error() {
    let project = tempdir().unwrap();
    write_doc_package(project.path(), "");
    fs::remove_file(project.path().join("README.md")).unwrap();
    let report = findings(project.path());
    let hit = contract_findings(&report)
        .into_iter()
        .find(|f| f.severity == Severity::Error)
        .expect("the missing front door is an error");
    assert!(hit.message.contains("README.md"), "{}", hit.message);
    assert!(hit.message.contains("KIND-DOC-MUST-DOCUMENT"));
}

/// A pin moves a page to the top of the navigation, so a pin at nothing
/// moves nothing. The manifest grammar can only see that the path LOOKS
/// like a document; this cell reads the tree and says which pin is
/// empty.
#[test]
fn a_pin_at_a_page_that_is_not_there_is_an_error() {
    let project = tempdir().unwrap();
    write_doc_package(
        project.path(),
        "\n[navigation]\npinned = [\"start/index\", \"start/gone\"]\n",
    );
    let pages = project.path().join("vibevm/vibespecs/start");
    fs::create_dir_all(&pages).unwrap();
    fs::write(pages.join("index.xml"), "<spec/>\n").unwrap();

    let hits: Vec<String> = contract_findings(&findings(project.path()))
        .into_iter()
        .filter(|f| f.severity == Severity::Error)
        .map(|f| f.message.clone())
        .collect();
    assert_eq!(hits.len(), 1, "only the empty pin is a finding: {hits:?}");
    assert!(hits[0].contains("start/gone"), "{}", hits[0]);
    assert!(hits[0].contains("NAV-PINNED"), "{}", hits[0]);
}

/// Write `<package>/vibevm/vibespecs/<document>.xml` for each document,
/// so a test can give a package a page tree without writing a page the
/// pivot would take: this rule reads the directory, never the documents.
fn write_pages(root: &Path, documents: &[&str]) {
    for document in documents {
        let path = root
            .join("vibevm/vibespecs")
            .join(format!("{document}.xml"));
        fs::create_dir_all(path.parent().expect("a page lives in a folder")).unwrap();
        fs::write(path, "<spec/>\n").unwrap();
    }
}

/// The declared learning path against the tree, both ways round: a
/// chapter naming a page the package does not carry, and a page of the
/// package no chapter holds. The second is the finding the rule exists
/// for — a page added later must be given its place, and the alternative
/// is a page missing from the reader's contents with nothing said.
#[test]
fn a_learning_path_must_name_pages_that_exist_and_cover_the_ones_that_do() {
    let project = tempdir().unwrap();
    write_doc_package(
        project.path(),
        "\n[[navigation.chapter]]\nid = \"start\"\ntitle = \"Getting started\"\n\
         pages = [\"start/index\", \"start/gone\"]\n",
    );
    write_pages(project.path(), &["start/index", "model/two-trees"]);

    let hits: Vec<String> = contract_findings(&findings(project.path()))
        .into_iter()
        .filter(|f| f.severity == Severity::Error)
        .map(|f| f.message.clone())
        .collect();
    assert_eq!(hits.len(), 2, "one each way round: {hits:?}");

    let missing = hits
        .iter()
        .find(|m| m.contains("start/gone"))
        .expect("a chapter page the package does not carry");
    assert!(missing.contains("chapter `start`"), "{missing}");
    assert!(missing.contains("NAV-CHAPTERS-CHECKED"), "{missing}");

    let uncovered = hits
        .iter()
        .find(|m| m.contains("model/two-trees"))
        .expect("a page of the package no chapter holds");
    assert!(uncovered.contains("no chapter"), "{uncovered}");
    assert!(uncovered.contains("NAV-CHAPTERS-CHECKED"), "{uncovered}");
}

/// A documentation with no chapters is not a documentation with an
/// incomplete path, so the rule does not run on it at all — which is what
/// keeps every manual written before the rows existed passing untouched.
/// A translation is skipped for the other reason: it takes its source's
/// path and names no pages, so measuring its coverage would measure the
/// source's (`##NAV-CHAPTERS-TRANSLATION`).
#[test]
fn a_package_that_declares_no_path_and_a_translation_are_both_left_alone() {
    let silent = tempdir().unwrap();
    write_doc_package(
        silent.path(),
        "\n[navigation]\npinned = [\"start/index\"]\n\n\
         [[navigation.section]]\nid = \"start\"\ntitle = \"Start\"\n",
    );
    write_pages(silent.path(), &["start/index", "model/two-trees"]);
    let report = findings(silent.path());
    assert!(
        contract_findings(&report).is_empty(),
        "no path declared, nothing to cover: {:?}",
        report.findings
    );

    let adapted = tempdir().unwrap();
    write_doc_package(
        adapted.path(),
        "\n[translates]\npackage = \"org.vibevm.core/vibevm-docs\"\nversion = \"^0.1\"\n\
         \n[i18n]\ncanonical = \"ru\"\n\n\
         [[navigation.chapter]]\nid = \"start\"\ntitle = \"Первые шаги\"\n",
    );
    write_pages(adapted.path(), &["start/index", "model/two-trees"]);
    let report = findings(adapted.path());
    assert!(
        contract_findings(&report)
            .iter()
            .all(|f| !f.message.contains("NAV-CHAPTERS-CHECKED")),
        "a translation's coverage is its source's: {:?}",
        report.findings
    );
}

/// The subject's end of the edge. A `primary` in a foreign group is
/// legal — a group may hand its documentation to another publisher — so
/// it warns rather than fails, and only when the groups actually differ.
#[test]
fn a_foreign_primary_warns_and_an_own_group_primary_does_not() {
    let foreign = tempdir().unwrap();
    fs::write(
        foreign.path().join("vibe.toml"),
        r#"[package]
group = "org.vibevm"
name = "wal"
kind = "flow"
version = "1.0.0"

[documentation]
primary = "com.elsewhere/wal-docs"
"#,
    )
    .unwrap();
    let report = findings(foreign.path());
    let hit = contract_findings(&report)
        .into_iter()
        .find(|f| f.severity == Severity::Warning)
        .expect("a foreign primary is worth saying once");
    assert!(hit.message.contains("com.elsewhere"), "{}", hit.message);
    assert!(hit.message.contains("org.vibevm"), "{}", hit.message);

    let own = tempdir().unwrap();
    fs::write(
        own.path().join("vibe.toml"),
        r#"[package]
group = "org.vibevm"
name = "wal"
kind = "flow"
version = "1.0.0"

[documentation]
primary = "org.vibevm/wal-docs"
"#,
    )
    .unwrap();
    let report = findings(own.path());
    assert!(
        contract_findings(&report).is_empty(),
        "the subject's own group is the ordinary case: {:?}",
        report.findings
    );
}

/// The three rules this cell does NOT restate are still enforced — by
/// the manifest grammar A2.4 gave them. `vibe check` errors, cites the
/// anchor and names the fix; the point of the split is that there is one
/// copy of the grammar, not that the rule is unguarded.
#[test]
fn the_manifest_grammar_still_refuses_a_doc_package_without_its_card() {
    for (missing, anchor) in [
        ("title", "CARD-TITLE"),
        ("abstract", "CARD-DESCRIPTION-AND-ABSTRACT"),
    ] {
        let project = tempdir().unwrap();
        write_doc_package(project.path(), "");
        let text = fs::read_to_string(project.path().join("vibe.toml")).unwrap();
        let stripped: String = text
            .lines()
            .filter(|line| !line.starts_with(missing))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(project.path().join("vibe.toml"), stripped).unwrap();
        let report = findings(project.path());
        let hit = report
            .findings
            .iter()
            .find(|f| f.check == CheckId::ManifestValidity && f.severity == Severity::Error)
            .unwrap_or_else(|| panic!("a doc package without `{missing}` must not pass"));
        assert!(hit.message.contains(anchor), "{}", hit.message);
    }

    let project = tempdir().unwrap();
    fs::write(
        project.path().join("vibe.toml"),
        r#"[package]
group = "org.vibevm.core"
name = "vibevm-docs"
kind = "doc"
version = "0.1.0"
title = "VibeVM Manual"
abstract = "Four answers."
"#,
    )
    .unwrap();
    let report = findings(project.path());
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.check == CheckId::ManifestValidity
                && f.severity == Severity::Error
                && f.message.contains("REL-DOCUMENTS-REQUIRED")),
        "documentation that documents nothing must not pass: {:?}",
        report.findings
    );
}

/// A real glossary page: two entries, the second of which opens with a
/// list where a definition belongs.
///
/// Written as a page the PIVOT takes, unlike [`write_pages`] above: whether
/// an entry states a definition is a question about a document, so this
/// rule reads the documents and a `<spec/>` stub would only ever be
/// reported as unreadable.
fn write_glossary(root: &Path, entries: &str) {
    let path = root.join("vibevm/vibespecs/glossary/index.xml");
    fs::create_dir_all(path.parent().expect("a page lives in a folder")).unwrap();
    fs::write(
        path,
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
               <title id=\"root\">Glossary</title>\n{entries}</spec>\n"
        ),
    )
    .unwrap();
}

/// The declared glossary against the package that declared it: the page has
/// to be there, and every entry has to name a term and state a definition
/// (`##GLOSSARY-CHECKED`).
#[test]
fn a_declared_glossary_must_be_a_page_of_entries() {
    let declaring = "\n[glossary]\npage = \"glossary/index\"\n";

    // A page nobody wrote: every link to a term in it is a dead link.
    let missing = tempdir().unwrap();
    write_doc_package(missing.path(), declaring);
    let found = findings(missing.path());
    let messages: Vec<&str> = contract_findings(&found)
        .iter()
        .map(|f| f.message.as_str())
        .collect();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("glossary/index") && m.contains("GLOSSARY-CHECKED")),
        "{messages:?}"
    );

    // A page whose entries are not entries.
    let broken = tempdir().unwrap();
    write_doc_package(broken.path(), declaring);
    write_glossary(
        broken.path(),
        "  <lock-file title=\"lock file\"><list ordered=\"false\">\
         <item>Not a definition.</item></list></lock-file>\n",
    );
    let found = findings(broken.path());
    assert!(
        contract_findings(&found)
            .iter()
            .any(|f| f.message.contains("does not open with a paragraph")),
        "{:?}",
        found.findings
    );

    // And the page the norm describes owes nothing.
    let good = tempdir().unwrap();
    write_doc_package(good.path(), declaring);
    write_glossary(
        good.path(),
        "  <lock-file title=\"lock file\"><p>The record of which exact versions a project \
         uses.</p></lock-file>\n",
    );
    let found = findings(good.path());
    assert!(contract_findings(&found).is_empty(), "{:?}", found.findings);
}

/// A documentation that declares no glossary is not a documentation with a
/// broken one, and the path `glossary/index.xml` means nothing by itself
/// (`##GLOSSARY-DECLARED`).
#[test]
fn a_package_that_declares_no_glossary_is_left_alone() {
    let project = tempdir().unwrap();
    write_doc_package(project.path(), "");
    // The page is even there, under the name the old convention used, and
    // its entries are broken. Nothing declared it, so nothing is judged.
    write_glossary(
        project.path(),
        "  <lock-file title=\"\"><list ordered=\"false\"><item>Nothing.</item></list>\
         </lock-file>\n",
    );
    let report = findings(project.path());
    assert!(
        contract_findings(&report).is_empty(),
        "{:?}",
        report.findings
    );
}
