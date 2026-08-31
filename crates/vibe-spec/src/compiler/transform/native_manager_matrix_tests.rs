use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use vibe_core::lifecycle::CompilePoint;
use vibe_core::manifest::ExtensionConfig;

use crate::compiler::builtin::{
    compile_artifact_native, compile_artifact_native_observed, compile_artifact_native_traced,
    without_verify_each,
};
use crate::compiler::emit::emitted_bytes_digest;

use super::lowering_worlds::{Declared, collected_host};
use super::native_identity::compiler_native_implementation_digest;
use super::native_manager_test_support::{
    FakeInvoker, ReplyMode, Sink, compile_mixed, fixture_world, plan, plan_with_registry,
    point_name, registry_with_ordered_builtin,
};

fn all_stages() -> crate::compiler::ir::ArtifactPlan {
    plan(vec![
        Declared::native("source", "compile:source"),
        Declared::native("document", "compile:document"),
        Declared::native("lane", "compile:lane"),
        Declared::native("emitted", "compile:emitted"),
    ])
}

#[test]
fn five_document_world_delivers_exact_native_call_identity_and_payload_5_5_1_1() {
    let world = fixture_world();
    let plan = all_stages();
    let invoker = FakeInvoker::new(ReplyMode::Ok);
    compile_artifact_native(plan, &world.source, &invoker).unwrap();
    let records = invoker.records();
    assert_eq!(records.len(), 12);
    for (point, calls, order, carrier) in [
        (CompilePoint::Source, 5, 0, "source-document"),
        (CompilePoint::Document, 5, 1, "document-document"),
        (CompilePoint::Lane, 1, 2, "lane-artifact"),
        (CompilePoint::Emitted, 1, 3, "emitted-artifact"),
    ] {
        let at: Vec<_> = records
            .iter()
            .filter(|record| record.point == point)
            .collect();
        assert_eq!(at.len(), calls);
        assert!(at.iter().all(|record| record.order == order));
        assert!(at.iter().all(|record| record.carrier == carrier));
        assert!(at.iter().all(|record| record.ir_schema == 1));
        assert!(at.iter().all(|record| record.config.is_empty()));
    }

    let registry = collected_host(vec![Declared::native("source", "compile:source")]);
    assert_eq!(
        records[0].implementation,
        compiler_native_implementation_digest(registry.enabled_compile_rows()[0]).unwrap()
    );
}

#[test]
fn mixed_builtin_native_execution_preserves_authored_dense_order_per_document() {
    let world = fixture_world();
    let ordered = Arc::new(Mutex::new(Vec::new()));
    let registry = registry_with_ordered_builtin(ordered.clone(), false);
    let plan = plan_with_registry(
        vec![
            Declared::native("native-first", "compile:source"),
            Declared::builtin("builtin-middle", "compile:source", "test-native-adjacent"),
            Declared::native("native-last", "compile:source"),
        ],
        &registry,
    );
    let invoker = FakeInvoker::with_ordered_log(ReplyMode::Ok, ordered.clone());
    compile_mixed(plan, &world, &registry, &invoker).unwrap();
    assert_eq!(
        *ordered.lock().unwrap(),
        vec![["native:0", "builtin", "native:2"]; 5]
            .concat()
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>()
    );
}

#[test]
fn execution_config_fuses_two_absences_but_preserves_every_nested_value() {
    let table = concat!(
        "message = 'hello'\n",
        "count = -7\n",
        "ratio = 1.5\n",
        "enabled = true\n",
        "when = 1979-05-27T07:32:00Z\n",
        "list = [1, 3, 2]\n",
        "[nested]\ninner = 'deep'\n",
    )
    .parse::<toml::Table>()
    .unwrap();
    let world = fixture_world();
    let plan = plan(vec![
        Declared::native("absent", "compile:source"),
        Declared::native("empty", "compile:source").configured(ExtensionConfig::default()),
        Declared::native("values", "compile:source").configured(ExtensionConfig::from_table(table)),
    ]);
    let invoker = FakeInvoker::new(ReplyMode::Ok);
    compile_artifact_native(plan, &world.source, &invoker).unwrap();
    let records = invoker.records();
    assert_eq!(records.len(), 15);
    for document in records.chunks_exact(3) {
        assert_eq!(document[0].config, BTreeMap::new());
        assert_eq!(document[1].config, BTreeMap::new());
        assert_eq!(
            document[2].config,
            BTreeMap::from([
                ("count".to_string(), Some(serde_json::json!(-7))),
                ("enabled".to_string(), Some(serde_json::json!(true))),
                ("list".to_string(), Some(serde_json::json!([1, 3, 2]))),
                ("message".to_string(), Some(serde_json::json!("hello"))),
                (
                    "nested".to_string(),
                    Some(serde_json::json!({"inner": "deep"})),
                ),
                ("ratio".to_string(), Some(serde_json::json!(1.5))),
                (
                    "when".to_string(),
                    Some(serde_json::json!("1979-05-27T07:32:00Z")),
                ),
            ])
        );
    }
}

