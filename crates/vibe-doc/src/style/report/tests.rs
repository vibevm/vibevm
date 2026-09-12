use super::*;

fn finding(page: &str, rule: Rule, severity: Severity) -> Finding {
    Finding {
        page: page.to_owned(),
        block: "p01".to_owned(),
        node: "p",
        rule,
        severity,
        message: "something".to_owned(),
        text: "some prose".to_owned(),
    }
}

fn page(name: &str, errors: usize, warnings: usize) -> PageScore {
    PageScore {
        page: name.to_owned(),
        errors,
        warnings,
        readability: 11.0,
    }
}

fn report(pages: Vec<PageScore>, findings: Vec<Finding>, min: u8) -> Report {
    Report {
        lang: "en".to_owned(),
        findings,
        pages,
        unreadable: Vec::new(),
        min_percent: min,
    }
}

/// Warnings never gate. A corridor sentence of thirty-six words is a
/// sentence somebody should look at, not a build somebody should stop.
#[test]
fn a_page_with_warnings_alone_is_clean() {
    let r = report(
        vec![page("a.xml", 0, 3)],
        vec![finding("a.xml", Rule::SentenceLength, Severity::Warning)],
        FULL,
    );
    assert_eq!(r.percent(), 100);
    assert!(r.ok());
    assert_eq!(r.warnings(), 1);
}

#[test]
fn the_bar_is_the_share_of_pages_that_carry_no_error() {
    let r = report(
        vec![page("a.xml", 1, 0), page("b.xml", 0, 0)],
        vec![finding("a.xml", Rule::Banned, Severity::Error)],
        FULL,
    );
    assert_eq!(r.percent(), 50);
    assert!(!r.ok());
    assert!(report(r.pages.clone(), r.findings.clone(), 50).ok());
}

/// A page the pivot refused hides its prose, and unknown is not clean.
#[test]
fn an_unreadable_page_is_counted_and_never_green() {
    let mut r = report(vec![page("a.xml", 0, 0)], Vec::new(), FULL);
    r.unreadable.push("b.xml".to_owned());
    assert_eq!(r.percent(), 50);
    assert!(!r.ok());
    assert!(
        r.render().contains("b.xml does not parse"),
        "{}",
        r.render()
    );
}

/// A run that is one page short of the bar must not print the bar.
#[test]
fn the_share_rounds_down() {
    let pages: Vec<PageScore> = (0..3)
        .map(|i| page(&format!("p{i}.xml"), usize::from(i == 0), 0))
        .collect();
    assert_eq!(report(pages, Vec::new(), FULL).percent(), 66);
}

#[test]
fn the_tally_names_every_rule_that_fired_and_no_other() {
    let r = report(
        vec![page("a.xml", 2, 1)],
        vec![
            finding("a.xml", Rule::Banned, Severity::Error),
            finding("a.xml", Rule::Banned, Severity::Error),
            finding("a.xml", Rule::TermsPerSentence, Severity::Warning),
        ],
        FULL,
    );
    let by_rule = r.by_rule();
    assert_eq!(by_rule.len(), 2);
    assert_eq!(by_rule[0], (Rule::Banned, 2, 0));
    assert_eq!(by_rule[1], (Rule::TermsPerSentence, 0, 1));
}

/// The finding is the queue entry: page, block, rule and the words that
/// fired it, on the line.
#[test]
fn a_rendered_finding_carries_the_page_the_block_and_the_prose() {
    let r = report(
        vec![page("model/a.xml", 1, 0)],
        vec![finding("model/a.xml", Rule::Banned, Severity::Error)],
        FULL,
    );
    let text = r.render();
    assert!(
        text.contains("ERROR model/a.xml p01 [p] banned-word"),
        "{text}"
    );
    assert!(text.contains("some prose"), "{text}");
    assert!(text.contains("readability (ARI)"), "{text}");
}

#[test]
fn a_long_quote_is_cut_and_marked() {
    let long = "word ".repeat(60);
    let cut = quote(&long);
    assert!(cut.ends_with('…'));
    assert_eq!(cut.chars().count(), QUOTE_LIMIT + 1);
}

const FULL: u8 = 100;
