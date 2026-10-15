use super::*;
mod fixture {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/next/adapters/m-08-c1/fixture.rs"
    ));
}
fn result(host: &ClientCapabilitiesHost) -> r::Result {
    let env::Body::Result(body) = host.describe().body else {
        panic!("{:?}", host.describe());
    };
    assert_eq!(body.effect_state, env::BodyResultEffectState::NoEffects);
    serde_json::from_str(body.payload_json.as_ref().unwrap()).unwrap()
}
#[test]
fn selected_package_descriptor_without_native_load() {
    let root = tempfile::tempdir().unwrap();
    fixture::host(root.path());
    assert!(
        result(&ClientCapabilitiesHost::new(root.path(), true).unwrap())
            .capabilities
            .is_empty()
    );
    let slot = fixture::add(root.path());
    let before = fixture::inventory(root.path());
    let host = ClientCapabilitiesHost::new(root.path(), true).unwrap();
    let value = result(&host);
    assert_eq!(result(&host), value);
    assert!(matches!(
        ClientCapabilitiesHost::new(root.path(), false)
            .unwrap()
            .describe()
            .body,
        env::Body::Error(_)
    ));

    assert!(value.capabilities.contains_key("org.example/new-client"));
    assert_eq!(before, fixture::inventory(root.path()));
    std::fs::write(
        slot.join("vibe.toml"),
        fixture::DESCRIPTOR.replace("new.key", "drift"),
    )
    .unwrap();
    assert!(matches!(host.describe().body, env::Body::Error(_)));
}
#[test]
fn wrong_context_operation_and_legacy_route_refuse() {
    let root = tempfile::tempdir().unwrap();
    fixture::host(root.path());
    let host = ClientCapabilitiesHost::new(root.path(), true).unwrap();
    let request = q::Request {
        schema: 1,
        context_id: "foreign".into(),
        target_id: host.target_id().into(),
        action: q::RequestAction::Read,
    };
    let bytes = serde_json::to_vec(&input(&request)).unwrap();
    assert!(matches!(host.exchange(&bytes).body, env::Body::Error(_)));
    let old = super::super::AdapterPlanHost::new(
        root.path(),
        super::super::AdapterTarget::Skills {
            scope: super::super::AdapterScope::Project,
            user_home: None,
        },
    )
    .unwrap();
    let route = super::super::AdapterPlanRoute::new(old, None).unwrap();
    assert!(matches!(route.exchange(&bytes).body, env::Body::Error(_)));
}
