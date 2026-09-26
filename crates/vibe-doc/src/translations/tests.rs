//! The mirror, and the four ways it breaks — and, after them, the bodies
//! a translation borrows from the source it mirrors.

use vibe_specdoc::doc::BlockNode;

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

/// A chapter row the source does not declare (`##NAV-CHAPTERS-TRANSLATION`).
///
/// A translation renames the source's chapters and invents none: the path
/// is one fact, declared once, in the documentation being adapted. A row
/// under an unknown id renames nothing — it is a title nobody will ever
/// show, which is exactly how a chapter id renamed on one side and not
/// the other looks.
#[test]
fn a_chapter_the_source_does_not_declare_is_a_problem() {
    let named = |id: &str| NavigationChapter {
        id: id.to_owned(),
        title: id.to_owned(),
        pages: Vec::new(),
        appendix: false,
    };
    let source = [named("start"), named("model")];

    let problems = unknown_chapters(&source, &[named("start"), named("modell")]);
    assert_eq!(
        problems,
        vec![Problem::UnknownChapter {
            id: "modell".to_owned()
        }],
        "only the row the source has no chapter for"
    );

    // The problem is the manifest's, not a page's: a report that named a
    // page for it would send its reader to the wrong file.
    assert_eq!(problems[0].page(), "vibe.toml");
    let text = problems[0].render("org.demo/lib-docs");
    assert!(text.contains("UNKNOWN CHAPTER modell"), "{text}");
    assert!(text.contains("org.demo/lib-docs"), "{text}");
    assert!(text.contains("NAV-CHAPTERS-TRANSLATION"), "{text}");

    // Renaming every chapter the source declares is the ordinary case,
    // and naming only some of them is legal too: a chapter a translation
    // does not name keeps the source's title.
    assert!(unknown_chapters(&source, &[named("model")]).is_empty());
    assert!(unknown_chapters(&source, &[]).is_empty());

    // A source with no path at all makes every row unknown, which is the
    // same defect written larger rather than a case of its own.
    assert_eq!(unknown_chapters(&[], &[named("start")]).len(), 1);
}

/// A translation declares the SAME glossary as the documentation it adapts:
/// its glossary is its own mirrored page at the same path, with the terms
/// in its language (`##GLOSSARY-TRANSLATION`).
#[test]
fn a_glossary_the_translation_does_not_mirror_is_a_problem() {
    let at = |page: &str| Some(page.to_owned());

    // The ordinary case: one path on both sides, and nothing to report.
    assert!(unmirrored_glossary(at("glossary/index"), at("glossary/index")).is_empty());
    // And a pair that declares none on either side is a pair of manuals
    // written before the table existed.
    assert!(unmirrored_glossary(None, None).is_empty());

    let missing = unmirrored_glossary(at("glossary/index"), None);
    assert_eq!(
        missing,
        vec![Problem::Glossary {
            source: at("glossary/index"),
            translation: None,
        }]
    );
    // The problem is the manifest's, as a chapter's is.
    assert_eq!(missing[0].page(), "vibe.toml");
    let text = missing[0].render("org.demo/lib-docs");
    assert!(text.contains("GLOSSARY MISSING glossary/index"), "{text}");
    assert!(text.contains("GLOSSARY-TRANSLATION"), "{text}");

    // Another page is another glossary, and the line names both.
    let other = unmirrored_glossary(at("glossary/index"), at("terms/index"));
    let text = other[0].render("org.demo/lib-docs");
    assert!(text.contains("terms/index"), "{text}");
    assert!(text.contains("glossary/index"), "{text}");

    // And an adaptation that invents one decides something about the
    // documentation it adapts, which is the same rule read backwards.
    let invented = unmirrored_glossary(None, at("terms/index"));
    let text = invented[0].render("org.demo/lib-docs");
    assert!(text.contains("declares none"), "{text}");
}

/// The world that reaches the borrowed-example fixture's source: the
/// checkout arm answers for the coordinate its adaptation adapts.
fn borrowed_world() -> SpecSources {
    SpecSources::for_checkout(
        fixture("borrowed/source"),
        Some("com.example.docs"),
        "borrowed",
    )
}

