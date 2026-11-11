//! The mirror merge rule, over values: one table, two authors.
//!
//! Nothing here opens a registry. The question the rule answers is
//! "who wrote this row last", and the only inputs it needs are the
//! source's table, the target's table and what the previous run
//! recorded carrying — which is exactly what the mirror record keeps.

use super::*;

fn group(s: &str) -> vibe_core::Group {
    vibe_core::Group::parse(s).expect("fixture group parses")
}

fn target(spelling: &str) -> AliasTarget {
    let (g, name) = spelling.split_once('/').expect("fixture is qualified");
    AliasTarget {
        group: group(g),
        name: name.to_owned(),
    }
}

fn table(rows: &[(&str, &str)]) -> AliasTable {
    let mut t = AliasTable::default();
    for (alias, spelling) in rows {
        t.insert(*alias, target(spelling));
    }
    t
}

fn recorded(rows: &[(&str, &str)]) -> Vec<(String, String)> {
    rows.iter()
        .map(|(a, t)| ((*a).to_owned(), (*t).to_owned()))
        .collect()
}

/// Every target package was mirrored.
fn all(_: &AliasTarget) -> bool {
    true
}

fn rows(table: &AliasTable) -> Vec<(String, String)> {
    table
        .iter()
        .map(|(alias, target)| (alias.clone(), aliases::spell(target)))
        .collect()
}

#[test]
fn a_first_run_copies_the_source_table() {
    let merge = mirrored(
        &table(&[("qwen", "org.vibevm.qwen/qwen-code")]),
        &AliasTable::default(),
        &[],
        all,
    );
    assert_eq!(
        rows(&merge.table),
        recorded(&[("qwen", "org.vibevm.qwen/qwen-code")])
    );
    assert_eq!(merge.recorded().len(), 1);
    assert!(merge.conflicts.is_empty());
}

#[test]
fn a_source_row_whose_package_was_not_mirrored_is_dropped_with_a_note() {
    // A spelling pointing at a package the mirror does not carry would
    // resolve to nothing, so it is left behind — and the run says so,
    // rather than the operator finding out from a failed install.
    let merge = mirrored(
        &table(&[
            ("qwen", "org.vibevm.qwen/qwen-code"),
            ("vim", "org.vibevm.vim/vim"),
        ]),
        &AliasTable::default(),
        &[],
        |t| t.name == "qwen-code",
    );
    assert_eq!(
        rows(&merge.table),
        recorded(&[("qwen", "org.vibevm.qwen/qwen-code")])
    );
    assert_eq!(
        merge.dropped,
        recorded(&[("vim", "org.vibevm.vim/vim")]),
        "the dropped row must be nameable, not silent"
    );
}

#[test]
fn an_untouched_row_takes_the_sources_new_value() {
    // The mirror owner never edited it, so the source is still its
    // author and a re-point upstream arrives here.
    let merge = mirrored(
        &table(&[("qwen", "com.acme/qwen-fork")]),
        &table(&[("qwen", "org.vibevm.qwen/qwen-code")]),
        &recorded(&[("qwen", "org.vibevm.qwen/qwen-code")]),
        all,
    );
    assert_eq!(
        rows(&merge.table),
        recorded(&[("qwen", "com.acme/qwen-fork")])
    );
    assert!(merge.conflicts.is_empty());
}

#[test]
fn a_row_the_mirror_owner_edited_is_never_overwritten() {
    // The target's value differs from what the last run recorded
    // carrying, so a local hand changed it. The source does not win,
    // and the disagreement is reported rather than resolved.
    let merge = mirrored(
        &table(&[("qwen", "org.vibevm.qwen/qwen-code")]),
        &table(&[("qwen", "com.corp/qwen-internal")]),
        &recorded(&[("qwen", "org.vibevm.qwen/qwen-code")]),
        all,
    );
    assert_eq!(
        rows(&merge.table),
        recorded(&[("qwen", "com.corp/qwen-internal")]),
        "a later mirror run must not undo the mirror owner's edit"
    );
    assert_eq!(
        merge.conflicts,
        vec![(
            "qwen".to_string(),
            "com.corp/qwen-internal".to_string(),
            "org.vibevm.qwen/qwen-code".to_string(),
        )]
    );
}

#[test]
fn a_row_the_mirror_owner_authored_outright_is_left_alone() {
    // No record of ever carrying it: it is the mirror's own, and the
    // source happening to agree or disagree changes nothing.
    let merge = mirrored(
        &table(&[("qwen", "org.vibevm.qwen/qwen-code")]),
        &table(&[("qwen", "com.corp/qwen-internal")]),
        &[],
        all,
    );
    assert_eq!(
        rows(&merge.table),
        recorded(&[("qwen", "com.corp/qwen-internal")])
    );
    assert_eq!(merge.conflicts.len(), 1);
}

#[test]
fn a_row_the_source_retired_goes_when_the_mirror_never_touched_it() {
    let merge = mirrored(
        &AliasTable::default(),
        &table(&[("qwen", "org.vibevm.qwen/qwen-code")]),
        &recorded(&[("qwen", "org.vibevm.qwen/qwen-code")]),
        all,
    );
    assert!(merge.table.is_empty());
    assert_eq!(merge.withdrawn, vec!["qwen".to_string()]);
}

#[test]
fn a_row_the_source_retired_stays_when_the_mirror_had_made_it_its_own() {
    let merge = mirrored(
        &AliasTable::default(),
        &table(&[("qwen", "com.corp/qwen-internal")]),
        &recorded(&[("qwen", "org.vibevm.qwen/qwen-code")]),
        all,
    );
    assert_eq!(
        rows(&merge.table),
        recorded(&[("qwen", "com.corp/qwen-internal")]),
        "the mirror owner's row is theirs to retire"
    );
    assert!(merge.withdrawn.is_empty());
}

#[test]
fn an_unchanged_table_is_carried_but_not_rewritten() {
    let source = table(&[("qwen", "org.vibevm.qwen/qwen-code")]);
    let merge = mirrored(
        &source,
        &source,
        &recorded(&[("qwen", "org.vibevm.qwen/qwen-code")]),
        all,
    );
    assert_eq!(merge.table, source);
    assert!(merge.conflicts.is_empty());
    assert!(merge.withdrawn.is_empty());
    assert_eq!(
        merge.recorded(),
        recorded(&[("qwen", "org.vibevm.qwen/qwen-code")]),
        "an unchanged row is still recorded as carried, or the next run \
         would read it as a mirror-owner edit"
    );
}
