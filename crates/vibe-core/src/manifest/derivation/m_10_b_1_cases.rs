use super::*;
fn source(owner: &str) -> RetainedSource {
    RetainedSource {
        owner: PackageRef::parse(&format!("flow:org.example/{owner}@=1.0.0")).unwrap(),
        resource: "payload".into(),
        descriptor: toml::from_str("file = 'payload.bin'\nmedia = 'application/octet-stream'\ninterpretation = 'opaque'\nrole = 'resource'\nloading = 'explicit'").unwrap(),
        files: vec![RetainedFile { path: "payload.bin".into(), bytes: vec![0, 255, 13, 10] }],
    }
}
fn binding() -> DerivationBinding {
    let base = source("upstream");
    DerivationBinding::new(
        base.clone(),
        source("local"),
        base.observation_id().unwrap(),
        DerivationRecipe::ReplaceSelectedInventory,
    )
    .unwrap()
}
#[test]
fn retained_round_trip_and_comments() {
    let binding = binding();
    let text = format!(
        "# retained author comment\n{}",
        toml::to_string(&binding).unwrap()
    );
    assert_eq!(DerivationBinding::parse(&text).unwrap(), binding);
    assert_eq!(retain_binding_document(&text).unwrap(), text);
    assert!(DerivationBinding::parse(&text.replace("schema = 1", "schema = 0")).is_err());
    assert!(
        DerivationBinding::parse(&text.replace("schema = 1", "legacy_base = 'implicit'")).is_err()
    );
}
#[test]
fn equal_bytes_keep_owner_and_base_provenance() {
    let first = binding();
    let base = source("another-base");
    let second = DerivationBinding::new(
        base.clone(),
        source("another-local"),
        base.observation_id().unwrap(),
        DerivationRecipe::ReplaceSelectedInventory,
    )
    .unwrap();
    assert_eq!(
        first.local().inventory_id().unwrap(),
        second.local().inventory_id().unwrap()
    );
    assert_ne!(
        first.provenance_id().unwrap(),
        second.provenance_id().unwrap()
    );
    assert_ne!(first.result_id().unwrap(), second.result_id().unwrap());
    assert_ne!(
        first.base_capture(),
        first.local().observation_id().unwrap()
    );
    assert_ne!(
        first.result_id().unwrap(),
        first.local().observation_id().unwrap()
    );
}
#[test]
fn duplicate_upstream_missing_and_stale_refuse() {
    let binding = binding();
    assert!(DerivationBinding::validate_set(&[binding.clone(), binding.clone()]).is_err());
    let mut aliased = binding.clone();
    aliased.0.local.resource = "different-resource".into();
    assert!(DerivationBinding::validate_set(&[binding.clone(), aliased]).is_err());
    let make = |base, local, capture| {
        DerivationBinding::new(
            base,
            local,
            capture,
            DerivationRecipe::ReplaceSelectedInventory,
        )
    };
    assert!(
        make(
            source("upstream"),
            source("upstream"),
            binding.base_capture().into()
        )
        .is_err()
    );
    assert!(make(source("upstream"), source("local"), String::new()).is_err());
    let mut changed = source("upstream");
    changed.files[0].bytes.push(1);
    assert!(make(changed, source("local"), binding.base_capture().into()).is_err());
}
#[test]
fn interpretation_shape_alias_and_cache_only_refuse() {
    let mut source = source("local");
    let text = toml::to_string(&source.descriptor).unwrap();
    assert!(toml::from_str::<SourceDescriptor>(&format!("directory = 'payload'\n{text}")).is_err());
    assert!(toml::from_str::<SourceDescriptor>(&text.replace("opaque", "guess")).is_err());
    source.files.clear();
    assert!(source.validate().is_err());
    source.descriptor = toml::from_str("directory = 'payload'\nmedia = 'application/octet-stream'\ninterpretation = 'opaque'\nrole = 'resource'\nloading = 'explicit'").unwrap();
    source.files = vec![
        RetainedFile {
            path: "payload/A".into(),
            bytes: vec![],
        },
        RetainedFile {
            path: "payload/a".into(),
            bytes: vec![],
        },
    ];
    assert!(source.validate().is_err());
}
#[test]
fn explicit_tree_xml_vibe_and_raw_interpretations() {
    for (interpretation, role, media) in [
        ("opaque", "resource", "application/octet-stream"),
        ("xml-data", "resource", "application/xml"),
        ("vibe-spec", "spec", "application/xml"),
    ] {
        let descriptor: SourceDescriptor = toml::from_str(&format!("directory = 'payload'\nmedia = '{media}'\ninterpretation = '{interpretation}'\nrole = '{role}'\nloading = 'explicit'")).unwrap();
        let mut base = source("upstream");
        base.descriptor = descriptor.clone();
        base.files[0].path = "payload/data.xml".into();
        let mut local = base.clone();
        local.owner = source("local").owner;
        assert!(
            DerivationBinding::new(
                base.clone(),
                local,
                base.observation_id().unwrap(),
                DerivationRecipe::ReplaceSelectedInventory
            )
            .is_ok()
        );
    }
}
#[test]
fn hash_framing_and_inventory_order_are_explicit() {
    let mut first = Framed::new("test/1");
    first.field("a", "bc");
    first.field("d", "e");
    let mut second = Framed::new("test/1");
    second.field("a", "b");
    second.field("cd", "e");
    assert_ne!(first.finish(), second.finish());
    let mut source = source("base");
    source.descriptor = toml::from_str("directory = 'payload'\nmedia = 'application/octet-stream'\ninterpretation = 'opaque'\nrole = 'resource'\nloading = 'explicit'").unwrap();
    source.files = vec![
        RetainedFile {
            path: "payload/a".into(),
            bytes: vec![0],
        },
        RetainedFile {
            path: "payload/b".into(),
            bytes: vec![255],
        },
    ];
    let id = source.observation_id().unwrap();
    source.files.reverse();
    assert_eq!(source.observation_id().unwrap(), id);
}
