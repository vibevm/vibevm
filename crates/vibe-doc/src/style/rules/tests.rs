use super::*;

use crate::style::banned::parse;
use crate::style::prose::nodes;

fn doc(body: &str) -> SpecDoc {
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
           <title id=\"root\">A page</title>\n{body}</spec>\n"
    );
    vibe_specdoc::from_xml_with(&xml, vibe_specdoc::Vocabulary::Doc).expect("fixture parses")
}

fn of(body: &str, f: impl Fn(&str, &Node) -> Vec<Finding>) -> Vec<Finding> {
    nodes(&doc(body))
        .iter()
        .flat_map(|n| f("a.xml", n))
        .collect()
}

fn words(n: usize) -> String {
    (1..=n).map(|i| format!("word{i} ")).collect()
}

#[test]
fn a_tic_is_an_error_and_a_deferral_is_its_own_rule() {
    let list = parse("robust\nsee the specification");
    let found = of(
        "<p>A robust design. For the rest, see the specification.</p>",
        |p, n| banned_words(p, n, &list),
    );
    let rules: Vec<Rule> = found.iter().map(|f| f.rule).collect();
    assert_eq!(rules, [Rule::Banned, Rule::Deferral]);
    assert!(found.iter().all(|f| f.severity == Severity::Error));
    // The finding quotes the sentence, not the word.
    assert!(found[0].text.contains("A robust design"), "{:?}", found[0]);
}

/// The one place a deferral is not a deferral: the quoted fact follows.
/// A citation is an addition, and this is what an addition looks like.
#[test]
fn a_deferral_whose_rule_block_follows_is_a_citation() {
    let list = parse("as described in");
    let with_rule = of(
        "<h title=\"H\"><p>A group name is a reversed domain, as described in the rule:</p>\
         <rule ref=\"spec://org.vibevm.core/vibevm/common/PROP-057#STYLE-LINT\"/></h>",
        |p, n| banned_words(p, n, &list),
    );
    assert!(with_rule.is_empty(), "{with_rule:?}");

    let alone = of(
        "<h title=\"H\"><p>A group name is a reversed domain, as described in the rule.</p></h>",
        |p, n| banned_words(p, n, &list),
    );
    assert_eq!(alone.len(), 1);
    assert_eq!(alone[0].rule, Rule::Deferral);
}

#[test]
fn a_step_over_twenty_words_is_an_error_and_a_corridor_over_thirty_five_is_a_warning() {
    let step = of(
        &format!(
            "<h title=\"H\"><list ordered=\"true\"><item>{}.</item></list></h>",
            words(21)
        ),
        length,
    );
    assert_eq!(step.len(), 1);
    assert_eq!(step[0].severity, Severity::Error);

    let corridor = of(&format!("<h title=\"H\"><p>{}.</p></h>", words(36)), length);
    assert_eq!(corridor.len(), 1);
    assert_eq!(corridor[0].severity, Severity::Warning);

    let short = of(&format!("<h title=\"H\"><p>{}.</p></h>", words(30)), length);
    assert!(short.is_empty(), "{short:?}");
}

/// Inside a procedure the limit is STE's, and it is an error: a step is
/// an instruction somebody follows with their hands busy.
#[test]
fn a_paragraph_in_a_procedure_is_held_to_twenty_five_words() {
    let body = format!(
        "<h title=\"H\"><p>1. Start here.</p><p>{}.</p></h>",
        words(26)
    );
    let found = of(&body, length);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].severity, Severity::Error);
    assert!(found[0].message.contains("25"), "{:?}", found[0]);
}

/// The prompt is the user's own voice, and X-028 leaves it alone.
#[test]
fn a_prompt_carries_no_sentence_limit() {
    let body = format!(
        "<prompt id=\"p\">{}.<outcome>{}.</outcome><assert>vibe check</assert></prompt>",
        words(60),
        words(40)
    );
    assert!(of(&body, length).is_empty());
}

#[test]
fn a_paragraph_of_more_than_six_sentences_is_a_warning() {
    let body = "<h title=\"H\"><p>One. Two. Three. Four. Five. Six. Seven.</p></h>";
    let found = of(body, length);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].rule, Rule::ParagraphLength);
    assert_eq!(found[0].severity, Severity::Warning);
}

#[test]
fn exclamation_marks_emoji_bold_and_a_second_dash_are_errors() {
    let found = of(
        "<h title=\"H\"><p>Ship it! Now — really — now. **Bold** and 🎉.</p></h>",
        |p, n| signs(p, n, "en"),
    );
    assert_eq!(found.len(), 4, "{found:?}");
    assert!(found.iter().all(|f| f.rule == Rule::Sign));
    assert!(found.iter().all(|f| f.severity == Severity::Error));
}

/// A cell is a container: bold marks a header and an arrow belongs in a
/// column of states.
#[test]
fn a_table_cell_is_left_alone_by_the_sign_rules() {
    let found = of(
        "<h title=\"H\"><table><tr><td>**Field**</td><td>a → b — c — d</td></tr></table></h>",
        |p, n| signs(p, n, "en"),
    );
    assert!(found.is_empty(), "{found:?}");
}

/// The manual's own typography is not an emoji.
#[test]
fn the_quotation_marks_and_the_ellipsis_are_not_signs() {
    let found = of(
        "<h title=\"H\"><p>It says «yes», and then …</p></h>",
        |p, n| signs(p, n, "en"),
    );
    assert!(found.is_empty(), "{found:?}");
}

/// In Russian the em dash is punctuation, not an aside (STYLE.md §10): a
/// paragraph that defines three things carries three of them by grammar.
#[test]
fn a_russian_paragraph_may_carry_as_many_dashes_as_its_grammar_needs() {
    let body = "<h title=\"H\"><p>Пакет — это папка. Манифест — её имя. Лок — её память.</p></h>";
    assert!(of(body, |p, n| signs(p, n, "ru")).is_empty());
    assert_eq!(of(body, |p, n| signs(p, n, "en")).len(), 1);
}

#[test]
fn the_five_forbidden_headings_are_errors_and_a_question_is_not() {
    let found = headings(
        "a.xml",
        &doc("<overview title=\"Overview\"><p>x.</p></overview>\
             <next title=\"Next steps\"><p>x.</p></next>\
             <q-offline title=\"I am on a plane. What works?\"><p>x.</p></q-offline>"),
    );
    assert_eq!(found.len(), 2);
    assert!(found.iter().all(|f| f.rule == Rule::Heading));
    assert_eq!(found[0].block, "overview");
}
