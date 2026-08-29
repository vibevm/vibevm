use std::sync::Arc;
use std::{cell::Cell, rc::Rc};

use super::*;
use crate::compiler::backend::{BackendRegistry, EmitBackend};
use crate::compiler::builtin::{compile_artifact_lane, compile_artifact_with_registry};
use crate::compiler::ir::{
    ArtifactContext, ArtifactFrame, ArtifactId, ArtifactInput, ArtifactPlan, ArtifactTarget,
    LaneChunk, LaneContribution, LaneInputDigest, LaneIr, LaneNode, LinkFenceSnapshot,
    LinkMarkerKey, StaticCompileMode,
};
use crate::compiler::pass::Pass;
use crate::{SectionSource, SpecAddress};

pub(super) struct Source(pub(super) &'static str);

impl SectionSource for Source {
    fn section_text(&self, _addr: &SpecAddress) -> Result<String, String> {
        Ok(self.0.to_string())
    }
}

struct CountingSource {
    calls: Rc<Cell<usize>>,
}

impl SectionSource for CountingSource {
    fn section_text(&self, _addr: &SpecAddress) -> Result<String, String> {
        self.calls.set(self.calls.get() + 1);
        Ok("# Entry {#root}\n".to_string())
    }
}

struct OpaqueBackend {
    id: BackendId,
    pass: PassName,
}

impl OpaqueBackend {
    fn new() -> Self {
        Self {
            id: BackendId::new("opaque").unwrap(),
            pass: PassName::new("emit:opaque").unwrap(),
        }
    }
}

impl EmitBackend for OpaqueBackend {
    fn id(&self) -> &BackendId {
        &self.id
    }

    fn pass_name(&self) -> &PassName {
        &self.pass
    }

    fn emit(
        &self,
        _lane: &LaneIr,
        _witness: &crate::compiler::ir::PreEmissionWitness,
    ) -> Result<Vec<u8>, BackendError> {
        Ok(vec![0x00, 0xff, b'\n'])
    }
}

fn seed() -> SpecAddress {
    SpecAddress::parse("spec://org.demo/pkg/boot/entry#root").unwrap()
}

#[test]
fn opaque_registered_backend_runs_the_complete_schedule_and_preserves_non_utf8() {
    let mut registry = BackendRegistry::default();
    registry.register(Arc::new(OpaqueBackend::new())).unwrap();
    reset_emit_invocations();
    let emitted = compile_artifact_with_registry(
        ArtifactPlan::custom_for_test(
            "opaque",
            vec![ArtifactInput::simple("org.demo/a", "boot/a.md", "# A\n").unwrap()],
        )
        .unwrap(),
        &Source("# Entry {#root}\n"),
        &registry,
    )
    .unwrap();
    assert_eq!(emitted.bytes(), [0x00, 0xff, b'\n']);
    assert_eq!(emit_invocations("opaque"), 1);
    assert_eq!(emitted.provenance().backend_id(), "opaque");
    assert_eq!(emitted.provenance().producer(), "emit:opaque");
}

#[test]
fn missing_selected_backend_fails_before_source_discovery() {
    let calls = Rc::new(Cell::new(0));
    let error = compile_artifact_with_registry(
        ArtifactPlan::compatibility(seed(), StaticCompileMode::Plain),
        &CountingSource {
            calls: calls.clone(),
        },
        &BackendRegistry::default(),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        crate::compiler::builtin::ArtifactCompileError::Registry { .. }
    ));
    assert_eq!(calls.get(), 0);
}

pub(super) fn full_lane() -> LaneIr {
    let normal = SpecAddress::parse("spec://org.demo/n/boot/n#root").unwrap();
    let plan = ArtifactPlan::static_lane(
        ArtifactTarget::StaticMarkdown,
        "vibevm/vibespecs/boot/STATIC.md",
        "vibevm/vibedeps",
        vec![
            ArtifactInput::simple("org.demo/a", "boot/a.md", "# A {#root}\n").unwrap(),
            ArtifactInput::normal("org.demo/n", "boot/n.md", normal).unwrap(),
            ArtifactInput::elided("org.demo/e", "boot/e.md").unwrap(),
            ArtifactInput::hoisted(
                "org.demo/h",
                "boot/h.md",
                SpecAddress::parse("spec://org.demo/h/boot/h").unwrap(),
            )
            .unwrap(),
            ArtifactInput::simple("org.demo/b", "boot/b.md", "# B {#root}\n").unwrap(),
        ],
    )
    .unwrap();
    compile_artifact_lane(plan, &Source("# Normal {#root}\n")).unwrap()
}

