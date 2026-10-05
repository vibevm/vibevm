//! Selected-subject composition, deliberately outside lifecycle freshness caches.
//! A saved JSON file is a claim to recheck, never execution authority.
mod files;
mod rustcheck;
mod wire;
pub use wire::*;

use files::Capture;
use lean_ai_native_check::ProofWitness;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn execute(context: &Context, request: Request) -> Reply {
    match dispatch(context, request) {
        Ok(result) => Reply {
            ok: true,
            result: Some(result),
            error: None,
        },
        Err(error) => Reply {
            ok: false,
            result: None,
            error: Some(error),
        },
    }
}

fn dispatch(context: &Context, request: Request) -> wire::Result<ProofResult> {
    let root = context.project_root.canonicalize().map_err(files::input)?;
    let (subject, path, verify) = match request {
        Request::Check { subject, witness } => (subject, witness, false),
        Request::Verify { subject, witness } => (subject, witness, true),
    };
    files::relative(&path)?;
    if verify {
        let (witness, witness_sha256) = stable_saved(&root, &path, |bytes| {
            let saved: JoinedWitness = serde_json::from_slice(bytes)
                .map_err(|e| Diagnostic::new("malformed_witness", e.to_string()))?;
            if saved.schema != SCHEMA
                || saved.subject != subject
                || saved.grade != "g1_model"
                || saved.correspondence != "not_established"
            {
                return Err(Diagnostic::new(
                    "witness_subject_mismatch",
                    "saved schema, subject or proof grade differs",
                ));
            }
            let fresh = check(context, &root, &subject, &path, Some(&saved))?;
            if fresh != saved {
                return Err(Diagnostic::new(
                    "stale_witness",
                    "fresh checker outcomes or exact bindings differ from the saved join",
                ));
            }
            Ok(fresh)
        })?;
        Ok(ProofResult {
            status: "matched".into(),
            witness_sha256,
            witness,
        })
    } else {
        let witness = check(context, &root, &subject, &path, None)?;
        let mut bytes = serde_json::to_vec_pretty(&witness).map_err(files::input)?;
        bytes.push(b'\n');
        files::atomic_write(&root, &path, &bytes)?;
        // A concurrent replacement is a refusal, not a successful report for
        // bytes the response never observed at the selected destination.
        if files::read(&root, &path)? != bytes {
            return Err(Diagnostic::new(
                "witness_unstable",
                "witness changed during publication",
            ));
        }
        Ok(ProofResult {
            status: "checked".into(),
            witness_sha256: files::hash(&bytes),
            witness,
        })
    }
}

fn stable_saved<T>(
    root: &Path,
    path: &Path,
    check: impl FnOnce(&[u8]) -> wire::Result<T>,
) -> wire::Result<(T, String)> {
    let bytes = files::read(root, path)?;
    let value = check(&bytes)?;
    if files::read(root, path)? != bytes {
        return Err(Diagnostic::new(
            "witness_unstable",
            "saved witness snapshot changed during fresh checking",
        ));
    }
    Ok((value, files::hash(&bytes)))
}

