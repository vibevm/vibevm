use super::*;
use vibe_core::manifest::derivation::DerivationRecipe;
use vibe_safefs::Project;

fn source(owner: &str, bytes: &[u8]) -> SourceInput {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("data.bin"), bytes).unwrap();
    let snapshot = Project::open(dir.path())
        .unwrap()
        .read_file_snapshot_bounded("data.bin", 1024)
        .unwrap()
        .unwrap();
    let descriptor = serde_json::from_value(serde_json::json!({"file":"data.bin","media":"application/octet-stream","interpretation":"opaque","role":"resource","loading":"explicit"})).unwrap();
    SourceInput::new(
        PackageRef::parse(&format!("flow:org.example/{owner}@=1.0.0")).unwrap(),
        descriptor,
        vec![CapturedSourceFile::new("data.bin".into(), snapshot).unwrap()],
    )
    .unwrap()
}
fn text(base: &SourceInput, local: &SourceInput) -> String {
    let base = base.retained_source("payload");
    let binding = DerivationBinding::new(
        base.clone(),
        local.retained_source("payload"),
        base.observation_id().unwrap(),
        DerivationRecipe::ReplaceSelectedInventory,
    )
    .unwrap();
    format!(
        "# local author comment\n{}",
        toml::to_string(&binding).unwrap()
    )
}
#[test]
fn retained_chain_survives_deleted_cache_and_round_trips() {
    let base = source("base", b"base");
    let local = source("local", b"\0\xff\r\n");
    let authored = text(&base, &local);
    let result = prepare_binding(&authored, Some(&base)).unwrap();
    assert_eq!(result.effects, Effects::NoExternalEffects);
    assert_eq!(
        result.value.strength(),
        crate::source_query::SourceStrength::Observed
    );
    let saved = serde_json::to_vec(result.value.record()).unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(cache.path().join("output"), b"disposable").unwrap();
    cache.close().unwrap();
    drop(result);
    drop(local);
    let read = BoundDerivative::read(serde_json::from_slice(&saved).unwrap(), Some(&base)).unwrap();
    assert_eq!(read.record().binding_toml, authored);
    assert_eq!(read.binding().base().files[0].bytes, b"base");
    assert_eq!(read.binding().local().files[0].bytes, b"\0\xff\r\n");
}
#[test]
fn missing_stale_base_and_forged_result_refuse() {
    let base = source("base", b"base");
    let local = source("local", b"local");
    let authored = text(&base, &local);
    assert!(prepare_binding(&authored, None).is_err());
    assert!(prepare_binding(&authored, Some(&source("base", b"changed"))).is_err());
    let mut record = prepare_binding(&authored, Some(&base))
        .unwrap()
        .value
        .record;
    record.result_identity = record.inventory_identity.clone();
    assert!(BoundDerivative::read(record, Some(&base)).is_err());
}
#[test]
fn identical_output_separate_owners_and_bases() {
    let base = source("base", b"identical");
    let local = source("local", b"identical");
    let other = source("other", b"identical");
    let first = prepare_binding(&text(&base, &local), Some(&base)).unwrap();
    let second = prepare_binding(&text(&other, &local), Some(&other)).unwrap();
    assert_eq!(
        first.value.record.inventory_identity,
        second.value.record.inventory_identity
    );
    assert_ne!(
        first.value.record.provenance_identity,
        second.value.record.provenance_identity
    );
    assert_ne!(
        first.value.record.result_identity,
        second.value.record.result_identity
    );
}
#[test]
fn cache_only_and_ambiguous_interpretation_refuse() {
    let base = source("base", b"base");
    let local = source("local", b"local");
    let authored = text(&base, &local);
    assert!(
        prepare_binding(
            &authored.replace("bytes = [108, 111, 99, 97, 108]", "cache = 'output'"),
            Some(&base)
        )
        .is_err()
    );
    assert!(
        prepare_binding(
            &authored.replace("interpretation = \"opaque\"", "interpretation = \"guess\""),
            Some(&base)
        )
        .is_err()
    );
}
#[test]
fn canonical_vibe_representation_collision_refuses() {
    let raw = source("base", b"base");
    let descriptor: vibe_core::manifest::SourceDescriptor = serde_json::from_value(serde_json::json!({"directory":"spec","media":"application/xml","interpretation":"vibe-spec","role":"spec","loading":"explicit"})).unwrap();
    let snapshot = raw.files().values().next().unwrap().snapshot().clone();
    let files = ["spec/PROP-001-name.xml", "spec/PROP-001-other.md"]
        .into_iter()
        .map(|path| CapturedSourceFile::new(path.into(), snapshot.clone()).unwrap())
        .collect();
    let base = SourceInput::new(raw.coordinate().clone(), descriptor, files).unwrap();
    let mut local = base.retained_source("payload");
    local.owner = source("local", b"local").coordinate().clone();
    let retained = base.retained_source("payload");
    let binding = DerivationBinding::new(
        retained.clone(),
        local,
        retained.observation_id().unwrap(),
        DerivationRecipe::ReplaceSelectedInventory,
    )
    .unwrap();
    assert!(prepare_binding(&toml::to_string(&binding).unwrap(), Some(&base)).is_err());
}
