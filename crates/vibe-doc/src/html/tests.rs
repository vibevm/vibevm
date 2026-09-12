//! The island's laws, one construct at a time. The whole-page shape is
//! pinned by the golden in `tests/island.rs`; these are the rules a
//! golden cannot state.

use std::collections::BTreeMap;

use vibe_specdoc::doc::DerivedKind;

use super::*;
use crate::citations::{RuleText, Source};
use crate::content::ExampleBody;

fn page(body: &str) -> SpecDoc {
    let text = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
           <title id=\"root\">Page</title>\n{body}\
         </spec>\n"
    );
    vibe_specdoc::from_xml_with(&text, vibe_specdoc::Vocabulary::Doc).expect("page parses")
}

fn rule_text(uri: &str, text: &str, lang: &str) -> BTreeMap<String, RuleText> {
    let mut map = BTreeMap::new();
    map.insert(
        uri.to_owned(),
        RuleText {
            uri: uri.to_owned(),
            anchor: uri.rsplit('#').next().unwrap_or("").to_owned(),
            text: text.to_owned(),
            lang: lang.to_owned(),
            source: Source::Checkout,
            path: std::path::PathBuf::from("spec.xml"),
        },
    );
    map
}

/// The contract that makes the content security policy possible: an
/// island can carry no executable and no style, whatever a page says.
#[test]
fn an_island_carries_no_script_and_no_style_even_when_a_page_quotes_one() {
    let doc = page(
        "  <p>Write `&lt;script&gt;alert(1)&lt;/script&gt;` nowhere.</p>\n  \
           <fence lang=\"html\">&lt;style&gt;body{}&lt;/style&gt;</fence>\n",
    );
    let island = to_html(&doc, &Content::new());
    assert!(!island.contains("<script"), "{island}");
    assert!(!island.contains("<style"), "{island}");
    assert!(
        island.contains("&lt;script&gt;alert(1)&lt;/script&gt;"),
        "{island}"
    );
}

/// A section keeps its named anchor and its heading depth: `#id` is how a
/// reader and an agent address the same place, and the anchors are
/// immutable (`##INV-ANCHORS-IMMUTABLE`).
#[test]
fn a_section_keeps_its_anchor_and_its_depth() {
    let doc = page(
        "  <section id=\"one\" title=\"One\">\n    \
             <section id=\"two\" title=\"Two\">\n      \
               <p>Deep.</p>\n    \
             </section>\n  \
           </section>\n",
    );
    let island = to_html(&doc, &Content::new());
    assert!(island.contains("<section id=\"one\">"), "{island}");
    assert!(island.contains("<h2>One</h2>"), "{island}");
    assert!(island.contains("<section id=\"two\">"), "{island}");
    assert!(island.contains("<h3>Two</h3>"), "{island}");
}

/// A fact's address and status ride on the block that carries it.
#[test]
fn a_fact_carries_its_address_and_status() {
    let doc = page(
        "  <p><A-RULE fact=\"true\" status=\"spec/done\" audience=\"user,dev\">Text.</A-RULE></p>\n",
    );
    let island = to_html(&doc, &Content::new());
    assert!(
        island.contains(
            "<p data-fact=\"A-RULE\" data-status=\"spec/done\" data-audience=\"user,dev\">Text.</p>"
        ),
        "{island}"
    );
}

/// A cited rule shows the fact's CURRENT text, names the specification's
/// language, and carries no revision of any kind.
#[test]
fn a_rule_quotes_the_current_text_names_the_specs_language_and_pins_nothing() {
    let doc = page("  <rule ref=\"spec://org.demo/lib/common/PROP-001#A~r7\"/>\n");
    let content = Content {
        rules: rule_text(
            "spec://org.demo/lib/common/PROP-001#A",
            "The rule **says** a thing.",
            "en",
        ),
        ..Content::new()
    };
    let island = to_html(&doc, &content);
    assert!(
        island.contains("data-uri=\"spec://org.demo/lib/common/PROP-001#A\""),
        "{island}"
    );
    assert!(
        island.contains("href=\"/doc/org.demo/lib/latest/common/PROP-001/#A\""),
        "{island}"
    );
    assert!(island.contains("lang=\"en\""), "{island}");
    assert!(
        island.contains("The rule <strong>says</strong> a thing."),
        "{island}"
    );
    assert!(
        !island.contains("data-rev"),
        "a citation is live and unpinned"
    );
    assert!(!island.contains("r7"), "{island}");
}

