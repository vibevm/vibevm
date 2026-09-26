//! The description a folded rule shows, branch by branch
//! (`##READER-RULE-FOLDED`).
//!
//! Every case here is a real shape the corpus writes: a bold lead that
//! summarises the rule, the one-word label the decision records open
//! with, a lead that is a whole sentence, and a rule that begins in the
//! middle of its own argument. The line a reader ends up with is short
//! enough that a wrong one is invisible in a build and obvious in a test.

use super::*;

/// A lead of two to eight words was written to be read on its own, and it
/// is taken as it stands — no ellipsis, because nothing was cut.
#[test]
fn a_bold_lead_is_the_description() {
    assert_eq!(
        gist(
            "**Cited rules are folded** (added 2026-09-26). In the HTML island every `rule` \
             block is a disclosure that starts closed."
        )
        .as_deref(),
        Some("Cited rules are folded")
    );
}

/// A lead's trailing colon or full stop belongs to the sentence it opened,
/// not to the line that repeats it.
#[test]
fn a_leads_own_punctuation_does_not_travel() {
    assert_eq!(
        gist(
            "**The layout is the address map:** a page lives under its group, its name and \
             its version, with the projections beside it as files."
        )
        .as_deref(),
        Some("The layout is the address map")
    );
}

/// A one-word lead names a KIND of statement rather than its subject, so
/// the line keeps what follows it.
#[test]
fn a_one_word_lead_keeps_the_words_after_it() {
    assert_eq!(
        gist(
            "**Decision:** a cited rule folds to one line in the HTML island, described in \
             the rule's own words."
        )
        .as_deref(),
        Some("Decision: a cited rule folds…")
    );
}

/// A lead of nine words or more is a sentence, and a sentence is cut.
#[test]
fn a_long_lead_is_cut() {
    assert_eq!(
        gist(
            "**The language of a doc package is the existing canonical of PROP-003:** there \
             is no separate field, and none is wanted."
        )
        .as_deref(),
        Some("The language of a doc package…")
    );
}

/// A bold run holding nothing but punctuation labels nothing, and the
/// rule's first words answer instead.
#[test]
fn a_lead_of_punctuation_is_no_lead() {
    assert_eq!(
        gist(
            "**.** The island keeps every variant of a conditional slot and marks it, never \
             dropping one."
        )
        .as_deref(),
        Some("The island keeps every variant…")
    );
}

/// With no lead the rule gives up its first words, at most six.
#[test]
fn a_rule_with_no_lead_gives_its_first_words() {
    assert_eq!(
        gist(
            "Offline resolution is therefore computed against the lock file alone, and a \
             registry is never consulted."
        )
        .as_deref(),
        Some("Offline resolution is therefore computed against…")
    );
}

/// A line that would end on «of» or «and» gives the word up: it spends the
/// space and says nothing, and the ellipsis already says that the sentence
/// went on.
#[test]
fn a_description_never_ends_on_a_function_word() {
    assert_eq!(
        gist("A build writes every artefact of the package in one walk over its pages.").as_deref(),
        Some("A build writes every artefact…")
    );
    // Two in a row, and both go.
    assert_eq!(
        gist("The block number rides at the head of the first item of the list.").as_deref(),
        Some("The block number rides…")
    );
}

/// The comma the sentence went on after is not punctuation of this line.
#[test]
fn a_cut_line_drops_the_punctuation_it_was_cut_at() {
    let described = gist(
        "One package is one language, and a translation of it is a package of its own \
         published beside it.",
    );
    assert_eq!(described.as_deref(), Some("One package is one language…"));
}

/// A rule of ten words or fewer is its own summary: describing it would
/// hide as much as the line said.
#[test]
fn a_short_rule_has_no_description() {
    // Ten words exactly.
    assert_eq!(
        gist("One package is one language and a translation is another."),
        None
    );
    // The same sentence, one clause longer.
    assert_eq!(
        gist("One package is one language and a translation is a separate package.").as_deref(),
        Some("One package is one language…")
    );
    // A bold lead does not buy a short rule a line either.
    assert_eq!(
        gist("**Numbered blocks.** Every flow block is numbered."),
        None
    );
}

/// The description reads the text through the island's own inline grammar:
/// a code span, a link and an emphasis become their words, and no markup
/// reaches the line.
#[test]
fn inline_markup_becomes_its_text() {
    let described = gist(
        "The command `vibe doc build` writes the [Markdown projection](../guide/md.xml) of \
         every page, and *nothing else* changes.",
    )
    .expect("the rule is long enough to describe");
    assert_eq!(described, "The command vibe doc build writes…");
    for markup in ["<code>", "<a ", "**", "`", "*"] {
        assert!(!described.contains(markup), "{described}");
    }
}

/// What a rule QUOTES comes back as the characters it quoted: the grammar
/// escapes them on the way out and the description unescapes them, so a
/// rule about markup is described in markup.
#[test]
fn a_quoted_angle_bracket_survives_the_round_trip() {
    assert_eq!(
        gist(
            "The shell receives `<script type=\"application/json\">` and parses nothing of \
             what the island holds."
        )
        .as_deref(),
        Some("The shell receives <script type=\"application/json\">…")
    );
}

/// The line a rule cannot describe is the EDITION's own sentence, so it is
/// in the edition's language — and in English for a language no edition of
/// this manual is written in yet.
#[test]
fn the_generic_line_is_the_editions_own() {
    assert_eq!(generic("ru"), "цитата из спецификации");
    assert_eq!(generic("en"), "quote from the specification");
    assert_eq!(generic("de"), "quote from the specification");
}
