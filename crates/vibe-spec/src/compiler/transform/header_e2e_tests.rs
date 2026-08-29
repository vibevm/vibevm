//! The active-transforms header at COMPILE level (R4 architecture §7.1):
//! where its bytes land in each lane, that an empty plan writes none, and
//! that a reader which does not know it is unharmed by it.
//!
//! The value-level grammar (tokens, order, codec) is `header_tests`; the
//! validator's refusal of a malformed TAPE lives in `emit::mutant_tests`,
//! beside the other engine-owned tape mutants and inside the cell whose
//! observer it exercises. This cell owns the third question: bytes.

use specmark::verifies;

use crate::compiler::backend::BackendRegistry;
use crate::compiler::builtin::compile_artifact_with_registries;
use crate::compiler::ir::{ArtifactInput, ArtifactPlan, ArtifactTarget, EmittedArtifact};
use crate::{SectionSource, SpecAddress};

use super::plan::{TransformPlan, TransformStage};
use super::registry_test_support::{identity_plan, identity_registry};

/// The two keys the header records, chosen so one of them exercises the
/// corner the codec exists for: `--` is legal in a package name and would
/// otherwise corrupt or forbid the comment.
const FIRST_KEY: &str = "org.demo/tools#first";
const DOUBLE_HYPHEN_KEY: &str = "org.demo/a--b#second";

/// The exact header line those two keys produce.
const HEADER_LINE: &str = "<!-- vibe:transforms org.demo/tools#first org.demo/a-%2Db#second -->";

/// No document is ever fetched: both contributions are `simple`, carried
/// verbatim.
struct EmptySource;

impl SectionSource for EmptySource {
    fn section_text(&self, _address: &SpecAddress) -> Result<String, String> {
        Ok(String::new())
    }
}

fn lane_plan(target: ArtifactTarget) -> ArtifactPlan {
    let path = if target == ArtifactTarget::StaticMarkdown {
        "vibevm/vibespecs/boot/STATIC.md"
    } else {
        "vibevm/vibespecs/boot/STATIC.xml"
    };
    ArtifactPlan::static_lane(
        target,
        path,
        "vibevm/vibedeps",
        vec![
            ArtifactInput::simple("org.demo/a", "boot/a.md", "# A {#root}\n\nbody a\n")
                .expect("a lawful simple contribution"),
            ArtifactInput::simple("org.demo/b", "boot/b.md", "# B {#root}\n\nbody b\n")
                .expect("a lawful simple contribution"),
        ],
    )
    .expect("a lawful artifact plan")
}

fn compile(target: ArtifactTarget, transforms: TransformPlan) -> EmittedArtifact {
    compile_artifact_with_registries(
        lane_plan(target).with_transforms(transforms),
        &EmptySource,
        &BackendRegistry::builtins(),
        &identity_registry(),
    )
    .expect("the lane compiles")
}

fn text(artifact: &EmittedArtifact) -> String {
    String::from_utf8(artifact.bytes().to_vec()).expect("a UTF-8 tape")
}

/// The two-entry active plan both lanes are driven with.
fn two_entry_plan() -> TransformPlan {
    identity_plan(&[
        (FIRST_KEY, TransformStage::Lane),
        (DOUBLE_HYPHEN_KEY, TransformStage::Emitted),
    ])
}

/// The header's EXACT byte position, in both lanes, stated as the only
/// difference from the header-free twin.
///
/// Asserting "the transformed tape is the untransformed tape with this one
/// line inserted at index 3" pins position and content together, and says the
/// stronger thing besides: the identity catalog moved no other byte, so the
/// header is the whole of what an active plan adds. It also proves the
/// engine-owned tape validator ADMITTED the header — the compile would have
/// refused otherwise, since `validate::current` inspects the complete tape.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn the_header_is_the_fourth_line_of_both_lanes_and_the_only_byte_difference() {
    for target in [ArtifactTarget::StaticMarkdown, ArtifactTarget::StaticXml] {
        let baseline = text(&compile(target.clone(), TransformPlan::empty()));
        let transformed = text(&compile(target.clone(), two_entry_plan()));

        let mut expected: Vec<&str> = baseline.split('\n').collect();
        expected.insert(3, HEADER_LINE);
        assert_eq!(
            transformed,
            expected.join("\n"),
            "{target:?}: the active plan adds exactly the header line, after the three \
             provenance lines and before the blank separator"
        );
        assert_eq!(
            transformed.matches("vibe:transforms").count(),
            1,
            "{target:?}: one header per artifact, never one per stage"
        );
    }
}

