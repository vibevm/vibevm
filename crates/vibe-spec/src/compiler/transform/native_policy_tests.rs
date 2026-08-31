use vibe_core::manifest::ExtensionKey;

use super::lowering_worlds::Declared;
use super::native_identity::NativeHandlerIdentity;
use super::native_manager_test_support::plan as artifact_plan;
use super::native_policy::session::{
    Availability, NativePolicyResult, NativePolicySession, PendingCapture, UnavailableDisposition,
};
use super::native_policy::{CompilerNativePolicy, CompilerNativePolicyError, CompilerPendingRef};
use super::plan::{ImplementationComponents, TransformPlan, TransformStage};

fn native_plan(ids: &[&'static str]) -> TransformPlan {
    artifact_plan(
        ids.iter()
            .map(|id| Declared::native(id, "compile:source"))
            .collect(),
    )
    .transforms()
    .clone()
}

fn different_implementation() -> super::native_identity::CompilerNativeImplementationDigest {
    NativeHandlerIdentity::candidate(Some("native/different"), None).digest()
}

fn capture_from_plan(plan: &TransformPlan, order: u32) -> PendingCapture {
    let entry = &plan.entries()[order as usize];
    let ImplementationComponents::Native { digest } = entry.seed().implementation().components()
    else {
        panic!("test capture requires native plan entry");
    };
    PendingCapture {
        reference: CompilerPendingRef {
            plan_digest: *plan.digest().unwrap().as_bytes(),
            order,
            key: entry.seed().key().clone(),
        },
        point: match entry.seed().stage() {
            TransformStage::Source => vibe_core::lifecycle::CompilePoint::Source,
            TransformStage::Document => vibe_core::lifecycle::CompilePoint::Document,
            TransformStage::Lane => vibe_core::lifecycle::CompilePoint::Lane,
            TransformStage::Emitted => vibe_core::lifecycle::CompilePoint::Emitted,
        },
        config: entry.config_digest().map(|value| *value.as_bytes()),
        implementation: digest,
    }
}

fn record_success(
    session: &NativePolicySession,
    plan: &TransformPlan,
    order: u32,
) -> Result<(), CompilerNativePolicyError> {
    let capture = capture_from_plan(plan, order);
    session.success(
        order,
        &capture.reference.key,
        capture.point,
        plan.entries()[order as usize].seed().config(),
        capture.implementation,
    )
}

fn record_unavailable(
    session: &NativePolicySession,
    plan: &TransformPlan,
    order: u32,
) -> Result<UnavailableDisposition, CompilerNativePolicyError> {
    let capture = capture_from_plan(plan, order);
    session.unavailable(
        order,
        &capture.reference.key,
        capture.point,
        plan.entries()[order as usize].seed().config(),
        capture.implementation,
    )
}

const fn available() -> Availability {
    Availability::Available
}

const fn unavailable() -> Availability {
    Availability::Unavailable
}

fn collect(plan: &TransformPlan, orders: &[u32]) -> super::native_policy::CompilerPendingSet {
    let session = NativePolicySession::new(plan, CompilerNativePolicy::collect()).unwrap();
    for order in orders {
        assert_eq!(
            record_unavailable(&session, plan, *order).unwrap(),
            UnavailableDisposition::ContinueOriginal
        );
    }
    match session.finish().unwrap() {
        NativePolicyResult::Collected(set) => set,
        _ => panic!("collect policy returns collected state"),
    }
}

fn assert_policy_error(result: Result<UnavailableDisposition, CompilerNativePolicyError>) {
    assert!(result.is_err());
}

#[test]
fn collect_normalizes_dense_order_and_repeated_identical_unavailability_coalesces() {
    let plan = native_plan(&["first", "middle", "last"]);
    let before = plan.digest_hex();
    let set = collect(&plan, &[2, 0, 2, 1, 0]);
    assert_eq!(set.len(), 3);
    assert!(!set.is_empty());
    assert_eq!(
        set.iter().map(|entry| entry.order()).collect::<Vec<_>>(),
        [0, 1, 2]
    );
    assert!(
        set.iter()
            .all(|entry| entry.plan_digest_bytes().len() == 32)
    );
    assert!(set.iter().all(|entry| entry.plan_digest_hex().len() == 64));
    assert_eq!(
        plan.digest_hex(),
        before,
        "pending state never changes plan identity"
    );
}

#[test]
fn every_private_capture_member_conflict_is_red() {
    let plan = native_plan(&["one"]);
    let key = ExtensionKey::authored("__host__/demo#different");
    let mut plan_changed = capture_from_plan(&plan, 0);
    plan_changed.reference.plan_digest[0] ^= 1;
    let mut key_changed = capture_from_plan(&plan, 0);
    key_changed.reference.key = key;
    let mut point_changed = capture_from_plan(&plan, 0);
    point_changed.point = vibe_core::lifecycle::CompilePoint::Document;
    let mut config_changed = capture_from_plan(&plan, 0);
    config_changed.config = Some([7; 32]);
    let mut implementation_changed = capture_from_plan(&plan, 0);
    implementation_changed.implementation = different_implementation();
    let mutations = [
        plan_changed,
        key_changed,
        point_changed,
        config_changed,
        implementation_changed,
    ];
    for mutation in mutations {
        let session = NativePolicySession::new(&plan, CompilerNativePolicy::collect()).unwrap();
        session
            .observe_capture_for_test(capture_from_plan(&plan, 0), unavailable())
            .unwrap();
        assert_policy_error(session.observe_capture_for_test(mutation, unavailable()));
    }
}

#[test]
fn collect_refuses_both_mixed_availability_directions() {
    let plan = native_plan(&["one"]);
    for (first, next) in [(available(), unavailable()), (unavailable(), available())] {
        let session = NativePolicySession::new(&plan, CompilerNativePolicy::collect()).unwrap();
        session
            .observe_capture_for_test(capture_from_plan(&plan, 0), first)
            .unwrap();
        assert_policy_error(session.observe_capture_for_test(capture_from_plan(&plan, 0), next));
    }
}

#[test]
fn resolve_counts_repeated_expected_success_and_allows_nonexpected_success() {
    let plan = native_plan(&["expected", "ordinary"]);
    let expected = collect(&plan, &[0]);
    let session = NativePolicySession::new(&plan, CompilerNativePolicy::resolve(expected)).unwrap();
    record_success(&session, &plan, 1).unwrap();
    record_success(&session, &plan, 0).unwrap();
    record_success(&session, &plan, 0).unwrap();
    let receipts = match session.finish().unwrap() {
        NativePolicyResult::Resolved(receipts) => receipts,
        _ => panic!("resolve returns receipts"),
    };
    assert_eq!(receipts.len(), 1);
    assert!(!receipts.is_empty());
    let values = receipts
        .iter()
        .map(|(entry, calls)| (entry.order(), calls))
        .collect::<Vec<_>>();
    assert_eq!(
        values,
        [(0, 2)],
        "handler ok and handler skip share success semantics"
    );
}

#[test]
fn resolve_refuses_residual_unexpected_and_missing_expected() {
    let plan = native_plan(&["expected", "ordinary"]);
    let expected = collect(&plan, &[0]);
    let residual =
        NativePolicySession::new(&plan, CompilerNativePolicy::resolve(expected)).unwrap();
    assert!(record_unavailable(&residual, &plan, 0).is_err());

    let expected = collect(&plan, &[0]);
    let unexpected =
        NativePolicySession::new(&plan, CompilerNativePolicy::resolve(expected)).unwrap();
    assert!(record_unavailable(&unexpected, &plan, 1).is_err());

    let expected = collect(&plan, &[0]);
    let missing = NativePolicySession::new(&plan, CompilerNativePolicy::resolve(expected)).unwrap();
    assert!(missing.finish().is_err());
}

#[test]
fn resolve_validation_rejects_stale_plan_order_key_and_native_identity() {
    let plan = native_plan(&["one", "two"]);

    let mut stale = collect(&plan, &[0]);
    stale.plan_digest.as_mut().unwrap()[0] ^= 1;
    assert!(NativePolicySession::new(&plan, CompilerNativePolicy::resolve(stale)).is_err());

    let mut missing = collect(&plan, &[0]);
    missing.entries[0].reference.order = 99;
    assert!(NativePolicySession::new(&plan, CompilerNativePolicy::resolve(missing)).is_err());

    let mut key = collect(&plan, &[0]);
    key.entries[0].reference.key = ExtensionKey::authored("__host__/demo#changed");
    assert!(NativePolicySession::new(&plan, CompilerNativePolicy::resolve(key)).is_err());

    let mut implementation = collect(&plan, &[0]);
    implementation.entries[0].implementation = different_implementation();
    assert!(
        NativePolicySession::new(&plan, CompilerNativePolicy::resolve(implementation)).is_err()
    );
}

#[test]
fn fail_and_empty_collect_resolve_own_no_pending_entries() {
    let plan = native_plan(&["one"]);
    let fail = NativePolicySession::new(&plan, CompilerNativePolicy::fail()).unwrap();
    assert_eq!(
        record_unavailable(&fail, &plan, 0).unwrap(),
        UnavailableDisposition::Hard
    );
    assert!(matches!(fail.finish().unwrap(), NativePolicyResult::Fail));

    let collect_session = NativePolicySession::new(&plan, CompilerNativePolicy::collect()).unwrap();
    record_success(&collect_session, &plan, 0).unwrap();
    let empty = match collect_session.finish().unwrap() {
        NativePolicyResult::Collected(set) => set,
        _ => panic!("collect result"),
    };
    assert!(empty.is_empty());
    let resolve = NativePolicySession::new(&plan, CompilerNativePolicy::resolve(empty)).unwrap();
    let receipts = match resolve.finish().unwrap() {
        NativePolicyResult::Resolved(receipts) => receipts,
        _ => panic!("resolve result"),
    };
    assert!(receipts.is_empty());
}

#[test]
fn fail_is_fieldless_and_accepts_arbitrary_empty_plan_calls_without_managed_state() {
    let plan = TransformPlan::empty();
    let key = ExtensionKey::authored("__host__/empty#arbitrary");
    let implementation = different_implementation();
    let session = NativePolicySession::new(&plan, CompilerNativePolicy::fail()).unwrap();
    assert!(matches!(session, NativePolicySession::Fail));
    session
        .success(
            u32::MAX,
            &key,
            vibe_core::lifecycle::CompilePoint::Emitted,
            None,
            implementation,
        )
        .unwrap();
    assert_eq!(
        session
            .unavailable(
                u32::MAX,
                &key,
                vibe_core::lifecycle::CompilePoint::Emitted,
                None,
                implementation,
            )
            .unwrap(),
        UnavailableDisposition::Hard
    );
    assert!(matches!(
        session.finish().unwrap(),
        NativePolicyResult::Fail
    ));
}

#[test]
fn public_debug_formats_expose_only_public_refs_counts_and_policy_mode() {
    let plan = native_plan(&["one"]);
    let set = collect(&plan, &[0]);
    let reference = set.iter().next().unwrap();
    let reference_debug = format!("{reference:?}");
    assert_eq!(
        reference_debug,
        format!(
            "CompilerPendingRef {{ plan_digest_hex: \"{}\", order: 0, key: \"__host__/demo#one\" }}",
            reference.plan_digest_hex()
        )
    );
    assert_eq!(
        format!("{set:?}"),
        format!("CompilerPendingSet {{ entries: [{reference_debug}] }}")
    );
    let policy = CompilerNativePolicy::resolve(set);
    assert_eq!(
        format!("{policy:?}"),
        format!(
            "CompilerNativePolicy::Resolve(CompilerPendingSet {{ entries: [{reference_debug}] }})"
        )
    );

    let empty_session = NativePolicySession::new(&plan, CompilerNativePolicy::collect()).unwrap();
    let empty = match empty_session.finish().unwrap() {
        NativePolicyResult::Collected(set) => set,
        _ => panic!("collect result"),
    };
    assert_eq!(format!("{empty:?}"), "CompilerPendingSet { entries: [] }");

    let expected = collect(&plan, &[0]);
    let resolve = NativePolicySession::new(&plan, CompilerNativePolicy::resolve(expected)).unwrap();
    record_success(&resolve, &plan, 0).unwrap();
    record_success(&resolve, &plan, 0).unwrap();
    let receipts = match resolve.finish().unwrap() {
        NativePolicyResult::Resolved(receipts) => receipts,
        _ => panic!("resolve result"),
    };
    assert_eq!(
        format!("{receipts:?}"),
        format!("CompilerInvocationReceipts {{ entries: [({reference_debug}, 2)] }}")
    );
    for hidden in ["point", "config", "implementation", "plan_digest: Some"] {
        assert!(!format!("{receipts:?}{policy:?}{empty:?}").contains(hidden));
    }
}

#[test]
fn poisoned_state_is_typed_and_session_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<NativePolicySession>();

    let plan = native_plan(&["one"]);
    let session = NativePolicySession::new(&plan, CompilerNativePolicy::collect()).unwrap();
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        session.hold_state_for_test(|| panic!("test poisons policy state"));
    }));
    let error = record_success(&session, &plan, 0).unwrap_err();
    assert!(error.to_string().contains("poisoned"));
}

#[test]
fn state_cell_has_no_payload_or_external_execution_dependency() {
    let source = format!(
        "{}\n{}",
        include_str!("native_policy.rs"),
        include_str!("native_policy/session.rs")
    );
    for forbidden in [
        "std::fs",
        "std::process",
        "vibe_workspace",
        "vibe_lifecycle",
        "DocumentSubject",
        "vibe_wire",
        "CompilerNativeCall",
    ] {
        assert!(
            !source.contains(forbidden),
            "forbidden state dependency `{forbidden}`"
        );
    }
    assert!(!source.to_ascii_lowercase().contains("cargo"));
    assert!(
        !source.contains("payload"),
        "document payload is outside pending capture"
    );
}
