use crate::doc::{Block, SpecDoc};
use crate::from_markdown;

fn doc(md: &str) -> SpecDoc {
    from_markdown(md).expect("parses")
}

#[test]
fn title_status_and_preamble() {
    let d = doc(
        "# T {#t}\n\n<status stage=\"spec\" state=\"work\"/>\n\n@fact:A First. @status:impl/done\n",
    );
    assert_eq!(d.title.as_ref().map(|t| t.text.as_str()), Some("T"));
    assert_eq!(d.title.and_then(|t| t.id), Some("t".into()));
    assert_eq!(
        d.status.as_ref().map(|s| s.stage),
        Some(progress_core::model::Stage::Spec)
    );
    assert!(d.sections.is_empty());
    assert_eq!(d.preamble.len(), 1);
    match &d.preamble[0] {
        Block::Paragraph(u) => {
            assert_eq!(u.text, "First.");
            let f = u.fact.as_ref().expect("fact");
            assert_eq!(f.id.as_deref(), Some("A"));
            assert!(f.status.is_some());
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn heading_without_h1_is_all_preamble() {
    let d = doc("Just prose, no heading at all.\n\nSecond paragraph.\n");
    assert!(d.title.is_none());
    assert_eq!(d.preamble.len(), 2);
}

#[test]
fn legacy_anchor_spelling_is_stripped_too() {
    let d = doc("# H {#h}\n\n##OLD Legacy claim. @impl/done\n");
    match &d.preamble[0] {
        Block::Paragraph(u) => {
            assert_eq!(u.text, "Legacy claim.");
            assert_eq!(
                u.fact.as_ref().and_then(|f| f.id.clone()),
                Some("OLD".into())
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn nested_sections_follow_heading_levels() {
    let d = doc("# T {#t}\n\n## A {#a}\n\ntext\n\n### B {#b}\n\ndeep\n\n## C {#c}\n\nafter\n");
    let ids: Vec<(Option<&str>, usize)> = d
        .sections
        .iter()
        .map(|s| (s.id.as_deref(), s.sections.len()))
        .collect();
    assert_eq!(
        ids,
        vec![(Some("a"), 1), (Some("c"), 0)],
        "B nests under A, C closes both"
    );
    assert_eq!(d.sections[0].sections[0].title, "B");
    assert_eq!(d.sections[0].blocks.len(), 1);
}

#[test]
fn second_h1_is_a_top_level_section() {
    let d = doc("# First {#f}\n\n## Under first {#u}\n\n# Second {#s}\n\nbody\n");
    assert_eq!(d.title.unwrap().id, Some("f".into()));
    let titles: Vec<&str> = d.sections.iter().map(|s| s.title.as_str()).collect();
    assert_eq!(titles, ["Under first", "Second"]);
}

#[test]
fn task_box_rides_at_the_head_of_the_item_text() {
    let d = doc("# H {#h}\n\n- [x] done thing\n- [ ] open thing\n");
    match &d.preamble[0] {
        Block::List { ordered, items } => {
            assert!(!ordered);
            assert_eq!(items[0].text, "[x] done thing");
            assert_eq!(items[1].text, "[ ] open thing");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn marker_at_first_token_moves_to_clean_text() {
    let d = doc("# H {#h}\n\n@fact:FIRST @impl/plan Words later.\n");
    match &d.preamble[0] {
        Block::Paragraph(u) => {
            assert_eq!(u.text, "Words later.");
            assert_eq!(
                u.fact.as_ref().unwrap().status.as_ref().unwrap().state,
                progress_core::model::State::Plan
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn quote_unit_strips_the_prefix_keeps_the_fact() {
    let d = doc("# H {#h}\n\n> ##Q A quoted norm. @spec/done\n");
    match &d.preamble[0] {
        Block::Quote(u) => {
            assert_eq!(u.text, "A quoted norm.");
            assert_eq!(u.fact.as_ref().and_then(|f| f.id.clone()), Some("Q".into()));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn typed_fact_binds_the_fence_below_it() {
    let d = doc("# H {#h}\n\n@fact/code:RUN run this @impl/done\n\n```bash\ncargo test\n```\n");
    let blocks = &d.preamble;
    match &blocks[1] {
        Block::Fence { lang, fact, text } => {
            assert_eq!(lang.as_deref(), Some("bash"));
            assert_eq!(fact.as_deref(), Some("RUN"));
            assert_eq!(text, "cargo test");
        }
        other => panic!("{other:?}"),
    }
    match &blocks[0] {
        Block::Paragraph(u) => assert_eq!(u.fact.as_ref().unwrap().id.as_deref(), Some("RUN")),
        other => panic!("{other:?}"),
    }
}

#[test]
fn source_markup_errors_are_loud() {
    let err = from_markdown("# H {#h}\n\n@fact:DUP one\n\n@fact:DUP two\n").unwrap_err();
    assert!(err.message.contains("twice"), "{}", err.message);
}

#[test]
fn thematic_break_and_comment_do_not_enter_the_ir() {
    let d = doc("# H {#h}\n\n---\n\n<!-- just a note -->\n\nReal prose.\n");
    assert_eq!(d.preamble.len(), 1);
}
