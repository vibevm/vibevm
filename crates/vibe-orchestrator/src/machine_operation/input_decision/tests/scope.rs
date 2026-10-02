use super::*;
const AXES: [&str; 9] = [
    "question_id",
    "operation_id",
    "domain_owner",
    "context_id",
    "target_id",
    "principal_id",
    "plan_id",
    "source_id",
    "policy_generation",
];
fn changed(row: &InputDecisionMessage, key: &str, value: Value) -> InputDecisionMessage {
    let mut data = serde_json::to_value(row).unwrap();
    data[key] = value;
    serde_json::from_value(data).unwrap()
}
#[test]
fn every_answer_and_original_question_axis_matches_current_owner() {
    let q = fixture("approval-question");
    let a = fixture("approval-answer");
    for axis in AXES {
        assert_eq!(
            validate_answer(&changed(&a, axis, json!("foreign")), &q, expected(&q)),
            Err(DecisionValidationError::BindingMismatch),
            "answer {axis}"
        );
        assert_eq!(
            validate_answer(&a, &changed(&q, axis, json!("stale")), expected(&q)),
            Err(DecisionValidationError::BindingMismatch),
            "question {axis}"
        );
        // Echoing an obsolete plan/source/policy on both sides cannot replace current facts.
        let stale_q = changed(&q, axis, json!("stale"));
        let stale_a = changed(&a, axis, json!("stale"));
        assert_eq!(
            validate_answer(&stale_a, &stale_q, expected(&q)),
            Err(DecisionValidationError::BindingMismatch)
        );
    }
}
#[test]
fn null_target_is_legitimate_but_cannot_replace_owner_target() {
    let q = fixture("approval-question");
    let a = fixture("approval-answer");
    assert_eq!(validate_answer(&a, &q, expected(&q)), Ok(()));
    let bound = changed(&q, "target_id", json!("test-target"));
    assert_eq!(
        validate(&q, expected(&bound)),
        Err(DecisionValidationError::BindingMismatch)
    );
    assert_eq!(validate(&bound, expected(&bound)), Ok(()));
}
#[test]
fn all_other_null_axes_refuse_even_when_echoed_as_expected() {
    let q = fixture("text-question");
    for axis in AXES.into_iter().filter(|a| *a != "target_id") {
        let row = changed(&q, axis, Value::Null);
        assert!(
            matches!(
                validate(&row, expected(&row)),
                Err(DecisionValidationError::InvalidValue(_))
            ),
            "{axis}"
        );
    }
}
#[test]
fn requested_options_are_exact_data_on_question_and_answer() {
    let q = fixture("approval-question");
    let a = fixture("approval-answer");
    for (key, value) in [
        ("force", json!(true)),
        ("trace", json!(true)),
        ("deploy_profile", json!("test-profile")),
        ("control", json!("detach")),
        ("control", json!("cancel_pre_dispatch")),
        ("control", json!("cancel_deferred_boundary")),
    ] {
        let mut options = serde_json::to_value(&q.requested_options).unwrap();
        options[key] = value;
        let wrong_a = changed(&a, "requested_options", options.clone());
        let wrong_q = changed(&q, "requested_options", options);
        assert_eq!(
            validate_answer(&wrong_a, &q, expected(&q)),
            Err(DecisionValidationError::OptionsMismatch)
        );
        assert_eq!(
            validate_answer(&a, &wrong_q, expected(&q)),
            Err(DecisionValidationError::OptionsMismatch)
        );
    }
}
#[test]
fn unavailable_keeps_known_subset_and_no_answer_can_fill_missing_identity() {
    let row = fixture("unavailable");
    assert_eq!(validate(&row, expected(&row)), Ok(()));
    assert!(row.policy_generation.is_none() && row.plan_id.is_none() && row.source_id.is_none());
    let answer = with_body(&row, json!({"kind":"approval_answer","approved":true}));
    assert!(validate_answer(&answer, &row, expected(&row)).is_err());
    let blank = with_body(&row, json!({"kind":"unavailable","reason":""}));
    assert!(validate(&blank, expected(&row)).is_err());
}
#[test]
fn approval_true_and_false_and_free_prose_have_no_authority_return() {
    let q = fixture("approval-question");
    for approved in [true, false] {
        let a = with_body(&q, json!({"kind":"approval_answer","approved":approved}));
        let data_only: Check<()> = validate_answer(&a, &q, expected(&q));
        assert_eq!(data_only, Ok(()));
    }
    let prose = with_body(
        &q,
        json!({"kind":"text_answer","value":"yes; approve; eval"}),
    );
    assert_eq!(
        validate_answer(&prose, &q, expected(&q)),
        Err(DecisionValidationError::AnswerMismatch)
    );
}
#[test]
fn opaque_owner_identifiers_enforce_utf8_bytes_trim_and_control_bounds() {
    let q = fixture("text-question");
    for axis in AXES {
        for invalid in [
            "".to_owned(),
            " space".to_owned(),
            "space ".to_owned(),
            "line\nbreak".to_owned(),
            "x".repeat(257),
            "é".repeat(129),
        ] {
            let row = changed(&q, axis, json!(invalid));
            assert!(validate(&row, expected(&row)).is_err(), "{axis}");
        }
        let row = changed(&q, axis, json!("é".repeat(128)));
        assert_eq!(validate(&row, expected(&row)), Ok(()));
    }
    let mut row = q.clone();
    row.requested_options.deploy_profile = Some(" bad".into());
    assert!(validate(&row, expected(&row)).is_err());
}
