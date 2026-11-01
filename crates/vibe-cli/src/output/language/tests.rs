//! The language's shapes, read with colour off (the default of a test
//! process) so every assertion is on plain text.

use std::time::Duration;

use super::block::LabelValue;
use super::width::measure;
use super::*;

#[test]
fn time_is_written_in_its_shortest_exact_form() {
    let cases = [
        (0, "0s"),
        (3, "3s"),
        (59, "59s"),
        (60, "1m"),
        (130, "2m 10s"),
        (3600, "1h"),
        (3840, "1h 4m"),
    ];
    for (seconds, text) in cases {
        assert_eq!(elapsed(Duration::from_secs(seconds)), text);
    }
    assert_eq!(elapsed(Duration::from_millis(400)), "0s");
}

#[test]
fn sizes_read_as_a_reader_says_them() {
    assert_eq!(bytes(512), "512 B");
    assert_eq!(bytes(12_400_000), "12.4 MB");
}

#[test]
fn a_long_path_is_shortened_in_its_middle_and_keeps_its_last_component() {
    let path = "/home/u/.vibe/cache/app-runs/org.vibevm.apps__vim/2026-11-01T10-51-02/";
    let short = meta::shorten(path, 48);
    assert!(short.chars().count() <= 48, "{short}");
    assert!(short.starts_with("/home/u/.vibe"), "{short}");
    assert!(short.ends_with("/…/2026-11-01T10-51-02/"), "{short}");
    assert_eq!(meta::shorten("/short/path", 48), "/short/path");
}

#[test]
fn metadata_joins_with_a_middle_dot_and_puts_state_words_last() {
    let text = join_meta(&[
        Meta::State("installed".into()),
        Meta::Time(Duration::from_secs(3)),
        Meta::Count("412 lines".into()),
    ]);
    assert_eq!(text, " · 3s · 412 lines  [installed]");
}

#[test]
fn a_rule_runs_to_the_width_and_carries_its_right_label() {
    let plain = rule::rule_at(
        "plan for org.example/tool@1.0.0",
        Some("installed by app"),
        60,
    );
    assert!(
        plain.starts_with("── plan for org.example/tool@1.0.0 ─"),
        "{plain}"
    );
    assert!(plain.ends_with("─ installed by app"), "{plain}");
    assert_eq!(measure(&plain), 60, "{plain}");
    let bare = rule::rule_at("answers", None, 40);
    assert_eq!(measure(&bare), 40, "{bare}");
}

#[test]
fn a_block_shares_one_left_edge_and_drops_metadata_before_cutting_a_name() {
    let block = Block::new(Glyph::Running, "build   Build vim")
        .with_meta(vec![Meta::Time(Duration::from_secs(14))])
        .with_body(vec![
            Line::Evidence("linking vim.exe".into()),
            Line::Child("requires org.example/gcc@15".into()),
        ]);
    assert_eq!(
        block.render_at(80),
        "● build   Build vim · 14s\n  │ linking vim.exe\n  └ requires org.example/gcc@15"
    );
    assert_eq!(fold_line("a very long child line", 10), "│ a very …");
    assert_eq!(hint("(l to show the live log)"), "(l to show the live log)");
    let narrow = Block::new(Glyph::Done, "org.example/a-very-long-identifier")
        .with_meta(vec![Meta::note("a note that does not fit")]);
    assert_eq!(narrow.render_at(40), "✓ org.example/a-very-long-identifier");
}

#[test]
fn a_refusal_puts_its_code_first_and_its_labels_in_one_column() {
    let refusal = Refusal {
        code: Some("consent_required".into()),
        headline: "1 remap package offer needs an answer".into(),
        why: Some("org.corp/intranet is offered".into()),
        fix: vec![
            "vibe remap accept org.corp/intranet".into(),
            "vibe remap reject org.corp/intranet".into(),
        ],
        see: Some("spec://org.vibevm.core/vibevm/common/PROP-066#consent".into()),
    };
    assert_eq!(
        refusal.block().render_at(100),
        "✗ consent_required  1 remap package offer needs an answer\n\
         \x20 why   org.corp/intranet is offered\n\
         \x20 fix   vibe remap accept org.corp/intranet\n\
         \x20       vibe remap reject org.corp/intranet\n\
         \x20 see   spec://org.vibevm.core/vibevm/common/PROP-066#consent"
    );
    let _ = LabelValue::Text(String::new());
}

