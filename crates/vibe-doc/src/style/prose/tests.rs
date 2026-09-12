use super::*;

fn doc(body: &str) -> SpecDoc {
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
           <title id=\"root\">A page</title>\n{body}</spec>\n"
    );
    vibe_specdoc::from_xml_with(&xml, vibe_specdoc::Vocabulary::Doc).expect("the fixture parses")
}

#[test]
fn every_node_carries_the_block_number_the_margin_shows() {
    let nodes = nodes(&doc("<p>One.</p><p>Two.</p>"));
    let paragraphs: Vec<&Node> = nodes.iter().filter(|n| n.kind == Kind::Paragraph).collect();
    assert_eq!(paragraphs[0].block, "p01");
    assert_eq!(paragraphs[1].block, "p02");
    assert_eq!(nodes[0].block, "title");
}

/// A page's first paragraph is the one `llms.txt` prints and the one the
/// skeleton law keeps free of terms. It is the preamble's first, never a
/// section's.
#[test]
fn the_intro_is_the_first_paragraph_of_the_preamble() {
    let nodes = nodes(&doc("<p>Intro.</p><s title=\"S\"><p>Body.</p></s>"));
    let intros: Vec<&Node> = nodes.iter().filter(|n| n.intro).collect();
    assert_eq!(intros.len(), 1);
    assert_eq!(intros[0].inline.text, "Intro.");
}

/// Read the procedure from the shape, not from the heading: the corpus
/// says «By hand», and nothing stops an author saying «The steps».
#[test]
fn a_section_of_numbered_steps_is_a_procedure_whatever_it_is_called() {
    let nodes = nodes(&doc(
        "<the-steps title=\"The steps\"><p>1. Run it.</p><p>2. Check it.</p></the-steps>\
         <why title=\"Why\"><p>Because.</p></why>",
    ));
    assert!(
        nodes
            .iter()
            .filter(|n| n.section == "The steps")
            .all(|n| n.procedural)
    );
    assert!(
        nodes
            .iter()
            .filter(|n| n.section == "Why")
            .all(|n| !n.procedural)
    );
}

#[test]
fn an_ordered_list_makes_a_procedure_and_each_item_is_a_step() {
    let nodes = nodes(&doc(
        "<how title=\"How\"><list ordered=\"true\"><item>Build it.</item>\
         <item>Ship it.</item></list></how>",
    ));
    let steps: Vec<&Node> = nodes.iter().filter(|n| n.kind == Kind::Step).collect();
    assert_eq!(steps.len(), 2);
    assert!(steps[0].procedural);
    // Both items are in one block, so both carry its number.
    assert_eq!(steps[0].block, steps[1].block);
}

/// A table is a container: its cells are read for tics and for nothing
/// else, which is what keeps a reference table from being told that it
/// names three terms in four words.
#[test]
fn a_table_cell_is_a_container_and_a_paragraph_is_a_corridor() {
    let nodes = nodes(&doc(
        "<t title=\"T\"><table><tr><td>Field</td><td>Meaning</td></tr></table><p>Prose.</p></t>",
    ));
    let cells: Vec<&Node> = nodes.iter().filter(|n| n.kind == Kind::Cell).collect();
    assert_eq!(cells.len(), 2);
    assert!(!cells[0].kind.is_corridor());
    assert!(
        nodes
            .iter()
            .any(|n| n.kind == Kind::Paragraph && n.kind.is_corridor())
    );
}

#[test]
fn a_prompt_yields_its_body_and_both_companions() {
    let nodes = nodes(&doc(
        "<prompt id=\"p\">Do the thing.<needs>a skill</needs><outcome>it worked</outcome>\
         <assert>vibe check</assert></prompt>",
    ));
    let kinds: Vec<Kind> = nodes.iter().map(|n| n.kind).collect();
    assert!(kinds.contains(&Kind::Prompt));
    assert!(kinds.contains(&Kind::Needs));
    assert!(kinds.contains(&Kind::Outcome));
}

/// The containers that carry no prose of their own must yield no node:
/// an example is a command and its golden output, and a rule's text is
/// fetched from the specification at build time.
#[test]
fn examples_rules_and_fences_carry_no_prose() {
    let nodes = nodes(&doc(
        "<x title=\"X\"><example id=\"e\" fixture=\"none\"><run>vibe --version</run>\
         <expect>vibe 1.0.0</expect></example>\
         <rule ref=\"spec://org.vibevm.core/vibevm/common/PROP-057#STYLE-LINT\"/>\
         <fence lang=\"text\">raw</fence></x>",
    ));
    assert!(nodes.iter().all(|n| n.kind == Kind::Title), "{nodes:?}");
}