fn check(
    context: &Context,
    root: &Path,
    subject: &str,
    output: &Path,
    saved: Option<&JoinedWitness>,
) -> wire::Result<JoinedWitness> {
    let mut inputs = Capture::default();
    let selection = inputs.add(root, "specmap.toml")?;
    let loaded = crate::composition::load(root)
        .map_err(|e| Diagnostic::new("composition_invalid", e.to_string()))?
        .ok_or_else(|| Diagnostic::new("composition_missing", "specmap.toml absent"))?;
    let composition = loaded.composition.as_ref().ok_or_else(|| {
        Diagnostic::new(
            "composition_missing",
            "selected Rust/Lean composition absent",
        )
    })?;
    if !subject.starts_with(&format!("spec://{}/", loaded.config.namespace)) {
        return Err(Diagnostic::new(
            "cross_package_proof_coverage_unsupported",
            "selected proof joins cover only the consumer's own spec namespace",
        ));
    }
    if subject != composition.subject {
        return Err(Diagnostic::new(
            "subject_mismatch",
            "request differs from the one enrolled full subject URI",
        ));
    }
    let consumer = inputs.add(root, "vibe.toml")?;
    let manifest: vibe_core::manifest::Manifest =
        toml::from_str(std::str::from_utf8(&consumer).map_err(files::input)?)
            .map_err(|e| Diagnostic::new("consumer_invalid", e.to_string()))?;
    let coordinate = if let Some(package) = manifest.package {
        format!("{}/{}", package.group, package.name)
    } else if let Some(project) = manifest.project {
        format!(
            "{}/{}",
            project
                .group
                .ok_or_else(|| Diagnostic::new("consumer_invalid", "project group absent"))?,
            project.name
        )
    } else {
        return Err(Diagnostic::new(
            "consumer_invalid",
            "one project/package coordinate required",
        ));
    };
    if coordinate != loaded.config.namespace {
        return Err(Diagnostic::new(
            "subject_mismatch",
            "consumer coordinate differs from source namespace",
        ));
    }
    let policy_path = composition.proof_policy.as_deref().ok_or_else(|| {
        Diagnostic::new(
            "proof_policy_missing",
            "composition has no explicit proof_policy",
        )
    })?;
    let policy_bytes = inputs.add(root, policy_path)?;
    let policy: wire::Policy =
        toml::from_str(std::str::from_utf8(&policy_bytes).map_err(files::input)?)
            .map_err(|e| Diagnostic::new("proof_policy_invalid", e.to_string()))?;
    if policy.schema != 1 || policy.subject != subject {
        return Err(Diagnostic::new(
            "proof_policy_invalid",
            "policy schema or selected subject differs",
        ));
    }
    if policy.grade != "g1_model" {
        return Err(Diagnostic::new(
            "proof_grade_unsupported",
            "only G1 model checking is implemented; G3 requires correspondence evidence",
        ));
    }
    if policy.rust.execution_id == policy.lean.execution_id
        || [&policy.rust.execution_id, &policy.lean.execution_id]
            .iter()
            .any(|id| !id.starts_with(&format!("{coordinate}#")) || id.ends_with('#'))
    {
        return Err(Diagnostic::new(
            "execution_identity_invalid",
            "select two distinct qualified contribution execution IDs",
        ));
    }
    let rust_inputs = rustcheck::capture(root, &policy.rust, &mut inputs)?;
    let carrier = inputs.add(root, &composition.lean_carrier)?;
    let carrier_value: toml::Value =
        toml::from_str(std::str::from_utf8(&carrier).map_err(files::input)?)
            .map_err(|e| Diagnostic::new("lean_carrier_invalid", e.to_string()))?;
    let sources = carrier_value
        .get("sources")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| Diagnostic::new("lean_carrier_invalid", "explicit sources absent"))?;
    for source in sources {
        inputs.add(
            root,
            source.as_str().ok_or_else(|| {
                Diagnostic::new("lean_carrier_invalid", "source must be a string")
            })?,
        )?;
    }
    let proofs = carrier_value
        .get("proof")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| Diagnostic::new("lean_carrier_invalid", "proof list absent"))?;
    for proof in proofs {
        inputs.add(
            root,
            proof
                .get("spec_file")
                .and_then(toml::Value::as_str)
                .ok_or_else(|| Diagnostic::new("lean_carrier_invalid", "spec file absent"))?,
        )?;
    }
    if inputs.0.contains_key(&files::relative(output)?)
        || matches!(
            output.to_str(),
            Some("specmap.json" | "package.specmap.json")
        )
    {
        return Err(Diagnostic::new(
            "invalid_path",
            "witness output overlaps a source or map input",
        ));
    }
    if let Some(saved) = saved {
        if saved.selection != files::binding("specmap.toml", &selection)
            || saved.policy != files::binding(policy_path, &policy_bytes)
            || saved.consumer != files::binding("vibe.toml", &consumer)
            || saved.rust.inputs != rust_inputs
        {
            return Err(Diagnostic::new(
                "stale_witness",
                "consumer, selection, policy, Rust source or dependency input changed",
            ));
        }
    }
    let current_map = crate::composition::build(root, &loaded, None)
        .map_err(|e| Diagnostic::new("composition_incomplete", e.to_string()))?;
    let current_map_bytes =
        specmap_core::index::to_canonical_bytes(&current_map).map_err(files::input)?;
    let snapshot = tempfile::tempdir().map_err(files::input)?;
    inputs.materialize(snapshot.path())?;
    let captured_config = crate::composition::load(snapshot.path())
        .map_err(files::input)?
        .ok_or_else(|| Diagnostic::new("composition_missing", "snapshot configuration absent"))?;
    let snapshot_map = crate::composition::build(snapshot.path(), &captured_config, None)
        .map_err(|e| Diagnostic::new("composition_incomplete", e.to_string()))?;
    if specmap_core::index::to_canonical_bytes(&snapshot_map).map_err(files::input)?
        != current_map_bytes
    {
        return Err(Diagnostic::new(
            "selected_closure_incomplete",
            "selected snapshot does not reproduce the current composed map",
        ));
    }
    with_stable_composed_map(root, &loaded, &current_map_bytes, || {
        let rust = rustcheck::check(
            context,
            snapshot.path(),
            &policy.rust,
            &composition.rust_symbol,
            rust_inputs,
        )?;
        inputs.recheck(snapshot.path())?;
        let lean_context = lean_ai_native_check::Context {
            project_root: snapshot.path().to_path_buf(),
            toolchain_dir: context.lean_toolchain_dir.clone(),
        };
        let lean_request = if let Some(saved) = saved {
            let path = PathBuf::from("_selected_saved_lean.json");
            fs::write(
                snapshot.path().join(&path),
                serde_json::to_vec(&saved.lean.proof).map_err(files::input)?,
            )
            .map_err(files::input)?;
            lean_ai_native_check::Request::VerifyWitness {
                carrier: composition.lean_carrier.clone().into(),
                subject: subject.into(),
                witness: path,
            }
        } else {
            lean_ai_native_check::Request::Check {
                carrier: composition.lean_carrier.clone().into(),
                subject: subject.into(),
            }
        };
        let reply = lean_ai_native_check::execute(&lean_context, lean_request);
        if !reply.ok {
            let error = reply.error.ok_or_else(|| {
                Diagnostic::new("lean_checker_invalid", "Lean failure lacked a diagnostic")
            })?;
            return Err(Diagnostic::new(
                format!("lean_{}", error.code),
                error.message,
            ));
        }
        let value = reply
            .result
            .ok_or_else(|| Diagnostic::new("lean_checker_invalid", "Lean witness absent"))?;
        let proof: ProofWitness = serde_json::from_value(if saved.is_some() {
            value
                .get("witness")
                .cloned()
                .ok_or_else(|| Diagnostic::new("lean_checker_invalid", "verified witness absent"))?
        } else {
            value
        })
        .map_err(|e| Diagnostic::new("lean_checker_invalid", e.to_string()))?;
        let unit = current_map
            .specUnits
            .iter()
            .find(|u| u.uri == subject)
            .ok_or_else(|| Diagnostic::new("subject_mismatch", "map subject absent"))?;
        if proof.subject != subject
            || proof.spec.unit_hash != unit.contentHash
            || proof.spec.revision != unit.revision.as_deref().copied()
            || proof.grade != "g1_model"
            || proof.outcome != "passed"
        {
            return Err(Diagnostic::new(
                "subject_mismatch",
                "fresh Lean witness does not bind the selected map unit",
            ));
        }
        inputs.recheck(root)?;
        inputs.recheck(snapshot.path())?;
        rustcheck::recheck_toolchain(context, &rust)?;
        if rustcheck::capture(root, &policy.rust, &mut Capture::default())? != rust.inputs {
            return Err(Diagnostic::new(
                "inputs_unstable",
                "Rust source closure changed during checks",
            ));
        }
        let witness = JoinedWitness {
        schema: SCHEMA.into(), subject: subject.into(), grade: "g1_model".into(), correspondence: "not_established".into(), spec: proof.spec.clone(),
        consumer: files::binding("vibe.toml", &consumer), selection: files::binding("specmap.toml", &selection), policy: files::binding(policy_path, &policy_bytes), map_sha256: files::hash(current_map_bytes.as_bytes()),
        checker_source_sha256: checker_source_hash(), rust,
        lean: LeanWitness { execution_id: policy.lean.execution_id, checker: "lean-ai-native-check/0.1.0".into(), outcome: "passed".into(), proof },
        assumptions: vec![
            "Rust tests and Lean sources are trusted executable code; this runner is not a sandbox".into(),
            "Selected libtests provide example evidence, not a theorem about the Rust implementation".into(),
            "Native OS, dynamic runtime, linker and compiler correctness are trusted; bin/lib and linker identities are captured".into(),
            "Two equal file snapshots do not lock paths against future or intervening external writes".into(),
        ],
    };
        Ok(witness)
    })
}

