//! Shape-only checks for producer-owned environment formats. Hash and target
//! cross-joins require a semantic validator; a generated reader cannot prove them.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-FORMAT-EPOCHS");

use serde::de::DeserializeOwned;
use vibe_wire::generated::environment::e1::{
    definition::Definition, exact_lock::ExactLock, generation_receipt::GenerationReceipt,
    payload_inventory::PayloadInventory, realization_error::RealizationError,
    realization_request::RealizationRequest,
};

#[path = "wire_support/mod.rs"]
mod wire_support;

fn path(format: &str, class: &str, name: &str) -> String {
    format!("formats/corpora/{format}/e1/{class}/{name}.json")
}

fn valid<T: DeserializeOwned + serde::Serialize>(format: &str) -> T {
    let path = path(format, "valid", "representative");
    let value: T = wire_support::round_trip(&path);
    assert_eq!(
        serde_json::to_value(&value).unwrap(),
        wire_support::read_json(&path)
    );
    value
}

fn structural_refusals<T: DeserializeOwned>(format: &str) {
    for name in ["unknown-root", "missing-schema"] {
        let path = path(format, "structural-invalid", name);
        assert!(
            serde_json::from_value::<T>(wire_support::read_json(&path)).is_err(),
            "{path}"
        );
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-FORMAT-EPOCHS",
    r = 1
)]
fn six_registered_roots_roundtrip_and_reject_unknown_or_missing_fields() {
    let _: Definition = valid("environment-definition");
    let _: ExactLock = valid("environment-exact-lock");
    let _: RealizationRequest = valid("environment-realization-request");
    let _: PayloadInventory = valid("environment-payload-inventory");
    let _: GenerationReceipt = valid("environment-generation-receipt");
    let _: RealizationError = valid("environment-realization-error");
    structural_refusals::<Definition>("environment-definition");
    structural_refusals::<ExactLock>("environment-exact-lock");
    structural_refusals::<RealizationRequest>("environment-realization-request");
    structural_refusals::<PayloadInventory>("environment-payload-inventory");
    structural_refusals::<GenerationReceipt>("environment-generation-receipt");
    structural_refusals::<RealizationError>("environment-realization-error");
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-EXACT-LOCK-IDENTITY",
    r = 2
)]
fn exact_recipe_and_produced_output_have_distinct_carriers() {
    let lock: ExactLock = valid("environment-exact-lock");
    let generation: GenerationReceipt = valid("environment-generation-receipt");
    assert_eq!(lock.recipe_epoch, 1);
    assert_eq!(generation.recipe_epoch, 1);
    assert_eq!(generation.lock_digest_sha256, lock.lock_digest_sha256);
    assert_ne!(generation.generation_digest_sha256, lock.lock_digest_sha256);
    assert_eq!(lock.selected_captures[0].source_capture_ref, "capture-a");
    assert_eq!(lock.selected_captures[0].package_group, "org.example");
    assert_eq!(lock.selected_captures[0].package_name, "app");
    assert_eq!(lock.selected_captures[0].exact_version, "1.1.0");
    assert_eq!(lock.selected_captures[0].content_hash_value, "content-a");
    assert_eq!(lock.selected_captures[0].capture_owner, "source-owner-a");
    assert_eq!(lock.selected_captures[0].configuration_ref, "config-a");
    assert_eq!(
        lock.selected_captures[0].role,
        vibe_wire::generated::environment::e1::exact_lock::DependencyRole::BuildAndRuntime
    );
    let reordered_path = path(
        "environment-exact-lock",
        "semantic-invalid",
        "reordered-equivalent-input",
    );
    let reordered: ExactLock =
        serde_json::from_value(wire_support::read_json(&reordered_path)).unwrap();
    assert_eq!(reordered.lock_digest_sha256, lock.lock_digest_sha256);
    assert_ne!(reordered.dependency_closure, lock.dependency_closure);
    let output_path = path(
        "environment-generation-receipt",
        "semantic-invalid",
        "changed-output-stale-digest",
    );
    let stale: GenerationReceipt =
        serde_json::from_value(wire_support::read_json(&output_path)).unwrap();
    assert_eq!(
        stale.generation_digest_sha256,
        generation.generation_digest_sha256
    );
    assert_ne!(
        stale.generated_output_digests_sha256,
        generation.generated_output_digests_sha256
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-076#ENVIRONMENT-PAYLOAD-INVENTORY",
    r = 1
)]
fn platform_metadata_arms_and_semantic_refusal_fixtures_remain_expressible() {
    let inventory: PayloadInventory = valid("environment-payload-inventory");
    assert_eq!(inventory.entries[0].path_components_hex[0], "757372");
    for name in [
        "macos-linux-principal",
        "unsupported-complete",
        "unsupported-epoch",
    ] {
        let path = path("environment-payload-inventory", "semantic-invalid", name);
        let _: PayloadInventory = serde_json::from_value(wire_support::read_json(&path)).unwrap();
    }
    let lock_path = path(
        "environment-exact-lock",
        "semantic-invalid",
        "changed-abi-stale-digest",
    );
    let _: ExactLock = serde_json::from_value(wire_support::read_json(&lock_path)).unwrap();
    let request_path = path(
        "environment-realization-request",
        "semantic-invalid",
        "wrong-payload-role",
    );
    let _: RealizationRequest =
        serde_json::from_value(wire_support::read_json(&request_path)).unwrap();
}
