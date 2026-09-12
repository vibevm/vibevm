//! The four tiers, their order, and the budget.

use super::*;
use crate::citations::SpecSources;
use crate::manifest::tests::{fixture, rendered_at};
use crate::manifest::{self, Options};

fn built() -> vibe_wire::generated::doc_manifest::DocManifest {
    manifest::build(
        &fixture("translations/source"),
        &SpecSources::new(),
        &Options::at(rendered_at()),
    )
    .expect("the fixture builds")
    .manifest
}

fn rendered_bodies() -> BTreeMap<String, String> {
    let set = crate::pages::read_package(&fixture("translations/source")).expect("pages");
    bodies(&set, &Content::new())
}

/// The four files a package publishes, named as the convention names
/// them.
#[test]
fn the_four_tiers_are_named_by_the_convention() {
    assert_eq!(
        Tier::ALL.iter().map(|t| t.file_name()).collect::<Vec<_>>(),
        vec![
            "llms.txt",
            "llms-small.txt",
            "llms-medium.txt",
            "llms-full.txt"
        ]
    );
    assert_eq!(Tier::Full.budget(), None);
    assert_eq!(Tier::Small.budget(), Some(SMALL_TOKEN_BUDGET));
}

/// The index opens with the card — the title, the star, the caption, the
/// publisher and the language — and then lists the pages with their
/// leading fact and their reading time.
#[test]
fn the_index_carries_the_card_and_one_line_a_page() {
    let text = index(&built(), "/doc/");
    assert!(text.starts_with("# The Pair\n"), "{text}");
    assert!(text.contains("A community documentation of com.example/subject"));
    assert!(text.contains("published by com.example.docs, in en."));
    assert!(text.contains("## Pages"));
    assert!(
        text.contains("- [One](/doc/com.example.docs/pair/0.1.0/guide/one/): The first page"),
        "{text}"
    );
    assert!(text.contains("min · user)"), "{text}");
    assert!(text.contains("Rendered 2026-09-12."), "{text}");
}

/// `##OBS-AUDIENCE-AGENT` puts pages written for agents first in
/// `llms.txt`. An agent that reads three lines and stops should read the
/// three lines addressed to it.
#[test]
fn pages_written_for_agents_stand_first_in_the_index() {
    let mut manifest = built();
    let last = manifest.pages.last_mut().expect("two pages");
    last.audiences = vec![Audience::Agent];
    let title = last.title.clone();
    let text = index(&manifest, "/doc/");
    let agents = text.find("## For agents").expect("an agent section");
    let pages = text.find("## Pages").expect("a page section");
    assert!(agents < pages, "{text}");
    assert!(
        text[agents..pages].contains(&format!("[{title}]")),
        "{text}"
    );
}

/// The corpus carries every page whole, each under the two addresses a
/// reader might want: the one a browser opens and the one a citation is
/// written as.
#[test]
fn the_full_corpus_carries_every_page_with_both_addresses() {
    let manifest = built();
    let text = render(Tier::Full, &manifest, &rendered_bodies(), "/doc/");
    assert!(text.contains("Source: /doc/com.example.docs/pair/0.1.0/guide/one/"));
    assert!(text.contains("Address: spec://com.example.docs/pair@0.1.0/guide/one"));
    assert!(text.contains("# One"));
    assert!(text.contains("# Two"));
    // The numbers the island and the `.xml` projection carry, in the same
    // file an agent reads.
    assert!(text.contains("[p01]"), "{text}");
    assert!(!text.contains("page(s) are not in this file"));
}

/// A budget admits whole pages only. An agent cannot tell a truncated
/// rule from a complete one, so a page that does not fit is left out and
/// counted, never cut.
#[test]
fn a_budget_leaves_pages_out_whole_and_says_how_many() {
    let manifest = built();
    let bodies = rendered_bodies();
    // A budget of one token admits the card and nothing else.
    let text = corpus(&manifest, &bodies, "/doc/", Some(1));
    assert!(text.contains("2 page(s) are not in this file"), "{text}");
    assert!(!text.contains("Address: spec://"), "{text}");
    assert!(text.contains("llms-full.txt"), "{text}");
}

/// A page the caller did not render is counted, not faked. Which pages
/// are missing is a question for the checks, and the corpus is a
/// projection.
#[test]
fn a_page_with_no_body_is_counted_rather_than_invented() {
    let manifest = built();
    let text = corpus(&manifest, &BTreeMap::new(), "/doc/", None);
    assert!(text.contains("2 page(s) are not in this file"), "{text}");
}

/// The address map of `##SITE-MOUNT` needs no index, so it is a pure
/// function — and a page address ends with a slash
/// (`##SITE-TRAILING-SLASH`).
#[test]
fn the_address_map_is_the_norms() {
    let manifest = built();
    assert_eq!(
        page_link(&manifest, "guide/one.xml", "/doc/"),
        "/doc/com.example.docs/pair/0.1.0/guide/one/"
    );
    assert_eq!(
        package_link(&manifest, "/doc/"),
        "/doc/com.example.docs/pair/0.1.0/"
    );
    // A translation is served under its own base; the language segment
    // belongs to the site's route table, not to this library.
    assert_eq!(
        page_link(&manifest, "guide/one.xml", "/doc/ru/"),
        "/doc/ru/com.example.docs/pair/0.1.0/guide/one/"
    );
}

/// Every tier renders, and only the index is free of page bodies.
#[test]
fn every_tier_renders_from_one_manifest() {
    let manifest = built();
    let bodies = rendered_bodies();
    let all = tiers(&manifest, &bodies, "/doc/");
    assert_eq!(all.len(), 4);
    for (name, text) in &all {
        assert!(text.starts_with("# The Pair\n"), "{name}");
    }
    assert!(!all[0].1.contains("Address: spec://"));
    assert!(all[3].1.contains("Address: spec://"));
}
