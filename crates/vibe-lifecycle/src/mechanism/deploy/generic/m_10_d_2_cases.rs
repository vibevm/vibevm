use super::tests::file_fixture;
use super::*;
use vibe_core::manifest::ClientScope;
use vibe_workspace::derivatives::MaterializedDerivative;
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/next/derivatives/service_fixture.rs"
));
const CLOCK: &str = "2026-10-19T00:00:00Z";

fn payload(result: &MaterializedDerivative) -> Vec<GenericPayload> {
    result.verify_tree().unwrap();
    vec![GenericPayload::File(
        result.result().binding().local().files[0].bytes.clone(),
    )]
}

#[test]
fn exact_derivative_payload_update_revocation_and_fresh_retained_cleanup() {
    let source = Fixture::new("opaque-file");
    let derivative = MaterializedDerivative::derive(
        &source.text,
        &source.base,
        &source.local,
        &source.output,
        "result",
    )
    .unwrap();
    let f = file_fixture();
    let plan = f.plan(payload(&derivative));
    let grant = f.host.approve(&plan).unwrap();
    f.host.apply(&plan, Some(&grant), CLOCK).unwrap();
    assert_eq!(
        f.bytes(ClientScope::Project, "new/file"),
        derivative.result().binding().local().files[0].bytes
    );
    // Upgrade via a freshly captured payload and exact plan, no inherited grant.
    let mut local = source.binding.local().clone();
    local.files[0]
        .bytes
        .extend_from_slice(b"new independently authored bytes");
    source
        .local
        .write_atomic(&local.files[0].path, &local.files[0].bytes)
        .unwrap();
    let binding = DerivationBinding::new(
        source.binding.base().clone(),
        local,
        source.binding.base_capture().to_owned(),
        DerivationRecipe::ReplaceSelectedInventory,
    )
    .unwrap();
    let updated = MaterializedDerivative::derive(
        &toml::to_string(&binding).unwrap(),
        &source.base,
        &source.local,
        &source.output,
        "updated",
    )
    .unwrap();
    assert_ne!(
        derivative.result().record().result_identity,
        updated.result().record().result_identity
    );
    let second = f.plan(payload(&updated));
    assert!(f.host.apply(&second, Some(&grant), CLOCK).is_err());
    f.host
        .apply(&second, Some(&f.host.approve(&second).unwrap()), CLOCK)
        .unwrap();
    let revoke = f
        .host
        .prepare_retained(plan.target(), RetainedOperation::Revocation)
        .unwrap();
    let revoked_grant = f.host.approve_retained(&revoke).unwrap();
    f.host
        .execute_retained(&revoke, Some(&revoked_grant), CLOCK)
        .unwrap();
    f.authority.selected.lock().unwrap().enabled = false;
    f.authority.selected.lock().unwrap().selected = false;
    assert!(matches!(
        f.host.inspect_receipt(plan.target()).unwrap(),
        ReceiptInspection::Retained {
            receipt: Some(_),
            ..
        }
    ));
    let cleanup = f
        .host
        .prepare_retained(plan.target(), RetainedOperation::Cleanup)
        .unwrap();
    assert!(
        f.host
            .execute_retained(&cleanup, Some(&revoked_grant), CLOCK)
            .is_err()
    );
    let cleanup_grant = f.host.approve_retained(&cleanup).unwrap();
    f.host
        .execute_retained(&cleanup, Some(&cleanup_grant), CLOCK)
        .unwrap();
    assert!(f.host.project.read_file("new/file").unwrap().is_none());
    assert_eq!(
        MaterializedDerivative::read(
            &source.output.subproject(&["updated"]).unwrap(),
            &source.base,
            &source.local,
            1 << 20
        )
        .unwrap()
        .result()
        .record(),
        updated.result().record()
    );
}