/// Bind the complete scan result on both sides of the expensive checkers.
/// Known-file rechecks alone cannot detect newly discovered scan-root members.
/// These are two observations, not a lock against future/intervening writes.
fn with_stable_composed_map<T>(
    root: &Path,
    loaded: &crate::composition::Loaded,
    before: &str,
    check: impl FnOnce() -> wire::Result<T>,
) -> wire::Result<T> {
    let checked = check()?;
    let final_map = crate::composition::build(root, loaded, None).map_err(|error| {
        Diagnostic::new(
            "composition_unstable",
            format!("complete map changed during checks: {error}"),
        )
    })?;
    let after = specmap_core::index::to_canonical_bytes(&final_map).map_err(files::input)?;
    if after != before {
        return Err(Diagnostic::new(
            "composition_unstable",
            "complete map bytes changed during fresh checking",
        ));
    }
    Ok(checked)
}

fn checker_source_hash() -> String {
    let mut bytes = Vec::new();
    for source in [
        include_str!("proof.rs"),
        include_str!("proof/wire.rs"),
        include_str!("proof/files.rs"),
        include_str!("proof/rustcheck.rs"),
        include_str!("composition.rs"),
        include_str!("../Cargo.toml"),
        include_str!("../../../Cargo.lock"),
    ] {
        bytes.extend_from_slice(&(source.len() as u64).to_be_bytes());
        bytes.extend_from_slice(source.as_bytes());
    }
    files::hash(&bytes)
}

#[cfg(test)]
mod tests;
