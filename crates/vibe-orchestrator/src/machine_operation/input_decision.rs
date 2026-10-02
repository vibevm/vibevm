//! PureValidation != Ownership != Admission.
//!
//! This single-text-input profile leaves other domain input kinds as extraction
//! gaps. Text, display summaries, options and approval booleans are only data.
//! Matching owner-supplied comparison values cannot construct a grant or owner.
//! The service must later rebind actual C.1/physical roots, source and policy.
//! `plan_id` is the question's prepared plan; only the domain supplies a resulting
//! selection plan. Unavailable generation never manufactures missing identities.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-INPUT-ADMISSION");

use std::{collections::BTreeSet, fmt};
pub use vibe_wire::generated::machine::e1::input_decision_message::*;

pub const MAX_FRAME_BYTES: usize = 65_536;
pub const MAX_TEXT_BYTES: usize = 4096;
pub const MAX_CHOICES: usize = 64;

/// Comparison data selected independently by the owner. No effects or grants.
/// Nullable fields also represent the known subset of an unavailable message.
#[derive(Debug, Clone, Copy)]
pub struct ExpectedBinding<'a> {
    pub question_id: Option<&'a str>,
    pub operation_id: Option<&'a str>,
    pub domain_owner: Option<&'a str>,
    pub context_id: Option<&'a str>,
    pub target_id: Option<&'a str>,
    pub principal_id: Option<&'a str>,
    pub plan_id: Option<&'a str>,
    pub source_id: Option<&'a str>,
    pub policy_generation: Option<&'a str>,
    pub requested_options: &'a RequestedOptions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecisionValidationError {
    FrameLimit,
    InvalidShape,
    UnsupportedEpoch,
    BindingMismatch,
    OptionsMismatch,
    InvalidValue(&'static str),
    AnswerMismatch,
}
impl fmt::Display for DecisionValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "decision data refused: {self:?}")
    }
}
impl std::error::Error for DecisionValidationError {}
pub type Check<T = ()> = Result<T, DecisionValidationError>;

fn require(valid: bool, reason: &'static str) -> Check {
    if valid {
        Ok(())
    } else {
        Err(DecisionValidationError::InvalidValue(reason))
    }
}
fn opaque(value: &str) -> Check {
    require(
        !value.is_empty()
            && value.len() <= 256
            && value.trim() == value
            && !value.chars().any(char::is_control),
        "invalid opaque identifier",
    )
}
fn text(value: &str) -> Check {
    require(value.len() <= MAX_TEXT_BYTES, "text byte cap")
}
fn unique<'a>(ids: impl Iterator<Item = &'a str>) -> Check {
    let mut seen = BTreeSet::new();
    for id in ids {
        opaque(id)?;
        require(seen.insert(id), "duplicate choice identifier")?;
    }
    Ok(())
}
// Count escaped serialized bytes before allocating any serializer storage.
fn size(message: &InputDecisionMessage) -> Check<usize> {
    struct Counter(usize);
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self
                .0
                .checked_add(bytes.len())
                .filter(|n| *n <= MAX_FRAME_BYTES)
                .ok_or_else(|| std::io::Error::other("decision frame cap"))?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter(0);
    serde_json::to_writer(&mut counter, message)
        .map_err(|_| DecisionValidationError::FrameLimit)?;
    Ok(counter.0)
}

/// Reject oversized input before invoking the canonical generated reader.
pub fn parse(bytes: &[u8], expected: ExpectedBinding<'_>) -> Check<InputDecisionMessage> {
    if bytes.len() > MAX_FRAME_BYTES {
        return Err(DecisionValidationError::FrameLimit);
    }
    let message =
        serde_json::from_slice(bytes).map_err(|_| DecisionValidationError::InvalidShape)?;
    validate(&message, expected)?;
    Ok(message)
}

