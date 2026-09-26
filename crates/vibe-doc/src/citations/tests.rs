//! The citation laws, on trees built here rather than on the live
//! manual: a law that only holds for one corpus is a coincidence.

use std::fs;
use std::path::Path;

use super::*;

const SELF: &str = "com.example.docs/manual";

/// Write a package that holds one specification and, beside it, a
/// documentation package citing it. Returns (spec package root, doc
/// package root).
fn world(tmp: &Path, page_body: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let spec_pkg = tmp.join("repo");
    let spec_dir = spec_pkg.join("vibevm/vibespecs/common");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(
        spec_pkg.join("vibe.toml"),
        "[project]\nname = \"host\"\ngroup = \"com.example\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    fs::write(
        spec_dir.join("PROP-001-the-thing.xml"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
           <title id=\"root\">PROP-001</title>\n  \
           <p><A-RULE fact=\"true\" status=\"spec/done\">The rule says a thing.</A-RULE></p>\n  \
           <section id=\"chapter\" title=\"A chapter\">\n    \
             <p><B-RULE fact=\"true\" status=\"spec/done\">And another thing.</B-RULE></p>\n  \
           </section>\n\
         </spec>\n",
    )
    .unwrap();

    let doc_pkg = tmp.join("docs");
    let page_dir = doc_pkg.join("vibevm/vibespecs/model");
    fs::create_dir_all(&page_dir).unwrap();
    fs::write(page_dir.join("page.xml"), page_body).unwrap();
    (spec_pkg, doc_pkg)
}

fn page(body: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
           <title id=\"root\">Page</title>\n{body}\
         </spec>\n"
    )
}

fn sources(spec_pkg: &Path) -> SpecSources {
    SpecSources::for_checkout(spec_pkg, Some("com.example"), "host")
}

/// The whole mechanism in one test: a page names an address and the
/// resolver hands back the fact's text as it stands right now.
#[test]
fn a_rule_resolves_to_the_current_text_of_the_fact_it_names() {
    let tmp = tempfile::tempdir().unwrap();
    let (spec_pkg, _) = world(tmp.path(), &page(""));
    let found = resolve(
        "spec://com.example/host/common/PROP-001#A-RULE",
        &sources(&spec_pkg),
    )
    .expect("resolves");
    assert_eq!(found.text, "The rule says a thing.");
    assert_eq!(found.anchor, "A-RULE");
    assert_eq!(found.source, Source::Checkout);
    // No `[i18n]` in the manifest ⇒ the registry default, not a guess.
    assert_eq!(found.lang, "en");
}

/// The lossy `PROP-NNN` truncation is inverted by the resolver that owns
/// that law — the address says `PROP-001`, the file is
/// `PROP-001-the-thing.xml`.
#[test]
fn the_address_names_the_id_and_the_file_carries_its_title() {
    let tmp = tempfile::tempdir().unwrap();
    let (spec_pkg, _) = world(tmp.path(), &page(""));
    let found = resolve(
        "spec://com.example/host/common/PROP-001#B-RULE",
        &sources(&spec_pkg),
    )
    .expect("resolves");
    assert!(
        found
            .path
            .to_string_lossy()
            .ends_with("PROP-001-the-thing.xml"),
        "{}",
        found.display_path_for_test()
    );
}

/// An anchor that does not exist is the one failure this check knows.
/// The message names the tombstone law, because a rename without one is
/// what produces this state.
#[test]
fn a_vanished_anchor_is_the_only_thing_the_check_refuses() {
    let tmp = tempfile::tempdir().unwrap();
    let (spec_pkg, _) = world(tmp.path(), &page(""));
    let err = resolve(
        "spec://com.example/host/common/PROP-001#GONE",
        &sources(&spec_pkg),
    )
    .expect_err("refused");
    assert!(err.contains("carries no anchor `GONE`"), "{err}");
    assert!(err.contains("INV-ANCHORS-IMMUTABLE"), "{err}");
}

/// A document address with no anchor is not a citation: a `rule` quotes
/// ONE fact, and a whole specification is not a quotation.
#[test]
fn an_address_with_no_anchor_is_refused_by_name() {
    let tmp = tempfile::tempdir().unwrap();
    let (spec_pkg, _) = world(tmp.path(), &page(""));
    let err = resolve(
        "spec://com.example/host/common/PROP-001",
        &sources(&spec_pkg),
    )
    .expect_err("refused");
    assert!(err.contains("names a document but no anchor"), "{err}");
}