/// The one lookup of «which source does this package adapt», and the two
/// answers it gives without failing.
#[test]
fn a_source_documentation_adapts_nothing_and_an_adaptation_names_its_instance() {
    assert_eq!(
        adapted(&fixture("manual"), &SpecSources::new()).expect("no translates, no work"),
        None
    );

    let found = adapted(&fixture("borrowed/adaptation"), &borrowed_world())
        .expect("the source is reachable")
        .expect("the adaptation declares one");
    assert_eq!(found.coordinate, "com.example.docs/borrowed");
    assert_eq!(found.instance.root, fixture("borrowed/source"));
    assert_eq!(found.instance.source, Source::Checkout);
}

/// The refusal an unreachable source has always produced, now produced
/// once: `check` and the borrowed-example reader ask one function.
#[test]
fn an_unreachable_source_is_the_same_refusal_the_check_gives() {
    let error = adapted(&fixture("borrowed/adaptation"), &SpecSources::new())
        .expect_err("nothing to mirror against");
    assert!(
        error.to_string().contains("com.example.docs/borrowed"),
        "{error}"
    );
    assert!(error.to_string().contains("vibe cache add"), "{error}");
}

/// The bundle a build hands the renderers: by page address, then by id,
/// each page's own body. The fixture's two pages author `demo` with
/// different commands on purpose, and every field travels.
#[test]
fn borrowed_bodies_are_kept_under_the_page_that_authored_them() {
    let found = borrowed(&fixture("borrowed/adaptation"), &borrowed_world());
    assert_eq!(
        found.keys().collect::<Vec<_>>(),
        vec!["guide/first.xml", "guide/second.xml"]
    );
    assert_eq!(found["guide/first.xml"]["demo"].run, "vibe --version");

    let second = &found["guide/second.xml"]["demo"];
    assert_eq!(second.run, "vibe list");
    assert_eq!(second.expect, "no packages");
    assert_eq!(second.lang.as_deref(), Some("ps1"));
    assert_eq!(second.exit, Some(2));
    assert_eq!(
        second.stderr.as_deref(),
        Some("error: nothing is installed")
    );
}

/// Nothing to borrow is an empty bundle and never a refusal: a renderer
/// must not abort over a text it could not fetch, so every way of having
/// no source lends nothing and the references render as the gaps they are.
#[test]
fn nothing_to_borrow_is_an_empty_bundle() {
    // A source no source holds.
    assert!(borrowed(&fixture("borrowed/adaptation"), &SpecSources::new()).is_empty());
    // Packages that adapt nothing at all.
    assert!(borrowed(&fixture("borrowed/source"), &borrowed_world()).is_empty());
    assert!(borrowed(&fixture("manual"), &SpecSources::new()).is_empty());
}

/// One page at a time, for a surface answering one request — and every
/// address it refuses before any of them could become a read.
#[test]
fn one_page_is_read_by_its_address_and_nothing_outside_the_source_is() {
    let dir = fixture("borrowed/adaptation");
    let world = borrowed_world();
    assert_eq!(
        borrowed_by(&dir, "guide/second.xml", &world)["demo"].run,
        "vibe list"
    );
    // A page the source does not carry.
    assert!(borrowed_by(&dir, "guide/third.xml", &world).is_empty());
    // And every spelling that is not a page address at all.
    for bad in [
        "",
        "../vibe.toml",
        "guide/../../vibe.toml",
        "/vibe.toml",
        "guide\\second.xml",
    ] {
        assert!(borrowed_by(&dir, bad, &world).is_empty(), "`{bad}`");
    }
}

/// Two authored examples under one id on one page: the FIRST in document
/// order wins.
///
/// The document is built rather than parsed, and that is the point. An
/// example's id is a fact anchor, so the pivot refuses a file that defines
/// one twice — the collision cannot arrive through a page, and a test that
/// wrote one would be testing the pivot's refusal instead. The tie-break
/// is proved on the walk, where the case can exist, because «which example
/// does this id name» must have one answer whatever reaches it.
#[test]
fn the_first_of_a_repeated_id_on_one_page_wins() {
    let example = |run: &str| BlockNode {
        when: None,
        block: Block::Example {
            id: "demo".to_owned(),
            fixture: "none".to_owned(),
            lang: None,
            exit: None,
            run: run.to_owned(),
            expect: String::new(),
            stderr: None,
        },
    };
    let doc = SpecDoc {
        preamble: vec![example("first"), example("second")],
        ..SpecDoc::default()
    };

    let found = borrowed::authored(&doc);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found["demo"].run, "first");
}
