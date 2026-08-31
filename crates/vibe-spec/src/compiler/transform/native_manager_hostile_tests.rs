use crate::compiler::builtin::{compile_artifact, compile_artifact_native, without_verify_each};

use super::lowering_worlds::Declared;
use super::native_manager_test_support::{FakeInvoker, ReplyMode, fixture_world, plan};

#[test]
fn strict_manager_reply_matrix_refuses_every_hostile_root_and_payload_shape() {
    for (label, point, mode) in [
        (
            "unknown root member",
            "compile:source",
            ReplyMode::UnknownRoot,
        ),
        ("unknown status", "compile:source", ReplyMode::UnknownStatus),
        (
            "illegal skip payload",
            "compile:source",
            ReplyMode::IllegalSkipPayload,
        ),
        ("wrong envelope", "compile:source", ReplyMode::WrongEnvelope),
        ("wrong ir schema", "compile:source", ReplyMode::WrongSchema),
        (
            "valid utf8 malformed json",
            "compile:source",
            ReplyMode::MalformedJson,
        ),
        ("invalid utf8", "compile:source", ReplyMode::InvalidUtf8),
        (
            "duplicate reply root",
            "compile:source",
            ReplyMode::DuplicateRoot,
        ),
        (
            "duplicate returned ir",
            "compile:source",
            ReplyMode::DuplicateIr,
        ),
        (
            "duplicate returned ir map",
            "compile:document",
            ReplyMode::DuplicateMap,
        ),
    ] {
        let world = fixture_world();
        let plan = plan(vec![Declared::native("hostile", point)]);
        let invoker = FakeInvoker::new(mode);
        let error = compile_artifact_native(plan, &world.source, &invoker)
            .unwrap_err()
            .to_string();
        assert_eq!(invoker.records().len(), 1, "{label}");
        assert!(error.len() < 1024, "{label}: {error}");
    }
}

#[test]
fn a_valid_wrong_carrier_refuses_before_domain_conversion() {
    let world = fixture_world();
    let source_plan = plan(vec![Declared::native("source", "compile:source")]);
    let recorder = FakeInvoker::new(ReplyMode::Ok);
    compile_artifact_native(source_plan, &world.source, &recorder).unwrap();
    let source_payload = recorder.records()[0].payload.clone();

    let document_plan = plan(vec![Declared::native("document", "compile:document")]);
    let invoker = FakeInvoker::new(ReplyMode::ReturnPayload(source_payload));
    let error = compile_artifact_native(document_plan, &world.source, &invoker)
        .unwrap_err()
        .to_string();
    assert!(error.contains("source-document"), "{error}");
    assert!(error.contains("document-document"), "{error}");
}

#[test]
fn source_document_and_lane_identity_or_provenance_forgeries_refuse_locally() {
    for (point, mode) in [
        ("compile:source", ReplyMode::ForgedSourceIdentity),
        ("compile:document", ReplyMode::ForgedDocumentIdentity),
        ("compile:lane", ReplyMode::ForgedLaneProvenance),
    ] {
        let world = fixture_world();
        let plan = plan(vec![Declared::native("forge", point)]);
        let invoker = FakeInvoker::new(mode);
        let error = without_verify_each(|| compile_artifact_native(plan, &world.source, &invoker))
            .unwrap_err()
            .to_string();
        assert_eq!(invoker.records().len(), 1);
        assert!(
            error.contains("transition") || error.contains("provenance"),
            "{point}: {error}"
        );
    }
}

#[test]
fn intrinsically_invalid_returned_ir_refuses_before_use() {
    let world = fixture_world();
    let plan = plan(vec![Declared::native("invalid", "compile:source")]);
    let invoker = FakeInvoker::new(ReplyMode::InvalidSource);
    let error = without_verify_each(|| compile_artifact_native(plan, &world.source, &invoker))
        .unwrap_err()
        .to_string();
    assert!(error.contains("returned canonical IR"), "{error}");
}

#[test]
fn no_invoker_and_buildable_source_unavailable_are_hard_attributed_errors() {
    let world = fixture_world();
    let plan = plan(vec![Declared::native("source", "compile:source")]);
    let no_invoker = compile_artifact(plan.clone(), &world.source).unwrap_err();
    assert!(no_invoker.to_string().contains("has no native invoker"));

    let invoker = FakeInvoker::new(ReplyMode::BuildableSourceUnavailable);
    let error = compile_artifact_native(plan, &world.source, &invoker)
        .unwrap_err()
        .to_string();
    assert!(error.contains("BuildableSourceUnavailable"), "{error}");
    assert!(!error.contains("pending"), "{error}");
}
