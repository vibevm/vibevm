//! The `xml-minify` binding's own acceptance: the catalog row, the stage
//! law, and the segmented adapter's frame/document boundary.
//!
//! The KERNEL's corpus lives with the kernel (`transforms/xml_minify/tests`)
//! and is not duplicated here: comment codec and invalid `--` payload,
//! CDATA/fence boundaries, leaf versus mixed-content parent, DTD/entity
//! refusal, the no-element stream and the character-data laws are proved
//! there, on the function this cell binds. What is proved HERE is everything
//! the binding adds — which segment the kernel is allowed to see, which bytes
//! it may never see, and what happens when the tape carries a shape no
//! document segment can hold.

use specmark::verifies;

use crate::compiler::emit::framing::{self, CommentSyntax};
use crate::compiler::ir::ContributionMeta;

use super::behavior::{TransformBehavior, TransformBehaviorError};
use super::plan::{TransformImplementation, TransformStage};
use super::plan_validate::bounded;
use super::registry::TransformRegistry;
use super::xml_minify_binding::{
    XML_MINIFY_EPOCH, XML_MINIFY_NAME, XmlMinify, XmlMinifyBindingError,
};

/// Run the behavior over one tape, taking bytes in and bytes out exactly as
/// the emitted wrapper does.
fn run(tape: &str) -> Result<String, TransformBehaviorError> {
    let bytes = XmlMinify.run_emitted(None, tape.as_bytes().to_vec())?;
    Ok(String::from_utf8(bytes).expect("the adapter answers UTF-8 for a UTF-8 tape"))
}

/// The exact typed refusal one tape raises.
#[track_caller]
fn refusal(tape: &str) -> XmlMinifyBindingError {
    let error = run(tape).expect_err("this tape refuses");
    let TransformBehaviorError::EmittedTape { preview, source } = error else {
        panic!("the binding refuses through the emitted-tape arm: {error}")
    };
    assert_eq!(preview, bounded(XML_MINIFY_NAME));
    source
}

/// One generated XML comment, spelled by the EMIT cell itself rather than by
/// a literal here, so a fixture cannot drift from the framing the adapter
/// reads back.
fn c1(origin: &str) -> String {
    framing::static_marker(
        CommentSyntax::Xml,
        &ContributionMeta::new(origin, "boot/x.xml").expect("a lawful contribution meta"),
    )
}

/// The candidate implementation identity a manifest's declaration lowers to.
fn candidate() -> TransformImplementation {
    TransformImplementation::builtin_candidate(XML_MINIFY_NAME, XML_MINIFY_EPOCH)
}

/// A generated comment INSIDE a document is document content, never framing:
/// the frame law is a LINE-START law, and the XML writer indents every
/// in-document comment, so an indented `<!-- … -->` must reach the kernel
/// with its whole document around it.
///
/// The distinction is load-bearing, not stylistic. If any `<!-- ` opened a
/// frame span, an in-document comment would SPLIT its document into two
/// fragments, each unparseable alone — the kernel would refuse a lawful
/// artifact ("not well formed"), and the refusal would fire exactly on the
/// real lanes that carry annotated documents. So: one document, one interior
/// indented comment, minified fine, comment preserved byte-exact by the
/// kernel's own comment law, and the whitespace around the sibling elements
/// gone — which proves the kernel saw ONE whole document.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#STAGE-EMITTED")]
fn an_indented_in_document_comment_is_content_and_never_splits_its_document() {
    let frame = c1("org.demo/tools");
    let tape = format!(
        "{frame}\n\n<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<spec>\n  <p>a</p>\n  \
         <!-- vibe:c1 an-annotation -->\n  <p>b</p>\n</spec>\n"
    );
    let minified = run(&tape).expect("an annotated document is lawful and minifies");
    assert!(
        minified.len() < tape.len(),
        "the document really shrank: {} → {}",
        tape.len(),
        minified.len()
    );
    assert!(
        minified.contains("<!-- vibe:c1 an-annotation -->"),
        "the kernel preserves the in-document comment byte-exact: {minified}"
    );
    assert!(
        minified.starts_with(&format!("{frame}\n")),
        "the frame line survives verbatim ahead of the document"
    );
    assert_eq!(
        minified.matches("<?xml").count(),
        1,
        "one document in, one document out — nothing split it"
    );
}

