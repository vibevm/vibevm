//! The documentation half of the manifest (PROP-057 §§4, 5, 7): the two
//! relation tables, the localization edge, the card, the image paths —
//! and the two keys that parse only so the refusal can name the field
//! that actually works.

use super::*;
use crate::manifest::Authorship;
use crate::package_ref::PackageKind;

/// A `doc` package with everything required and nothing more, as a TOML
/// body the tests append to.
const DOC_HEAD: &str = "[package]\nname = \"vibevm-docs\"\ngroup = \"org.vibevm.core\"\n\
                        kind = \"doc\"\nversion = \"0.1.0\"\ntitle = \"VibeVM Manual\"\n\
                        abstract = \"What it covers, for whom, what it assumes known, what it \
                        leaves out.\"\n";

const SUBJECT: &str = "\n[[documents]]\npackage = \"org.vibevm.core/vibevm\"\nversion = \"^1.0\"\n";

fn doc_manifest(extra: &str) -> Result<Manifest> {
    Manifest::parse_str(&format!("{DOC_HEAD}{SUBJECT}{extra}"))
}

fn refusal(extra: &str) -> String {
    doc_manifest(extra)
        .expect_err("this manifest must be refused")
        .to_string()
}

/// The whole documentation surface parses into the domain type — the
/// shape the core manual's own manifest is written in.
#[test]
fn a_documentation_manifest_parses_whole() {
    let manifest = doc_manifest(
        "\n[translates]\npackage = \"org.vibevm.core/vibevm-docs\"\nversion = \"^0.3\"\n\
         \n[media]\nicon = \"media/icon.png\"\nbanner = \"media/banner.jpg\"\n\
         preview = \"media/preview.png\"\n\n[i18n]\ncanonical = \"ru\"\n",
    )
    .expect("the documentation manifest parses");

    let meta = manifest.require_package().unwrap();
    assert_eq!(meta.kind, PackageKind::Doc);
    assert_eq!(meta.title.as_deref(), Some("VibeVM Manual"));
    assert!(
        meta.abstract_text
            .as_deref()
            .unwrap()
            .starts_with("What it covers")
    );

    assert_eq!(manifest.documents.len(), 1);
    assert_eq!(manifest.documents[0].package, "org.vibevm.core/vibevm");
    assert_eq!(manifest.documents[0].version, "^1.0");

    let translates = manifest.translates.as_ref().expect("the source edge");
    assert_eq!(translates.package, "org.vibevm.core/vibevm-docs");

    let media = manifest.media.as_ref().expect("the card's images");
    assert_eq!(media.declared().len(), 3);

    // The language rides on the field that already existed.
    assert_eq!(manifest.i18n.canonical, "ru");
}

/// A round trip through the wire keeps every table: a document that
/// cannot be written back is a document the tooling would silently
/// truncate.
#[test]
fn the_documentation_tables_survive_a_round_trip() {
    let manifest = doc_manifest(
        "\n[documentation]\nprimary = \"org.vibevm.core/vibevm-docs\"\n\
         \n[media]\nicon = \"media/icon.png\"\n",
    )
    .expect("parses");
    let rendered = toml::to_string_pretty(&manifest).expect("serialises");
    let back = Manifest::parse_str(&rendered).expect("reads back");
    assert_eq!(manifest, back);
}

/// `[documentation]` is the SUBJECT's table, so it is legal in a package
/// of any kind — the whole point is that the subject assigns officiality.
#[test]
fn a_subject_of_any_kind_may_point_at_its_documentation() {
    let manifest = Manifest::parse_str(
        "[package]\nname = \"multi-user-planning\"\ngroup = \"org.vibevm.world\"\n\
         kind = \"flow\"\nversion = \"1.0.0\"\n\n[documentation]\n\
         primary = \"org.vibevm.world/multi-user-planning-docs\"\n\
         official = [\"org.vibevm.world/multi-user-planning-tutorials\"]\n",
    )
    .expect("a flow may name its documentation");
    let documentation = manifest.documentation.as_ref().unwrap();
    assert_eq!(documentation.official.len(), 1);
    assert!(!documentation.is_empty());
}

