//! T6b execution-parity tests for carried transform plans: the honest
//! replacement for T4's inert-carriage oracles (ABI §6.3). Parity now holds
//! BECAUSE the injected identity behaviors actually ran — not because
//! carriage is inert — and the production empty-catalog refusal is explicit.

use std::cell::RefCell;
use std::collections::BTreeMap;

use specmark::verifies;

use super::{CountingSource, Fixture, fixture};
use crate::SectionSource;
use crate::SpecAddress;
use crate::compiler::backend::BackendRegistry;
use crate::compiler::builtin::{
    compile_artifact, compile_artifact_with_registries, parse_invocations, reset_parse_invocations,
};
use crate::compiler::emit::{emit_invocations, emitted_bytes_digest, reset_emit_invocations};
use crate::compiler::ir::emitted_output_fingerprint;
use crate::compiler::ir::{ArtifactPlan, EmittedArtifact};
use crate::compiler::transform::plan::TransformStage;
use crate::compiler::transform::registry_test_support::{identity_plan, identity_registry};

/// The full four-stage identity plan attached to one fixture's plan: every
/// position executes, bytes and provenance still match the plain compile.
fn attach_four_stage(plan: ArtifactPlan) -> ArtifactPlan {
    plan.with_transforms(identity_plan(&[
        ("org.demo/tools#src", TransformStage::Source),
        ("org.demo/tools#doc", TransformStage::Document),
        ("org.demo/tools#lane", TransformStage::Lane),
        ("org.demo/tools#emit", TransformStage::Emitted),
    ]))
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn identity_execution_emits_bytes_provenance_and_fingerprint_identical_to_no_transform() {
    // Two fresh fixtures keep the counting source's interior mutability out
    // of the comparison (the T4 oracle's discipline, kept).
    let plain: Fixture = fixture();
    let carried: Fixture = fixture();
    reset_parse_invocations();
    reset_emit_invocations();
    crate::compiler::transform::registry_test_support::reset_identity_invocations();

    let plain_emitted = compile_artifact(plain.plan, &plain.source).unwrap();
    let parses_plain = parse_invocations();
    let carried_emitted = compile_artifact_with_registries(
        attach_four_stage(carried.plan),
        &carried.source,
        &BackendRegistry::builtins(),
        &identity_registry(),
    )
    .unwrap();

    // The identity behaviors really ran at every position — the shared
    // vehicles themselves counted 5/5/1/1 — and the same documents were
    // parsed in both worlds (source/document wrappers live inside the parse
    // closure).
    assert_eq!(
        crate::compiler::transform::registry_test_support::identity_invocations(),
        (5, 5, 1, 1),
        "parity is CAUSED by execution at all four positions"
    );
    assert_eq!(parse_invocations(), parses_plain * 2);
    // WHOLE-value parity, against the plain artifact carrying the ONE honest
    // difference an ACTIVE plan makes (R4 architecture §7.1): its header line
    // and the byte digest that follows from it. Comparing whole values —
    // rather than picking fields — is the point: a rebuilt provenance, a
    // moved rename list or a changed contribution witness would still fail
    // here, and only the header is allowed to move.
    assert_eq!(carried_emitted, expected_with_header(&plain_emitted));
    // The equal-bytes emitted wrapper returned the ORIGINAL artifact: the
    // live fingerprint law holds on the returned value.
    assert_eq!(
        carried_emitted.output_fingerprint(),
        emitted_output_fingerprint(carried_emitted.bytes())
    );
    assert_eq!(emit_invocations("static-xml"), 2);
    // The plain compile — an owner that activates nothing — records nothing.
    assert!(
        !std::str::from_utf8(plain_emitted.bytes())
            .unwrap()
            .contains("vibe:transforms"),
        "an empty plan emits no header at all"
    );
}

/// The plain artifact as the four-stage ACTIVE plan would have written it:
/// the header line inserted after the three provenance lines, and the byte
/// digest recomputed over the result.
///
/// Written out longhand rather than asserted piecewise, so the comparison
/// above stays a whole-value one.
fn expected_with_header(plain: &crate::compiler::ir::EmittedArtifact) -> EmittedArtifact {
    let text = std::str::from_utf8(plain.bytes()).expect("a UTF-8 tape");
    let mut lines: Vec<&str> = text.split('\n').collect();
    lines.insert(
        3,
        "<!-- vibe:transforms org.demo/tools#src org.demo/tools#doc \
         org.demo/tools#lane org.demo/tools#emit -->",
    );
    let bytes = lines.join("\n").into_bytes();
    let mut expected = plain.clone();
    expected.provenance.bytes_digest = emitted_bytes_digest(&bytes);
    expected.bytes = bytes;
    expected
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn a_real_failure_keeps_its_typed_error_with_identity_transforms_running() {
    // The same dangling-#use world fails identically with and without the
    // resolved identity plan: same typed variant, same message, same
    // attribution — the running transforms did not absorb or reattribute the
    // engine's fault.
    let plain: Fixture = fixture();
    let carried: Fixture = fixture();
    let attached = carried.plan.with_transforms(identity_plan(&[(
        "org.demo/tools#doc",
        TransformStage::Document,
    )]));

    let mut dangling = DanglingSource::default();
    dangling.documents.insert(
        spec("spec://org.demo/alpha/boot/entry#root").without_pin(),
        "# Alpha {#root}\n#use spec://org.demo/missing/boot/base#root\nALPHA\n".to_string(),
    );

    let plain_error = compile_artifact(plain.plan, &dangling).unwrap_err();
    let carried_error = compile_artifact_with_registries(
        attached,
        &dangling,
        &BackendRegistry::builtins(),
        &identity_registry(),
    )
    .unwrap_err();
    assert_eq!(plain_error.to_string(), carried_error.to_string());
    assert_eq!(
        std::mem::discriminant(&plain_error),
        std::mem::discriminant(&carried_error),
        "the typed variant must not move under executed transforms"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn a_nonempty_plan_under_the_empty_production_catalog_refuses_before_any_parse() {
    // T6b's explicit empty-catalog refusal: the same identity plan that runs
    // under the injected catalog refuses under production, and no source is
    // even loaded (not merely unparsed).
    let world: Fixture = fixture();
    let attached = world.plan.with_transforms(identity_plan(&[(
        "org.demo/tools#doc",
        TransformStage::Document,
    )]));
    reset_parse_invocations();
    let loads_before = total_loads(&world.source);

    let error = compile_artifact(attached, &world.source).unwrap_err();
    assert!(matches!(
        error,
        crate::compiler::builtin::ArtifactCompileError::Transform(_)
    ));
    assert_eq!(
        parse_invocations(),
        0,
        "resolution precedes the first parse"
    );
    assert_eq!(
        total_loads(&world.source),
        loads_before,
        "resolution precedes the first source read"
    );
}

fn spec(raw: &str) -> SpecAddress {
    SpecAddress::parse(raw).unwrap()
}

fn total_loads(source: &CountingSource) -> usize {
    source.loads.borrow().values().sum()
}

#[derive(Default)]
struct DanglingSource {
    documents: BTreeMap<String, String>,
    loads: RefCell<BTreeMap<String, usize>>,
}

impl SectionSource for DanglingSource {
    fn section_text(&self, addr: &SpecAddress) -> Result<String, String> {
        let key = addr.without_pin();
        *self.loads.borrow_mut().entry(key.clone()).or_insert(0) += 1;
        self.documents
            .get(&key)
            .cloned()
            .ok_or_else(|| format!("missing {key}"))
    }

    fn expand_pattern(&self, addr: &SpecAddress) -> Result<Vec<SpecAddress>, String> {
        Ok(vec![addr.clone()])
    }
}