#[test]
fn derivative_publication_interruption_retains_intent_then_recovers_with_new_authority() {
    let source = Fixture::new("opaque-file");
    let derivative = MaterializedDerivative::derive(
        &source.text,
        &source.base,
        &source.local,
        &source.output,
        "result",
    )
    .unwrap();
    let f = file_fixture();
    let first = f.plan(vec![GenericPayload::File(b"old".to_vec())]);
    f.host
        .apply(&first, Some(&f.host.approve(&first).unwrap()), CLOCK)
        .unwrap();
    let plan = f.plan(payload(&derivative));
    let grant = f.host.approve(&plan).unwrap();
    vibe_safefs::fail_after_publish(Some("new/file"));
    let effect = f.host.apply(&plan, Some(&grant), CLOCK);
    vibe_safefs::fail_after_publish(None);
    assert!(effect.is_err());
    let state = super::super::state::DeployState::observe_bound(&f.host.settings)
        .unwrap()
        .unwrap();
    let home = f.host.home(plan.target());
    let pending = state.read_generic(&home).unwrap().unwrap().pending.unwrap();
    assert_eq!(
        pending.resources[0].progress,
        vibe_wire::generated::deploy_generic_state::ResourceProgress::Uncertain
    );
    assert!(state.read_intent(&home).unwrap().is_some());
    assert_eq!(state.read_receipt(&home).unwrap().unwrap().generation, 0);
    let admitted = f.admitted(payload(&derivative));
    assert!(f.host.prepare_recovery(plan.target(), None).is_err());
    let recovery = f
        .host
        .prepare_recovery(plan.target(), Some(&admitted))
        .unwrap();
    f.host
        .execute_retained(
            &recovery,
            Some(&f.host.approve_retained(&recovery).unwrap()),
            CLOCK,
        )
        .unwrap();
    assert_eq!(state.read_receipt(&home).unwrap().unwrap().generation, 1);
    assert!(
        state
            .read_generic(&home)
            .unwrap()
            .unwrap()
            .pending
            .is_none()
    );
    assert_eq!(
        f.bytes(ClientScope::Project, "new/file"),
        derivative.result().binding().local().files[0].bytes
    );
}

#[test]
fn retained_cleanup_refuses_changed_receipt_and_foreign_digest_without_rollback() {
    let source = Fixture::new("opaque-file");
    let derivative = MaterializedDerivative::derive(
        &source.text,
        &source.base,
        &source.local,
        &source.output,
        "result",
    )
    .unwrap();
    let f = file_fixture();
    let plan = f.plan(payload(&derivative));
    f.host
        .apply(&plan, Some(&f.host.approve(&plan).unwrap()), CLOCK)
        .unwrap();
    let cleanup = f
        .host
        .prepare_retained(plan.target(), RetainedOperation::Cleanup)
        .unwrap();
    let grant = f.host.approve_retained(&cleanup).unwrap();
    let state = super::super::state::DeployState::observe_bound(&f.host.settings)
        .unwrap()
        .unwrap();
    let home = f.host.home(plan.target());
    let original = state.read_receipt(&home).unwrap().unwrap();
    let mut changed = original.clone();
    changed.evidence = Some("foreign record edit".into());
    state.write_receipt(&home, &changed).unwrap();
    assert!(
        f.host
            .execute_retained(&cleanup, Some(&grant), CLOCK)
            .is_err()
    );
    assert_eq!(
        f.bytes(ClientScope::Project, "new/file"),
        derivative.result().binding().local().files[0].bytes
    );
    state.write_receipt(&home, &original).unwrap();
    // Deliberate external tampering is a negative input, never a simulated effect.
    f.write(ClientScope::Project, "new/file", b"foreign third digest");
    assert!(
        f.host
            .execute_retained(&cleanup, Some(&grant), CLOCK)
            .is_err()
    );
    assert_eq!(
        f.bytes(ClientScope::Project, "new/file"),
        b"foreign third digest"
    );
}
