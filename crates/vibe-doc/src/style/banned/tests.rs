use super::*;

#[test]
fn comments_and_blank_lines_are_not_entries() {
    let list = parse("# the header\n\n  delve  \n\n# another\nnote that\n");
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].phrase, "delve");
}

#[test]
fn matching_is_case_insensitive_and_on_whole_words() {
    let list = parse("just\ndelve");
    let hits = scan(
        "Delve into it, and JUST adjust the justified column.",
        &list,
    );
    let phrases: Vec<&str> = hits.iter().map(|h| h.phrase.as_str()).collect();
    assert_eq!(phrases, ["delve", "just"]);
}

/// A phrase matches as a sequence of words, across whatever whitespace
/// the author wrapped it in — an XML page breaks a paragraph wherever the
/// line got long.
#[test]
fn a_phrase_matches_across_a_line_break() {
    let list = parse("it is important to note");
    assert_eq!(
        scan("Here it is\n  important to note that.", &list).len(),
        1
    );
}

/// One occurrence, one finding. «a number of» must not also be reported
/// as «a», and «see the specification» must not be reported twice
/// because it stands both in the list and in the deferral rule.
#[test]
fn the_longest_entry_wins_one_position() {
    let list = parse("a number of\na");
    let hits = scan("There are a number of them.", &list);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].phrase, "a number of");
}

#[test]
fn the_three_deferrals_are_marked_as_deferrals() {
    let list = parse("see the specification\ndelve");
    let hits = scan("For the rest, see the specification.", &list);
    assert_eq!(hits.len(), 1);
    assert!(hits[0].deferral);
    assert!(!parse("delve")[0].is_deferral());
}

/// The list is package data, per language. A package written in a
/// language it ships no list for is refused by name: an empty list would
/// report every page clean, which is the wrong kind of green.
#[test]
fn a_missing_list_is_refused_by_name() {
    let dir = tempfile::tempdir().expect("temp dir");
    let e = read(dir.path(), "ru").expect_err("refused");
    assert!(e.to_string().contains("banned.ru.txt"), "{e}");
    assert!(e.to_string().contains("STYLE-LINT"), "{e}");
}

#[test]
fn a_list_is_read_from_the_package_style_directory() {
    let dir = tempfile::tempdir().expect("temp dir");
    std::fs::create_dir_all(dir.path().join(STYLE_DIR)).expect("style dir");
    std::fs::write(list_path(dir.path(), "en"), "# c\nrobust\n").expect("write");
    let list = read(dir.path(), "en").expect("read");
    assert_eq!(list.lang, "en");
    assert_eq!(list.entries.len(), 1);
}
