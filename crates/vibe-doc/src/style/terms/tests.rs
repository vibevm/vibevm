use super::*;

use crate::glossary::Glossary;
use crate::style::glossary;
use crate::style::prose;

/// The glossary these tests are judged against: three terms on the page the
/// linter was TOLD about, rather than on a path it guessed
/// (`##GLOSSARY-DECLARED`).
fn declared() -> Glossary {
    let dir = tempfile::tempdir().expect("temp dir");
    std::fs::write(
        dir.path().join("vibe.toml"),
        "[glossary]\npage = \"glossary/index\"\n",
    )
    .expect("write");
    let pages = dir.path().join("vibevm/vibespecs/glossary");
    std::fs::create_dir_all(&pages).expect("page dir");
    std::fs::write(
        pages.join("index.xml"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
           <title id=\"root\">Glossary</title>\n  \
           <lock-file title=\"lock file\"><p>A record.</p></lock-file>\n  \
           <registry title=\"registry\"><p>A place.</p></registry>\n  \
           <store title=\"store\"><p>A cache.</p></store>\n\
         </spec>\n",
    )
    .expect("write");
    let set = crate::pages::read_package(dir.path()).expect("read");
    crate::glossary::read(dir.path(), &set)
        .expect("read")
        .expect("the glossary is declared and present")
}

fn glossary_terms() -> Vec<glossary::Term> {
    glossary::terms(Some(&declared()))
}

fn findings(body: &str) -> Vec<Finding> {
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
           <title id=\"root\">A page</title>\n{body}</spec>\n"
    );
    let doc =
        vibe_specdoc::from_xml_with(&xml, vibe_specdoc::Vocabulary::Doc).expect("fixture parses");
    check(
        "model/a-page.xml",
        &prose::nodes(&doc),
        &glossary_terms(),
        Some(&declared()),
    )
}

fn rules_of(body: &str) -> Vec<Rule> {
    findings(body).into_iter().map(|f| f.rule).collect()
}

/// The first paragraph says what the page is for to a stranger, and it is
/// the page's line in `llms.txt`. A term there is an error whatever
/// follows it.
#[test]
fn a_term_in_the_first_paragraph_is_an_error() {
    let found = findings("<p>This page explains the lock file, the record of versions.</p>");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].rule, Rule::TermInFirstParagraph);
    assert!(found[0].message.contains("lock file"), "{:?}", found[0]);
}

#[test]
fn a_first_use_linked_to_the_glossary_entry_introduces_the_term() {
    let rules = rules_of(
        "<p>Intro without any term at all.</p>\
         <s title=\"S\"><p>The [lock file](../glossary/index.xml#lock-file) is written.</p></s>",
    );
    assert!(rules.is_empty(), "{rules:?}");
}

#[test]
fn a_first_use_in_the_italics_of_a_definition_introduces_the_term() {
    let rules = rules_of(
        "<p>Intro without any term at all.</p>\
         <s title=\"S\"><p>This file is the *lock file*.</p></s>",
    );
    assert!(rules.is_empty(), "{rules:?}");
}

#[test]
fn a_gloss_in_the_same_sentence_introduces_the_term() {
    let rules = rules_of(
        "<p>Intro without any term at all.</p>\
         <s title=\"S\"><p>The lock file, the record of which exact versions this \
         build used, is written here.</p></s>",
    );
    assert!(rules.is_empty(), "{rules:?}");
}

/// The loose test — «is there a comma anywhere in the sentence» — reports
/// a page as clean whenever it happens to use a comma. The gloss has to
/// follow the term.
#[test]
fn a_comma_somewhere_else_in_the_sentence_is_not_a_gloss() {
    let rules = rules_of(
        "<p>Intro without any term at all.</p>\
         <s title=\"S\"><p>Once the build finishes, the lock file is written.</p></s>",
    );
    assert_eq!(rules, [Rule::TermBeforeIntroduction]);
}

