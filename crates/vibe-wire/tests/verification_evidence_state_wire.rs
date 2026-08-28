//! The durable half of the R7.5 evidence substrate: what
//! `.vibe/lifecycle.toml` records, how strictly it is read, and why
//! its measurement is a SEPARATE shape from the comparison row on the
//! external wire.
//!
//! Split from `verification_evidence_wire.rs` at the 600-line budget,
//! along the seam between the published report member (there) and the
//! machine state feeding it (here).

use std::collections::BTreeSet;

use vibe_wire::generated::lifecycle_state::LifecycleState;

#[path = "wire_support/mod.rs"]
mod support;
use support::{read_json, repo_root, round_trip};

/// A definition's whole member set — required and optional together.
fn members(definition: &serde_json::Value) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for key in ["properties", "optionalProperties"] {
        if let Some(block) = definition.get(key).and_then(|v| v.as_object()) {
            set.extend(block.keys().cloned());
        }
    }
    set
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|item| (*item).to_string()).collect()
}

/// Durable state carries the MEASUREMENT half: an execution's manifest
/// witness and an artifact's content witness, both attributed to the
/// run that took them — and a legacy row beside them with neither.
#[test]
fn the_measured_state_corpus_carries_witnesses_and_still_reads_legacy_rows() {
    let state: LifecycleState = round_trip("formats/corpora/lifecycle/e1/state_measured.json");
    let compile = &state.execution["org.demo/provider#compile"];
    let measurement = compile
        .input_measurement
        .as_ref()
        .expect("the measured row carries its measurement");
    assert_eq!(measurement.execution, "org.demo/provider#compile");
    assert_eq!(measurement.patterns, ["src/**", "Cargo.toml"]);
    assert_eq!(
        measurement.measured_run_id,
        state.run.run_id.clone().unwrap()
    );
    assert_eq!(measurement.witness.files, Some(7));
    assert_eq!(measurement.witness.bytes.as_deref(), Some("1234"));
    assert_ne!(
        measurement.witness.digest, measurement.declaration_fingerprint,
        "the declaration fingerprint and the input manifest are siblings, never aliases"
    );
    let artifact = &compile.artifacts[0];
    assert_eq!(
        artifact.witness.as_ref().unwrap().algorithm,
        "sha256:file-v1"
    );
    assert_eq!(
        artifact.measured_run_id.as_deref(),
        state.run.run_id.as_deref()
    );
    // Durable state keeps the ABSOLUTE machine path it needs to reopen
    // the artifact; the evidence row carries the project-relative one.
    // The single normalisation between them is P2's, and this pair of
    // assertions — here and in the verified-corpus test above — is
    // what will keep it honest.
    assert!(
        artifact.path.starts_with("C:/") || artifact.path.starts_with('/'),
        "durable state keeps the absolute path: {}",
        artifact.path
    );

    // An authored empty declared scope is measured, counted at zero,
    // and distinguishable from the legacy row that has no measurement.
    let tests = &state.execution["org.demo/provider#unit-tests"];
    let scope = tests.input_measurement.as_ref().unwrap();
    assert!(scope.patterns.is_empty());
    assert_eq!(scope.witness.files, Some(0));

    let legacy = &state.execution["org.demo/provider#legacy-output"];
    assert!(legacy.input_measurement.is_none());
    assert!(legacy.artifacts[0].witness.is_none());
    assert!(legacy.artifacts[0].measured_run_id.is_none());

    // …and the pre-R7.5 corpus still reads through the strict reader
    // with none of the new members anywhere.
    let old: LifecycleState = round_trip("formats/corpora/lifecycle/e1/state.json");
    assert!(
        old.execution
            .values()
            .all(|row| row.input_measurement.is_none()
                && row.artifacts.iter().all(|a| a.witness.is_none()))
    );
}

/// The state reader stays STRICT: an unknown member anywhere in the
/// new sub-records is a refusal, not a silently dropped field. This is
/// the property that forced the durable twin to be its own definition
/// instead of the shared fragment.
#[test]
fn the_state_reader_refuses_unknown_members_in_the_new_records() {
    for pointer in ["input_measurement", "artifacts"] {
        let mut state = read_json("formats/corpora/lifecycle/e1/state_measured.json");
        let row = state["execution"]["org.demo/provider#compile"]
            .as_object_mut()
            .unwrap();
        match pointer {
            "input_measurement" => {
                row["input_measurement"]["observed"] = serde_json::json!({"a": 1});
            }
            _ => {
                row["artifacts"][0]["witness"]["status"] = serde_json::json!("matched");
            }
        }
        assert!(
            serde_json::from_value::<LifecycleState>(state).is_err(),
            "{pointer}: the strict state reader refuses a member its schema does not name"
        );
    }
}

