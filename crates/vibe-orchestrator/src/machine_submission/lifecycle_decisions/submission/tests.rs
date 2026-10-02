use super::*;
#[cfg(target_os = "linux")]
#[test]
fn canonical_request_and_plan_bind_owned_admission_before_attachment_without_question_nonce() {
    let fixture = native_tests::Fixture::new(false);
    let host = fixture.host();
    let first = native_tests::approved(&host, RequestedOptionsControl::None);
    let next = native_tests::approved(&host, RequestedOptionsControl::None);
    let old_request = first.base.request.clone();
    let old_plan = first.base.plan.clone();
    let a = AdmittedProfile::from_owned(&host, &first).unwrap();
    let b = AdmittedProfile::from_owned(&host, &next).unwrap();
    assert_ne!(first.question_id, next.question_id);
    assert_eq!(
        a.request(&old_request).unwrap(),
        b.request(&old_request).unwrap()
    );
    assert_eq!(a.plan(&old_plan).unwrap(), b.plan(&old_plan).unwrap());
    let request: serde_json::Value =
        serde_json::from_slice(&a.request(&old_request).unwrap()).unwrap();
    assert_eq!(request["accepted_plan"], first.identity());
    assert_eq!(request["requested_options"]["control"], "none");
    assert_eq!(
        request["base_v4"],
        serde_json::from_slice::<serde_json::Value>(&old_request).unwrap()
    );
    assert_eq!(first.base.request, old_request);
    assert_eq!(first.base.plan, old_plan);
    // An independently approved resulting plan cannot attach this request.
    let other = native_tests::approved(&host, RequestedOptionsControl::Detach);
    let c = AdmittedProfile::from_owned(&host, &other).unwrap();
    assert_ne!(
        a.request(&old_request).unwrap(),
        c.request(&old_request).unwrap()
    );
    assert_ne!(a.plan(&old_plan).unwrap(), c.plan(&old_plan).unwrap());
}