#[test]
fn backend_transition_and_current_validators_reject_every_owned_witness_mutation() {
    let lane = full_lane();
    let backend = Arc::new(static_md::StaticMarkdownBackend::new());
    let emitted = EmitPass::new(backend.clone(), None)
        .run(lane.clone())
        .unwrap();
    let witness = capture_witness(&lane, backend.id(), None).unwrap();

    let mut context = emitted.clone();
    context.provenance.context = ArtifactContext::new(
        ArtifactId::new("static-xml").unwrap(),
        ArtifactTarget::StaticXml,
        ArtifactFrame::StaticLane {
            generated_path: "vibevm/vibespecs/boot/STATIC.xml".to_string(),
            source_root: "vibevm/vibedeps".to_string(),
        },
        StaticCompileMode::QualifyPerNode,
    )
    .unwrap();
    assert!(
        validate::transition(backend.id(), backend.pass_name(), &witness, &lane, &context).is_err()
    );

    let mut id = emitted.clone();
    id.provenance.backend = BackendId::new("other").unwrap();
    assert!(validate::transition(backend.id(), backend.pass_name(), &witness, &lane, &id).is_err());
    let mut pass = emitted.clone();
    pass.provenance.producer = PassName::new("emit:other").unwrap();
    assert!(
        validate::transition(backend.id(), backend.pass_name(), &witness, &lane, &pass).is_err()
    );
    let mut digest = emitted.clone();
    digest.provenance.source_lane_digest = LaneInputDigest([9; 32]);
    assert!(
        validate::transition(backend.id(), backend.pass_name(), &witness, &lane, &digest).is_err()
    );
    let mut renames = emitted.clone();
    renames.provenance.renames.reverse();
    assert!(
        validate::transition(backend.id(), backend.pass_name(), &witness, &lane, &renames).is_err()
    );
    let mut order = emitted.clone();
    order.provenance.contributions.reverse();
    assert!(
        validate::transition(backend.id(), backend.pass_name(), &witness, &lane, &order).is_err()
    );
    let mut bytes = emitted;
    bytes.bytes.push(0);
    assert!(
        validate::current(
            backend.id(),
            backend.pass_name(),
            &witness,
            bytes.bytes(),
            bytes.provenance(),
        )
        .is_err()
    );
}

#[test]
fn concrete_renderers_and_xml_pivots_execute_exactly_once() {
    let markdown_lane = full_lane();
    let markdown = Arc::new(static_md::StaticMarkdownBackend::new());
    static_md::reset_render_calls();
    let _emitted = EmitPass::new(markdown.clone(), None)
        .run(markdown_lane.clone())
        .unwrap();
    assert_eq!(static_md::render_calls(), 1);
    assert_eq!(
        static_md::render_calls(),
        1,
        "validators re-rendered Markdown"
    );

    let xml_plan = ArtifactPlan::static_lane(
        ArtifactTarget::StaticXml,
        "vibevm/vibespecs/boot/STATIC.xml",
        "vibevm/vibedeps",
        vec![
            ArtifactInput::simple("org.demo/a", "boot/a.md", "# A {#root}\n").unwrap(),
            ArtifactInput::simple("org.demo/b", "boot/b.md", "# B {#root}\n").unwrap(),
        ],
    )
    .unwrap();
    let xml_lane = compile_artifact_lane(xml_plan, &Source("")).unwrap();
    let xml = Arc::new(static_xml::StaticXmlBackend::new());
    static_xml::reset_pivot_calls();
    let _emitted = EmitPass::new(xml.clone(), None)
        .run(xml_lane.clone())
        .unwrap();
    assert_eq!(static_xml::pivot_calls(), 2);
    assert_eq!(
        static_xml::pivot_calls(),
        2,
        "validators repeated the XML pivot"
    );
}

