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

/// The committed fixture pair: two pages, mirrored, with one deliberate
/// divergence on the second page. The set of pages and the set of anchors
/// agree — this atom's half of the mirror holds — and the adaptation
/// borrows its example rather than authoring one.
#[test]
fn the_fixture_pair_mirrors_its_pages_anchors_and_examples() {
    let report = compare(
        &read("translations/source"),
        &read("translations/adaptation"),
    );
    assert_eq!(report.pages, 2);
    assert!(
        report.problems.is_empty(),
        "the pair mirrors paths, anchors and examples: {:?}",
        report.problems
    );
    assert!(report.unreadable.is_empty());
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
        report.problems[0].render().contains("example ref="),
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
    assert_eq!(
        report.problems,
        vec![Problem::UnknownExampleRef {
            page: "a.xml".to_owned(),
            id: "v".to_owned()
        }]
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
