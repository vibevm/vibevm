//! How a stopped agent execution is named: where the stop landed, the
//! provider-side effect it leaves, and what local state it put back.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-EFFECTS-LAW");

use vibe_core::cancellation::CancellationToken;

use super::AgentError;

pub(super) fn stop_at(token: &CancellationToken, stage: &str) -> Result<(), AgentError> {
    token.checkpoint().map_err(|_| cancelled(token, stage))
}

pub(super) fn cancelled(token: &CancellationToken, stage: &str) -> AgentError {
    AgentError::Cancelled {
        stage: stage.to_string(),
        remote: token.llm_effect().to_string(),
        restored: Vec::new(),
        retained: Vec::new(),
    }
}

pub(super) fn local_clause(restored: &[String], retained: &[String]) -> String {
    let mut clause = if restored.is_empty() {
        "no declared output was published".to_string()
    } else {
        format!(
            "the already-published owned output(s) {} were restored",
            restored.join(", ")
        )
    };
    if !retained.is_empty() {
        clause.push_str(&format!(
            "; {} did not match this publication and {} left untouched",
            retained.join(", "),
            if retained.len() == 1 { "was" } else { "were" }
        ));
    }
    clause
}

pub(super) fn retained_fix(retained: &[String]) -> &'static str {
    if retained.is_empty() {
        ""
    } else {
        ", and reconcile the outputs named as left untouched"
    }
}