/// Documentation that documents nothing has no place to be shown.
#[test]
fn a_doc_package_without_a_subject_is_refused() {
    let error = Manifest::parse_str(DOC_HEAD).expect_err("no subject, no documentation");
    let message = error.to_string();
    assert!(message.contains("[[documents]]"), "{message}");
    assert!(
        message.contains("spec://org.vibevm.core/vibevm/common/PROP-057#REL-DOCUMENTS-REQUIRED"),
        "{message}"
    );
}

/// The table IS the claim to be documentation, so no other kind may make
/// it.
#[test]
fn a_non_doc_package_may_not_declare_a_subject() {
    let error = Manifest::parse_str(&format!(
        "[package]\nname = \"wal\"\ngroup = \"org.vibevm\"\nkind = \"flow\"\n\
         version = \"0.1.0\"\n{SUBJECT}"
    ))
    .expect_err("a flow documents nothing");
    let message = error.to_string();
    assert!(message.contains("kind = \"flow\""), "{message}");
    assert!(
        message.contains("spec://org.vibevm.core/vibevm/common/PROP-057#KIND-DOC-MUST-DOCUMENT"),
        "{message}"
    );
}

/// A translation of documentation is itself documentation.
#[test]
fn only_a_doc_package_may_translate() {
    let error = Manifest::parse_str(
        "[package]\nname = \"wal\"\ngroup = \"org.vibevm\"\nkind = \"flow\"\nversion = \"0.1.0\"\n\
         \n[translates]\npackage = \"org.vibevm.core/vibevm-docs\"\nversion = \"^0.3\"\n",
    )
    .expect_err("a flow translates nothing");
    let message = error.to_string();
    assert!(
        message.contains("spec://org.vibevm.core/vibevm/common/PROP-057#LOC-PACKAGE-PER-LANGUAGE"),
        "{message}"
    );
}

/// An edge names a coordinate, never a versioned or kind-prefixed
/// pkgref — the version of the relation lives in its own field.
#[test]
fn an_edge_that_is_not_a_coordinate_is_refused() {
    for (table, body) in [
        (
            "[[documents]].package",
            "[package]\nname = \"vibevm-docs\"\ngroup = \"org.vibevm.core\"\nkind = \"doc\"\n\
             version = \"0.1.0\"\ntitle = \"t\"\nabstract = \"a\"\n\n[[documents]]\n\
             package = \"org.vibevm.core/vibevm@1.0.0\"\nversion = \"^1.0\"\n",
        ),
        (
            "[documentation].primary",
            "[package]\nname = \"wal\"\ngroup = \"org.vibevm\"\nkind = \"flow\"\n\
             version = \"0.1.0\"\n\n[documentation]\nprimary = \"doc:org.vibevm/wal-docs\"\n",
        ),
    ] {
        let message = Manifest::parse_str(body)
            .expect_err("a pkgref is not a coordinate")
            .to_string();
        assert!(message.contains(table), "{table}: {message}");
        assert!(
            message.contains(
                "spec://org.vibevm.core/vibevm/common/PROP-057#REL-DOCUMENTATION-UNVERSIONED"
            ),
            "{table}: {message}"
        );
    }
}

/// The edge carries a RANGE, because one documentation version serves
/// many subject versions.
#[test]
fn a_subject_version_that_is_not_a_constraint_is_refused() {
    let message = Manifest::parse_str(&format!(
        "{DOC_HEAD}\n[[documents]]\npackage = \"org.vibevm.core/vibevm\"\nversion = \"latest\"\n"
    ))
    .expect_err("`latest` is not a semver constraint")
    .to_string();
    assert!(message.contains("[[documents]].version"), "{message}");
    assert!(
        message.contains("spec://org.vibevm.core/vibevm/common/PROP-057#REL-DOCUMENTS-REQUIRED"),
        "{message}"
    );
}

/// `primary` is already official; repeating it would show the same
/// package twice.
#[test]
fn primary_is_not_repeated_among_the_official_list() {
    let message = Manifest::parse_str(
        "[package]\nname = \"wal\"\ngroup = \"org.vibevm\"\nkind = \"flow\"\nversion = \"0.1.0\"\n\
         \n[documentation]\nprimary = \"org.vibevm/wal-docs\"\n\
         official = [\"org.vibevm/wal-docs\"]\n",
    )
    .expect_err("the repetition is refused")
    .to_string();
    assert!(message.contains("org.vibevm/wal-docs"), "{message}");
    assert!(
        message.contains(
            "spec://org.vibevm.core/vibevm/common/PROP-057#REL-DOCUMENTATION-UNVERSIONED"
        ),
        "{message}"
    );
}

