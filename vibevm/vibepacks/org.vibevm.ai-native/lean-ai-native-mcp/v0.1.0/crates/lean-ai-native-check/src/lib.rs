//! A targeted model-proof runner. Declared relations, checked proofs and
//! implementation refinement are separate evidence classes.

mod carrier;
mod files;
mod process;
mod proof;
mod wire;

pub use wire::*;

use std::path::PathBuf;

/// Trusted startup context; request bodies cannot override this boundary.
#[derive(Clone, Debug)]
pub struct Context {
    pub project_root: PathBuf,
    pub toolchain_dir: Option<PathBuf>,
}

/// All front doors project the same typed result, including failures.
pub fn execute(context: &Context, request: Request) -> Reply {
    match dispatch(context, request) {
        Ok(value) => Reply {
            ok: true,
            result: Some(value),
            error: None,
        },
        Err(error) => Reply {
            ok: false,
            result: None,
            error: Some(error),
        },
    }
}

fn dispatch(context: &Context, request: Request) -> Result<serde_json::Value, Diagnostic> {
    let root = context
        .project_root
        .canonicalize()
        .map_err(|e| Diagnostic::new("project_unavailable", e.to_string()))?;
    match request {
        Request::Scan { carrier } => {
            let captured = carrier::capture(&root, &carrier)?;
            serde_json::to_value(captured.inventory()).map_err(Diagnostic::serialization)
        }
        Request::Check { carrier, subject } => {
            let captured = carrier::capture(&root, &carrier)?;
            let witness = proof::check(context, &captured, &subject)?;
            captured.recheck(&root)?;
            serde_json::to_value(witness).map_err(Diagnostic::serialization)
        }
        Request::VerifyWitness {
            carrier,
            subject,
            witness,
        } => {
            let saved_bytes = files::read_relative(&root, &witness)?;
            let saved: ProofWitness = serde_json::from_slice(&saved_bytes)
                .map_err(|e| Diagnostic::new("malformed_witness", e.to_string()))?;
            if saved.schema != WITNESS_SCHEMA
                || saved.subject != subject
                || saved.grade != "g1_model"
                || saved.outcome != "passed"
            {
                return Err(Diagnostic::new(
                    "witness_subject_mismatch",
                    "witness schema, subject, grade or outcome differs from the requested claim",
                ));
            }
            let captured = carrier::capture(&root, &carrier)?;
            let fresh = proof::check(context, &captured, &subject)?;
            captured.recheck(&root)?;
            if saved != fresh {
                return Err(Diagnostic::new(
                    "stale_witness",
                    "fresh proof check or exact input/toolchain binding differs from the saved witness",
                ));
            }
            Ok(serde_json::json!({"status":"matched", "witness":fresh}))
        }
    }
}