#[test]
fn ok_skip_and_fail_are_distinct_at_every_stage_and_fail_stops_later_rows() {
    for point in [
        CompilePoint::Source,
        CompilePoint::Document,
        CompilePoint::Lane,
        CompilePoint::Emitted,
    ] {
        let world = fixture_world();
        let single = plan(vec![Declared::native("one", point_name(point))]);
        let ok = FakeInvoker::new(ReplyMode::Ok);
        let expected = compile_artifact_native(single.clone(), &world.source, &ok).unwrap();
        let skip = FakeInvoker::new(ReplyMode::SkipOrder(0));
        let skipped = compile_artifact_native(single, &world.source, &skip).unwrap();
        assert_eq!(skipped, expected, "skip is exact at {point:?}");

        let two = plan(vec![
            Declared::native("first", point_name(point)),
            Declared::native("later", point_name(point)),
        ]);
        let fail = FakeInvoker::new(ReplyMode::FailOrder(0));
        let error = compile_artifact_native(two, &world.source, &fail).unwrap_err();
        assert_eq!(fail.records().len(), 1, "fail stops later {point:?} calls");
        assert!(error.to_string().contains("entry 0"));
        assert!(error.to_string().len() < 1024);
    }
}

#[test]
fn selector_miss_is_zero_call_and_a_lawful_source_body_mutation_passes() {
    let world = fixture_world();
    let missed = plan(vec![
        Declared::native("miss", "compile:source").scoped(&["nowhere/**"]),
    ]);
    let invoker = FakeInvoker::new(ReplyMode::Ok);
    compile_artifact_native(missed, &world.source, &invoker).unwrap();
    assert!(invoker.records().is_empty());

    let mutated = plan(vec![Declared::native("body", "compile:source")]);
    let invoker = FakeInvoker::new(ReplyMode::LawfulSourceMutation);
    let emitted = compile_artifact_native(mutated, &world.source, &invoker).unwrap();
    assert!(String::from_utf8_lossy(emitted.bytes()).contains("Lawful native body mutation"));
}

#[test]
fn native_local_verification_does_not_change_adjacent_builtin_behavior() {
    let world = fixture_world();
    let log = Arc::new(Mutex::new(Vec::new()));
    let registry = registry_with_ordered_builtin(log.clone(), true);
    let mixed = plan_with_registry(
        vec![
            Declared::native("native-before", "compile:source"),
            Declared::builtin("builtin-forges", "compile:source", "test-native-adjacent"),
            Declared::native("native-after", "compile:source"),
        ],
        &registry,
    );
    let invoker = FakeInvoker::new(ReplyMode::Ok);
    without_verify_each(|| compile_mixed(mixed, &world, &registry, &invoker)).unwrap();
    assert_eq!(log.lock().unwrap().len(), 5);
    assert_eq!(invoker.records().len(), 10);
}

#[test]
fn emitted_equal_discards_temporary_provenance_and_changed_bytes_reconstruct_once() {
    let world = fixture_world();
    let plan = plan(vec![Declared::native("emitted", "compile:emitted")]);
    let equal = FakeInvoker::new(ReplyMode::TemporaryEmittedProvenance);
    let emitted = compile_artifact_native(plan.clone(), &world.source, &equal).unwrap();
    assert!(emitted.provenance().emitted_transforms.is_empty());

    let changed = FakeInvoker::new(ReplyMode::ChangedEmittedBytes);
    let emitted = compile_artifact_native(plan, &world.source, &changed).unwrap();
    assert!(emitted.bytes().ends_with(b"!"));
    assert_eq!(
        emitted.provenance().bytes_digest,
        emitted_bytes_digest(emitted.bytes())
    );
    assert_eq!(
        emitted
            .provenance()
            .emitted_transforms
            .iter()
            .map(|pass| pass.as_str())
            .collect::<Vec<_>>(),
        vec!["transform:emitted:__host__/demo#emitted"]
    );
}

#[test]
fn one_stack_borrowed_invoker_runs_plain_traced_and_observed_entries() {
    let world = fixture_world();
    let plan = plan(vec![Declared::native("emitted", "compile:emitted")]);
    let invoker = FakeInvoker::new(ReplyMode::Ok);
    let sink = Sink;
    compile_artifact_native(plan.clone(), &world.source, &invoker).unwrap();
    compile_artifact_native_traced(plan.clone(), &world.source, &invoker, &sink).unwrap();
    compile_artifact_native_observed(plan, &world.source, &invoker, Arc::new(Sink)).unwrap();
    assert_eq!(invoker.records().len(), 3);
}
