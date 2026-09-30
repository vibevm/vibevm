//! RED arms for every scalar law of the deploy journal pair, plus the
//! positives that keep them honest — including the receipt's status/
//! finalisation matrix in all four of its rows. The arms are minimal
//! mutations of one legal base value, so a refusal names the law, not
//! a fixture's accident.

use chrono::TimeZone;

use crate::behaviour::deploy_records::{
    DeployIntentError, DeployReceiptError, INTENT_EPOCH, RECEIPT_EPOCH, validate_intent,
    validate_native_operation_binding, validate_operation_binding_pair,
    validate_operation_binding_root, validate_receipt,
};
use crate::generated::deploy_intent::{
    DeployIntent, DeployTargetIdentity, PlannedResource, Rfc3339Timestamp as IntentTimestamp,
};
use crate::generated::deploy_receipt::{
    DeployIdentity, DeployReceipt, DestinationScope, OwnedResource, ProviderIdentity,
    ReceiptStatus, Rfc3339Timestamp as ReceiptTimestamp,
};
use crate::generated::environment::e1::operation_binding::OperationBinding;
use crate::generated::native::e1::deploy_request::NativeOperationBinding;
use crate::generated::shared::EnvironmentOperationBinding;

const HEX64: &str = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

fn ts() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc
        .with_ymd_and_hms(2026, 8, 29, 12, 30, 0)
        .single()
        .expect("one instant")
}

/// One legal minimal intent: a first deployment (no prior generation),
/// one planned resource, no prior digest.
fn intent_base() -> DeployIntent {
    DeployIntent {
        schema: INTENT_EPOCH,
        plan_hash: HEX64.to_string(),
        target: DeployTargetIdentity {
            project: "demo".to_string(),
            package: None,
            profile: "local".to_string(),
            target: "local-helper".to_string(),
            generation: 0,
        },
        resources: vec![PlannedResource {
            resource: "~/.vibe/bin/vibe-helper".to_string(),
            desired_digest: HEX64.to_string(),
            prior_digest: None,
        }],
        started_at: ts(),
        prior_generation: None,
        operation_binding: None,
    }
}

/// One legal mid-flight receipt: `applied`, so no `finalized_at`, and
/// no optional member carried.
fn receipt_base() -> DeployReceipt {
    DeployReceipt {
        schema: RECEIPT_EPOCH,
        identity: DeployIdentity {
            project: "demo".to_string(),
            package: None,
        },
        profile: "local".to_string(),
        target: "local-helper".to_string(),
        generation: 0,
        artifact_digest: HEX64.to_string(),
        provider: ProviderIdentity {
            key: "org.vibevm/vibe#vibe-bin".to_string(),
            version: None,
            content_hash: None,
        },
        desired_config_digest: HEX64.to_string(),
        scope: DestinationScope::User,
        resources: vec![OwnedResource {
            resource: "~/.vibe/bin/vibe-helper".to_string(),
            post_digest: HEX64.to_string(),
        }],
        reversible: true,
        applied_at: ts(),
        finalized_at: None,
        status: ReceiptStatus::Applied,
        evidence: None,
        prior_state_handle: None,
        operation_binding: None,
    }
}

