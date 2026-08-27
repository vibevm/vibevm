//! The shared real-schedule fixture: a tiny honest world, plans for both
//! built-in backends, and the artifacts the production schedule really
//! produces from them. Every carrier test that needs a REAL builtin value
//! (rather than a corpus document) starts here.

use std::collections::BTreeMap;

use crate::compiler::builtin::{compile_artifact, compile_compatibility_artifact};
use crate::compiler::ir::{
    ArtifactContext, ArtifactFrame, ArtifactId, ArtifactInput, ArtifactPlan, ArtifactTarget,
    EmittedArtifact, StaticCompileMode,
};
use crate::{SectionSource, SpecAddress};

/// The smallest honest section source: three packages, one shared base.
pub(super) struct Map(pub(super) BTreeMap<String, String>);

impl SectionSource for Map {
    fn section_text(&self, addr: &SpecAddress) -> Result<String, String> {
        self.0
            .get(&addr.without_pin())
            .cloned()
            .ok_or_else(|| format!("missing {}", addr.without_pin()))
    }
}

/// The proven whole-artifact fixture shape: two roots, one shared base, all
/// anchored, no anchor collisions after qualification.
pub(super) fn world() -> Map {
    let mut map = BTreeMap::new();
    map.insert(
        "spec://org.demo/alpha/boot/entry#root".to_string(),
        "# Alpha {#root}\n#use spec://org.demo/shared/boot/base#root\nALPHA\n".to_string(),
    );
    map.insert(
        "spec://org.demo/shared/boot/base#root".to_string(),
        "# Shared {#root}\n##SHARED shared\n".to_string(),
    );
    map.insert(
        "spec://org.demo/beta/boot/entry#root".to_string(),
        "# Beta {#root}\n##BETA beta\n".to_string(),
    );
    Map(map)
}

pub(super) fn generated_path(target: &ArtifactTarget) -> String {
    let extension = if target.is_static_markdown() {
        "STATIC.md"
    } else {
        "STATIC.xml"
    };
    format!("vibevm/vibespecs/boot/{extension}")
}

pub(super) fn context(target: ArtifactTarget) -> ArtifactContext {
    ArtifactContext::new(
        ArtifactId::new(target.backend_id()).unwrap(),
        target.clone(),
        ArtifactFrame::StaticLane {
            generated_path: generated_path(&target),
            source_root: "vibevm/vibespecs".to_string(),
        },
        StaticCompileMode::QualifyPerNode,
    )
    .unwrap()
}

fn root(origin: &str) -> ArtifactInput {
    ArtifactInput::normal(
        format!("org.demo/{origin}"),
        "boot/entry.md",
        SpecAddress::parse(&format!("spec://org.demo/{origin}/boot/entry#root")).unwrap(),
    )
    .unwrap()
}

/// One normal root — the plan the whole-artifact schedule is proven on.
pub(super) fn plan_for(target: ArtifactTarget) -> ArtifactPlan {
    ArtifactPlan::new(context(target), vec![root("alpha")]).unwrap()
}

/// Two normal roots, so the emitted tape carries two ordered contribution
/// markers that a reorder can actually break.
pub(super) fn two_root_plan(target: ArtifactTarget) -> ArtifactPlan {
    ArtifactPlan::new(context(target), vec![root("alpha"), root("beta")]).unwrap()
}

/// A normal root plus a whole-document hoist, so the tape carries the
/// emitter's OWN hoisted marker spelling beside a static one.
pub(super) fn hoisted_plan(target: ArtifactTarget) -> ArtifactPlan {
    ArtifactPlan::new(
        context(target),
        vec![
            root("alpha"),
            ArtifactInput::hoisted(
                "org.demo/gamma",
                "manual/part.md",
                SpecAddress::parse("spec://org.demo/gamma/manual/part.md").unwrap(),
            )
            .unwrap(),
        ],
    )
    .unwrap()
}

/// The artifact the real built-in schedule emits for `plan`.
pub(super) fn emit(plan: ArtifactPlan) -> EmittedArtifact {
    compile_artifact(plan, &world()).unwrap()
}

/// The artifact the real BUILTIN COMPATIBILITY row emits: one seed lowered
/// through `compile_compatibility_artifact`, i.e. artifact `static-fragment`
/// under the `static-md` target/backend and the compatibility frame.
pub(super) fn compatibility_emitted(mode: StaticCompileMode) -> EmittedArtifact {
    compile_compatibility_artifact(
        &SpecAddress::parse("spec://org.demo/alpha/boot/entry#root").unwrap(),
        &world(),
        mode,
    )
    .expect("the built-in compatibility backend lowers the seed")
}

pub(super) fn both_targets() -> [ArtifactTarget; 2] {
    [ArtifactTarget::StaticMarkdown, ArtifactTarget::StaticXml]
}
