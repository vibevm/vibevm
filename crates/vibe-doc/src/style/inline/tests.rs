use super::*;

/// The reason this module exists: a field name in code font is not the
/// marketing word that shares its spelling. A scan of the raw text would
/// tell the author of `[requires] capabilities` to rename the product.
#[test]
fn a_code_span_becomes_one_mask_and_none_of_its_words() {
    let read = read(
        "In the manifest: `[requires] capabilities = [\"ui:page-host@^1\"]` and nothing else.",
    );
    assert!(!read.text.contains("capabilities"), "{}", read.text);
    assert_eq!(read.text.matches(CODE_MASK).count(), 1);
}

#[test]
fn a_link_keeps_its_words_and_surrenders_its_target() {
    let read = read("Open the [lock file](../glossary/index.xml#lock-file) now.");
    assert_eq!(read.text, "Open the lock file now.");
    assert_eq!(read.links.len(), 1);
    assert_eq!(read.links[0].target, "../glossary/index.xml#lock-file");
    let at = read.links[0].range;
    assert_eq!(&read.text[at.0..at.1], "lock file");
}

#[test]
fn a_link_whose_text_holds_code_masks_that_code_too() {
    let read = read("See [`vibe install`](../howto/install-a-package.xml).");
    assert_eq!(read.text.matches(CODE_MASK).count(), 1);
    assert_eq!(read.links.len(), 1);
}

#[test]
fn italics_are_located_and_bold_is_counted() {
    let read = read("A *manifest* is the file; **never** write bold in prose.");
    assert_eq!(
        read.text,
        "A manifest is the file; never write bold in prose."
    );
    assert_eq!(read.bold, 2);
    let at = read.text.find("manifest").expect("kept");
    assert!(read.is_italic((at, at + "manifest".len())));
}

/// An asterisk that never closes is a literal asterisk. Treating it as an
/// italic running to the end of the paragraph would mark every term after
/// it as «introduced», which is the one way this rule could go quiet.
#[test]
fn an_unpaired_asterisk_opens_no_italic() {
    let read = read("A star * and a lock file after it.");
    assert!(read.italic.is_empty());
}

/// A parenthesis after a bracket is not a link, and an unterminated
/// backtick is a backtick: neither may swallow the rest of the paragraph,
/// because the findings behind it would vanish with it.
#[test]
fn brackets_and_backticks_that_are_prose_stay_prose() {
    let read = read("The set [a, b] (two of them) and a stray ` tick.");
    assert!(read.links.is_empty());
    assert!(read.text.contains("[a, b] (two of them)"), "{}", read.text);
    assert!(read.text.contains('`'), "{}", read.text);
}