#[test]
fn only_the_first_use_on_a_page_is_judged() {
    let found = findings(
        "<p>Intro without any term at all.</p>\
         <s title=\"S\"><p>A store is here.</p><p>The store again, and again the store.</p></s>",
    );
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].rule, Rule::TermBeforeIntroduction);
}

/// Three terms in one sentence means the sentence is a container in
/// disguise. It is a warning, because a corridor is where the author's
/// judgement lives.
#[test]
fn three_terms_in_one_sentence_is_a_warning() {
    let found = findings(
        "<p>Intro without any term at all.</p>\
         <s title=\"S\"><p>The [lock file](../glossary/index.xml#lock-file) names the \
         [registry](../glossary/index.xml#registry) that filled the \
         [store](../glossary/index.xml#store).</p></s>",
    );
    let density: Vec<&Finding> = found
        .iter()
        .filter(|f| f.rule == Rule::TermsPerSentence)
        .collect();
    assert_eq!(density.len(), 1);
    assert_eq!(density[0].severity, Severity::Warning);
}

/// Judging the glossary's own entries against «introduce before use»
/// would ask each entry to link to itself.
#[test]
fn the_glossary_page_is_not_judged_against_itself() {
    let xml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
               <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
                 <title id=\"root\">Glossary</title>\n  \
                 <p>The lock file and the registry and the store.</p>\n\
               </spec>\n";
    let doc = vibe_specdoc::from_xml_with(xml, vibe_specdoc::Vocabulary::Doc).expect("parses");
    assert!(
        check(
            "glossary/index.xml",
            &prose::nodes(&doc),
            &glossary_terms(),
            Some(&declared()),
        )
        .is_empty()
    );
}

/// A term inside a table cell is a reference table doing its job, and a
/// term in the user's voice is the user's voice.
#[test]
fn containers_and_prompts_do_not_carry_a_first_use() {
    let rules = rules_of(
        "<p>Intro without any term at all.</p>\
         <s title=\"S\"><table><tr><td>lock file</td><td>the store</td></tr></table>\
         <prompt id=\"p\">Open the lock file in the store.<assert>vibe check</assert></prompt></s>",
    );
    assert!(rules.is_empty(), "{rules:?}");
}

/// The link counts whatever its own text says: an adaptation links the
/// term in the case its sentence needs, and this linter inflects English
/// and nothing else. From the link on, the term is introduced.
#[test]
fn a_link_to_the_entry_introduces_the_term_whatever_its_text_says() {
    let rules = rules_of(
        "<p>Intro without any term at all.</p>\
         <s title=\"S\"><p>Записано в [лок-файле](../glossary/index.xml#lock-file); then the \
         lock file is read.</p><p>The lock file again.</p></s>",
    );
    assert!(rules.is_empty(), "{rules:?}");
}

/// A link that comes after the first use sent the reader nowhere yet.
#[test]
fn a_link_that_comes_after_the_first_use_does_not_introduce_it() {
    let rules = rules_of(
        "<p>Intro without any term at all.</p>\
         <s title=\"S\"><p>The lock file is read, see \
         [the record](../glossary/index.xml#lock-file).</p></s>",
    );
    assert_eq!(rules, [Rule::TermBeforeIntroduction]);
}

/// `#registry` is not `#index-registry`: the fragment is matched whole.
#[test]
fn a_link_to_another_entry_introduces_nothing() {
    let rules = rules_of(
        "<p>Intro without any term at all.</p>\
         <s title=\"S\"><p>See the [index](../glossary/index.xml#index-registry); the \
         registry is here.</p></s>",
    );
    assert_eq!(rules, [Rule::TermBeforeIntroduction]);
}

/// A code span may stand between the term and its gloss: in Russian the
/// qualifier follows the noun — «семейство `phase:`, группа точек…».
#[test]
fn a_code_span_between_the_term_and_its_gloss_is_part_of_the_phrase() {
    let rules = rules_of(
        "<p>Intro without any term at all.</p>\
         <s title=\"S\"><p>The store `main`, the place where every fetched byte is kept, \
         fills up.</p></s>",
    );
    assert!(rules.is_empty(), "{rules:?}");
}
