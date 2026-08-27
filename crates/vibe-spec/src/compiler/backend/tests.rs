use super::*;

struct DummyBackend {
    id: BackendId,
    pass: PassName,
}

impl DummyBackend {
    fn new(id: &str, pass: &str) -> Self {
        Self {
            id: BackendId::new(id).unwrap(),
            pass: PassName::new(pass).unwrap(),
        }
    }
}

impl EmitBackend for DummyBackend {
    fn id(&self) -> &BackendId {
        &self.id
    }

    fn pass_name(&self) -> &PassName {
        &self.pass
    }

    fn emit(&self, _lane: &LaneIr, _witness: &PreEmissionWitness) -> Result<Vec<u8>, BackendError> {
        Ok(Vec::new())
    }
}

#[test]
fn backend_ids_reject_blank_uppercase_colon_slash_and_overlength() {
    let invalid = ["", "Static", "static:md", "static/md"];
    for invalid in invalid {
        assert!(BackendId::new(invalid).is_err(), "accepted `{invalid}`");
    }
    assert!(BackendId::new("a".repeat(65)).is_err());
    for valid in ["a", "static-md", "x.y_z-9"] {
        assert_eq!(BackendId::new(valid).unwrap().as_str(), valid);
    }
}

#[test]
fn registry_stores_behavior_and_rejects_collision_and_wrong_pass_identity() {
    let mut registry = BackendRegistry::default();
    registry
        .register(Arc::new(DummyBackend::new("opaque", "emit:opaque")))
        .unwrap();
    assert!(matches!(
        registry.register(Arc::new(DummyBackend::new("opaque", "emit:opaque"))),
        Err(BackendRegistryError::Collision { .. })
    ));
    assert!(matches!(
        registry.register(Arc::new(DummyBackend::new("other", "emit:wrong"))),
        Err(BackendRegistryError::PassIdentity { .. })
    ));
}

#[test]
fn removing_selected_registration_is_a_typed_failure() {
    let registry = BackendRegistry::default();
    assert!(matches!(
        registry.selected(&ArtifactTarget::StaticMarkdown),
        Err(BackendRegistryError::Missing { ref backend }) if backend == "static-md"
    ));
}

#[test]
fn explicit_replacement_keeps_the_stable_engine_owned_identity() {
    let mut registry = BackendRegistry::builtins();
    let replaced = registry
        .replace(Arc::new(DummyBackend::new("static-md", "emit:static-md")))
        .unwrap();
    assert_eq!(replaced.id().as_str(), "static-md");
    let selected = registry.selected(&ArtifactTarget::StaticMarkdown).unwrap();
    assert_eq!(selected.pass_name().as_str(), "emit:static-md");
    assert!(matches!(
        BackendRegistry::default().replace(Arc::new(DummyBackend::new("x", "emit:x"))),
        Err(BackendRegistryError::ReplacementMissing { .. })
    ));
}