/// The frozen production catalog: one row, its exact name, epoch and stage
/// (ABI §4's `[(name, epoch)]` golden).
///
/// The golden is what makes a silent behavior change red: an observable
/// change to the segmented adapter must bump the epoch, and bumping the epoch
/// moves this assertion first.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn the_production_catalog_is_exactly_xml_minify_at_epoch_one_on_the_emitted_stage() {
    let registry = TransformRegistry::builtins();
    let catalog = registry.catalog();
    assert_eq!(
        catalog
            .iter()
            .map(|(name, epoch, stage)| ((*name).to_owned(), *epoch, (*stage).clone()))
            .collect::<Vec<_>>(),
        vec![("xml-minify".to_owned(), 1, TransformStage::Emitted)],
        "the production catalog ships exactly the behaviors that exist"
    );
    assert_eq!(XML_MINIFY_NAME, "xml-minify");
    assert_eq!(XML_MINIFY_EPOCH, 1);
}

/// The kernel is a bytes→bytes transform, so EMITTED is the one stage it can
/// serve without a new serializer — and every other stage refuses through the
/// registry's own stage law, never through a silent no-op.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#STAGE-EMITTED")]
fn every_stage_but_emitted_refuses_through_the_registrys_own_law() {
    let registry = TransformRegistry::builtins();
    for stage in [
        TransformStage::Source,
        TransformStage::Document,
        TransformStage::Lane,
    ] {
        let Err(error) = registry.resolve(&candidate(), &stage) else {
            panic!("{stage:?}: a bytes→bytes kernel cannot serve a structured carrier")
        };
        assert!(
            error.to_string().contains("Emitted"),
            "{stage:?}: the refusal names the declared stage: {error}"
        );
        // The trait's own default says the same thing underneath: every
        // carrier method the behavior did not override answers the typed
        // wrong-stage refusal, so a stage that slipped past the registry
        // would still never silently no-op.
        assert!(matches!(
            XmlMinify.wrong_stage(stage.clone()),
            TransformBehaviorError::WrongStage {
                declared: TransformStage::Emitted,
                ..
            }
        ));
    }
    assert!(
        registry
            .resolve(&candidate(), &TransformStage::Emitted)
            .is_ok()
    );
}

/// The frame region is preserved BYTE for byte, and only document segments
/// shrink.
///
/// The assertion is stated as a partition rather than as a substring search:
/// every engine comment span and every inter-segment byte of the input
/// survives in the output at the same relative order, while the two documents
/// lose exactly their indentation. That is §2.2's ruling in executable form.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#STAGE-EMITTED")]
fn only_document_segments_shrink_and_every_frame_byte_survives() {
    let head = c1("head");
    let marker = c1("first");
    let second = c1("second");
    let tape = format!(
        "{head}\n\n{marker}\n\n<?xml version=\"1.0\"?>\n<spec>\n  <a/>\n  <b/>\n</spec>\n\n\
         {second}\n\n<?xml version=\"1.0\"?>\n<spec>\n  <c/>\n</spec>\n"
    );
    let expected = format!(
        "{head}\n\n{marker}\n\n<?xml version=\"1.0\"?>\n<spec><a/><b/></spec>\n\n\
         {second}\n\n<?xml version=\"1.0\"?>\n<spec><c/></spec>\n"
    );
    assert_eq!(run(&tape).expect("a lawful XML lane minifies"), expected);
    assert!(expected.len() < tape.len(), "the tape got strictly smaller");
}

/// A tape whose every gap is whitespace — an all-elided lane, or one with no
/// document at all — is returned UNCHANGED rather than refused.
///
/// This is the sharpest statement of why segmentation is not decoration. The
/// whole tape IS an XML stream containing no element, which the kernel
/// refuses by name; handing it the frame region would therefore break a
/// perfectly lawful artifact. The segmenter never shows it to the kernel,
/// because the frame is not a document segment.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#STAGE-EMITTED")]
fn an_all_elided_frame_only_tape_is_unchanged_rather_than_refused() {
    let elided = c1("vibe:static org.demo/a — boot/a.xml; zone elided");
    let tape = format!("{}\n\n{elided}\n\n", c1("head"));
    assert_eq!(run(&tape).expect("a frame-only lane is lawful"), tape);
    // The kernel, handed the same bytes whole, refuses them — the exact
    // difference segmentation makes.
    assert!(crate::transforms::minify_emitted_xml(tape.trim()).is_err());
}

