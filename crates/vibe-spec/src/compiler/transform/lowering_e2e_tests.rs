//! T10B end to end: a plan the LOWERING produced from a REAL collected
//! registry actually schedules and runs, at every one of the four positions,
//! over boot-shaped inputs — and, being the identity catalog, changes not one
//! byte.
//!
//! **Where the seam sits, and why it is here.** The whole chain the workspace
//! walks is: durable world → owner-scoped view → one kernel collector →
//! `enabled_compile_rows()` → `TransformPlan::from_effective_rows` →
//! `ArtifactPlan::with_transforms` → compile. Everything from the collector
//! rightwards is exercised in this cell against a registry the kernel really
//! collected. What cannot be exercised from `vibe-workspace` is the LAST
//! link at every stage: the production catalog ships one emitted behavior,
//! and the four-tier identity catalog is `#[cfg(test)]` inside `vibe-spec`
//! (`compile_artifact_with_registries`, deliberately never widened into
//! `feature = "test-support"`). Widening it to reach a workspace test would
//! put a test-only registry in the production surface — the exact thing the
//! seam's own doc refuses. So execution is proven HERE, at
//! `compile_artifact_with_registries`, over a plan produced by the real
//! lowering from a real collected registry; the workspace side proves the
//! links to its left (the typed inputs, the owner scoping, and the
//! empty-plan byte identity that is R4 §11.3's proof).
//!
//! **The inputs are the boot-built shape.** Each declared contribution is
//! built through the T10B typed constructors with a real
//! [`DocumentProvider`], exactly as `boot_artifacts` now builds them — so
//! what runs here is what runs on a boot lane, not a shape unique to a test.

use specmark::verifies;

use crate::compiler::backend::BackendRegistry;
use crate::compiler::builtin::compile_artifact_with_registries;
use crate::compiler::ir::{
    ArtifactContext, ArtifactFrame, ArtifactId, ArtifactInput, ArtifactPlan, ArtifactTarget,
    DocumentProvider, EmittedArtifact, StaticCompileMode,
};
use crate::{SectionSource, SpecAddress};
use vibe_core::{Group, PackageName};

use std::collections::BTreeMap;

use super::lowering_worlds::{Declared, collected_host};
use super::plan::TransformPlan;
use super::registry_test_support::{
    identity_invocations, identity_registry, reset_identity_invocations,
};

/// A section source over one fixed document map, keyed the way the compiler
/// keys documents.
struct World(BTreeMap<String, String>);

impl SectionSource for World {
    fn section_text(&self, address: &SpecAddress) -> Result<String, String> {
        let key = address.without_pin();
        self.0
            .get(&key)
            .cloned()
            .ok_or_else(|| format!("missing {key}"))
    }
}

/// The typed provider a dependency contribution declares.
fn dependency(name: &str) -> DocumentProvider {
    DocumentProvider::Dependency {
        group: Group::parse("org.demo").expect("a valid test group"),
        name: PackageName::parse(name).expect("a valid test package name"),
    }
}

/// One boot-shaped artifact plan: a declared `normal` root reaching a second
/// document, plus a declared `simple` contribution — the two document
/// producing kinds, both carrying typed providers exactly as the boot
/// adapter now builds them.
fn boot_shaped_plan() -> (ArtifactPlan, World) {
    let root = SpecAddress::parse("spec://org.demo/alpha/boot/entry#root")
        .expect("the declared root address parses");
    // The reached document belongs to a DIFFERENT package, exactly as the
    // shared compiler fixture's does: one package's lane cannot carry two
    // `#root` facts.
    let reached = SpecAddress::parse("spec://org.demo/shared/boot/base#root")
        .expect("the reached address parses");
    let world = World(BTreeMap::from([
        (
            root.without_pin(),
            format!("# Alpha {{#root}}\n#use {}\nALPHA\n", reached.without_pin()),
        ),
        (
            reached.without_pin(),
            "# Base {#root}\n##BASE base\n".to_string(),
        ),
    ]));
    let context = ArtifactContext::new(
        ArtifactId::new("static-xml").expect("a valid artifact id"),
        ArtifactTarget::StaticXml,
        ArtifactFrame::StaticLane {
            generated_path: "vibevm/vibespecs/boot/STATIC.xml".to_string(),
            source_root: "vibevm/vibedeps".to_string(),
        },
        StaticCompileMode::QualifyPerNode,
    )
    .expect("a valid artifact context");
    let plan = ArtifactPlan::new(
        context,
        vec![
            ArtifactInput::normal_declared_by(
                "org.demo/alpha",
                "boot/alpha.md",
                root,
                dependency("alpha"),
            )
            .expect("a lawful typed normal contribution"),
            ArtifactInput::simple_declared_by(
                "org.demo/local",
                "boot/local.md",
                "# Local {#root}\nLOCAL\n",
                dependency("local"),
            )
            .expect("a lawful typed simple contribution"),
        ],
    )
    .expect("a lawful artifact plan");
    (plan, world)
}