/// A card without a name is a card a shelf cannot draw.
#[test]
fn a_doc_package_without_a_title_is_refused() {
    let message = Manifest::parse_str(
        "[package]\nname = \"vibevm-docs\"\ngroup = \"org.vibevm.core\"\nkind = \"doc\"\n\
         version = \"0.1.0\"\nabstract = \"a\"\n\
         \n[[documents]]\npackage = \"org.vibevm.core/vibevm\"\nversion = \"^1.0\"\n",
    )
    .expect_err("no title, no card")
    .to_string();
    assert!(
        message.contains("spec://org.vibevm.core/vibevm/common/PROP-057#CARD-TITLE"),
        "{message}"
    );
}

/// Four answers, and they must be there.
#[test]
fn a_doc_package_without_an_abstract_is_refused() {
    let message = Manifest::parse_str(
        "[package]\nname = \"vibevm-docs\"\ngroup = \"org.vibevm.core\"\nkind = \"doc\"\n\
         version = \"0.1.0\"\ntitle = \"VibeVM Manual\"\n\
         \n[[documents]]\npackage = \"org.vibevm.core/vibevm\"\nversion = \"^1.0\"\n",
    )
    .expect_err("no abstract, no card")
    .to_string();
    assert!(
        message.contains(
            "spec://org.vibevm.core/vibevm/common/PROP-057#CARD-DESCRIPTION-AND-ABSTRACT"
        ),
        "{message}"
    );
}

/// The bound is counted in characters, so an adaptation in a
/// multi-byte script gets the same room as the English source. One
/// character under passes; one over is refused.
#[test]
fn the_abstract_is_bounded_in_characters_not_bytes() {
    let at_limit = "я".repeat(crate::manifest::ABSTRACT_LIMIT);
    let over_limit = "я".repeat(crate::manifest::ABSTRACT_LIMIT + 1);

    let head = |abstract_text: &str| {
        format!(
            "[package]\nname = \"vibevm-docs-ru\"\ngroup = \"org.vibevm.core\"\nkind = \"doc\"\n\
             version = \"0.1.0\"\ntitle = \"Руководство\"\nabstract = \"{abstract_text}\"\n\
             {SUBJECT}"
        )
    };
    Manifest::parse_str(&head(&at_limit)).expect("exactly the limit is allowed");
    let message = Manifest::parse_str(&head(&over_limit))
        .expect_err("one character over is refused")
        .to_string();
    assert!(message.contains("1001 characters"), "{message}");
    assert!(
        message.contains(
            "spec://org.vibevm.core/vibevm/common/PROP-057#CARD-DESCRIPTION-AND-ABSTRACT"
        ),
        "{message}"
    );
}

/// Images are source files of this package's own tree. A path that
/// could leave it is refused by shape, before any file is opened.
#[test]
fn a_media_path_may_not_leave_the_package() {
    for path in ["../elsewhere/icon.png", "/etc/icon.png"] {
        let message = refusal(&format!("\n[media]\nicon = \"{path}\"\n"));
        assert!(message.contains("[media].icon"), "{path}: {message}");
        assert!(
            message.contains("spec://org.vibevm.core/vibevm/common/PROP-057#CARD-MEDIA-SOURCE"),
            "{path}: {message}"
        );
    }
}

/// `lang` parses and is refused by name. The campaign plan that came
/// before PROP-057 asked for this key, so an author will write it — and
/// a bare «unknown field» would leave them guessing which of the
/// manifest's thirty keys the language belongs to.
#[test]
fn lang_is_refused_and_names_the_field_that_works() {
    let message = Manifest::parse_str(&format!(
        "[package]\nname = \"vibevm-docs\"\ngroup = \"org.vibevm.core\"\nkind = \"doc\"\n\
         version = \"0.1.0\"\ntitle = \"t\"\nabstract = \"a\"\nlang = \"ru\"\n{SUBJECT}"
    ))
    .expect_err("`lang` is not a manifest field")
    .to_string();
    assert!(message.contains("[i18n].canonical"), "{message}");
    assert!(
        message.contains("spec://org.vibevm.core/vibevm/common/PROP-057#LOC-LANGUAGE-FIELD"),
        "{message}"
    );
}