/// Bounded output; successful serialization proves no availability or authority.
pub fn serialize(message: &InputDecisionMessage, expected: ExpectedBinding<'_>) -> Check<Vec<u8>> {
    validate(message, expected)?;
    let mut bytes = Vec::with_capacity(size(message)?);
    serde_json::to_writer(&mut bytes, message)
        .map_err(|_| DecisionValidationError::InvalidShape)?;
    Ok(bytes)
}

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-INPUT-ADMISSION",
    r = 1
)]
pub fn validate(message: &InputDecisionMessage, expected: ExpectedBinding<'_>) -> Check {
    size(message)?;
    if message.schema != 1 {
        return Err(DecisionValidationError::UnsupportedEpoch);
    }
    let unavailable = matches!(&message.body, Body::Unavailable(_));
    for (actual, current, nullable) in [
        (&message.question_id, expected.question_id, false),
        (&message.operation_id, expected.operation_id, false),
        (&message.domain_owner, expected.domain_owner, false),
        (&message.context_id, expected.context_id, false),
        (&message.target_id, expected.target_id, true),
        (&message.principal_id, expected.principal_id, false),
        (&message.plan_id, expected.plan_id, false),
        (&message.source_id, expected.source_id, false),
        (
            &message.policy_generation,
            expected.policy_generation,
            false,
        ),
    ] {
        require(
            unavailable || nullable || actual.is_some(),
            "missing question/answer binding",
        )?;
        if let Some(id) = actual {
            opaque(id)?;
        }
        if actual.as_deref() != current {
            return Err(DecisionValidationError::BindingMismatch);
        }
    }
    if let Some(profile) = &message.requested_options.deploy_profile {
        opaque(profile)?;
    }
    if &message.requested_options != expected.requested_options {
        return Err(DecisionValidationError::OptionsMismatch);
    }
    match &message.body {
        Body::SelectionQuestion(row) => {
            text(&row.prompt)?;
            require(
                row.choices.len() <= MAX_CHOICES
                    && row.min_choices <= row.max_choices
                    && usize::from(row.max_choices) <= row.choices.len()
                    && usize::from(row.max_choices) <= MAX_CHOICES,
                "selection range/count",
            )?;
            unique(row.choices.iter().map(|c| c.id.as_str()))?;
            for choice in &row.choices {
                require(choice.label.len() <= 1024, "choice label byte cap")?;
            }
        }
        Body::SelectionAnswer(row) => {
            require(
                row.choice_ids.len() <= MAX_CHOICES,
                "selection answer count",
            )?;
            unique(row.choice_ids.iter().map(String::as_str))?;
        }
        Body::TextQuestion(row) => {
            text(&row.prompt)?;
            require(
                row.min_bytes <= row.max_bytes && usize::from(row.max_bytes) <= MAX_TEXT_BYTES,
                "text range",
            )?;
        }
        Body::TextAnswer(row) => text(&row.value)?,
        Body::ApprovalQuestion(row) => {
            text(&row.prompt)?;
            text(&row.effect_summary)?;
        }
        Body::ApprovalAnswer(_) => {}
        Body::Unavailable(row) => {
            text(&row.reason)?;
            require(!row.reason.is_empty(), "empty unavailable reason")?;
        }
    }
    Ok(())
}

/// Validate both values against CURRENT independent owner comparisons and the
/// original typed question. Even approval `true` returns only `Result<()>`.
pub fn validate_answer(
    answer: &InputDecisionMessage,
    question: &InputDecisionMessage,
    expected: ExpectedBinding<'_>,
) -> Check {
    validate(question, expected)?;
    validate(answer, expected)?;
    match (&answer.body, &question.body) {
        (Body::SelectionAnswer(a), Body::SelectionQuestion(q)) => {
            require(
                a.choice_ids.len() >= usize::from(q.min_choices)
                    && a.choice_ids.len() <= usize::from(q.max_choices),
                "answer cardinality",
            )?;
            require(
                a.choice_ids
                    .iter()
                    .all(|id| q.choices.iter().any(|c| c.id == *id)),
                "undeclared choice",
            )
        }
        (Body::TextAnswer(a), Body::TextQuestion(q)) => require(
            a.value.len() >= usize::from(q.min_bytes) && a.value.len() <= usize::from(q.max_bytes),
            "answer text byte range",
        ),
        (Body::ApprovalAnswer(_), Body::ApprovalQuestion(_)) => Ok(()),
        _ => Err(DecisionValidationError::AnswerMismatch),
    }
}

#[cfg(test)]
#[path = "input_decision/tests.rs"]
mod tests;