/// Compatibility for this recoverable state is FORWARD ONLY, and that
/// is said out loud rather than hoped for. Current code reads a legacy
/// file (proven above); a PRE-R7.5 strict reader — modelled here by
/// the exact `deny_unknown_fields` shape the old schema generated —
/// refuses a file carrying the new members. The downgrade is honest
/// and recoverable: `.vibe/lifecycle.toml` is machine state, and
/// losing it costs one full re-run and nothing else. It is not a
/// reason to weaken the current strict reader, which is what caught
/// the divergence in the first place.
#[test]
fn new_state_is_forward_only_for_a_pre_r75_strict_reader() {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct OldStateArtifact {
        #[allow(dead_code)]
        id: String,
        #[allow(dead_code)]
        path: String,
        #[allow(dead_code)]
        kind: String,
    }

    let state = read_json("formats/corpora/lifecycle/e1/state_measured.json");
    let row = &state["execution"]["org.demo/provider#compile"];
    assert!(
        serde_json::from_value::<OldStateArtifact>(row["artifacts"][0].clone()).is_err(),
        "a pre-R7.5 strict reader refuses the witnessed artifact — forward-only, by design"
    );

    // The legacy row inside the SAME file still reads through the old
    // shape, so the incompatibility is exactly the new members and
    // nothing else.
    let legacy = &state["execution"]["org.demo/provider#legacy-output"];
    serde_json::from_value::<OldStateArtifact>(legacy["artifacts"][0].clone())
        .expect("an unwitnessed row is still readable by the old shape");
}

/// The durable state twin and the shared comparison row agree on the
/// identity half they share. The duplication is forced (the state
/// format is `foreign_parsers = "none"`, so its structs are stamped
/// `deny_unknown_fields` and cannot be the permissive shared block);
/// this test is what keeps a forced duplication from becoming a
/// silent divergence.
#[test]
fn the_durable_measurement_and_the_shared_comparison_row_share_one_identity_half() {
    let vocabularies = read_json("formats/vocabularies.json");
    let state = read_json("schemas/lifecycle_state.jtd.json");

    let identity = set(&[
        "execution",
        "phase",
        "declaration_fingerprint",
        "patterns",
        "measured_run_id",
    ]);
    let shared = members(&vocabularies["input_measurement"]);
    let twin = members(&state["definitions"]["state_input_measurement"]);
    assert!(identity.is_subset(&shared) && identity.is_subset(&twin));
    assert_eq!(
        shared
            .difference(&identity)
            .cloned()
            .collect::<BTreeSet<_>>(),
        set(&["status", "measured", "observed", "reason_code"]),
        "the shared row adds exactly the comparison half"
    );
    assert_eq!(
        twin.difference(&identity).cloned().collect::<BTreeSet<_>>(),
        set(&["witness"]),
        "the durable twin adds exactly one witness and no verdict"
    );

    // The witness records themselves are the same shape on both sides,
    // required and optional halves alike.
    for key in ["properties", "optionalProperties"] {
        let shared_block = vocabularies["digest_witness"][key]
            .as_object()
            .map(|b| b.keys().cloned().collect::<BTreeSet<_>>())
            .unwrap_or_default();
        let twin_block = state["definitions"]["state_digest_witness"][key]
            .as_object()
            .map(|b| b.keys().cloned().collect::<BTreeSet<_>>())
            .unwrap_or_default();
        assert_eq!(shared_block, twin_block, "digest witness {key} drift");
    }

    // And the state format really does carry the role that forces the
    // split — if it ever stops, the twin should be reconsidered, not
    // silently kept.
    let registry = std::fs::read_to_string(repo_root().join("formats/REGISTRY.toml")).unwrap();
    let parsed: toml::Value = toml::from_str(&registry).unwrap();
    assert_eq!(
        parsed["format"]["lifecycle-state"]["foreign_parsers"].as_str(),
        Some("none"),
        "the durable twin exists because the state reader is strict"
    );
}