/// `[translations]` parses and is refused by name: which adaptations a
/// manual has is computed from the `translates` edges at every render,
/// never stored where it can go stale.
#[test]
fn a_stored_translations_table_is_refused() {
    let message = doc_manifest("\n[translations]\nru = \"org.vibevm.core/vibevm-docs-ru\"\n")
        .expect_err("the table does not exist")
        .to_string();
    assert!(message.contains("[translates]"), "{message}");
    assert!(
        message.contains("spec://org.vibevm.core/vibevm/common/PROP-057#LOC-NO-TRANSLATIONS-TABLE"),
        "{message}"
    );
}

/// One card field spelled beside the title, so a test can say what a
/// `doc` package declares ABOUT ITSELF rather than inside a table.
fn card_field(line: &str) -> Result<Manifest> {
    Manifest::parse_str(&format!(
        "[package]\nname = \"vibevm-docs\"\ngroup = \"org.vibevm.core\"\nkind = \"doc\"\n\
         version = \"0.1.0\"\ntitle = \"t\"\nabstract = \"a\"\n{line}{SUBJECT}"
    ))
}

/// `authorship` says who wrote the PROSE, in one of the three words the
/// vocabulary holds. A documentation that says nothing says «unknown»,
/// which is why the field is an option and not a defaulted value.
#[test]
fn a_documentation_may_say_who_wrote_its_prose() {
    let manifest = card_field("authorship = \"ai\"\n").expect("a documentation may declare it");
    assert_eq!(
        manifest.require_package().unwrap().authorship,
        Some(Authorship::Ai)
    );

    let silent = card_field("").expect("the field is optional");
    assert!(silent.require_package().unwrap().authorship.is_none());
}

/// A word outside the three is refused, and the refusal names the field
/// and the words that work: the value is a closed vocabulary, not free
/// text a reader of a shelf would have to interpret.
#[test]
fn an_unknown_authorship_is_refused_by_name() {
    let message = card_field("authorship = \"robot\"\n")
        .expect_err("`robot` is not one of the three")
        .to_string();
    assert!(message.contains("authorship"), "{message}");
    assert!(message.contains("human"), "{message}");
    assert!(message.contains("mixed"), "{message}");
}

/// Prose is what a `doc` package carries, so only a `doc` package may
/// say whose it is.
#[test]
fn only_a_doc_package_may_declare_authorship() {
    let message = Manifest::parse_str(
        "[package]\nname = \"wal\"\ngroup = \"org.vibevm\"\nkind = \"flow\"\nversion = \"0.1.0\"\n\
         authorship = \"human\"\n",
    )
    .expect_err("a flow carries no prose")
    .to_string();
    assert!(message.contains("kind = \"flow\""), "{message}");
    assert!(
        message.contains("spec://org.vibevm.core/vibevm/common/PROP-057#CARD-AUTHORSHIP"),
        "{message}"
    );
}

/// `[navigation]` carries two statements and nothing else: the pages
/// listed first, in the order written, and the names of the folders.
#[test]
fn a_documentation_may_pin_pages_and_name_its_sections() {
    let manifest = doc_manifest(
        "\n[navigation]\npinned = [\"start/what-vibevm-is\", \"start/index\"]\n\n\
         [[navigation.section]]\nid = \"start\"\ntitle = \"Start\"\n\n\
         [[navigation.section]]\nid = \"model\"\ntitle = \"Model\"\n",
    )
    .expect("the navigation parses");
    let navigation = manifest.navigation.as_ref().expect("the table");
    assert_eq!(
        navigation.pinned,
        vec!["start/what-vibevm-is", "start/index"]
    );
    assert_eq!(navigation.sections.len(), 2);
    assert_eq!(navigation.sections[0].id, "start");
    assert_eq!(navigation.sections[1].title, "Model");

    assert!(doc_manifest("").expect("optional").navigation.is_none());
}

