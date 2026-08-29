//! `xml-minify` on a REAL compiled artifact, through the REAL production
//! catalog (R4 architecture §8's last three bullets, §11 row 12).
//!
//! Everything here goes through `compile_artifact_with_registries` with
//! `TransformRegistry::builtins()` — not an injected test catalog — so what is
//! proved is the shipping behavior at the shipping position: after the
//! untransformed emitter and tape oracle succeed, through the T9 manager path,
//! on bytes an emitter really wrote.
//!
//! Three claims that a smaller test could not make. The tape got strictly
//! smaller AND every document's parsed node set is unchanged — losslessness
//! per document, which is the form §8 states. The FRAME survived byte for byte
//! next to a real §7.1 header, which is what keeps the artifact's own wire
//! gate satisfied. And the whole thing is idempotent, which is what makes a
//! fingerprint-fresh skip honest.

use specmark::verifies;
use vibe_core::manifest::ExtensionKey;

use crate::compiler::backend::BackendRegistry;
use crate::compiler::builtin::{ArtifactCompileError, compile_artifact_with_registries};
use crate::compiler::ir::{ArtifactInput, ArtifactPlan, ArtifactTarget, EmittedArtifact};
use crate::{SectionSource, SpecAddress};

use super::plan::{
    TransformImplementation, TransformPlan, TransformProvider, TransformSeed, TransformStage,
};
use super::plan_test_support::default_dependency;
use super::registry::TransformRegistry;
use super::xml_minify_binding::{XML_MINIFY_EPOCH, XML_MINIFY_NAME};

/// The activating declaration's key — what the §7.1 header records and what
/// the schedule builds its pass name from.
const MINIFY_KEY: &str = "org.demo/tools#minify";

/// The exact header line one active `xml-minify` entry writes.
const HEADER_LINE: &str = "<!-- vibe:transforms org.demo/tools#minify -->";

/// No document is ever fetched: every contribution is `simple`.
struct EmptySource;

impl SectionSource for EmptySource {
    fn section_text(&self, _address: &SpecAddress) -> Result<String, String> {
        Ok(String::new())
    }
}

/// One `xml-minify` entry, exactly as the T10B lowering would build it from a
/// manifest row: the workspace supplies a NAME, and the epoch comes off the
/// production catalog.
fn minify_plan() -> TransformPlan {
    let epoch = TransformRegistry::builtins()
        .epoch_of(XML_MINIFY_NAME)
        .expect("the production catalog knows its own behavior");
    assert_eq!(epoch, XML_MINIFY_EPOCH);
    TransformPlan::build(vec![TransformSeed::new(
        ExtensionKey::authored(MINIFY_KEY),
        TransformProvider::from(&default_dependency()),
        TransformStage::Emitted,
        TransformImplementation::builtin_candidate(XML_MINIFY_NAME, epoch),
        None,
        None,
    )])
    .expect("a one-entry emitted plan builds")
}

/// The XML lane fixture: two `simple` contributions, each a real document the
/// XML backend renders indented.
fn lane_plan(contributions: Vec<ArtifactInput>) -> ArtifactPlan {
    ArtifactPlan::static_lane(
        ArtifactTarget::StaticXml,
        "vibevm/vibespecs/boot/STATIC.xml",
        "vibevm/vibedeps",
        contributions,
    )
    .expect("a lawful artifact plan")
}

/// One authored contribution with `sections` nested sections — a document
/// whose XML rendering really is indented, which is what a minifier has to
/// bite on.
fn authored(title: &str, sections: usize) -> String {
    let mut body = format!("# {title} {{#root}}\n");
    for index in 0..sections {
        body.push_str(&format!(
            "\n## Section {index} {{#s{index}}}\n\nparagraph {index} of {title}\n"
        ));
    }
    body
}

fn two_documents() -> Vec<ArtifactInput> {
    vec![
        ArtifactInput::simple("org.demo/a", "boot/a.md", authored("A", 12))
            .expect("a lawful simple contribution"),
        ArtifactInput::simple("org.demo/b", "boot/b.md", authored("B", 12))
            .expect("a lawful simple contribution"),
    ]
}