/// Compile one boot-shaped lane with the given transform plan attached.
fn compile(transforms: TransformPlan) -> EmittedArtifact {
    let (plan, world) = boot_shaped_plan();
    compile_artifact_with_registries(
        plan.with_transforms(transforms),
        &world,
        &BackendRegistry::builtins(),
        &identity_registry(),
    )
    .expect("the boot-shaped lane compiles")
}

/// §4.6: a workspace-lowered plan really schedules.
///
/// The world declares one contribution at each of the four staged points,
/// they are collected by the kernel, lowered by [`TransformPlan`]'s own
/// public entry shape, attached to the artifact, and every one of them RUNS —
/// proven by the behaviors' own invocation counters, not by a pass count.
/// Being the identity catalog, the emitted bytes are those of the
/// untransformed lane, byte for byte: the transforms are demonstrably in the
/// schedule AND demonstrably neutral, which is the only combination that
/// separates "it ran and did nothing" from "it never ran".
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION")]
fn a_lowered_plan_from_a_collected_registry_runs_at_all_four_positions_and_moves_no_byte() {
    let registry = collected_host(vec![
        Declared::builtin("src", "compile:source", "test-identity-source"),
        Declared::builtin("doc", "compile:document", "test-identity-document"),
        Declared::builtin("lane", "compile:lane", "test-identity-lane"),
        Declared::builtin("emit", "compile:emitted", "test-identity-emitted"),
    ]);
    let lowered = TransformPlan::from_effective_rows_with(
        &registry.enabled_compile_rows(),
        &identity_registry(),
    )
    .expect("the collected compile family lowers");
    assert_eq!(lowered.len(), 4, "one entry per declared position");

    reset_identity_invocations();
    let baseline = compile(TransformPlan::empty());
    assert_eq!(
        identity_invocations(),
        (0, 0, 0, 0),
        "the empty plan appends no pass, so no behavior can have run"
    );

    reset_identity_invocations();
    let transformed = compile(lowered);
    let (source, document, lane, emitted) = identity_invocations();
    assert_eq!(
        (lane, emitted),
        (1, 1),
        "a lane/emitted transform runs once per artifact"
    );
    assert!(
        source >= 2 && source == document,
        "a source/document transform runs once per addressed document, \
         and both positions see the same documents: got {source} / {document}"
    );

    // The identity catalog is neutral: it ran, and it moved no byte OF THE
    // ARTIFACT BODY. The one byte difference an active plan is entitled to is
    // the header line R4 architecture §7.1 requires it to record — inserted
    // after the three provenance lines, and nowhere else. Stating it as "the
    // baseline plus exactly this line" keeps the neutrality claim exact
    // rather than relaxing it to a containment check.
    let baseline_text = String::from_utf8(baseline.bytes().to_vec()).expect("a UTF-8 tape");
    let mut expected: Vec<&str> = baseline_text.split('\n').collect();
    expected.insert(
        3,
        "<!-- vibe:transforms __host__/demo#src __host__/demo#doc \
         __host__/demo#lane __host__/demo#emit -->",
    );
    assert_eq!(
        String::from_utf8(transformed.bytes().to_vec()).expect("a UTF-8 tape"),
        expected.join("\n"),
        "the identity catalog moved no body byte; the active plan added exactly its header"
    );
}

/// The negative control the byte assertion needs: a plan the lowering
/// produced is not inert BECAUSE it is empty. One entry fewer is one
/// invocation fewer, so the counters above are reading a live schedule.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn dropping_a_declared_position_drops_exactly_its_invocations() {
    let both = collected_host(vec![
        Declared::builtin("lane", "compile:lane", "test-identity-lane"),
        Declared::builtin("emit", "compile:emitted", "test-identity-emitted"),
    ]);
    let one = collected_host(vec![Declared::builtin(
        "lane",
        "compile:lane",
        "test-identity-lane",
    )]);
    let lower = |registry: &vibe_extension_registry::ExtensionRegistry| {
        TransformPlan::from_effective_rows_with(
            &registry.enabled_compile_rows(),
            &identity_registry(),
        )
        .expect("the collected world lowers")
    };

    reset_identity_invocations();
    compile(lower(&both));
    assert_eq!(identity_invocations(), (0, 0, 1, 1));

    reset_identity_invocations();
    compile(lower(&one));
    assert_eq!(
        identity_invocations(),
        (0, 0, 1, 0),
        "removing the emitted declaration removed exactly its invocation"
    );
}
