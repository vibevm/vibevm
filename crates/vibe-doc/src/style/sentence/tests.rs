use super::*;

fn texts(text: &str) -> Vec<String> {
    split(text).into_iter().map(|s| s.text).collect()
}

#[test]
fn a_full_stop_ends_a_sentence_and_a_run_of_marks_ends_one_too() {
    assert_eq!(texts("One. Two! Three?"), ["One.", "Two!", "Three?"]);
    assert_eq!(texts("Really?! Yes."), ["Really?!", "Yes."]);
}

/// The corpus writes a procedure as numbered steps inside one paragraph.
/// A splitter that broke on the step number would report a two-word
/// sentence where a reader sees a step, and the length rule would then be
/// measuring the wrong thing.
#[test]
fn a_step_number_does_not_close_a_sentence() {
    let steps = texts("1. Build the package. 2. Check that the token file exists.");
    assert_eq!(steps.len(), 2);
    assert!(steps[1].starts_with("2."), "{steps:?}");
}

#[test]
fn an_abbreviation_keeps_its_sentence() {
    assert_eq!(
        texts("Use a range, e.g. ^1.0, and stop."),
        ["Use a range, e.g. ^1.0, and stop."]
    );
}

#[test]
fn a_version_number_does_not_split_a_sentence() {
    assert_eq!(texts("It pins 1.0.0 exactly.").len(), 1);
}

#[test]
fn the_range_of_a_sentence_addresses_the_text_it_came_from() {
    let text = "First one. Second one.";
    let s = split(text);
    assert_eq!(&text[s[1].range.0..s[1].range.1], "Second one.");
}

/// A command is one word. Counting the parts of `vibe install
/// org.vibevm.world/wal` would push every sentence that shows a command
/// over the limit, for a reason no author could act on.
#[test]
fn a_masked_code_span_counts_as_one_word() {
    let masked = crate::style::inline::read("Run `vibe install org.vibevm.world/wal` now.");
    assert_eq!(count_words(&masked.text), 3);
}

#[test]
fn punctuation_alone_is_not_a_word() {
    assert_eq!(count_words("one — two"), 2);
}

#[test]
fn the_readability_of_long_words_in_long_sentences_is_higher() {
    let easy = readability("Run it. Check it. Ship it.");
    let hard = readability(
        "The resolver materialises the declared closure into the dependency directory once \
         the index feed is reconciled with the recorded lock pins.",
    );
    assert!(hard > easy, "{hard} should be harder than {easy}");
}