#[test]
fn shipping_backends_refuse_wrong_targets_before_render() {
    let markdown_lane = full_lane();
    static_xml::reset_pivot_calls();
    let error = EmitPass::new(Arc::new(static_xml::StaticXmlBackend::new()), None)
        .run(markdown_lane)
        .unwrap_err();
    assert!(matches!(error, EmitPassError::TargetMismatch { .. }));
    assert_eq!(static_xml::pivot_calls(), 0);

    let xml_plan = ArtifactPlan::static_lane(
        ArtifactTarget::StaticXml,
        "vibevm/vibespecs/boot/STATIC.xml",
        "vibevm/vibedeps",
        vec![ArtifactInput::simple("org.demo/a", "boot/a.md", "# A\n").unwrap()],
    )
    .unwrap();
    let xml_lane = compile_artifact_lane(xml_plan, &Source("")).unwrap();
    static_md::reset_render_calls();
    let error = EmitPass::new(Arc::new(static_md::StaticMarkdownBackend::new()), None)
        .run(xml_lane)
        .unwrap_err();
    assert!(matches!(error, EmitPassError::TargetMismatch { .. }));
    assert_eq!(static_md::render_calls(), 0);
}

#[test]
fn validator_cell_cannot_reenter_any_renderer_or_pivot() {
    let source = [include_str!("validate.rs"), include_str!("validate/xml.rs")].join("\n");
    for forbidden in [
        "emit_markdown",
        "emit_xml",
        "from_markdown",
        "to_xml",
        "static_md",
        "static_xml",
    ] {
        assert!(
            !source.contains(forbidden),
            "validator contains `{forbidden}`"
        );
    }
}

