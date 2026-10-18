use crate::derivatives::{capture_retained, prepare_binding};
use crate::source_query::{SourceQueryService, derivatives::DerivativeCatalog};
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/next/derivatives/service_fixture.rs"
));

#[test]
fn all_five_families_retain_exact_passive_source_navigation() {
    for family in [
        "commonmark",
        "xml-data",
        "vibe-spec",
        "opaque-file",
        "nested-tree",
    ] {
        let f = Fixture::new(family);
        let current = capture_retained(&f.base, f.binding.base()).unwrap();
        let bound = prepare_binding(&f.text, Some(&current)).unwrap().value;
        let record = bound.record().clone();
        let catalog = DerivativeCatalog::new(vec![record.clone()]).unwrap();
        let shown = catalog.inspect(&record.result_identity).unwrap();
        assert_eq!(shown, bound.show().value, "{family}");
        for expected in &f.expected {
            let row = shown
                .local
                .iter()
                .find(|file| file.path == expected.path)
                .unwrap();
            assert_eq!(row.raw, expected.bytes, "{family}");
            assert_eq!(row.vibe_projection.is_some(), family == "vibe-spec");
        }
        fs::remove_dir_all(f.base.root_path()).unwrap();
        fs::remove_dir_all(f.local.root_path()).unwrap();
        assert_eq!(catalog.inspect(&record.result_identity).unwrap(), shown);
        assert!(catalog.inspect("unknown").is_err());
        assert!(!f._home.path().join("DO_NOT_EXECUTE").exists());
    }
}

fn service(
    f: &Fixture,
    record: vibe_wire::generated::derivative_result::DerivativeResult,
) -> SourceQueryService {
    let manifest = format!(
        "[package]\ngroup='org.example'\nname='local'\nkind='flow'\nversion='1.0.0'\n[[package.sources]]\n{}",
        toml::to_string(&f.binding.local().descriptor).unwrap()
    );
    fs::write(f.local.root_path().join("vibe.toml"), manifest).unwrap();
    SourceQueryService::prepare_host(f.local.root_path())
        .unwrap()
        .with_derivative_catalog(DerivativeCatalog::new(vec![record]).unwrap())
}
fn request() -> vibe_wire::generated::source_query::e1::request::Request {
    serde_json::from_value(serde_json::json!({"schema":1,"address":"spec://org.example/local/common/POLICY#RETRY","mode":"read","format":"md","children":true,"chunk_bytes":16})).unwrap()
}
#[test]
fn selected_query_binds_retained_result_and_refuses_stale_bytes_and_cursor() {
    let f = Fixture::new("vibe-spec");
    let current = capture_retained(&f.base, f.binding.base()).unwrap();
    let bound = prepare_binding(&f.text, Some(&current)).unwrap().value;
    let record = bound.record().clone();
    let service = service(&f, record.clone());
    let mut q = request();
    let first = service.query(&q).unwrap();
    assert_eq!(
        serde_json::from_str::<vibe_wire::generated::derivative_result::DerivativeResult>(
            first.derivative_result_json.as_deref().unwrap()
        )
        .unwrap(),
        record
    );
    assert_eq!(
        first.observation.strength,
        vibe_wire::generated::source_query::e1::result::ObservationStrength::Observed
    );
    let mut malformed = serde_json::to_value(&first).unwrap();
    malformed["derivative_result_json"] = "{}".into();
    assert!(vibe_wire::behaviour::source_query::read_result(&malformed).is_err());
    q.cursor = first.chunk.next_cursor.clone();
    assert!(q.cursor.is_some());
    let second = service.query(&q).unwrap();
    assert_eq!(second.chunk.offset, first.chunk.end);
    // Change the independently bound base but retain equal local bytes. An old
    // source cursor cannot cross result identity even with identical output.
    let mut base = f.binding.base().clone();
    base.files[0].bytes.extend_from_slice(b"\n");
    let binding = DerivationBinding::new(
        base.clone(),
        f.binding.local().clone(),
        base.observation_id().unwrap(),
        DerivationRecipe::ReplaceSelectedInventory,
    )
    .unwrap();
    fs::write(
        f.base.root_path().join(&base.files[0].path),
        &base.files[0].bytes,
    )
    .unwrap();
    let current = capture_retained(&f.base, &base).unwrap();
    let updated = prepare_binding(&toml::to_string(&binding).unwrap(), Some(&current))
        .unwrap()
        .value;
    let new_service = SourceQueryService::prepare_host(f.local.root_path())
        .unwrap()
        .with_derivative_catalog(DerivativeCatalog::new(vec![updated.record().clone()]).unwrap());
    assert!(matches!(
        new_service.query(&q).unwrap_err().code,
        vibe_wire::generated::source_query::e1::error::ErrorCode::StaleCursor
    ));
    q.cursor = None;
    assert!(
        new_service
            .query(&q)
            .unwrap()
            .derivative_result_json
            .unwrap()
            .contains(&updated.record().result_identity)
    );
    fs::write(
        f.local.root_path().join(&f.binding.local().files[0].path),
        b"changed",
    )
    .unwrap();
    assert!(service.query(&q).is_err());
    assert_eq!(
        service
            .inspect_retained_derivative(&record.result_identity)
            .unwrap(),
        bound.show().value
    );
}

#[test]
fn altered_record_and_competing_local_bindings_refuse() {
    let f = Fixture::new("opaque-file");
    let current = capture_retained(&f.base, f.binding.base()).unwrap();
    let bound = prepare_binding(&f.text, Some(&current)).unwrap().value;
    let mut changed = bound.record().clone();
    changed.inventory_identity = changed.base_identity.clone();
    assert!(DerivativeCatalog::new(vec![changed]).is_err());
    assert!(DerivativeCatalog::new(vec![bound.record().clone(), bound.record().clone()]).is_err());
}

