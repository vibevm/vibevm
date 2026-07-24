use super::*;

const DOC: &str = "\
preamble line
# Title {#root}
intro under title
## First {#first}
first body
### Deep {#deep}
deep body
## Second {#second}
second body
";

#[test]
fn builds_hierarchy() {
    let t = DocTree::parse(DOC);
    let root = t.root();
    // One top-level heading (Title) under the synthetic root.
    let top = t.children(root);
    assert_eq!(top.len(), 1);
    let title = top[0];
    assert_eq!(t.node(title).id.as_deref(), Some("root"));
    assert_eq!(t.node(title).level, 1);

    // Title owns First and Second (both h2).
    let under_title = t.children(title);
    assert_eq!(under_title.len(), 2);
    assert_eq!(t.node(under_title[0]).id.as_deref(), Some("first"));
    assert_eq!(t.node(under_title[1]).id.as_deref(), Some("second"));

    // First owns Deep (h3); Second owns nothing.
    assert_eq!(t.children(under_title[0]).len(), 1);
    assert_eq!(
        t.node(t.children(under_title[0])[0]).id.as_deref(),
        Some("deep")
    );
    assert!(t.children(under_title[1]).is_empty());
}

#[test]
fn find_by_anchor_and_heading_text() {
    let t = DocTree::parse(DOC);
    let deep = t.find_by_anchor("deep").unwrap();
    assert_eq!(t.node(deep).heading, "Deep");
    assert_eq!(t.node(deep).level, 3);
    assert!(t.find_by_anchor("missing").is_none());
}

#[test]
fn span_covers_subtree_and_stops_at_sibling() {
    let t = DocTree::parse(DOC);
    // `First` spans its own body plus `Deep`, and stops at `Second`.
    let first = t.find_by_anchor("first").unwrap();
    let text = t.text(first);
    assert!(text.contains("first body"));
    assert!(text.contains("### Deep"));
    assert!(text.contains("deep body"));
    assert!(!text.contains("Second"));
}

#[test]
fn root_spans_whole_document_including_preamble() {
    let t = DocTree::parse(DOC);
    let text = t.text(t.root());
    assert!(text.contains("preamble line"));
    assert!(text.contains("second body"));
}

#[test]
fn headings_in_fences_are_not_nodes() {
    let src = "\
# Real {#real}
```
# Fake heading in code
```
after
";
    let t = DocTree::parse(src);
    assert!(t.find_by_anchor("real").is_some());
    // The fenced `#` produced no node: Real has no children.
    let real = t.find_by_anchor("real").unwrap();
    assert!(t.children(real).is_empty());
    assert_eq!(t.len(), 2); // root + Real
}

#[test]
fn duplicate_anchor_keeps_first_and_reports() {
    let src = "\
# One {#dup}
a
# Two {#dup}
b
";
    let t = DocTree::parse(src);
    let first = t.find_by_anchor("dup").unwrap();
    assert_eq!(t.node(first).heading, "One");
    assert_eq!(t.duplicate_anchors(), &["dup".to_string()]);
}

#[test]
fn heading_without_anchor_has_none() {
    let t = DocTree::parse("# Plain heading\nbody\n");
    let top = t.children(t.root())[0];
    assert_eq!(t.node(top).id, None);
    assert_eq!(t.node(top).heading, "Plain heading");
}

#[test]
fn anchor_trailing_marker_is_captured() {
    let t = DocTree::parse("## Name {#tag} :replace\nbody\n");
    let n = t.find_by_anchor("tag").unwrap();
    assert_eq!(t.node(n).heading, "Name");
    assert_eq!(t.node(n).trailing, ":replace");
}

#[test]
fn hash_without_space_is_not_a_heading() {
    let t = DocTree::parse("#notaheading\ntext\n");
    assert_eq!(t.len(), 1); // root only
}

#[test]
fn resolve_flat_and_tree_path() {
    let t = DocTree::parse(DOC);
    // A single segment matches flat.
    assert_eq!(t.resolve_path(&["first".into()]), t.find_by_anchor("first"));
    // A tree path descends: `deep` is a child of `first`.
    let deep = t.resolve_path(&["first".into(), "deep".into()]).unwrap();
    assert_eq!(t.node(deep).heading, "Deep");
    // An empty path is the whole document.
    assert_eq!(t.resolve_path(&[]), Some(t.root()));
    // A wrong descent fails: `second` is a sibling of `first`, not a child.
    assert!(t.resolve_path(&["first".into(), "second".into()]).is_none());
    // A missing first segment fails.
    assert!(t.resolve_path(&["nope".into()]).is_none());
}