#[test]
fn a_list_aligns_its_columns_over_every_row_and_drops_meta_then_title() {
    let rows = vec![
        Row::new(vec![
            Cell::text("1."),
            Cell::text("fetch"),
            Cell::text("(sources)"),
            Cell::text("Download the sources"),
        ]),
        Row::new(vec![
            Cell::text("2."),
            Cell::text("build"),
            Cell::text("(recipe)"),
            Cell::text("Build vim"),
        ])
        .meta(vec![Meta::note("needs command:gcc")])
        .child("requires org.example/gcc@15 (build)"),
    ];
    let list = List::new(rows).meta_column().children_under(1);
    assert_eq!(
        list.render_at(100),
        "  1.  fetch  (sources)  Download the sources\n\
         \x20 2.  build  (recipe)   Build vim             needs command:gcc\n\
         \x20     └ requires org.example/gcc@15 (build)"
    );
    let narrow = list.render_at(46);
    assert_eq!(
        narrow.lines().nth(2),
        Some("                        needs command:gcc"),
        "metadata that does not fit moves under the title: {narrow}"
    );
    assert!(narrow.contains("Download the sources"), "{narrow}");
    let narrower = list.render_at(30);
    assert!(!narrower.contains("Download"), "{narrower}");
    assert!(narrower.contains("fetch"), "{narrower}");
}

#[test]
fn inline_metadata_follows_the_last_cell() {
    let list = List::new(vec![
        Row::new(vec![
            Cell::text("fetch"),
            Cell::text("Download the sources"),
        ])
        .glyph(Glyph::Done, Role::Ok)
        .meta(vec![Meta::Time(Duration::from_secs(3))]),
        Row::new(vec![Cell::text("docs"), Cell::text("Copy")])
            .glyph(Glyph::Skipped, Role::Dim)
            .dim(true)
            .meta(vec![Meta::note("skipped: feature docs off")]),
    ]);
    assert_eq!(
        list.render_at(80),
        "✓ fetch  Download the sources · 3s\n○ docs   Copy · skipped: feature docs off"
    );
}

#[test]
fn the_ascii_set_spells_every_glyph_in_one_column() {
    let all = [
        Glyph::Running,
        Glyph::Done,
        Glyph::Failed,
        Glyph::Skipped,
        Glyph::Trailer,
        Glyph::Question,
        Glyph::Warning,
        Glyph::Fold,
        Glyph::Child,
    ];
    let ascii: Vec<&str> = all.iter().map(|g| g.in_set(GlyphSet::Ascii)).collect();
    assert_eq!(ascii, ["*", "+", "x", "-", "*", "?", "!", "|", "`"]);
    for glyph in all {
        assert_eq!(measure(glyph.in_set(GlyphSet::Unicode)), 1);
        assert!(glyph.in_set(GlyphSet::Ascii).is_ascii());
    }
}

#[test]
fn a_trailer_writes_its_path_in_full() {
    let line = trailer(
        true,
        "installed org.example/tool@1.0.0",
        vec![Meta::note("1 step"), Meta::Time(Duration::from_secs(134))],
        Some("/a/very/long/path/that/is/never/shortened/in/the/trailer/at/all/run"),
    );
    assert_eq!(
        line,
        "✻ installed org.example/tool@1.0.0 · 1 step · 2m 14s · \
         log /a/very/long/path/that/is/never/shortened/in/the/trailer/at/all/run"
    );
    assert_eq!(warning("the cache is stale"), "! the cache is stale");
}