/// The XML lane writes the header as a PLAIN comment, not a `vibe:c1` one.
///
/// That is the point of encoding the tokens: the payload carries no `--` and
/// no terminal `-`, so it is XML-comment-safe unconditionally and needs no
/// second wrapper. The three provenance lines above it ARE `vibe:c1`, so this
/// assertion also proves the two channels did not get confused.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn the_xml_lane_writes_the_header_unwrapped_beneath_three_c1_lines() {
    let tape = text(&compile(ArtifactTarget::StaticXml, two_entry_plan()));
    let lines: Vec<&str> = tape.split('\n').collect();
    for line in &lines[..3] {
        assert!(
            line.starts_with("<!-- vibe:c1 "),
            "the provenance lines stay c1: {line}"
        );
    }
    assert_eq!(lines[3], HEADER_LINE);
    assert!(
        !lines[3].contains("vibe:c1"),
        "the header is not double-encoded: {}",
        lines[3]
    );
}

/// An EMPTY plan writes zero bytes of header — the committed-artifact law.
///
/// Compiling with `TransformPlan::empty()` and compiling with no plan
/// attached at all are the same artifact, byte for byte, in both lanes.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn an_empty_plan_leaves_every_artifact_byte_identical() {
    for target in [ArtifactTarget::StaticMarkdown, ArtifactTarget::StaticXml] {
        let unattached = compile_artifact_with_registries(
            lane_plan(target.clone()),
            &EmptySource,
            &BackendRegistry::builtins(),
            &identity_registry(),
        )
        .expect("the lane compiles");
        let attached = compile(target.clone(), TransformPlan::empty());
        assert_eq!(
            attached.bytes(),
            unattached.bytes(),
            "{target:?}: attaching the empty plan is byte-inert"
        );
        assert!(
            !text(&attached).contains("vibe:transforms"),
            "{target:?}: an owner that activates nothing records nothing"
        );
    }
}

/// The static decompiler's classification: the header is SKIPPABLE
/// non-provenance.
///
/// That classification is per-KIND and it is made of two decisions, both
/// asserted here on the real emitted line: the generated-comment codec
/// reports it as NOT a `vibe:c1` provenance comment (`Ok(None)` — the arm the
/// decompiler falls through to), and its payload does not open with the
/// `vibe:static ` marker prefix the decompiler keeps. A comment failing
/// neither test is dropped silently, which is exactly the required behaviour.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn the_header_is_classified_skippable_non_provenance() {
    let tape = text(&compile(ArtifactTarget::StaticXml, two_entry_plan()));
    let header = tape.split('\n').nth(3).expect("the header line");
    assert_eq!(header, HEADER_LINE);
    assert_eq!(
        vibe_specdoc::decode_generated_xml_comment(header).expect("a complete comment decodes"),
        None,
        "the header is not a c1 provenance comment"
    );
    let payload = header
        .strip_prefix("<!-- ")
        .and_then(|value| value.strip_suffix(" -->"))
        .expect("the legacy-shaped comment interior");
    assert!(
        !payload.starts_with("vibe:static "),
        "the header carries no contribution marker: {payload}"
    );
}

/// The XML lane's semantic law: `from_xml(after)` equals `from_xml(before)`
/// for every document in the tape.
///
/// The header sits outside every document, so an active plan changes no
/// parsed node — asserted document by document rather than asserted once
/// about the whole tape, because the per-document form is the one R4 §8 (and
/// the minify binding after it) actually states.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn the_header_leaves_every_xml_documents_node_set_identical() {
    let before = text(&compile(ArtifactTarget::StaticXml, TransformPlan::empty()));
    let after = text(&compile(ArtifactTarget::StaticXml, two_entry_plan()));
    let before_docs = xml_documents(&before);
    let after_docs = xml_documents(&after);

    assert_eq!(before_docs.len(), 2, "the fixture carries two documents");
    assert_eq!(before_docs.len(), after_docs.len());
    for (before, after) in before_docs.iter().zip(&after_docs) {
        assert_eq!(
            vibe_specdoc::from_xml(after).expect("the emitted document parses"),
            vibe_specdoc::from_xml(before).expect("the emitted document parses"),
            "the header perturbs no document"
        );
    }
}

/// Every `<?xml …?> … </spec>` document in one emitted XML tape, in order.
fn xml_documents(tape: &str) -> Vec<&str> {
    const DECL: &str = "<?xml version=";
    const CLOSE: &str = "</spec>";
    let mut documents = Vec::new();
    let mut cursor = 0;
    while let Some(relative) = tape[cursor..].find(DECL) {
        let start = cursor + relative;
        let end = start
            + tape[start..]
                .find(CLOSE)
                .expect("an opened document closes")
            + CLOSE.len();
        documents.push(&tape[start..end]);
        cursor = end;
    }
    documents
}
