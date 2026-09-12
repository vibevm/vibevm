//! The mirror, and the four ways it breaks.

use super::*;
use crate::manifest::tests::fixture;

fn read(package: &str) -> PageSet {
    pages::read_package(&fixture(package)).expect("the fixture package reads")
}

fn package(pages: &[(&str, &str)]) -> PageSet {
    let mut set = PageSet::default();
    for (rel, body) in pages {
        let text = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
               <title id=\"root\">T</title>\n{body}\
             </spec>\n"
        );
        set.pages.push(Page {
            rel: (*rel).to_owned(),
            path: std::path::PathBuf::from(rel),
            doc: vibe_specdoc::from_xml_with(&text, vibe_specdoc::Vocabulary::Doc)
                .expect("the page parses"),
        });
    }
    set
}

/// A package that adapts nothing is a source documentation. The run is
/// green and says why, because «check everything» must not be a way to
/// fail for the packages the check does not apply to.
#[test]
fn a_source_documentation_has_nothing_to_mirror() {
    let report = check(&fixture("manual"), &SpecSources::new()).expect("no translates, no work");
    assert!(report.ok());
    assert!(report.adapts.is_none());
    assert!(report.render().contains("nothing to mirror"), "{report:?}");
}

/// A source the four sources cannot reach is an ERROR, not a green run: a
/// mirror check with nothing to mirror against has no verdict to give,
/// and green would be the worst of the three possible answers.
#[test]
fn a_source_no_source_holds_is_an_error_and_not_a_silence() {
    let error = check(&fixture("translations/adaptation"), &SpecSources::new())
        .expect_err("nothing to check against");
    assert!(
        error.to_string().contains("com.example.docs/pair"),
        "{error}"
    );
    assert!(error.to_string().contains("vibe cache add"), "{error}");
    assert!(error.to_string().contains("PROP-057#LOC-MIRROR"), "{error}");
}

/// The committed fixture pair: two pages, mirrored, with ONE deliberate
/// divergence. Paths, anchors and examples all agree — the adaptation
/// borrows its example rather than authoring one — and the second page
/// parts from its source at `p02`, where the source has a list and the
/// adaptation has a paragraph.
#[test]
fn the_fixture_pair_parts_at_one_block_and_nowhere_else() {
    let report = compare(
        &read("translations/source"),
        &read("translations/adaptation"),
    );
    assert_eq!(report.pages, 2);
    assert_eq!(
        report.problems,
        vec![Problem::Block {
            page: "guide/two.xml".to_owned(),
            at: "p02".to_owned(),
            source: "list".to_owned(),
            translation: "p".to_owned(),
        }],
        "the pair mirrors everything else"
    );
    assert!(report.unreadable.is_empty());
    assert!(!report.ok());
    let line = report.problems[0].render("com.example.docs/pair");
    assert!(line.contains("BLOCK p02 guide/two.xml"), "{line}");
    assert!(
        line.contains("spec://com.example.docs/pair/guide/two#p02"),
        "{line}"
    );
    assert!(line.contains("translation: guide/two.xml#p02"), "{line}");
}

/// A paragraph the adaptation merged into its neighbour keeps every
/// anchor and still moves every number below it. This is the case
/// matching anchors cannot catch, and the reason the block comparison
/// exists at all (F-44).
#[test]
fn a_merged_paragraph_is_caught_though_every_anchor_still_matches() {
    let source = package(&[(
        "a.xml",
        "  <p><LEAD fact=\"true\" status=\"doc/work\">one</LEAD></p>\n  \
           <p>two</p>\n  <p>three</p>\n",
    )]);
    let translation = package(&[(
        "a.xml",
        "  <p><LEAD fact=\"true\" status=\"doc/work\">один</LEAD></p>\n  \
           <p>два. три</p>\n",
    )]);
    let report = compare(&source, &translation);
    assert_eq!(
        report.problems,
        vec![Problem::Block {
            page: "a.xml".to_owned(),
            at: "p03".to_owned(),
            source: "p".to_owned(),
            translation: "nothing".to_owned(),
        }]
    );
}

