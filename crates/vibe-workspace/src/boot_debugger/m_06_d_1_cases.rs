use super::*;

fn scenario() -> PreparedScenario {
    let host = HostCapture { id: "host-capture".into(),
        manifest: Manifest::parse_str("[project]\nname='fixture'\nversion='1.0.0'\n[requires.packages]\n'org.example/app'={version='1.0.0',link='static'}\n").unwrap(), files: BTreeMap::new() };
    let package = SelectedPackage {
        manifest: Manifest::parse_str("[package]\ngroup='org.example'\nname='app'\nkind='flow'\nversion='1.0.0'\n[boot_snippet]\nsource='boot.md'\n[[boot_snippet.fragment]]\nsource='conditional.md'\nwhen='os:linux'\n").unwrap(),
        files: BTreeMap::from([("boot.md".into(), b"# Fixture\n\nexact bytes\n".to_vec()), ("conditional.md".into(), b"# Conditional\n".to_vec())]),
        requires: vec![], source_identity: "source-owner-capture".into(),
    };
    let world = FrozenWorld::from_resolution(
        host,
        vec![package],
        "catalog".into(),
        "normalized-request-and-choice-witness".into(),
        vec![],
    )
    .unwrap();
    PreparedScenario::new(world.clone(), world, "os:linux".into()).unwrap()
}
fn evaluate(s: &PreparedScenario) -> Preview {
    s.evaluate(
        s.identity(),
        s.baseline.identity(),
        s.candidate.identity(),
        s.candidate.catalog(),
        &BTreeMap::new(),
    )
    .unwrap()
}
#[test]
fn builtin_deterministic_with_conditions_and_exact_carrier() {
    let s = scenario();
    let a = evaluate(&s);
    assert_eq!(a.status, "complete");
    assert!(a.artifact.as_ref().unwrap().contains("exact bytes"));
    assert!(a.carrier.is_some());
    assert_eq!(a.placements[0].loading, "static");
    assert_eq!(a.placements[1].loading, "dynamic");
    assert_eq!(a.placements[1].when.as_deref(), Some("os:linux"));
    assert_eq!(
        serde_json::to_value(&a).unwrap(),
        serde_json::to_value(evaluate(&s)).unwrap()
    );
    assert!(matches!(
        a.measurement.unwrap().totals.runtime_reads,
        measurement::Count::Unavailable { .. }
    ));
}
#[test]
fn rejects_override_and_stale_capture() {
    let s = scenario();
    assert!(
        s.evaluate(
            s.identity(),
            s.baseline.identity(),
            s.candidate.identity(),
            "changed catalog",
            &BTreeMap::new()
        )
        .unwrap_err()
        .contains("stale")
    );
    assert!(
        s.evaluate(
            s.identity(),
            s.baseline.identity(),
            "changed source",
            s.candidate.catalog(),
            &BTreeMap::new()
        )
        .unwrap_err()
        .contains("stale")
    );
    let overrides = BTreeMap::from([("org.example/app".into(), "dynamic".into())]);
    assert!(
        s.evaluate(
            s.identity(),
            s.baseline.identity(),
            s.candidate.identity(),
            s.candidate.catalog(),
            &overrides
        )
        .unwrap_err()
        .contains("incompatible loading")
    );
}
#[test]
fn missing_source_and_owner_stage_are_explicit() {
    let mut s = scenario();
    s.candidate.packages[0].files.remove("boot.md");
    let p = compile_world(&s.candidate, &BTreeMap::new()).unwrap();
    assert_eq!(p.status, "incomplete");
    assert!(p.missing[0].contains("source capture:"));
    assert!(p.artifact.is_none());
    s.candidate.host.manifest = Manifest::parse_str("[project]\nname='fixture'\nversion='1.0.0'\n[[extension]]\nid='native'\npoint='compile:emitted'\nhandler={kind='native',crate_dir='never-built'}\n").unwrap();
    let p = compile_world(&s.candidate, &BTreeMap::new()).unwrap();
    assert_eq!(p.missing, vec!["native compiler preparation: host"]);
    assert!(p.measurement.is_none());
}

#[test]
fn dynamic_only_world_has_no_invented_static_artifact() {
    let mut s = scenario();
    s.candidate.host.manifest.requires.links.clear();
    let preview = compile_world(&s.candidate, &BTreeMap::new()).unwrap();
    assert_eq!(preview.status, "complete");
    assert!(preview.placements.iter().all(|p| p.loading == "dynamic"));
    assert!(preview.artifact.is_none());
    assert!(preview.carrier.is_none());
    assert!(preview.measurement.is_none());
}