/// A rule this build could not resolve shows its address and says so —
/// a blank quotation would read as a rule that says nothing.
#[test]
fn an_unresolved_rule_shows_its_address_and_is_marked() {
    let doc = page("  <rule ref=\"spec://org.demo/lib/common/PROP-001#GONE\"/>\n");
    let island = to_html(&doc, &Content::new());
    assert!(island.contains("data-unresolved=\"true\""), "{island}");
    assert!(
        island.contains("spec://org.demo/lib/common/PROP-001#GONE"),
        "{island}"
    );
}

/// An example is a command and the output it must produce, both
/// verbatim. A non-zero exit is on the page because a reader needs to
/// know the command is expected to fail; a zero is the default.
#[test]
fn an_example_shows_the_command_and_the_output_and_only_a_failing_exit() {
    let doc = page(
        "  <example id=\"v\" fixture=\"none\">\n    <run>vibe --version</run>\n    \
             <expect>vibe 1.0.0</expect>\n  </example>\n  \
           <example id=\"bad\" fixture=\"none\" exit=\"2\">\n    <run>vibe nope</run>\n    \
             <expect></expect>\n    <stderr>error: no such command</stderr>\n  </example>\n",
    );
    let island = to_html(&doc, &Content::new());
    assert!(
        island.contains("data-example=\"v\" data-fixture=\"none\""),
        "{island}"
    );
    assert!(
        island.contains("<code class=\"language-sh\">vibe --version</code>"),
        "{island}"
    );
    assert!(island.contains("<code>vibe 1.0.0</code>"), "{island}");
    assert!(island.contains("data-exit=\"2\""), "{island}");
    assert!(island.contains("class=\"example-stderr\""), "{island}");
    assert_eq!(
        island.matches("data-exit").count(),
        1,
        "zero is the default"
    );
}

/// A translation's `example ref` borrows the source page's body: a
/// command has no translation.
#[test]
fn an_example_ref_renders_the_body_it_borrows() {
    let doc = page("  <example ref=\"v\"/>\n");
    let mut content = Content::new();
    content.examples.insert(
        "v".to_owned(),
        ExampleBody {
            run: "vibe --version".to_owned(),
            expect: "vibe 1.0.0".to_owned(),
            ..ExampleBody::default()
        },
    );
    let island = to_html(&doc, &content);
    assert!(island.contains("data-example-ref=\"v\""), "{island}");
    assert!(island.contains("vibe --version"), "{island}");

    // Without the source in hand the island says so rather than
    // inventing a command.
    let bare = to_html(&doc, &Content::new());
    assert!(bare.contains("data-unresolved=\"true\""), "{bare}");
    assert!(!bare.contains("vibe --version"), "{bare}");
}

/// A `derived` block is the fence its generator built at THIS build; the
/// page never carries the text.
#[test]
fn a_derived_block_is_the_fence_this_build_generated() {
    let doc = page("  <derived kind=\"cli-help\" ref=\"vibe list --help\"/>\n");
    let mut content = Content::new();
    content.derived.insert(
        Content::derived_key(DerivedKind::CliHelp, "vibe list --help"),
        "Usage: vibe list [OPTIONS]".to_owned(),
    );
    let island = to_html(&doc, &content);
    assert!(island.contains("data-derived=\"cli-help\""), "{island}");
    assert!(island.contains("Usage: vibe list [OPTIONS]"), "{island}");

    let bare = to_html(&doc, &Content::new());
    assert!(bare.contains("data-unresolved=\"true\""), "{bare}");
}