#[test]
fn lane_digest_changes_for_every_top_level_and_nested_field_family() {
    let lane = full_lane();
    let expected = digest::lane_digest(&lane);
    let mut mutants = Vec::new();
    let mut nodes = lane.clone();
    nodes.source_node_count += 1;
    mutants.push(nodes);
    let mut link = lane.clone();
    link.source_link_digest.0[0] ^= 1;
    mutants.push(link);
    let mut path = lane.clone();
    path.frame.generated_path.as_mut().unwrap().push('x');
    mutants.push(path);
    let mut root = lane.clone();
    root.frame.source_root.as_mut().unwrap().push('x');
    mutants.push(root);
    let mut renames = lane.clone();
    renames.frame.renames.reverse();
    mutants.push(renames);
    let mut rename_field = lane.clone();
    rename_field.frame.renames[0].origin.push('x');
    mutants.push(rename_field);
    let mut order = lane.clone();
    order.contributions.reverse();
    mutants.push(order);
    let mut meta_lane = lane.clone();
    let LaneContribution::Simple {
        meta: meta_value, ..
    } = &mut meta_lane.contributions[0]
    else {
        unreachable!()
    };
    meta_value.origin.push('x');
    mutants.push(meta_lane);
    let mut body_lane = lane.clone();
    let LaneContribution::Simple { chunks, .. } = &mut body_lane.contributions[0] else {
        unreachable!()
    };
    let LaneChunk::Node(node) = &mut chunks[0] else {
        unreachable!()
    };
    let LaneNode::Simple {
        body: body_value, ..
    } = node.as_mut()
    else {
        unreachable!()
    };
    body_value.push('x');
    mutants.push(body_lane);
    let contexts = [
        ArtifactContext::testing(
            ArtifactId::new("changed").unwrap(),
            lane.context().target(),
            lane.context().frame().clone(),
            lane.context().mode(),
        ),
        ArtifactContext::testing(
            lane.context().artifact().clone(),
            ArtifactTarget::StaticXml,
            lane.context().frame().clone(),
            lane.context().mode(),
        ),
        ArtifactContext::testing(
            lane.context().artifact().clone(),
            lane.context().target(),
            ArtifactFrame::CompatibilityFragment,
            lane.context().mode(),
        ),
        ArtifactContext::testing(
            lane.context().artifact().clone(),
            lane.context().target(),
            lane.context().frame().clone(),
            StaticCompileMode::Plain,
        ),
    ];
    for context in contexts {
        mutants.push(LaneIr::assembled(
            context,
            lane.source_node_count,
            lane.source_link_digest.clone(),
            lane.frame.clone(),
            lane.contributions.clone(),
        ));
    }
    let mut elided = lane.clone();
    let LaneContribution::Elided { meta } = &mut elided.contributions[2] else {
        unreachable!()
    };
    meta.path.push('x');
    mutants.push(elided);
    let mut hoisted = lane.clone();
    let LaneContribution::Hoisted { target, .. } = &mut hoisted.contributions[3] else {
        unreachable!()
    };
    *target = SpecAddress::parse("spec://org.demo/h/boot/other").unwrap();
    mutants.push(hoisted);
    for (index, mutant) in mutants.into_iter().enumerate() {
        assert_ne!(digest::lane_digest(&mutant), expected, "mutant {index}");
    }

    let normal = compile_artifact_lane(
        ArtifactPlan::compatibility(
            SpecAddress::parse("spec://org.demo/pkg/boot/entry#root~r7").unwrap(),
            StaticCompileMode::Plain,
        ),
        &Source("# Entry {#root}\n"),
    )
    .unwrap();
    let normal_digest = digest::lane_digest(&normal);
    let mut identity = normal.clone();
    let LaneContribution::Normal {
        meta,
        seed,
        seed_address,
        ..
    } = &mut identity.contributions[0]
    else {
        unreachable!()
    };
    meta.path.push('x');
    seed.0 += 1;
    *seed_address = SpecAddress::parse("spec://org.demo/pkg/boot/entry#root~r8").unwrap();
    assert_ne!(digest::lane_digest(&identity), normal_digest);
    let mut pin = normal.clone();
    let LaneContribution::Normal { chunks, .. } = &mut pin.contributions[0] else {
        unreachable!()
    };
    let LaneChunk::Node(node) = &mut chunks[1] else {
        unreachable!()
    };
    let LaneNode::Normal {
        requested_address, ..
    } = node.as_mut()
    else {
        unreachable!()
    };
    *requested_address = SpecAddress::parse("spec://org.demo/pkg/boot/entry#root~r8").unwrap();
    assert_ne!(digest::lane_digest(&pin), normal_digest);
    let mut marker_lane = normal.clone();
    let LaneContribution::Normal { chunks, .. } = &mut marker_lane.contributions[0] else {
        unreachable!()
    };
    let LaneChunk::NormalOpen {
        marker: marker_value,
        ..
    } = &mut chunks[0]
    else {
        unreachable!()
    };
    *marker_value = LinkMarkerKey::from_address(
        &SpecAddress::parse("spec://org.demo/pkg/boot/other#root").unwrap(),
    );
    assert_ne!(digest::lane_digest(&marker_lane), normal_digest);
    let mut positions = normal.clone();
    let LaneContribution::Normal { chunks, .. } = &mut positions.contributions[0] else {
        unreachable!()
    };
    let LaneChunk::NormalOpen { occurrence, .. } = &mut chunks[0] else {
        unreachable!()
    };
    *occurrence += 1;
    assert_ne!(digest::lane_digest(&positions), normal_digest);
    let mut close = normal.clone();
    let LaneContribution::Normal { chunks, .. } = &mut close.contributions[0] else {
        unreachable!()
    };
    let LaneChunk::NormalClose { marker, .. } = chunks.last_mut().unwrap() else {
        unreachable!()
    };
    *marker = LinkMarkerKey::from_address(
        &SpecAddress::parse("spec://org.demo/pkg/boot/close#root").unwrap(),
    );
    assert_ne!(digest::lane_digest(&close), normal_digest);
    let mut newline = normal.clone();
    let LaneContribution::Normal { chunks, .. } = &mut newline.contributions[0] else {
        unreachable!()
    };
    if let Some(LaneChunk::ForcedNewline { contribution, .. }) = chunks
        .iter_mut()
        .find(|chunk| matches!(chunk, LaneChunk::ForcedNewline { .. }))
    {
        *contribution += 1;
    }
    assert_ne!(digest::lane_digest(&newline), normal_digest);
    let mut fence = normal;
    let LaneContribution::Normal { chunks, .. } = &mut fence.contributions[0] else {
        unreachable!()
    };
    let LaneChunk::Node(node) = &mut chunks[1] else {
        unreachable!()
    };
    let LaneNode::Normal { fence_after, .. } = node.as_mut() else {
        unreachable!()
    };
    *fence_after = LinkFenceSnapshot::Open {
        delimiter: '~',
        run: 4,
    };
    assert_ne!(digest::lane_digest(&fence), normal_digest);
}
