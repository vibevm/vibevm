use super::*;

#[test]
fn slot_plan_rows_are_a_measured_list() {
    let none: [String; 0] = [];
    let rows = [
        PlanRow {
            id: "greet",
            handler: "script",
            title: Some("Greet the project"),
            run: true,
            reason: None,
            conditions: &none,
            optional: false,
        },
        PlanRow {
            id: "docs-only",
            handler: "script",
            title: Some("Copy the docs when pandoc exists"),
            run: false,
            reason: Some("when `command_present = \"pandoc-nope\"` does not hold"),
            conditions: &none,
            optional: false,
        },
        PlanRow {
            id: "optional-tool",
            handler: "script",
            title: Some("Use an optional tool"),
            run: false,
            reason: Some("unmet need `command:nosuchtool`"),
            conditions: &none,
            optional: true,
        },
    ];
    assert_eq!(
        plan_list(&rows, 200),
        "  1.  greet          (script)  Greet the project\n  2.  docs-only      (script)  Copy the docs when pandoc exists  when command_present = \"pandoc-nope\" does not hold  [skip]\n  3.  optional-tool  (script)  Use an optional tool              unmet need command:nosuchtool  [skip]"
    );
    assert_eq!(
        plan_rule_label("org.vibevm.fixture/x@1.0.0", "slot:post-install"),
        "plan for org.vibevm.fixture/x@1.0.0 · slot:post-install"
    );
}

#[test]
fn slot_plan_rows_fit_viewport() {
    let conditions = ["metadata條件 requires a configured optional command".to_owned()];
    let id = "contribution識別子-with-a-long-name";
    let handler = "script處理器-with-a-long-name";
    let title = "Title標題 describing the complete contribution";
    let rows = [PlanRow {
        id,
        handler,
        title: Some(title),
        run: true,
        reason: None,
        conditions: &conditions,
        optional: true,
    }];
    for width in [32, 48] {
        let rendered = plan_list(&rows, width);
        let plain = console::strip_ansi_codes(&rendered);
        assert!(!plain.trim().is_empty(), "empty list at {width}");
        for line in plain.lines() {
            let cells = console::measure_text_width(line);
            assert!(
                cells <= width,
                "{cells} terminal cells exceed {width}: {line:?}"
            );
        }
        // Distinct prefixes prove each field survives, not just a bounded,
        // clipped line of the identifier or an empty metadata continuation.
        for field in [
            "contribution識別子",
            "script處理器",
            "Title標題",
            "metadata條件",
        ] {
            assert!(
                plain.contains(field),
                "missing field {field:?} at {width}: {plain:?}"
            );
        }
    }
}