/// A hoisted contribution refuses the artifact BY NAME (R4 architecture §8).
///
/// Never a silent skip and never a corrupting minify: the refusal carries the
/// marker's byte offset and a bounded preview of the origin it names, so the
/// declaration that cannot yet be handled is identified.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#STAGE-EMITTED")]
fn a_hoisted_contribution_refuses_the_artifact_naming_its_origin() {
    let head = c1("head");
    let hoisted = framing::hoisted_marker(CommentSyntax::Xml, "org.demo/hoisted-pkg");
    let tape = format!("{head}\n\n{hoisted}\n#use spec://org.demo/hoisted-pkg\n\n");
    let XmlMinifyBindingError::HoistedContribution { origin, offset } = refusal(&tape) else {
        panic!("a hoisted contribution has its own arm")
    };
    assert_eq!(origin, bounded("org.demo/hoisted-pkg"));
    assert_eq!(offset, head.len() + 2, "the marker's own byte offset");

    // The refusal is not "the `#use` line failed to parse": the marker alone
    // is what the artifact declares, so a hoisted contribution refuses even
    // where the line that follows it would have parsed as a document.
    let benign = format!("{head}\n\n{hoisted}\n\n<spec><a/></spec>\n");
    assert!(matches!(
        refusal(&benign),
        XmlMinifyBindingError::HoistedContribution { .. }
    ));
}

/// A refusing document segment names its offset in the WHOLE tape and keeps
/// the kernel's own typed diagnostic.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#STAGE-EMITTED")]
fn a_refusing_document_segment_carries_the_kernels_own_typed_diagnostic() {
    let head = c1("head");
    let tape = format!("{head}\n\n<!DOCTYPE spec>\n<spec/>\n");
    let XmlMinifyBindingError::Segment { offset, source } = refusal(&tape) else {
        panic!("a segment refusal has its own arm")
    };
    assert_eq!(offset, head.len() + 2, "the segment's start in the tape");
    assert!(
        source.diagnostic().contains("DTD declarations"),
        "the kernel's own reason survives: {source}"
    );

    // A Markdown lane is the general case of the same law: its body is prose
    // outside any element, so the binding refuses it rather than blessing
    // non-XML text.
    let markdown = format!(
        "{}\n\n<!-- vibe:static org.demo/a — boot/a.md -->\n\n# Heading\n\nbody\n",
        c1("head")
    );
    assert!(matches!(
        refusal(&markdown),
        XmlMinifyBindingError::Segment { .. }
    ));
}

/// Non-UTF-8 bytes and an unterminated engine comment each refuse typed, and
/// neither reaches the kernel.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#STAGE-EMITTED")]
fn a_malformed_tape_refuses_typed_before_the_kernel_sees_it() {
    let error = XmlMinify
        .run_emitted(None, vec![b'<', 0xFF, b'>'])
        .expect_err("invalid UTF-8 is not a tape");
    let TransformBehaviorError::EmittedTape { source, .. } = error else {
        panic!("the binding refuses through the emitted-tape arm")
    };
    assert_eq!(source, XmlMinifyBindingError::NotUtf8 { offset: 1 });

    let unterminated = "<!-- vibe:c1 head\n<spec/>\n";
    assert_eq!(
        refusal(unterminated),
        XmlMinifyBindingError::UnterminatedFrameComment { offset: 0 }
    );
}

/// The adapter is idempotent, and its no-change answer is the caller's OWN
/// bytes — the value T9's reconstruction compares against the original.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#STAGE-EMITTED")]
fn the_adapter_is_idempotent_and_a_no_op_returns_the_input_bytes() {
    let tape = format!("{}\n\n<spec>\n  <a/>\n  <b/>\n</spec>\n", c1("head"));
    let once = run(&tape).expect("the lane minifies");
    let twice = run(&once).expect("the minified lane minifies");
    assert_eq!(once, twice, "minify(minify(x)) == minify(x)");
    assert!(once.len() < tape.len());

    // A second pass changed nothing, so it handed the input straight back.
    let input = once.clone().into_bytes();
    let output = XmlMinify
        .run_emitted(None, input.clone())
        .expect("an already-minified lane is lawful");
    assert_eq!(output, input);
}

/// Configuration is delivered and ignored: `xml-minify` takes none, so the
/// three config states produce the same bytes.
///
/// Stated rather than assumed, because plan identity DOES distinguish the
/// three (a configured row digests differently) and a reader could otherwise
/// conclude the behavior reads what it is handed.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#STAGE-EMITTED")]
fn the_behavior_takes_no_configuration_and_ignores_every_state_of_it() {
    let tape = format!("{}\n\n<spec>\n  <a/>\n</spec>\n", c1("head"));
    let configured = super::plan_test_support::empty_config();
    let with_config = XmlMinify
        .run_emitted(Some(&configured), tape.as_bytes().to_vec())
        .expect("a configured invocation is lawful");
    let without = XmlMinify
        .run_emitted(None, tape.as_bytes().to_vec())
        .expect("an unconfigured invocation is lawful");
    assert_eq!(with_config, without);
}