/// The learning path parses beside the pinning and changes nothing about
/// it: chapters in the order written, pages in the order written, and the
/// appendix mark where the author put it (`##NAV-CHAPTERS`).
#[test]
fn a_documentation_may_declare_a_learning_path() {
    let manifest = doc_manifest(
        "\n[navigation]\npinned = [\"start/what-vibevm-is\"]\n\n\
         [[navigation.chapter]]\nid = \"start\"\ntitle = \"Getting started\"\n\
         pages = [\"start/what-vibevm-is\", \"start/index\"]\n\n\
         [[navigation.chapter]]\nid = \"reference\"\ntitle = \"Appendices\"\n\
         pages = [\"glossary/index\"]\nappendix = true\n",
    )
    .expect("the learning path parses");
    let navigation = manifest.navigation.as_ref().expect("the table");
    assert_eq!(navigation.chapters.len(), 2);
    assert_eq!(navigation.chapters[0].id, "start");
    assert_eq!(navigation.chapters[0].title, "Getting started");
    assert_eq!(
        navigation.chapters[0].pages.as_deref().unwrap(),
        [
            "start/what-vibevm-is".to_string(),
            "start/index".to_string()
        ]
    );
    assert!(!navigation.chapters[0].appendix);
    assert!(navigation.chapters[1].appendix);

    // A package that declares no path is the state every documentation
    // written before the rows was in, and it stays legal.
    let pinned_only = doc_manifest("\n[navigation]\npinned = [\"start/index\"]\n")
        .expect("a navigation without chapters")
        .navigation
        .expect("the table");
    assert!(pinned_only.chapters.is_empty());
}

/// A chapter is a handle and a name: without the id nothing can rename
/// it, without the title the contents numbers a blank line, and two rows
/// under one id make every mention of it ambiguous.
#[test]
fn a_chapter_needs_an_id_and_a_title_and_may_not_share_either() {
    let blank_id =
        refusal("\n[[navigation.chapter]]\nid = \"  \"\ntitle = \"Getting started\"\npages = []\n");
    assert!(blank_id.contains("empty `id`"), "{blank_id}");
    assert!(blank_id.contains("NAV-CHAPTERS-CHECKED"), "{blank_id}");

    let blank_title =
        refusal("\n[[navigation.chapter]]\nid = \"start\"\ntitle = \"\"\npages = []\n");
    assert!(blank_title.contains("empty `title`"), "{blank_title}");
    assert!(
        blank_title.contains("NAV-CHAPTERS-CHECKED"),
        "{blank_title}"
    );

    let twice = refusal(
        "\n[[navigation.chapter]]\nid = \"start\"\ntitle = \"One\"\npages = [\"a/one\"]\n\n\
         [[navigation.chapter]]\nid = \"start\"\ntitle = \"Two\"\npages = [\"a/two\"]\n",
    );
    assert!(twice.contains("the id `start` twice"), "{twice}");
    assert!(twice.contains("NAV-CHAPTERS-CHECKED"), "{twice}");
}

/// The path meets every page ONCE, and a chapter holds pages spelled as
/// pins are spelled — so a page in two chapters and a page carrying its
/// extension are both refused.
#[test]
fn a_chapter_page_is_a_document_path_named_once_on_the_whole_path() {
    let twice = refusal(
        "\n[[navigation.chapter]]\nid = \"start\"\ntitle = \"One\"\n\
         pages = [\"start/index\", \"model/two-trees\"]\n\n\
         [[navigation.chapter]]\nid = \"model\"\ntitle = \"Two\"\n\
         pages = [\"model/two-trees\"]\n",
    );
    assert!(twice.contains("`model/two-trees` twice"), "{twice}");
    assert!(twice.contains("NAV-CHAPTERS-CHECKED"), "{twice}");

    for spelled in ["start/index.xml", "/start/index", "../x", ""] {
        let message = refusal(&format!(
            "\n[[navigation.chapter]]\nid = \"start\"\ntitle = \"One\"\npages = [\"{spelled}\"]\n"
        ));
        assert!(message.contains("not a document path"), "{message}");
        assert!(message.contains("NAV-CHAPTERS-CHECKED"), "{message}");
    }
}

/// `pages` is a list of paths and `appendix` is a yes or a no. Neither
/// refusal is written here: the grammar of a TOML value is TOML's, and a
/// second copy of it in this crate would be a second answer to «is this a
/// list of strings».
#[test]
fn a_chapter_whose_fields_are_the_wrong_shape_does_not_parse() {
    let pages = refusal(
        "\n[[navigation.chapter]]\nid = \"start\"\ntitle = \"One\"\npages = \"start/index\"\n",
    );
    assert!(pages.contains("pages"), "{pages}");

    let appendix = refusal(
        "\n[[navigation.chapter]]\nid = \"start\"\ntitle = \"One\"\npages = []\nappendix = \"yes\"\n",
    );
    assert!(appendix.contains("appendix"), "{appendix}");
}