/// Only the FIRST divergence on a page is reported: after a block is
/// added every number below it differs, and listing all of them would
/// bury the one line that says where to look.
#[test]
fn only_the_first_diverging_block_of_a_page_is_reported() {
    let source = package(&[(
        "a.xml",
        "  <p>one</p>\n  <p>two</p>\n  <p>three</p>\n  <p>four</p>\n",
    )]);
    let translation = package(&[(
        "a.xml",
        "  <p>один</p>\n  <quote>два</quote>\n  <quote>три</quote>\n  <p>четыре</p>\n",
    )]);
    let report = compare(&source, &translation);
    assert_eq!(report.problems.len(), 1);
    assert_eq!(
        report.problems[0],
        Problem::Block {
            page: "a.xml".to_owned(),
            at: "p02".to_owned(),
            source: "p".to_owned(),
            translation: "quote".to_owned(),
        }
    );
}

/// An authored example and a borrowed one are ONE kind to this
/// comparison. A translation replaces the first with the second by law,
/// so telling them apart here would paint every correct adaptation red —
/// and the real violation has a rule of its own.
#[test]
fn a_borrowed_example_matches_the_example_it_borrows() {
    let source = package(&[(
        "a.xml",
        "  <p>one</p>\n  <example id=\"v\" fixture=\"none\"><run>vibe --version</run>\
           <expect>vibe 1.0.0</expect></example>\n",
    )]);
    let translation = package(&[("a.xml", "  <p>один</p>\n  <example ref=\"v\"/>\n")]);
    let report = compare(&source, &translation);
    assert!(report.problems.is_empty(), "{:?}", report.problems);
}

/// The `footnotes` section is numbered in no projection, so it is outside
/// the mirror too: a translation may carry different apparatus without
/// moving a single `pNN`.
#[test]
fn the_footnotes_section_is_outside_the_block_comparison() {
    let source = package(&[(
        "a.xml",
        "  <p>one</p>\n  <section id=\"footnotes\" title=\"Footnotes\">\n    \
           <p>a</p>\n    <p>b</p>\n  </section>\n",
    )]);
    let translation = package(&[(
        "a.xml",
        "  <p>один</p>\n  <section id=\"footnotes\" title=\"Сноски\">\n    \
           <p>а</p>\n  </section>\n",
    )]);
    let report = compare(&source, &translation);
    assert!(report.problems.is_empty(), "{:?}", report.problems);
}

/// A mirror is file for file. A page on one side and not the other is a
/// defect of the TRANSLATION either way: a page that exists in one
/// language only belongs in the source first.
#[test]
fn a_page_on_one_side_only_is_named_by_its_address() {
    let source = package(&[("a.xml", "  <p>one</p>\n"), ("b.xml", "  <p>two</p>\n")]);
    let translation = package(&[("a.xml", "  <p>один</p>\n"), ("c.xml", "  <p>три</p>\n")]);
    let report = compare(&source, &translation);
    assert!(report.problems.contains(&Problem::MissingPage {
        page: "b.xml".to_owned()
    }));
    assert!(report.problems.contains(&Problem::ExtraPage {
        page: "c.xml".to_owned()
    }));
    assert!(!report.ok());
}

/// An anchor set difference is an ERROR, not a warning (R-18): a citation
/// must resolve in every language, and adding an anchor is forbidden
/// rather than merely unmirrored.
#[test]
fn an_anchor_on_one_side_only_is_an_error_in_both_directions() {
    let source = package(&[(
        "a.xml",
        "  <p><LEAD fact=\"true\" status=\"doc/work\">one</LEAD></p>\n  \
           <section id=\"why\" title=\"Why\">\n    <p>because</p>\n  </section>\n",
    )]);
    let translation = package(&[(
        "a.xml",
        "  <p><LEAD fact=\"true\" status=\"doc/work\">один</LEAD></p>\n  \
           <section id=\"zachem\" title=\"Зачем\">\n    <p>потому</p>\n  </section>\n",
    )]);
    let report = compare(&source, &translation);
    assert!(report.problems.contains(&Problem::AnchorMissing {
        page: "a.xml".to_owned(),
        anchor: "why".to_owned()
    }));
    assert!(report.problems.contains(&Problem::AnchorExtra {
        page: "a.xml".to_owned(),
        anchor: "zachem".to_owned()
    }));
}

