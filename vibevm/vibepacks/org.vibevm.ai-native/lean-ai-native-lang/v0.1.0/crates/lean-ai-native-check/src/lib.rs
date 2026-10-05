//! A targeted model-proof runner. Declared relations, checked proofs and
//! implementation refinement are separate evidence classes.

mod carrier;
mod files;
mod process;
mod proof;
mod wire;

pub use wire::*;

use std::path::{Path, PathBuf};

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
            let (fresh, witness_sha256) = with_stable_witness(&root, &witness, |saved_bytes| {
                let saved: ProofWitness = serde_json::from_slice(saved_bytes)
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
                Ok(fresh)
            })?;
            Ok(
                serde_json::json!({"status":"matched", "witness_sha256":witness_sha256, "witness":fresh}),
            )
        }
    }
}

/// Bind the saved-witness snapshot on both sides of the expensive checker.
/// This describes two observations, not a lock on future writes to the path.
fn with_stable_witness<T>(
    root: &Path,
    witness: &Path,
    check: impl FnOnce(&[u8]) -> Result<T, Diagnostic>,
) -> Result<(T, String), Diagnostic> {
    let before = files::read_relative(root, witness)?;
    let checked = check(&before)?;
    if files::read_relative(root, witness)? != before {
        return Err(Diagnostic::new(
            "witness_unstable",
            "saved witness bytes changed during fresh checking",
        ));
    }
    Ok((checked, files::hash(&before)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn witness_replacement_during_checker_refuses_matched() {
        let root = tempfile::tempdir().expect("fixture");
        let path = Path::new("proof.json");
        fs::write(root.path().join(path), b"{}").expect("saved snapshot");
        let result = with_stable_witness(root.path(), path, |before| {
            assert_eq!(before, b"{}");
            // Deterministic external-writer action at the checker boundary;
            // no timing window, sleeps, or mock production success path.
            let replacement = root.path().join("replacement.json");
            fs::write(&replacement, br#"{"forged":true}"#).expect("replacement");
            fs::rename(replacement, root.path().join(path)).expect("replace witness");
            Ok(())
        });
        assert_eq!(
            result.expect_err("changed snapshot must refuse").code,
            "witness_unstable"
        );
    }

    #[test]
    fn unchanged_witness_returns_the_observed_byte_digest() {
        let root = tempfile::tempdir().expect("fixture");
        let path = Path::new("proof.json");
        fs::write(root.path().join(path), b"{}").expect("saved snapshot");
        let (_, digest) =
            with_stable_witness(root.path(), path, |_| Ok(())).expect("stable snapshot");
        assert_eq!(
            digest,
            "sha256:44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a"
        );
    }
}