#[test]
fn paragraph_fact_is_a_child_leaf_of_its_section() {
    let t = DocTree::parse("## Sec {#sec}\n##fact-a a refined statement\n");
    let sec = t.find_by_anchor("sec").unwrap();
    let fact = t.find_by_anchor("fact-a").unwrap();
    assert_eq!(t.node(fact).kind, NodeKind::Fact);
    assert_eq!(t.node(fact).parent, Some(sec));
    assert!(t.children(sec).contains(&fact));
    // The leaf's span is its own line, and text() slices exactly it.
    assert_eq!(t.text(fact), "##fact-a a refined statement");
}

#[test]
fn list_item_and_nested_facts_attach_to_the_enclosing_section() {
    let src = "\
# Top {#top}
- ##one first item
  - ##two nested item
";
    let t = DocTree::parse(src);
    let top = t.find_by_anchor("top").unwrap();
    let one = t.find_by_anchor("one").unwrap();
    let two = t.find_by_anchor("two").unwrap();
    assert_eq!(t.node(one).kind, NodeKind::Fact);
    assert_eq!(t.node(two).kind, NodeKind::Fact);
    // Both nest under the enclosing section, at any indent.
    assert_eq!(t.node(one).parent, Some(top));
    assert_eq!(t.node(two).parent, Some(top));
}

#[test]
fn facts_in_fences_are_not_nodes() {
    let src = "\
## Sec {#sec}
```
##fake-fact in code
```
after
";
    let t = DocTree::parse(src);
    assert!(t.find_by_anchor("fake-fact").is_none());
}

#[test]
fn invalid_fact_id_after_hashes_is_prose() {
    // `##9bad` (non-letter head) and `##bad!` (glued glyph) mint no node; they
    // are consecutive lines, so they form one non-fact paragraph.
    let t = DocTree::parse("## Sec {#sec}\n##9bad here\n\n##bad! there\n");
    assert!(t.find_by_anchor("9bad").is_none());
    assert!(t.find_by_anchor("bad").is_none());
    assert!(t.duplicate_anchors().is_empty());
    // The section has no fact children.
    let sec = t.find_by_anchor("sec").unwrap();
    assert!(t.children(sec).is_empty());
}

#[test]
fn fact_and_heading_share_one_namespace() {
    // A fact id colliding with a heading anchor is a recorded duplicate —
    // exactly as heading-vs-heading is.
    let t = DocTree::parse("## Dup {#dup}\nbody\n## Other {#other}\n##dup again\n");
    assert_eq!(t.duplicate_anchors(), &["dup".to_string()]);
    // The first occurrence (the heading) wins the flat index.
    let first = t.find_by_anchor("dup").unwrap();
    assert_eq!(t.node(first).kind, NodeKind::Heading);
}

#[test]
fn a_fact_resolves_like_any_node() {
    let t = DocTree::parse("# Doc {#root}\n##fact-x the unit\n");
    let fact = t.resolve_path(&["fact-x".into()]).unwrap();
    assert_eq!(t.node(fact).kind, NodeKind::Fact);
    assert_eq!(t.text(fact), "##fact-x the unit");
}

#[test]
fn facts_under_lists_a_sections_subtree_facts() {
    let src = "\
# Root {#root}
##lead-fact intro
## Sub {#sub}
- ##item-fact under sub
";
    let t = DocTree::parse(src);
    let root = t.find_by_anchor("root").unwrap();
    let ids: Vec<&str> = t.facts_under(root).into_iter().map(|(_, a)| a).collect();
    // Both the section's own fact and the sub-section's fact, doc-ordered.
    assert_eq!(ids, ["lead-fact", "item-fact"]);
}

#[test]
fn text_without_drops_named_fact_spans() {
    // Blank lines separate the two paragraph facts (a paragraph anchors only on
    // its first token, so consecutive `##…` lines would be one unit).
    let src = "\
## Sec {#sec}
prose before

##drop-me overridden statement

##keep-me surviving statement
";
    let t = DocTree::parse(src);
    let sec = t.find_by_anchor("sec").unwrap();
    let drop = t.find_by_anchor("drop-me").unwrap();
    let out = t.text_without(sec, &[drop]);
    assert!(out.contains("prose before"));
    assert!(out.contains("##keep-me"));
    assert!(!out.contains("##drop-me"), "overridden span kept: {out}");
}