/// A slot's condition rides as data, and EVERY variant stays in the
/// island: one build serves every platform and every agent, so the
/// numbers and the anchors mean the same thing everywhere.
#[test]
fn a_conditional_slot_is_kept_and_marked_never_dropped() {
    let doc = page(
        "  <p when=\"os:windows\">On Windows.</p>\n  \
           <p when=\"os:linux\">On Linux.</p>\n  \
           <section id=\"s\" title=\"S\" when=\"agent:codex\"><p>x</p></section>\n",
    );
    let island = to_html(&doc, &Content::new());
    assert!(
        island.contains("<p data-when=\"os:windows\">On Windows.</p>"),
        "{island}"
    );
    assert!(
        island.contains("<p data-when=\"os:linux\">On Linux.</p>"),
        "{island}"
    );
    assert!(island.contains("data-when=\"agent:codex\""), "{island}");
}

/// A fence's language becomes the class a highlighter reads; a bare
/// fence claims no language rather than guessing one.
#[test]
fn a_fence_carries_its_language_as_a_class_and_a_bare_one_claims_none() {
    let doc = page("  <fence lang=\"rust\">let x = 1;</fence>\n  <fence>plain</fence>\n");
    let island = to_html(&doc, &Content::new());
    assert!(
        island.contains("<code class=\"language-rust\">let x = 1;</code>"),
        "{island}"
    );
    assert!(island.contains("<code>plain</code>"), "{island}");
}

/// A prompt is a block with its text, what the agent needs, what the
/// person sees, and the commands that must pass afterwards.
#[test]
fn a_prompt_carries_its_text_needs_outcome_and_asserts() {
    let doc = page(
        "  <prompt id=\"p\">Install vibe.\n    <needs>a terminal</needs>\n    \
             <outcome>`vibe --version` prints a version</outcome>\n    \
             <assert>vibe --version</assert>\n  </prompt>\n",
    );
    let island = to_html(&doc, &Content::new());
    assert!(island.contains("data-prompt=\"p\""), "{island}");
    assert!(
        island.contains("<code class=\"language-prompt\">Install vibe.</code>"),
        "{island}"
    );
    assert!(
        island.contains("class=\"prompt-needs\">a terminal</p>"),
        "{island}"
    );
    assert!(island.contains("class=\"prompt-outcome\""), "{island}");
    assert!(
        island.contains("<li><code>vibe --version</code></li>"),
        "{island}"
    );
}

/// An illustrative prompt SAYS it asserts nothing; silence would be a
/// prompt whose asserts somebody forgot.
#[test]
fn an_illustrative_prompt_says_it_asserts_nothing() {
    let doc = page("  <prompt id=\"p\" assert=\"none\">Ask for anything.</prompt>\n");
    let island = to_html(&doc, &Content::new());
    assert!(island.contains("data-assert=\"none\""), "{island}");
}

/// The first row of a table is its header when the source had one.
#[test]
fn a_table_renders_its_first_row_as_the_header() {
    let doc = page(
        "  <table>\n    <tr><td>Verb</td><td>Does</td></tr>\n    \
             <tr><td>`list`</td><td>lists</td></tr>\n  </table>\n",
    );
    let island = to_html(&doc, &Content::new());
    assert!(island.contains("<th>Verb</th>"), "{island}");
    assert!(island.contains("<td><code>list</code></td>"), "{island}");
}

/// Rendering is a pure function of the document and the bundle: two
/// renders of one page are one file, so a diff of two islands shows what
/// moved.
#[test]
fn two_renders_of_one_page_are_the_same_bytes() {
    let doc = page("  <p>One.</p>\n  <section id=\"s\" title=\"S\"><p>Two.</p></section>\n");
    assert_eq!(
        to_html(&doc, &Content::new()),
        to_html(&doc, &Content::new())
    );
}
