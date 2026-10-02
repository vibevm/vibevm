use super::*;
use serde_json::{Value, json};
fn fixture(name: &str) -> InputDecisionMessage {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../formats/corpora/machine-input-decision-message/e1/valid")
        .join(format!("{name}.json"));
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn expected(row: &InputDecisionMessage) -> ExpectedBinding<'_> {
    ExpectedBinding {
        question_id: row.question_id.as_deref(),
        operation_id: row.operation_id.as_deref(),
        domain_owner: row.domain_owner.as_deref(),
        context_id: row.context_id.as_deref(),
        target_id: row.target_id.as_deref(),
        principal_id: row.principal_id.as_deref(),
        plan_id: row.plan_id.as_deref(),
        source_id: row.source_id.as_deref(),
        policy_generation: row.policy_generation.as_deref(),
        requested_options: &row.requested_options,
    }
}
fn with_body(row: &InputDecisionMessage, body: Value) -> InputDecisionMessage {
    let mut value = serde_json::to_value(row).unwrap();
    value["body"] = body;
    serde_json::from_value(value).unwrap()
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-INPUT-ADMISSION",
    r = 1
)]
fn seven_arms_parse_validate_and_bounded_serialize() {
    for name in [
        "selection-question",
        "selection-answer",
        "text-question",
        "text-answer",
        "approval-question",
        "approval-answer",
        "unavailable",
    ] {
        let row = fixture(name);
        let bytes = serialize(&row, expected(&row)).unwrap();
        assert_eq!(parse(&bytes, expected(&row)).unwrap(), row);
    }
}
#[test]
fn typed_answer_pairs_validate_as_unit_data() {
    for (answer, question) in [
        ("selection-answer", "selection-question"),
        ("text-answer", "text-question"),
        ("approval-answer", "approval-question"),
    ] {
        let q = fixture(question);
        let a = fixture(answer);
        let result: Result<(), DecisionValidationError> = validate_answer(&a, &q, expected(&q));
        assert_eq!(result, Ok(()));
    }
}
#[test]
fn crossed_and_unavailable_questions_refuse() {
    for question in [
        "selection-question",
        "text-question",
        "approval-question",
        "unavailable",
    ] {
        let q = fixture(question);
        for answer in ["selection-answer", "text-answer", "approval-answer"] {
            let a = fixture(answer);
            if question.split('-').next() != answer.split('-').next() {
                assert!(validate_answer(&a, &q, expected(&q)).is_err());
            }
        }
    }
    let a = fixture("approval-answer");
    assert_eq!(
        validate_answer(&a, &a, expected(&a)),
        Err(DecisionValidationError::AnswerMismatch)
    );
}
#[path = "tests/bounds.rs"]
mod bounds;
#[path = "tests/scope.rs"]
mod scope;
