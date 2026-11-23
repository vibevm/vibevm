use super::*;

/// Independent diagnostic of the unchanged default renderer.
#[test]
fn slot_plan_legacy_narrow_diagnostic() {
    let list = long_plan();
    for width in [32, 48] {
        let rendered = list.render_at(width);
        let plain = console::strip_ansi_codes(&rendered);
        eprintln!(
            "legacy width={width}, rows={plain:?}, cells={:?}",
            plain.lines().map(measure).collect::<Vec<_>>()
        );
    }
}

fn long_plan() -> List {
    List::new(vec![
        Row::new(vec![
            Cell::text("1."),
            Cell::text("contribution識別子-with-a-long-name"),
            Cell::new("(script處理器-with-a-long-name)", Role::Dim),
            Cell::text("Title標題 describing the complete contribution"),
        ])
        .meta(vec![
            Meta::note("metadata條件 requires a configured optional command"),
            Meta::note("optional"),
        ]),
    ])
    .meta_column()
}

#[test]
fn lossless_plan_retains_all_fields_at_narrow_widths() {
    let list = long_plan();
    for width in [32, 48] {
        let rendered = list.render_lossless_at(width);
        let plain = console::strip_ansi_codes(&rendered);
        eprintln!(
            "lossless width={width}, rows={plain:?}, cells={:?}",
            plain.lines().map(measure).collect::<Vec<_>>()
        );
        assert!(!plain.trim().is_empty());
        assert!(plain.lines().all(|line| measure(line) <= width));
        for field in [
            "contribution識別子",
            "script處理器",
            "Title標題",
            "metadata條件",
        ] {
            assert!(plain.contains(field), "missing {field}: {plain}");
        }
        let joined = plain.replace('\n', "");
        for cell in &list.rows[0].cells {
            assert!(joined.contains(&cell.text), "lost cell: {}", cell.text);
        }
        assert!(joined.contains("metadata條件 requires a configured optional command"));
        assert!(joined.contains("optional"));
    }
}

#[test]
fn lossless_wide_output_and_default_narrow_output_are_unchanged() {
    let list = long_plan();
    assert_eq!(list.render_lossless_at(200), list.render_at(200));
    assert_eq!(
        list.render_at(32),
        "  1.  contribution識別子-with-a…\n      metadata條件 requires a c…"
    );
    assert_eq!(
        list.render_at(48),
        "  1.  contribution識別子-with-a-long-name\n      metadata條件 requires a configured option…"
    );
}

#[test]
fn lossless_multiline_fields_keep_row_association() {
    let list = List::new(vec![
        Row::new(vec![Cell::text("FIRST"), Cell::text("CELLONE\nCELLTWO")])
            .meta(vec![Meta::note("METAONE\nMETATWO")])
            .child("CHILDONE\nCHILDTWO"),
        Row::new(vec![Cell::text("SECOND_DISTINCT"), Cell::text("TAIL")]),
    ])
    .meta_column();
    for width in [32, 48] {
        let rendered = list.render_lossless_at(width);
        let plain = console::strip_ansi_codes(&rendered);
        assert!(plain.lines().all(|line| measure(line) <= width), "{plain}");
        let joined = plain.replace('\n', "");
        for marker in [
            "FIRST",
            "CELLONE",
            "CELLTWO",
            "METAONE",
            "METATWO",
            "CHILDONE",
            "CHILDTWO",
            "SECOND_DISTINCT",
            "TAIL",
        ] {
            assert_eq!(joined.matches(marker).count(), 1, "{marker}: {plain}");
        }
        let markers = [
            "FIRST",
            "CELLONE",
            "CELLTWO",
            "METAONE",
            "METATWO",
            "CHILDONE",
            "CHILDTWO",
            "SECOND_DISTINCT",
            "TAIL",
        ];
        for pair in markers.windows(2) {
            assert!(
                joined.find(pair[0]).unwrap() < joined.find(pair[1]).unwrap(),
                "wrong ownership/order: {plain}"
            );
        }
    }
    assert_eq!(list.render_lossless_at(200), list.render_at(200));
}