fn binding_base() -> EnvironmentOperationBinding {
    serde_json::from_value(serde_json::json!({
        "binding_epoch":1,"binding_id":"binding-a","host_context_id":"context-a",
        "instance_ref":"instance-a","selected_generation_ref":null,
        "target_mode":"workspace_projection","target_role":"produced_tool_runtime","target_os":"linux",
        "target_architecture":"x86_64","target_abi":"gnu","target_libc":"glibc",
        "principal_ref":"principal-a","storage_binding_ref":"storage-a",
        "root_bindings":[{"root_role":"project","capability_ref":"root-project-a"}],
        "admission_epoch":1
    }))
    .unwrap()
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-OPERATION-BINDING",
    r = 3
)]
fn bound_intent_receipt_and_native_copy_agree_without_granting_effects() {
    let binding = binding_base();
    let mut intent = intent_base();
    let mut receipt = receipt_base();
    intent.operation_binding = Some(binding.clone());
    receipt.operation_binding = Some(binding.clone());
    validate_intent(&intent).unwrap();
    validate_receipt(&receipt).unwrap();
    validate_operation_binding_pair(&intent, &receipt).unwrap();
    let native: NativeOperationBinding =
        serde_json::from_value(serde_json::to_value(&binding).unwrap()).unwrap();
    validate_native_operation_binding(&native, &binding).unwrap();
    let mut changed = native;
    changed.principal_ref = "foreign-principal".into();
    assert!(validate_native_operation_binding(&changed, &binding).is_err());
    receipt.operation_binding.as_mut().unwrap().instance_ref = "other-instance".into();
    assert!(validate_operation_binding_pair(&intent, &receipt).is_err());
    receipt.operation_binding = None;
    assert!(validate_operation_binding_pair(&intent, &receipt).is_err());
    let mut intent = intent_base();
    intent.operation_binding = Some(binding);
    intent.operation_binding.as_mut().unwrap().binding_id = " binding-a".into();
    assert!(
        matches!(validate_intent(&intent), Err(DeployIntentError::InvalidBinding { field }) if field == "binding_id")
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-OPERATION-BINDING",
    r = 3
)]
fn native_six_arms_share_one_bound_invocation_identity() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../schemas/native/e1/deploy_request.jtd.json"
    );
    let schema: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    for arm in [
        "plan",
        "fingerprint",
        "apply",
        "verify",
        "remove",
        "recover",
    ] {
        assert_eq!(
            schema["mapping"][arm]["properties"]["identity"]["ref"], "invocation_identity",
            "{arm}"
        );
    }
    assert_eq!(
        schema["definitions"]["invocation_identity"]["optionalProperties"]["operation_binding"]["ref"],
        "native_operation_binding"
    );
    let vocab_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../formats/vocabularies.json"
    );
    let vocab: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(vocab_path).unwrap()).unwrap();
    assert_eq!(
        schema["definitions"]["native_operation_binding"]["properties"],
        vocab["environment_operation_binding"]["properties"],
        "native copy must match strict binding fields"
    );
    let native: NativeOperationBinding =
        serde_json::from_value(serde_json::to_value(binding_base()).unwrap()).unwrap();
    let identity = crate::generated::native::e1::deploy_request::InvocationIdentity {
        provider: "provider-a".into(),
        mechanism: "deploy:tool".into(),
        target: "target-a".into(),
        profile: "local".into(),
        operation_binding: Some(native),
    };
    assert!(
        serde_json::to_value(identity)
            .unwrap()
            .get("operation_binding")
            .is_some()
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-OPERATION-BINDING",
    r = 3
)]
fn bound_root_epoch_and_required_nullable_fields_refuse() {
    let binding = binding_base();
    validate_operation_binding_root(&OperationBinding {
        schema: 1,
        binding: binding.clone(),
    })
    .unwrap();
    assert!(
        validate_operation_binding_root(&OperationBinding {
            schema: 2,
            binding: binding.clone()
        })
        .is_err()
    );
    let mut duplicate = binding.clone();
    duplicate
        .root_bindings
        .push(duplicate.root_bindings[0].clone());
    let mut intent = intent_base();
    intent.operation_binding = Some(duplicate);
    assert!(
        matches!(validate_intent(&intent), Err(DeployIntentError::InvalidBinding { field }) if field == "root_bindings.root_role")
    );
    let mut raw = serde_json::to_value(binding).unwrap();
    raw.as_object_mut()
        .unwrap()
        .remove("selected_generation_ref");
    assert!(serde_json::from_value::<EnvironmentOperationBinding>(raw).is_err());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-OPERATION-BINDING",
    r = 3
)]
fn registered_binding_corpus_distinguishes_shape_and_semantic_refusals() {
    let base = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../formats/corpora/environment-operation-binding/e1/"
    );
    let read = |class: &str, name: &str| -> serde_json::Value {
        let path = format!("{base}{class}/{name}.json");
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    };
    let value: OperationBinding = serde_json::from_value(read("valid", "local")).unwrap();
    validate_operation_binding_root(&value).unwrap();
    for name in [
        "unknown-root",
        "missing-selected-generation",
        "unknown-target-mode",
        "missing-target-mode",
    ] {
        assert!(
            serde_json::from_value::<OperationBinding>(read("structural-invalid", name)).is_err(),
            "{name}"
        );
    }
    for name in ["unsupported-binding-epoch", "duplicate-root-role"] {
        let value: OperationBinding =
            serde_json::from_value(read("semantic-invalid", name)).unwrap();
        assert!(validate_operation_binding_root(&value).is_err(), "{name}");
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-LEGACY-LOCAL-OPERATION",
    r = 1
)]
fn legacy_none_omits_binding_and_preserves_pair() {
    let intent = intent_base();
    let receipt = receipt_base();
    validate_operation_binding_pair(&intent, &receipt).unwrap();
    assert!(
        serde_json::to_value(&intent)
            .unwrap()
            .get("operation_binding")
            .is_none()
    );
    assert!(
        serde_json::to_value(&receipt)
            .unwrap()
            .get("operation_binding")
            .is_none()
    );
}