/// A translation takes the path of the documentation it adapts and only
/// renames its chapters: a row with `id` and `title` is legal, and the
/// same row carrying `pages` is refused, because two copies of one order
/// are two things to keep in step (`##NAV-CHAPTERS-TRANSLATION`).
#[test]
fn a_translation_names_its_chapters_and_may_not_re_declare_their_pages() {
    const ADAPTS: &str = "\n[translates]\npackage = \"org.vibevm.core/vibevm-docs\"\nversion = \"^0.3\"\n\
         \n[i18n]\ncanonical = \"ru\"\n";

    let named = doc_manifest(&format!(
        "{ADAPTS}\n[[navigation.chapter]]\nid = \"start\"\ntitle = \"Первые шаги\"\n"
    ))
    .expect("a translation may name its chapters");
    let chapters = &named.navigation.as_ref().expect("the table").chapters;
    assert_eq!(chapters[0].title, "Первые шаги");
    assert!(
        chapters[0].pages.is_none(),
        "an unlisted `pages` stays unlisted"
    );

    // Even an EMPTY list is the translation listing pages: the legal
    // shape is a row without the key at all.
    for spelled in ["[]", "[\"start/index\"]"] {
        let message = refusal(&format!(
            "{ADAPTS}\n[[navigation.chapter]]\nid = \"start\"\ntitle = \"Первые шаги\"\n\
             pages = {spelled}\n"
        ));
        assert!(message.contains("declares [translates]"), "{message}");
        assert!(message.contains("NAV-CHAPTERS-TRANSLATION"), "{message}");
    }
}

/// A pin names a DOCUMENT, so it carries no extension: one document is
/// served as three projections, and a pin at one of them would be a pin
/// at one format.
#[test]
fn a_pinned_path_carrying_an_extension_is_refused() {
    // Spelled as TOML writes them, so the backslash is the escaped one a
    // Windows author would reach for.
    for (spelled, value) in [
        ("start/index.xml", "start/index.xml"),
        ("/start/index", "/start/index"),
        ("start\\\\index", "start\\index"),
        ("../x", "../x"),
        ("", ""),
    ] {
        let message = doc_manifest(&format!("\n[navigation]\npinned = [\"{spelled}\"]\n"))
            .expect_err("not a document path")
            .to_string();
        assert!(message.contains(value), "{message}");
        assert!(
            message.contains("spec://org.vibevm.core/vibevm/common/PROP-057#NAV-PINNED"),
            "{message}"
        );
    }
}

/// A section is one folder of the page tree, and it is named because a
/// reader sees the name — so a nested id and an empty title are both
/// refused.
#[test]
fn a_section_is_one_folder_under_a_name_a_reader_sees() {
    let nested = doc_manifest("\n[[navigation.section]]\nid = \"start/deep\"\ntitle = \"Start\"\n")
        .expect_err("a section is one segment")
        .to_string();
    assert!(nested.contains("start/deep"), "{nested}");

    let blank = doc_manifest("\n[[navigation.section]]\nid = \"start\"\ntitle = \"  \"\n")
        .expect_err("an empty title shows a blank heading")
        .to_string();
    assert!(blank.contains("title"), "{blank}");
}

/// Only documentation has a page tree, so only documentation may say
/// how it is listed.
#[test]
fn only_a_doc_package_may_declare_a_navigation() {
    let message = Manifest::parse_str(
        "[package]\nname = \"wal\"\ngroup = \"org.vibevm\"\nkind = \"flow\"\nversion = \"0.1.0\"\n\
         \n[navigation]\npinned = [\"start/index\"]\n",
    )
    .expect_err("a flow has no pages")
    .to_string();
    assert!(message.contains("kind = \"flow\""), "{message}");
    assert!(
        message.contains("spec://org.vibevm.core/vibevm/common/PROP-057#NAV-PINNED"),
        "{message}"
    );
}

/// The card is optional for every other kind — nothing here forces a
/// flow to grow a title.
#[test]
fn the_card_stays_optional_outside_documentation() {
    let manifest = Manifest::parse_str(
        "[package]\nname = \"wal\"\ngroup = \"org.vibevm\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
    )
    .expect("a flow needs no card");
    let meta = manifest.require_package().unwrap();
    assert!(meta.title.is_none());
    assert!(meta.abstract_text.is_none());
    assert!(manifest.media.is_none());
}
