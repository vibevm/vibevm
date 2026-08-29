//! The one world the T8 selector tests cannot borrow from `artifact_tests`: a
//! two-document `#use` world whose REACHED document carries a declared path
//! spelled with a caller-chosen separator.
//!
//! It exists for `BACKLOG.md` `B-117`. Every boundary that authors a
//! contribution path already refuses a backslashed one — `ArtifactInput`'s four
//! constructors, the wire scalar gate, the inter-pass verifier — so no plan can
//! put such a path in front of a selector by declaring it. A document the
//! compiler REACHES crosses none of those boundaries: its declared path is its
//! own address' `doc_path`, and `SpecAddress::parse` refuses whitespace and
//! empty segments but admits a backslash inside a segment. This world is that
//! one shape, minimal and parameterised by exactly the byte under test, so the
//! forward-slashed twin differs in nothing else.
//!
//! The declared root's own contribution path is `roots/main.md`, deliberately
//! outside the `boot/*` scope the tests author: the root is then skipped rather
//! than run, and the reached document is the first thing any behavior could
//! have seen.

use std::collections::BTreeMap;

use vibe_core::{Group, PackageName};

use crate::compiler::ir::{
    ArtifactContext, ArtifactFrame, ArtifactId, ArtifactInput, ArtifactPlan, ArtifactTarget,
    DocumentProvider, StaticCompileMode,
};
use crate::{SectionSource, SpecAddress};

/// A section source over a fixed document map, keyed the way the compiler keys
/// documents (pin dropped, address rebuilt canonically).
pub(super) struct UseWorld(BTreeMap<String, String>);

impl SectionSource for UseWorld {
    fn section_text(&self, address: &SpecAddress) -> Result<String, String> {
        let key = address.without_pin();
        self.0
            .get(&key)
            .cloned()
            .ok_or_else(|| format!("missing {key}"))
    }
}

/// One declared root plus the document it reaches through `#use`, whose
/// `doc_path` is spelled exactly `used`.
pub(super) fn use_world(used: &str) -> (ArtifactPlan, UseWorld) {
    build(used, DocumentProvider::Undetermined)
}

/// The same two-document world, with the root declared BY a typed provider.
///
/// This is the world the T8 reached-verdict test could not build before T10B
/// (`R4-TRANSFORM-PLAN-ABI` §5.1): with the root's provider `Undetermined`,
/// an authored `packages` dimension refused at the root before any reached
/// document was ever judged, so the two absences could never be observed in
/// one live compile. With the root TYPED, they can — the root answers its own
/// provider, and the document it reaches answers `Unclaimed`.
pub(super) fn typed_use_world() -> (ArtifactPlan, UseWorld) {
    build(
        "boot/entry",
        DocumentProvider::Dependency {
            group: Group::parse("org.demo").expect("a valid test group"),
            name: PackageName::parse("back").expect("a valid test package name"),
        },
    )
}

fn build(used: &str, provider: DocumentProvider) -> (ArtifactPlan, UseWorld) {
    let root = SpecAddress::parse("spec://org.demo/back/roots/main#root")
        .expect("the declared root address parses");
    let reached = SpecAddress::parse(&format!("spec://org.demo/back/{used}#root"))
        .expect("a backslash is legal inside a path segment");
    let world = UseWorld(BTreeMap::from([
        (
            root.without_pin(),
            format!("# Root {{#root}}\n#use {}\nROOT\n", reached.without_pin()),
        ),
        (
            reached.without_pin(),
            "# Reached {#root}\n##REACHED reached\n".to_string(),
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
            ArtifactInput::normal_declared_by("org.demo/back", "roots/main.md", root, provider)
                .expect("a lawful contribution row"),
        ],
    )
    .expect("a lawful artifact plan");
    (plan, world)
}