fn compile(
    contributions: Vec<ArtifactInput>,
    transforms: TransformPlan,
) -> Result<EmittedArtifact, ArtifactCompileError> {
    compile_artifact_with_registries(
        lane_plan(contributions).with_transforms(transforms),
        &EmptySource,
        &BackendRegistry::builtins(),
        &TransformRegistry::builtins(),
    )
}

fn tape(artifact: &EmittedArtifact) -> String {
    String::from_utf8(artifact.bytes().to_vec()).expect("a UTF-8 tape")
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

/// Every engine-framed comment line of one tape, in order — the tape's
/// "decompiled" contribution/provenance record.
fn frame_comments(tape: &str) -> Vec<&str> {
    tape.lines()
        .filter(|line| line.starts_with("<!-- vibe:c1 "))
        .collect()
}

/// §8's three closing bullets on a real artifact, in one place because they
/// are one claim: the tape shrank, and nothing a reader can recover from it
/// moved.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#STAGE-EMITTED")]
fn a_real_lane_shrinks_while_every_documents_nodes_and_the_frame_survive() {
    let baseline = compile(two_documents(), TransformPlan::empty()).expect("the lane compiles");
    let minified = compile(two_documents(), minify_plan()).expect("the minified lane compiles");
    let before = tape(&baseline);
    let after = tape(&minified);

    assert!(
        after.len() < before.len(),
        "a real indented XML lane gets strictly smaller: {} → {}",
        before.len(),
        after.len()
    );

    // Per DOCUMENT, which is the form §8 states — a whole-tape claim would be
    // satisfied by a transform that moved content between documents.
    let before_documents = xml_documents(&before);
    let after_documents = xml_documents(&after);
    assert_eq!(
        before_documents.len(),
        2,
        "the fixture carries two documents"
    );
    assert_eq!(before_documents.len(), after_documents.len());
    for (before, after) in before_documents.iter().zip(&after_documents) {
        assert!(after.len() < before.len(), "each document shrank");
        assert_eq!(
            vibe_specdoc::from_xml(after).expect("the minified document parses"),
            vibe_specdoc::from_xml(before).expect("the emitted document parses"),
            "minifying preserves the parsed node set exactly"
        );
    }

    // The frame: the same provenance and contribution comments, in the same
    // order, byte for byte — plus the one active-transforms header, which is
    // NOT a `vibe:c1` comment and so is not in this set.
    assert_eq!(
        frame_comments(&after),
        frame_comments(&before),
        "no engine-framed comment moved a byte"
    );
    assert_eq!(
        after.lines().nth(3),
        Some(HEADER_LINE),
        "the active plan records exactly its own entry, in the §7.1 position"
    );
    assert_eq!(after.matches("vibe:transforms").count(), 1);
}

/// Idempotence at the artifact level: recompiling the same world yields the
/// same bytes, and re-minifying the already-minified tape moves nothing.
///
/// The second half is what a fingerprint-fresh skip rests on. A transform
/// that shrank a little more on every run would make every regeneration a
/// change, and no freshness claim about it could be honest.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#STAGE-EMITTED")]
fn the_minified_artifact_is_stable_and_the_transform_is_idempotent() {
    let first = compile(two_documents(), minify_plan()).expect("the lane compiles");
    let second = compile(two_documents(), minify_plan()).expect("the lane recompiles");
    assert_eq!(first.bytes(), second.bytes(), "one world, one artifact");

    let once = tape(&first);
    let twice = crate::transforms::minify_emitted_xml(&once)
        .expect("an already-minified lane is still lawful XML");
    assert_eq!(
        twice.as_ref(),
        once.as_str(),
        "minify(minify(x)) == minify(x) on the real tape"
    );
}

/// The T9 law at the shipping position: bytes that did not move return the
/// ORIGINAL artifact, so nothing is recorded and nothing is recomputed.
///
/// An all-elided lane is the honest way to reach that arm on a real compile:
/// it has framing and contribution markers but no document segment, so the
/// segmented adapter has nothing to minify. It is also §8's
/// "all-elided/no-element stream" bullet at artifact level — the whole tape IS
/// an element-free XML stream, which the bare kernel refuses by name.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#STAGE-EMITTED")]
fn an_all_elided_lane_returns_the_original_artifact_untouched() {
    let elided = || {
        vec![
            ArtifactInput::elided("org.demo/a", "boot/a.md").expect("a lawful elided contribution"),
            ArtifactInput::elided("org.demo/b", "boot/b.md").expect("a lawful elided contribution"),
        ]
    };
    let baseline = compile(elided(), TransformPlan::empty()).expect("the lane compiles");
    let minified = compile(elided(), minify_plan()).expect("the minified lane compiles");

    // The header is the ONLY difference: the transform itself moved no byte.
    let before = tape(&baseline);
    let mut expected: Vec<&str> = before.split('\n').collect();
    expected.insert(3, HEADER_LINE);
    assert_eq!(tape(&minified), expected.join("\n"));

    // And it recorded nothing, because it changed nothing (ABI §6.5).
    assert!(
        minified.provenance.emitted_transforms.is_empty(),
        "byte-equal output returns the original artifact, un-appended"
    );
    assert_eq!(
        minified.provenance.bytes_digest,
        crate::compiler::emit::emitted_bytes_digest(minified.bytes()),
        "no digest was recomputed over different bytes"
    );

    // The bare kernel, handed the same frame-only tape, refuses it — the
    // exact difference the segmented adapter makes.
    assert!(crate::transforms::minify_emitted_xml(&tape(&minified)).is_err());
}

/// A changed tape records exactly one `transform:emitted:<key>` entry, and
/// every other provenance member survives (ABI §6.5).
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#STAGE-EMITTED")]
fn a_changed_tape_records_the_one_pass_that_rewrote_it_and_nothing_else() {
    let baseline = compile(two_documents(), TransformPlan::empty()).expect("the lane compiles");
    let minified = compile(two_documents(), minify_plan()).expect("the minified lane compiles");

    assert_eq!(
        minified
            .provenance
            .emitted_transforms
            .iter()
            .map(|pass| pass.as_str().to_owned())
            .collect::<Vec<_>>(),
        vec![format!("transform:emitted:{MINIFY_KEY}")],
    );
    assert_eq!(
        minified.provenance.bytes_digest,
        crate::compiler::emit::emitted_bytes_digest(minified.bytes()),
        "the digest was recomputed through the ONE digest cell"
    );
    assert_ne!(
        minified.provenance.bytes_digest,
        baseline.provenance.bytes_digest
    );
    // Copied unchanged: what the closure, link, assemble and emit stages did
    // is not restated by a post-backend rewrite of the bytes.
    assert_eq!(minified.provenance.backend, baseline.provenance.backend);
    assert_eq!(minified.provenance.producer, baseline.provenance.producer);
    assert_eq!(
        minified.provenance.source_lane_digest,
        baseline.provenance.source_lane_digest
    );
    assert_eq!(
        minified.provenance.contributions,
        baseline.provenance.contributions
    );
    assert_eq!(minified.provenance.renames, baseline.provenance.renames);
}

/// A hoisted contribution refuses the WHOLE compile, by name — never a silent
/// skip and never a corrupted lane (R4 architecture §8).
///
/// The refusal is raised at the emitted position on a tape the backend really
/// wrote, so what is proved is that the shape exists in a real lane and that
/// the adapter recognises it there.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#STAGE-EMITTED")]
fn a_hoisted_contribution_refuses_the_whole_compile_by_name() {
    let hoisted = || {
        vec![
            ArtifactInput::simple("org.demo/a", "boot/a.md", authored("A", 3))
                .expect("a lawful simple contribution"),
            ArtifactInput::hoisted(
                "org.demo/shared",
                "boot/shared.md",
                SpecAddress::parse("spec://org.demo/shared/boot/entry")
                    .expect("a lawful package address"),
            )
            .expect("a lawful hoisted contribution"),
        ]
    };
    // Without the transform the same lane compiles: the refusal is the
    // adapter's, not the fixture's.
    let baseline = compile(hoisted(), TransformPlan::empty()).expect("the hoisted lane compiles");
    assert!(tape(&baseline).contains("#use spec://org.demo/shared"));

    let error = compile(hoisted(), minify_plan()).expect_err("a hoisted lane refuses");
    let rendered = format!("{error:#}");
    assert!(
        rendered.contains("hoisted contribution"),
        "the refusal names the shape it cannot handle: {rendered}"
    );
    assert!(
        rendered.contains("org.demo"),
        "the refusal names the contribution: {rendered}"
    );
}