#[test]
fn the_minimal_intent_and_receipt_validate() {
    validate_intent(&intent_base()).unwrap();
    validate_receipt(&receipt_base()).unwrap();
}

#[test]
fn the_full_intent_and_finalised_receipt_validate() {
    let mut intent = intent_base();
    intent.target.package = Some("org.demo/tools".to_string());
    intent.prior_generation = Some(0);
    intent.resources[0].prior_digest = Some(HEX64.to_string());
    intent.resources.push(PlannedResource {
        resource: "~/.vibe/store/payloads/vibe-helper-1.2.3.exe".to_string(),
        desired_digest: HEX64.to_string(),
        prior_digest: None,
    });
    validate_intent(&intent).unwrap();

    let mut receipt = receipt_base();
    receipt.identity.package = Some("org.demo/tools".to_string());
    receipt.generation = 1;
    receipt.provider.version = Some("0.3.0".to_string());
    receipt.provider.content_hash = Some(format!("sha256:{HEX64}"));
    receipt.scope = DestinationScope::Workspace;
    receipt.resources.push(OwnedResource {
        resource: ".vibe/bin/vibe-helper".to_string(),
        post_digest: HEX64.to_string(),
    });
    receipt.evidence = Some("launcher resolves the active receipt".to_string());
    receipt.prior_state_handle = Some("receipts/demo/local-helper/g0000".to_string());
    receipt.status = ReceiptStatus::Verified;
    receipt.finalized_at = Some(ts());
    validate_receipt(&receipt).unwrap();

    // `failed` and `rolled-back` are terminal too — both finalise.
    receipt.status = ReceiptStatus::Failed;
    validate_receipt(&receipt).unwrap();
    receipt.status = ReceiptStatus::RolledBack;
    validate_receipt(&receipt).unwrap();
}

#[test]
fn a_newer_epoch_refuses_on_both_records() {
    let mut intent = intent_base();
    intent.schema = 2;
    assert_eq!(
        validate_intent(&intent),
        Err(DeployIntentError::SchemaEpoch { found: 2 })
    );
    let mut receipt = receipt_base();
    receipt.schema = 2;
    assert_eq!(
        validate_receipt(&receipt),
        Err(DeployReceiptError::SchemaEpoch { found: 2 })
    );
}

#[test]
fn a_short_plan_hash_refuses() {
    let mut intent = intent_base();
    intent.plan_hash = HEX64[..63].to_string();
    assert!(matches!(
        validate_intent(&intent),
        Err(DeployIntentError::PlanHashNotHex { .. })
    ));
}

#[test]
fn the_identity_members_refuse_blank_and_grammar_breaks() {
    let mut intent = intent_base();
    intent.target.project = " ".to_string();
    assert!(matches!(
        validate_intent(&intent),
        Err(DeployIntentError::UnsafeScalar { field, .. }) if field == "target.project"
    ));

    let mut intent = intent_base();
    intent.target.package = Some("demo\n".to_string());
    assert!(matches!(
        validate_intent(&intent),
        Err(DeployIntentError::UnsafeScalar { field, .. }) if field == "target.package"
    ));

    let mut intent = intent_base();
    intent.target.profile = "Local".to_string();
    assert!(matches!(
        validate_intent(&intent),
        Err(DeployIntentError::ProfileNotPortableToken { .. })
    ));

    let mut intent = intent_base();
    intent.target.target = "local helper".to_string();
    assert!(matches!(
        validate_intent(&intent),
        Err(DeployIntentError::TargetNotPortableToken { .. })
    ));
}

#[test]
fn every_resource_row_refuses_its_own_break() {
    let mut intent = intent_base();
    intent.resources[0].resource = "".to_string();
    assert_eq!(
        validate_intent(&intent),
        Err(DeployIntentError::UnsafeResource {
            row: 0,
            value: crate::behaviour::compiler_trace_index::ScalarPreview::of(""),
        })
    );

    let mut intent = intent_base();
    intent.resources[0].desired_digest = HEX64.to_uppercase();
    assert!(matches!(
        validate_intent(&intent),
        Err(DeployIntentError::DesiredDigestNotHex { row: 0, .. })
    ));

    let mut intent = intent_base();
    intent.resources[0].prior_digest = Some(HEX64[..63].to_string());
    assert!(matches!(
        validate_intent(&intent),
        Err(DeployIntentError::PriorDigestNotHex { row: 0, .. })
    ));

    let mut receipt = receipt_base();
    receipt.resources[0].resource = " ".to_string();
    assert!(matches!(
        validate_receipt(&receipt),
        Err(DeployReceiptError::UnsafeResource { row: 0, .. })
    ));

    let mut receipt = receipt_base();
    receipt.resources[0].post_digest = HEX64[..63].to_string();
    assert!(matches!(
        validate_receipt(&receipt),
        Err(DeployReceiptError::PostDigestNotHex { row: 0, .. })
    ));
}

