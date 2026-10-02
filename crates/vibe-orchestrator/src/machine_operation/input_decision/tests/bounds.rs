use super::*;
#[test]
fn unsupported_epoch_refuses_after_generated_shape() {
    let q = fixture("text-question");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../formats/corpora/machine-input-decision-message/e1/semantic-invalid/unsupported-epoch.json");
    let row: InputDecisionMessage = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(
        validate(&row, expected(&q)),
        Err(DecisionValidationError::UnsupportedEpoch)
    );
    assert_eq!(
        parse(&serde_json::to_vec(&row).unwrap(), expected(&q)),
        Err(DecisionValidationError::UnsupportedEpoch)
    );
}
#[test]
fn selection_rejects_unknown_duplicate_and_out_of_cardinality_answers() {
    let q = fixture("selection-question");
    for ids in [
        json!([]),
        json!(["unknown"]),
        json!(["a", "a"]),
        json!(["a", "b", "c"]),
    ] {
        let a = with_body(&q, json!({"kind":"selection_answer","choice_ids":ids}));
        assert!(validate_answer(&a, &q, expected(&q)).is_err());
    }
    let a = with_body(
        &q,
        json!({"kind":"selection_answer","choice_ids":["a","b"]}),
    );
    assert_eq!(validate_answer(&a, &q, expected(&q)), Ok(()));
}
#[test]
fn selection_question_ranges_choice_ids_labels_and_counts_are_bounded() {
    let q = fixture("selection-question");
    for (min, max) in [(2, 1), (0, 3), (0, 65)] {
        let bad = with_body(
            &q,
            json!({"kind":"selection_question","prompt":"","choices":[{"id":"a","label":""},{"id":"b","label":""}],"min_choices":min,"max_choices":max}),
        );
        assert!(validate(&bad, expected(&q)).is_err());
    }
    let mut full = q.clone();
    if let Body::SelectionQuestion(row) = &mut full.body {
        row.choices = (0..64)
            .map(|n| Choice {
                id: format!("test-{n}"),
                label: "é".repeat(512),
            })
            .collect();
        row.min_choices = 0;
        row.max_choices = 64;
    }
    // 64 maximum labels may overflow total frame cap: use shorter labels for count boundary.
    if let Body::SelectionQuestion(row) = &mut full.body {
        for c in &mut row.choices {
            c.label = "label".into();
        }
    }
    assert_eq!(validate(&full, expected(&q)), Ok(()));
    let mut answer = fixture("selection-answer");
    if let Body::SelectionAnswer(row) = &mut answer.body {
        row.choice_ids = (0..64).map(|n| format!("test-{n}")).collect();
    }
    assert_eq!(validate_answer(&answer, &full, expected(&q)), Ok(()));
    if let Body::SelectionAnswer(row) = &mut answer.body {
        row.choice_ids.push("overflow".into());
    }
    assert!(validate(&answer, expected(&q)).is_err());
    for defect in ["count", "duplicate", "blank-id", "label"] {
        let mut bad = full.clone();
        if let Body::SelectionQuestion(row) = &mut bad.body {
            match defect {
                "count" => row.choices.push(Choice {
                    id: "extra".into(),
                    label: "".into(),
                }),
                "duplicate" => row.choices[1].id = row.choices[0].id.clone(),
                "blank-id" => row.choices[0].id = "".into(),
                _ => row.choices[0].label = "é".repeat(513),
            }
        }
        assert!(validate(&bad, expected(&q)).is_err(), "{defect}");
    }
    let empty = with_body(
        &q,
        json!({"kind":"selection_question","prompt":"","choices":[],"min_choices":0,"max_choices":0}),
    );
    assert_eq!(validate(&empty, expected(&q)), Ok(()));
}
#[test]
fn text_answer_range_uses_utf8_bytes_and_never_interprets_prose() {
    let q = fixture("text-question");
    let four = with_body(
        &q,
        json!({"kind":"text_question","prompt":"","min_bytes":4,"max_bytes":4}),
    );
    for (s, valid) in [("éé", true), ("é", false), ("ééé", false), ("yes!", true)] {
        let a = with_body(&q, json!({"kind":"text_answer","value":s}));
        assert_eq!(validate_answer(&a, &four, expected(&q)).is_ok(), valid);
    }
    for (min, max) in [(2, 1), (0, 4097)] {
        let bad = with_body(
            &q,
            json!({"kind":"text_question","prompt":"","min_bytes":min,"max_bytes":max}),
        );
        assert!(validate(&bad, expected(&q)).is_err());
    }
}
#[test]
fn every_text_member_has_4096_byte_cap() {
    for name in [
        "selection-question",
        "text-question",
        "text-answer",
        "approval-question",
        "unavailable",
    ] {
        let original = fixture(name);
        let fields: &[&str] = match name {
            "text-answer" => &["value"],
            "approval-question" => &["prompt", "effect_summary"],
            "unavailable" => &["reason"],
            _ => &["prompt"],
        };
        for field in fields {
            for (count, valid) in [(2048, true), (2049, false)] {
                let mut body = serde_json::to_value(&original.body).unwrap();
                body[field] = json!("é".repeat(count));
                let row = with_body(&original, body);
                assert_eq!(
                    validate(&row, expected(&original)).is_ok(),
                    valid,
                    "{name}.{field}"
                );
            }
        }
    }
}
#[test]
fn counting_writer_enforces_escaped_frame_size_before_output_allocation() {
    let q = fixture("selection-question");
    let mut oversized = q.clone();
    if let Body::SelectionQuestion(row) = &mut oversized.body {
        row.choices = (0..64)
            .map(|n| Choice {
                id: format!("test-{n}"),
                label: "\0".repeat(1024),
            })
            .collect();
        row.max_choices = 64;
    }
    assert_eq!(
        serialize(&oversized, expected(&q)),
        Err(DecisionValidationError::FrameLimit)
    );
    assert_eq!(
        validate(&oversized, expected(&q)),
        Err(DecisionValidationError::FrameLimit)
    );
    let mut bytes = serialize(&q, expected(&q)).unwrap();
    bytes.resize(MAX_FRAME_BYTES, b' ');
    assert_eq!(parse(&bytes, expected(&q)).unwrap(), q);
    bytes.push(b' ');
    assert_eq!(
        parse(&bytes, expected(&q)),
        Err(DecisionValidationError::FrameLimit)
    );
    assert_eq!(
        parse(&vec![b'x'; MAX_FRAME_BYTES + 1], expected(&q)),
        Err(DecisionValidationError::FrameLimit)
    );
    assert_eq!(
        parse(b"{}", expected(&q)),
        Err(DecisionValidationError::InvalidShape)
    );
}