/// A section anchor resolves to the section's HEADING, never to its
/// subtree: a container is not a rule, and inlining one would quote a
/// chapter where the author asked for a sentence.
#[test]
fn a_section_anchor_quotes_the_heading_and_not_the_chapter() {
    let tmp = tempfile::tempdir().unwrap();
    let (spec_pkg, _) = world(tmp.path(), &page(""));
    let found = resolve(
        "spec://com.example/host/common/PROP-001#chapter",
        &sources(&spec_pkg),
    )
    .expect("resolves");
    assert_eq!(found.text, "A chapter");
}

/// The three prose forms are not citations, and the reason each one is
/// not is different (X-029).
#[test]
fn the_three_prose_forms_are_classified_apart_from_citations() {
    assert_eq!(
        classify("spec://com.example/host/common/PROP-001#A-RULE", SELF),
        Classification::Citation
    );
    assert_eq!(
        classify("spec://…#ANCHOR", SELF),
        Classification::Placeholder
    );
    assert_eq!(
        classify("spec://org.vibevm.core/vibevm/<path>/<DOC>#X", SELF),
        Classification::Placeholder
    );
    // A page is XML, so an author who writes `<page>` in prose escapes
    // it — and this walk reads the page's raw bytes.
    assert_eq!(
        classify("spec://com.example.docs/manual/&lt;page&gt;#x", SELF),
        Classification::Placeholder
    );
    assert_eq!(
        classify(
            "spec://org.acme/notes-flow/flows/notes/PROTOCOL#RETRY-COUNT",
            "x/y"
        ),
        Classification::Teaching
    );
    assert_eq!(
        classify("spec://com.example.docs/manual/model/boot-lane#p7", SELF),
        Classification::SelfAddress
    );
}

/// A tutorial shows the reader the exact address their own project mints,
/// and no package can resolve it; a look-alike name is still a citation.
#[test]
fn a_tutorial_project_address_is_teaching_and_a_look_alike_is_not() {
    assert_eq!(
        classify(
            "spec://hello-vibevm/modules/calculator/PROP-001#division-by-zero",
            SELF
        ),
        Classification::Teaching
    );
    assert_eq!(
        classify("spec://hello-vibevmx/modules/calculator/PROP-001#x", SELF),
        Classification::Citation
    );
}

/// A placeholder BEATS the teaching group: `spec://org.acme/…/X` cannot
/// be resolved because of its ellipsis, and reporting it as a teaching
/// address would name the wrong reason.
#[test]
fn a_placeholder_wins_over_the_teaching_group() {
    assert_eq!(
        classify("spec://org.acme/…/PROTOCOL#X", "x/y"),
        Classification::Placeholder
    );
}

/// Only `rule` mints an edge, and the edge carries the page, its line and
/// the address — never a pin.
#[test]
fn every_rule_mints_one_unpinned_edge_that_names_its_line() {
    let tmp = tempfile::tempdir().unwrap();
    let body = "  <p>Prose about spec://com.example/host/common/PROP-001#A-RULE here.</p>\n  \
                <rule ref=\"spec://com.example/host/common/PROP-001#A-RULE\"/>\n  \
                <rule ref=\"spec://com.example/host/common/PROP-001#B-RULE\"/>\n";
    let (_, doc_pkg) = world(tmp.path(), &page(body));
    let edges = edges(&doc_pkg, SELF).expect("edges");
    assert_eq!(edges.len(), 2, "prose is checked but mints no edge");
    assert_eq!(edges[0].file, "vibevm/vibespecs/model/page.xml");
    assert_eq!(edges[0].page, "model/page.xml");
    assert_eq!(edges[0].line, 5, "the line of the first <rule>");
    assert_eq!(edges[1].line, 6);
}

/// A `<rule` shown INSIDE a fence — which the authoring page does, to
/// teach the element — is not an element and must not shift the lines of
/// the real ones.
#[test]
fn a_rule_quoted_in_a_fence_does_not_steal_a_line() {
    let tmp = tempfile::tempdir().unwrap();
    let body = "  <fence lang=\"xml\">&lt;rule ref=\"spec://com.example/host/common/PROP-999#SHOWN\"/&gt;</fence>\n  \
                <rule ref=\"spec://com.example/host/common/PROP-001#A-RULE\"/>\n";
    let (_, doc_pkg) = world(tmp.path(), &page(body));
    let edges = edges(&doc_pkg, SELF).expect("edges");
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].line, 5, "the real element, not the illustration");
}