#[test]
fn actual_builtin_read_and_closure_keep_exact_derivative_and_repeated_occurrences() {
    use super::hypothetical::*;
    use std::collections::BTreeMap;
    let f = Fixture::new("vibe-spec");
    let current = capture_retained(&f.base, f.binding.base()).unwrap();
    let bound = prepare_binding(&f.text, Some(&current)).unwrap().value;
    let catalog = DerivativeCatalog::new(vec![bound.record().clone()]).unwrap();
    let host = HostCapture { id:"captured-host".into(), manifest:vibe_core::manifest::Manifest::parse_str("[project]\nname='fixture'\nversion='1.0.0'\n[requires.packages]\n'org.example/app'={version='=1.0.0',link='static'}\n'org.example/local'={version='=1.0.0',link='dynamic'}\n").unwrap(), files:BTreeMap::new() };
    let local = SelectedPackage { manifest:vibe_core::manifest::Manifest::parse_str(&format!("[package]\ngroup='org.example'\nname='local'\nkind='flow'\nversion='1.0.0'\nformat='normal'\n[[package.sources]]\n{}", toml::to_string(&f.binding.local().descriptor).unwrap())).unwrap(), requires:vec![], files:f.binding.local().files.iter().map(|f|(f.path.clone(),f.bytes.clone())).collect(), source_identity:"selected-local".into() };
    let app = SelectedPackage { manifest:vibe_core::manifest::Manifest::parse_str("[package]\ngroup='org.example'\nname='app'\nkind='flow'\nversion='1.0.0'\nformat='normal'\n[boot_snippet]\nsource='vibevm/vibespecs/boot/entry.md'\n[[boot_snippet.fragment]]\nsource='vibevm/vibespecs/boot/repeated.md'\n").unwrap(), requires:vec![], files:BTreeMap::from([("vibevm/vibespecs/boot/entry.md".into(), b"# Root {#root}\n\n#use spec://org.example/local/common/POLICY#RETRY\n".to_vec()), ("vibevm/vibespecs/boot/repeated.md".into(),b"# Repeated {#repeated}\n\n#use spec://org.example/local/common/POLICY#RETRY\n".to_vec())]), source_identity:"selected-app".into() };
    let world = FrozenWorld::from_resolution(
        host,
        vec![local.clone(), app],
        "selected-catalog".into(),
        "actual resolution input".into(),
        vec![],
    )
    .unwrap()
    .with_derivative_catalog(catalog.clone())
    .unwrap();
    let scenario = PreparedScenario::new(world.clone(), world.clone(), "os:linux".into()).unwrap();
    let preview = scenario
        .evaluate(
            scenario.identity(),
            world.identity(),
            world.identity(),
            world.catalog(),
            &BTreeMap::new(),
        )
        .unwrap();
    assert_eq!(preview.status, "complete", "{:?}", preview.missing);
    let (graph, reads, artifact, _) = preview.produced.unwrap().into_parts();
    let read = reads.iter().find(|r| r.record == *bound.record()).unwrap();
    assert_eq!(read.path, f.binding.local().files[0].path);
    assert_eq!(
        std::str::from_utf8(&artifact)
            .unwrap()
            .matches("Retry at most once in the local profile.")
            .count(),
        2
    );
    let source = graph
        .sources
        .iter()
        .position(|s| s.identity == vibe_spec::SourceIdentity::Spec(read.address.clone()))
        .unwrap_or_else(|| panic!("read={read:?}; graph={graph:?}"));
    let occurrences: Vec<_> = graph
        .contributions
        .iter()
        .enumerate()
        .flat_map(|(c, row)| {
            row.occurrences
                .iter()
                .enumerate()
                .filter(move |(_, o)| o.source == source)
                .map(move |(i, _)| (c, i))
        })
        .collect();
    assert_eq!(occurrences.len(), 2);
    assert_ne!(occurrences[0], occurrences[1]);
    let mut base = f.binding.base().clone();
    base.files[0].bytes.extend_from_slice(b"\n");
    let rebound = DerivationBinding::new(
        base.clone(),
        f.binding.local().clone(),
        base.observation_id().unwrap(),
        DerivationRecipe::ReplaceSelectedInventory,
    )
    .unwrap();
    fs::write(
        f.base.root_path().join(&base.files[0].path),
        &base.files[0].bytes,
    )
    .unwrap();
    let current = capture_retained(&f.base, &base).unwrap();
    let updated = prepare_binding(&toml::to_string(&rebound).unwrap(), Some(&current))
        .unwrap()
        .value;
    let new_world = world
        .clone()
        .with_derivative_catalog(DerivativeCatalog::new(vec![updated.record().clone()]).unwrap())
        .unwrap();
    assert_ne!(new_world.identity(), world.identity());
    assert!(
        scenario
            .evaluate(
                scenario.identity(),
                world.identity(),
                new_world.identity(),
                world.catalog(),
                &BTreeMap::new()
            )
            .is_err()
    );
    assert_eq!(read.record, *bound.record());
    // A current capture with changed local bytes cannot admit the old record.
    let mut changed = local;
    changed.files.values_mut().next().unwrap().push(b'!');
    let bad = FrozenWorld::from_resolution(
        HostCapture {
            id: "other".into(),
            manifest: vibe_core::manifest::Manifest::parse_str(
                "[project]\nname='empty'\nversion='1.0.0'\n",
            )
            .unwrap(),
            files: BTreeMap::new(),
        },
        vec![changed],
        "catalog".into(),
        "selection".into(),
        vec![],
    )
    .unwrap();
    assert!(bad.with_derivative_catalog(catalog).is_err());
}