/// A translation must not author examples: command output is checked
/// once, on the source, and a second copy is a second thing to go wrong.
#[test]
fn an_example_the_translation_authored_is_a_problem_by_name() {
    let source = package(&[(
        "a.xml",
        "  <example id=\"v\" fixture=\"none\"><run>vibe --version</run>\
           <expect>vibe 1.0.0</expect></example>\n",
    )]);
    let translation = package(&[(
        "a.xml",
        "  <example id=\"v\" fixture=\"none\"><run>vibe --version</run>\
           <expect>vibe 1.0.0</expect></example>\n",
    )]);
    let report = compare(&source, &translation);
    assert!(report.problems.contains(&Problem::OwnExample {
        page: "a.xml".to_owned(),
        id: "v".to_owned()
    }));
    assert!(
        report.problems[0]
            .render("org.demo/lib-docs")
            .contains("example ref="),
        "{:?}",
        report.problems
    );
}

/// A borrowed example must exist on the source PAGE, not merely somewhere
/// in the source package: the reference is resolved when that page is
/// projected, and a neighbouring page's example is not in reach.
#[test]
fn a_borrowed_example_must_exist_on_the_source_page() {
    let source = package(&[
        ("a.xml", "  <p>one</p>\n"),
        (
            "b.xml",
            "  <example id=\"v\" fixture=\"none\"><run>vibe --version</run>\
               <expect>vibe 1.0.0</expect></example>\n",
        ),
    ]);
    let translation = package(&[
        ("a.xml", "  <example ref=\"v\"/>\n"),
        ("b.xml", "  <example ref=\"v\"/>\n"),
    ]);
    let report = compare(&source, &translation);
    assert!(
        report.problems.contains(&Problem::UnknownExampleRef {
            page: "a.xml".to_owned(),
            id: "v".to_owned()
        }),
        "{:?}",
        report.problems
    );
    // The neighbouring page borrows an example its source page does
    // carry, so it is not named here.
    assert!(
        !report.problems.iter().any(|p| matches!(
            p,
            Problem::UnknownExampleRef { page, .. } if page == "b.xml"
        )),
        "{:?}",
        report.problems
    );
}

/// An unreadable page on either side hides its structure, so it is
/// reported rather than counted clean.
#[test]
fn an_unreadable_page_is_reported_and_not_counted_green() {
    let mut translation = package(&[("a.xml", "  <p>один</p>\n")]);
    translation.unreadable.push(crate::pages::UnreadablePage {
        rel: "b.xml".to_owned(),
        path: std::path::PathBuf::from("b.xml"),
        message: "not a page".to_owned(),
    });
    let report = compare(&package(&[("a.xml", "  <p>one</p>\n")]), &translation);
    assert!(!report.ok());
    assert!(
        report.render().contains("unreadable b.xml"),
        "{}",
        report.render()
    );
}

/// The report names the source it compared against, so a reader knows
/// which instance the verdict is about.
#[test]
fn the_report_names_the_coordinate_and_where_it_came_from() {
    let mut report = compare(&package(&[]), &package(&[]));
    report.adapts = Some("org.demo/lib-docs".to_owned());
    report.source = Some(Source::Store);
    let text = report.render();
    assert!(
        text.contains("adapting org.demo/lib-docs found in the machine store"),
        "{text}"
    );
    assert!(text.contains("0 problem(s)"), "{text}");
}