/// A pin that strayed into the attribute is dropped by the pivot and the
/// edge is minted from the bare address — an unpinned edge cannot go
/// suspect, which is the whole point.
#[test]
fn a_pin_in_the_attribute_never_reaches_the_edge() {
    let tmp = tempfile::tempdir().unwrap();
    let body = "  <rule ref=\"spec://com.example/host/common/PROP-001#A-RULE~r3\"/>\n";
    let (_, doc_pkg) = world(tmp.path(), &page(body));
    let edges = edges(&doc_pkg, SELF).expect("edges");
    assert_eq!(
        edges[0].uri,
        "spec://com.example/host/common/PROP-001#A-RULE"
    );
    assert_eq!(edges[0].line, 4, "the pinned element is still located");
}

/// The check counts every class and fails only on a real citation that
/// does not resolve.
#[test]
fn the_check_fails_on_a_dead_citation_and_passes_over_the_illustrations() {
    let tmp = tempfile::tempdir().unwrap();
    let body = "  <p>See spec://org.acme/notes/flows/n/PROTOCOL#X and spec://… too.</p>\n  \
                <rule ref=\"spec://com.example/host/common/PROP-001#A-RULE\"/>\n  \
                <rule ref=\"spec://com.example/host/common/PROP-001#GONE\"/>\n";
    let (spec_pkg, doc_pkg) = world(tmp.path(), &page(body));
    let report = check(&doc_pkg, SELF, &sources(&spec_pkg)).expect("checked");
    assert!(!report.ok());
    assert_eq!(report.unresolved.len(), 1);
    assert_eq!(
        report.unresolved[0].uri,
        "spec://com.example/host/common/PROP-001#GONE"
    );
    let counts = report.counts();
    assert_eq!(counts.rules, 2);
    assert_eq!(counts.citations, 2);
    assert_eq!(counts.placeholders, 1);
    assert_eq!(counts.teaching, 1);
    assert!(
        report.render().contains("UNRESOLVED"),
        "{}",
        report.render()
    );
}

/// A page the pivot cannot read hides its citations. Reporting it is the
/// difference between «clean» and «not looked at».
#[test]
fn an_unreadable_page_is_reported_rather_than_counted_clean() {
    let tmp = tempfile::tempdir().unwrap();
    let (spec_pkg, doc_pkg) = world(tmp.path(), &page(""));
    fs::write(
        doc_pkg.join("vibevm/vibespecs/model/broken.xml"),
        "<?xml version=\"1.0\"?>\n<spec xmlns=\"https://vibevm.org/spec/1\"><nope/></spec>\n",
    )
    .unwrap();
    let report = check(&doc_pkg, SELF, &sources(&spec_pkg)).expect("checked");
    assert!(!report.ok());
    assert_eq!(report.unreadable, vec!["model/broken.xml".to_string()]);
}

/// A self-address resolves inside the documentation package. A page that
/// is not there is a dead link like any other; a positional `pNN` is not
/// an anchor at all and is deliberately not chased.
#[test]
fn a_self_address_is_resolved_against_the_package_and_pnn_is_left_alone() {
    let tmp = tempfile::tempdir().unwrap();
    let body = "  <p>Read spec://com.example.docs/manual/model/page#p7 and \
                spec://com.example.docs/manual/model/page#root and \
                spec://com.example.docs/manual/model/missing#x.</p>\n";
    let (spec_pkg, doc_pkg) = world(tmp.path(), &page(body));
    let report = check(&doc_pkg, SELF, &sources(&spec_pkg)).expect("checked");
    assert_eq!(report.unresolved.len(), 1, "{:#?}", report.unresolved);
    assert!(report.unresolved[0].uri.ends_with("missing#x"));
    assert_eq!(report.counts().self_addresses, 3);
}

/// The chain answers nearest-first, and a world with no source at all
/// says so rather than pretending an address is a typo.
#[test]
fn an_empty_world_names_itself_in_the_refusal() {
    let err = resolve(
        "spec://com.example/host/common/PROP-001#A-RULE",
        &SpecSources::new(),
    )
    .expect_err("refused");
    assert!(err.contains("no source at all"), "{err}");
}

impl RuleText {
    /// Test-only: the path a failure message would print.
    fn display_path_for_test(&self) -> String {
        self.path.to_string_lossy().replace('\\', "/")
    }
}
