//! Versioned profile choice through the one retained Deploy owner.
use super::*;

fn request(format: FormatId, action: serde_json::Value) -> Vec<u8> {
    serde_json::to_vec(&json!({"schema":1,
        "operation_id":crate::machine_operation::fixed_deploy::OPERATION_ID,
        "domain_owner":crate::machine_operation::fixed_deploy::DOMAIN_OWNER,
        "source_id":null,"evidence_id":null,
        "body":{"stage":"input","format_id":format.id(),
            "payload_json":json!({"schema":1,"action":action}).to_string()}}))
    .unwrap()
}

#[test]
fn profiled_request_selects_authored_allowed_profile_and_key_cannot_change_it() {
    let fixture = Fixture::new();
    let manifest = fixture.project.path().join("vibe.toml");
    let mut authored = fs::read_to_string(&manifest).unwrap();
    authored.push_str("\n[deploy.profiles.alt]\ntargets = ['local-helper']\n");
    fs::write(&manifest, authored).unwrap();
    let service = fixture.service();
    let host = service
        .issue(60_000)
        .unwrap()
        .with_allowed_profiles(vec![("alt".into(), target())])
        .unwrap();
    let route = FixedDeployRoute::new(Arc::new(host));
    let profiled = request(
        FormatId::MachineProfiledDeployActionRequest,
        json!({"kind":"submit","key":"one-key","deploy_profile":"alt"}),
    );
    let first =
        FixedDeployRoute::exchange_local(Some(&route), &profiled, fixture.project.path()).unwrap();
    let first_json = serde_json::to_value(&first).unwrap();
    assert_eq!(first_json["body"]["stage"], "result", "{first_json}");
    let reply: serde_json::Value =
        serde_json::from_str(first_json["body"]["payload_json"].as_str().unwrap()).unwrap();
    let receipt: serde_json::Value =
        serde_json::from_str(reply["reply"]["receipt_json"].as_str().unwrap()).unwrap();
    assert_eq!(receipt["profile"], "alt");

    let foreign = request(
        FormatId::MachineProfiledDeployActionRequest,
        json!({"kind":"submit","key":"foreign-key","deploy_profile":"foreign"}),
    );
    let refused =
        FixedDeployRoute::exchange_local(Some(&route), &foreign, fixture.project.path()).unwrap();
    let refused_json = serde_json::to_value(refused).unwrap();
    assert_eq!(refused_json["body"]["stage"], "error");
    let legacy_same_key = request(
        FormatId::MachineFixedDeployActionRequest,
        json!({"kind":"submit","key":"one-key"}),
    );
    let changed =
        FixedDeployRoute::exchange_local(Some(&route), &legacy_same_key, fixture.project.path())
            .unwrap();
    let changed_json = serde_json::to_value(changed).unwrap();
    assert_eq!(changed_json["body"]["stage"], "error");
    let replay =
        FixedDeployRoute::exchange_local(Some(&route), &profiled, fixture.project.path()).unwrap();
    assert_eq!(replay, first);

    let lease = LifecycleLease::acquire(fixture.project.path()).unwrap();
    let pointer = service.read(&lease).unwrap().unwrap();
    let namespace = service.validate_namespace(&lease, &pointer).unwrap();
    assert!(
        SubmissionReservations::inspect_existing_key(
            &lease,
            &namespace,
            &service.scope,
            "foreign-key"
        )
        .unwrap()
        .is_none()
    );
}

#[test]
fn explicit_primary_profile_and_legacy_absence_never_alias_one_key() {
    for profiled_first in [false, true] {
        let fixture = Fixture::new();
        let service = fixture.service();
        let route = FixedDeployRoute::new(Arc::new(service.issue(60_000).unwrap()));
        let legacy = request(
            FormatId::MachineFixedDeployActionRequest,
            json!({"kind":"submit","key":"same-primary"}),
        );
        let profiled = request(
            FormatId::MachineProfiledDeployActionRequest,
            json!({"kind":"submit","key":"same-primary","deploy_profile":"local"}),
        );
        let (first, changed) = if profiled_first {
            (&profiled, &legacy)
        } else {
            (&legacy, &profiled)
        };
        let completed =
            FixedDeployRoute::exchange_local(Some(&route), first, fixture.project.path()).unwrap();
        assert_eq!(
            serde_json::to_value(&completed).unwrap()["body"]["stage"],
            "result"
        );
        let refused =
            FixedDeployRoute::exchange_local(Some(&route), changed, fixture.project.path())
                .unwrap();
        assert_eq!(
            serde_json::to_value(refused).unwrap()["body"]["stage"],
            "error"
        );
        let replay =
            FixedDeployRoute::exchange_local(Some(&route), first, fixture.project.path()).unwrap();
        assert_eq!(replay, completed);
        let state: serde_json::Value = serde_json::from_slice(
            &fs::read(
                fixture
                    .project
                    .path()
                    .join(".vibe/machine-submissions.json"),
            )
            .unwrap(),
        )
        .unwrap();
        let namespaces = state["namespaces"].as_object().unwrap();
        assert_eq!(namespaces.len(), 1);
        assert_eq!(
            namespaces.values().next().unwrap()["keys"]
                .as_object()
                .unwrap()
                .len(),
            1
        );
    }
}