#[test]
fn the_receipt_digests_and_provider_refuse_bad_spellings() {
    let mut receipt = receipt_base();
    receipt.artifact_digest = HEX64[..63].to_string();
    assert!(matches!(
        validate_receipt(&receipt),
        Err(DeployReceiptError::DigestNotHex { member, .. }) if member == "artifact_digest"
    ));

    let mut receipt = receipt_base();
    receipt.desired_config_digest = format!("sha256:{HEX64}");
    assert!(matches!(
        validate_receipt(&receipt),
        Err(DeployReceiptError::DigestNotHex { member, .. }) if member == "desired_config_digest"
    ));

    let mut receipt = receipt_base();
    receipt.provider.key = "org.vibevm/vibe".to_string();
    assert!(matches!(
        validate_receipt(&receipt),
        Err(DeployReceiptError::BadProviderKey { .. })
    ));

    let mut receipt = receipt_base();
    receipt.provider.version = Some("".to_string());
    assert!(matches!(
        validate_receipt(&receipt),
        Err(DeployReceiptError::UnsafeScalar { field, .. }) if field == "provider.version"
    ));

    let mut receipt = receipt_base();
    receipt.provider.content_hash = Some(HEX64.to_string());
    assert!(matches!(
        validate_receipt(&receipt),
        Err(DeployReceiptError::BadContentHash { .. })
    ));

    let mut receipt = receipt_base();
    receipt.evidence = Some("\n".to_string());
    assert!(matches!(
        validate_receipt(&receipt),
        Err(DeployReceiptError::UnsafeScalar { field, .. }) if field == "evidence"
    ));

    let mut receipt = receipt_base();
    receipt.prior_state_handle = Some(" ".to_string());
    assert!(matches!(
        validate_receipt(&receipt),
        Err(DeployReceiptError::UnsafeScalar { field, .. }) if field == "prior_state_handle"
    ));
}

#[test]
fn the_finalisation_matrix_holds_in_all_four_rows() {
    // applied ⇒ no finalized_at: the base proves it.
    // applied + finalized_at ⇒ refuse.
    let mut receipt = receipt_base();
    receipt.finalized_at = Some(ts());
    assert_eq!(
        validate_receipt(&receipt),
        Err(DeployReceiptError::AppliedFinalised)
    );

    // terminal without finalized_at ⇒ refuse, for all three terminal
    // statuses.
    for status in [
        ReceiptStatus::Verified,
        ReceiptStatus::Failed,
        ReceiptStatus::RolledBack,
    ] {
        let mut receipt = receipt_base();
        receipt.status = status.clone();
        assert_eq!(
            validate_receipt(&receipt),
            Err(DeployReceiptError::TerminalNotFinalised {
                status: status.clone()
            }),
            "{status:?} must refuse without finalized_at"
        );
        // …and the same status with one validates — the matrix is
        // exact, not one-directional.
        receipt.finalized_at = Some(ts());
        validate_receipt(&receipt).unwrap();
    }
}

#[test]
fn the_intent_timestamps_are_typed_end_to_end() {
    // The generated reader refuses a non-RFC 3339 timestamp before the
    // cell ever runs — the date has one spelling, and it is not a
    // string. A blank or malformed `started_at` cannot reach the
    // validator as text at all.
    let json = serde_json::json!({
        "schema": INTENT_EPOCH,
        "plan_hash": HEX64,
        "target": {
            "project": "demo",
            "profile": "local",
            "target": "local-helper",
            "generation": 0
        },
        "resources": [],
        "started_at": "not-a-timestamp"
    });
    assert!(serde_json::from_value::<DeployIntent>(json).is_err());
    let _: IntentTimestamp = "2026-08-29T12:30:00Z".parse().expect("RFC 3339 parses");
    let _: ReceiptTimestamp = "2026-08-29T12:30:00Z".parse().expect("RFC 3339 parses");
}
