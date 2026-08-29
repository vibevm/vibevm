//! Manager-boundary verifier reds: pass attribution, error precedence,
//! stop-before-next-pass, the honest gather boundary, and the test-only
//! enabling seam's default-off inertness.

use std::collections::BTreeMap;
use std::convert::Infallible;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

use specmark::verifies;

use super::super::builtin::compile_artifact_prefix;
use super::super::ir::{
    ArtifactPlan, ClosureIr, DocumentAddress, DocumentIr, Documents, SourceFormatId, SourceIr,
    StaticCompileMode,
};
use super::super::pass::{
    AnyIr, DynPass, IrPayload, Pass, PassDescriptor, PassName, PassSegment, PassSegmentError,
};
use super::super::pipeline::CompilerPipeline;
use super::IrVerifier;

fn name(value: &str) -> PassName {
    PassName::new(value).unwrap()
}

fn spec_source(anchor: &str, text: &str) -> SourceIr {
    SourceIr::reached(
        DocumentAddress::Spec(
            crate::SpecAddress::parse(&format!("spec://org.demo/pkg/common/{anchor}#{anchor}"))
                .unwrap(),
        ),
        SourceFormatId::new("markdown").unwrap(),
        text,
    )
}

fn parse_like(name: PassName) -> impl Pass {
    struct ParseLike {
        name: PassName,
    }
    impl Pass for ParseLike {
        type Input = SourceIr;
        type Output = DocumentIr;
        type Error = Infallible;

        fn name(&self) -> &PassName {
            &self.name
        }

        fn run(&self, input: SourceIr) -> Result<DocumentIr, Infallible> {
            let (address, format, subject, text) = input.into_parts();
            Ok(DocumentIr::new(
                SourceIr::new(address, format, subject, text.clone()),
                crate::DocTree::parse(&text),
            ))
        }
    }
    ParseLike { name }
}

/// A document transform that forges a duplicate fact anchor.
struct BreakAnchors {
    name: PassName,
}

impl Pass for BreakAnchors {
    type Input = DocumentIr;
    type Output = DocumentIr;
    type Error = Infallible;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, input: DocumentIr) -> Result<DocumentIr, Infallible> {
        let (source, _tree) = input.into_parts();
        let forged = format!("{}\n##dup once\n\n##dup twice\n", source.text());
        Ok(DocumentIr::new(source, crate::DocTree::parse(&forged)))
    }
}

/// An identity transform that counts its invocations.
struct Counting {
    name: PassName,
    invocations: Arc<AtomicUsize>,
}

impl Pass for Counting {
    type Input = DocumentIr;
    type Output = DocumentIr;
    type Error = Infallible;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, input: DocumentIr) -> Result<DocumentIr, Infallible> {
        self.invocations
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(input)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("the pass refuses to run")]
struct Refusal;

struct Failing {
    name: PassName,
}

impl Pass for Failing {
    type Input = DocumentIr;
    type Output = DocumentIr;
    type Error = Refusal;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, _input: DocumentIr) -> Result<DocumentIr, Refusal> {
        Err(Refusal)
    }
}

/// Returns the wrong erased carrier (a source value with blank identity)
/// while declaring a document output — only possible past the typed surface.
struct LyingOutput {
    name: PassName,
}

impl DynPass for LyingOutput {
    fn descriptor(&self) -> PassDescriptor {
        PassDescriptor {
            name: self.name.clone(),
            input: DocumentIr::SHAPE,
            output: DocumentIr::SHAPE,
        }
    }

    fn run_erased(&self, _input: AnyIr) -> Result<AnyIr, PassSegmentError> {
        let damaged = SourceIr::reached(
            DocumentAddress::StaticEntry {
                origin: " ".to_string(),
                path: "boot/entry.md".to_string(),
            },
            SourceFormatId::new("markdown").unwrap(),
            "text",
        );
        Ok(AnyIr::Source(damaged))
    }
}

/// A first artifact pass that counts invocations and returns a valid closure.
struct CountingClose {
    name: PassName,
    invocations: Arc<AtomicUsize>,
}

impl Pass for CountingClose {
    type Input = Documents;
    type Output = ClosureIr;
    type Error = Infallible;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, input: Documents) -> Result<ClosureIr, Infallible> {
        self.invocations
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let _ = input.len();
        Ok(super::closure_tests::minimal_closure())
    }
}

fn verified_document_pipeline() -> CompilerPipeline {
    let mut pipeline = CompilerPipeline::default();
    pipeline
        .push_document(parse_like(name("parse-like")))
        .unwrap();
    pipeline.enable_verify_each_for_tests();
    pipeline
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_corrupting_pass_fails_under_its_own_name_and_the_next_pass_never_runs() {
    let mut pipeline = verified_document_pipeline();
    pipeline
        .push_document(BreakAnchors {
            name: name("break-anchors"),
        })
        .unwrap();
    let invocations = Arc::new(AtomicUsize::new(0));
    pipeline
        .push_document(Counting {
            name: name("count-next"),
            invocations: invocations.clone(),
        })
        .unwrap();

    let error = pipeline
        .run_documents(vec![spec_source("root", "# Doc {#root}\n")])
        .unwrap_err();
    match error {
        super::super::pipeline::CompilerPipelineError::Segment(
            PassSegmentError::VerificationFailed { pass, source, .. },
        ) => {
            assert_eq!(pass.as_str(), "break-anchors");
            assert!(
                matches!(&*source, super::VerificationError::DuplicateId { .. }),
                "{source:?}"
            );
        }
        other => panic!("expected attribution to break-anchors, got {other:?}"),
    }
    assert_eq!(
        invocations.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "no later pass may run after failure"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_wrong_runtime_output_carrier_wins_over_semantic_verification() {
    let mut segment = PassSegment::default();
    segment
        .push_erased_for_test(Box::new(LyingOutput {
            name: name("lying-output"),
        }))
        .unwrap();

    let error = segment
        .run_checked(AnyIr::Document(document()), Some(IrVerifier))
        .unwrap_err();
    assert!(
        matches!(error, PassSegmentError::WrongOutput { ref pass, .. } if pass.as_str() == "lying-output"),
        "{error:?}"
    );
    // The damaged carrier would fail verification (the blank static origin is
    // proven above); reaching WrongOutput instead proves the verifier never
    // blessed it — output shape is checked before semantics.
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_pass_own_error_wins_and_no_later_pass_runs() {
    let mut pipeline = verified_document_pipeline();
    pipeline
        .push_document(Failing {
            name: name("refusing"),
        })
        .unwrap();
    let invocations = Arc::new(AtomicUsize::new(0));
    pipeline
        .push_document(Counting {
            name: name("count-next"),
            invocations: invocations.clone(),
        })
        .unwrap();

    let error = pipeline
        .run_documents(vec![spec_source("root", "# Doc {#root}\n")])
        .unwrap_err();
    match error {
        super::super::pipeline::CompilerPipelineError::Segment(PassSegmentError::PassFailed {
            pass,
            source,
        }) => {
            assert_eq!(pass.as_str(), "refusing");
            assert!(source.downcast_ref::<Refusal>().is_some());
        }
        other => panic!("expected the pass's own error, got {other:?}"),
    }
    assert_eq!(invocations.load(std::sync::atomic::Ordering::SeqCst), 0);
}

fn document() -> DocumentIr {
    let source = spec_source("root", "");
    DocumentIr::new(source, crate::DocTree::parse("# Doc {#root}\nbody\n"))
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn duplicate_gather_keys_refuse_at_the_boundary_before_any_artifact_pass() {
    let mut pipeline = CompilerPipeline::default();
    pipeline
        .push_document(parse_like(name("parse-like")))
        .unwrap();
    pipeline.enable_verify_each_for_tests();
    let invocations = Arc::new(AtomicUsize::new(0));
    pipeline
        .push_artifact(CountingClose {
            name: name("first-artifact"),
            invocations: invocations.clone(),
        })
        .unwrap();

    // The real ordering: gather, then the artifact segment on what it returned.
    // With the guard removed, `gather_documents` yields Ok and `run_to_closure`
    // runs the counting pass — so the invocation count is load-bearing here,
    // unlike a bare `run_documents`, which never reaches the artifact segment.
    let parsed = pipeline
        .run_documents(vec![spec_source("root", "# One {#root}\n")])
        .expect("a clean single-document batch gathers")
        .into_vec();
    let dirty = vec![
        parsed[0].clone(),
        DocumentIr::new(
            spec_source("root", "# Two {#root}\n"),
            crate::DocTree::parse("# Two {#root}\n"),
        ),
    ];
    let error = pipeline
        .gather_documents(dirty)
        .and_then(|documents| pipeline.run_to_closure(documents).map(|_| ()))
        .unwrap_err();
    let boundary = error.to_string();
    match error {
        super::super::pipeline::CompilerPipelineError::GatherVerification { source } => {
            assert!(
                matches!(
                    source.as_ref(),
                    super::VerificationError::DuplicateDocument {
                        first: 0,
                        second: 1,
                        ..
                    }
                ),
                "{source:?}"
            );
            assert!(
                boundary.contains("gather-documents"),
                "the boundary, not a pass, is named: {boundary}"
            );
        }
        other => panic!("expected the gather boundary error, got {other:?}"),
    }
    assert_eq!(
        invocations.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "close never ran on the dirty batch"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn an_invalid_segment_input_names_the_boundary_and_skips_every_pass() {
    let mut pipeline = CompilerPipeline::default();
    pipeline
        .push_document(parse_like(name("parse-like")))
        .unwrap();
    pipeline.enable_verify_each_for_tests();
    let invocations = Arc::new(AtomicUsize::new(0));
    pipeline
        .push_artifact(CountingClose {
            name: name("first-artifact"),
            invocations: invocations.clone(),
        })
        .unwrap();

    let dirty = Documents::new(vec![DocumentIr::new(
        SourceIr::reached(
            DocumentAddress::StaticEntry {
                origin: String::new(),
                path: "boot/entry.md".to_string(),
            },
            SourceFormatId::new("markdown").unwrap(),
            "",
        ),
        crate::DocTree::parse("# Doc {#root}\n"),
    )]);
    let error = pipeline.run_to_closure(dirty).unwrap_err();
    match error {
        super::super::pipeline::CompilerPipelineError::Segment(
            PassSegmentError::InputVerification { input, source },
        ) => {
            assert_eq!(input, Documents::SHAPE);
            assert!(
                matches!(
                    &*source,
                    super::VerificationError::BlankSourceIdentity { .. }
                ),
                "{source:?}"
            );
        }
        other => panic!("expected the honest input boundary error, got {other:?}"),
    }
    assert_eq!(invocations.load(std::sync::atomic::Ordering::SeqCst), 0);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn default_construction_stays_verifier_off_and_leaves_dirty_batches_untouched() {
    // The same corrupt batch the checked pipeline refuses gathers fine under
    // the production default: R3.3 adds no production gate, byte or error.
    let mut pipeline = CompilerPipeline::default();
    pipeline
        .push_document(parse_like(name("parse-like")))
        .unwrap();
    let documents = pipeline
        .run_documents(vec![
            spec_source("root", "# One {#root}\n"),
            spec_source("root", "# Two {#root}\n"),
        ])
        .unwrap();
    assert_eq!(documents.len(), 2, "the verifier is off by default");

    // The test-only seam is the sole enabling surface.
    let mut enabled = CompilerPipeline::default();
    enabled
        .push_document(parse_like(name("parse-like")))
        .unwrap();
    enabled.enable_verify_each_for_tests();
    assert!(
        enabled
            .run_documents(vec![
                spec_source("root", "# One {#root}\n"),
                spec_source("root", "# Two {#root}\n"),
            ])
            .is_err()
    );
}

// --- the real built-in schedule under verification ----------------------

struct MapSource(BTreeMap<String, String>);

impl crate::SectionSource for MapSource {
    fn section_text(&self, address: &crate::SpecAddress) -> Result<String, String> {
        self.0
            .get(&address.without_pin())
            .cloned()
            .ok_or_else(|| format!("missing {}", address.without_pin()))
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn the_real_builtin_prefix_runs_green_under_verify_each_over_a_diamond() {
    let source = MapSource(BTreeMap::from([
        (
            "spec://org.demo/alpha/boot/entry#root".to_string(),
            "#use spec://org.demo/shared/boot/base#root\n# Alpha {#root}\nalpha body\n".to_string(),
        ),
        (
            "spec://org.demo/shared/boot/base#root".to_string(),
            "#use spec://org.demo/omega/boot/entry#root\n# Shared {#root}\nshared body\n"
                .to_string(),
        ),
        (
            "spec://org.demo/omega/boot/entry#root".to_string(),
            "# Omega {#root}\nomega body\n".to_string(),
        ),
    ]));
    let seed = crate::SpecAddress::parse("spec://org.demo/alpha/boot/entry#root").unwrap();
    let plan = ArtifactPlan::compatibility(seed, StaticCompileMode::Plain);
    let closure = compile_artifact_prefix(plan, &source)
        .expect("every built-in prefix pass output verifies over a legal diamond fixture");
    assert_eq!(closure.contributions.len(), 1);
}

/// The engine and the verifier decide cycles with one component law, so they
/// agree on a graph a DFS-shaped rule reads differently depending on its root.
///
/// `v -> x -> u -> v` is contract-only and admissible; `v -> w -> u -> v` runs
/// through a non-contract node and is not. A three-colour walk from `v` that
/// takes `x` first finishes `u` before it ever examines `w -> u`, so it reports
/// only the admitted loop and compiles — while the verifier, rooted at arena
/// index 0 (the deepest dependency, never the seed), reported the illegal one.
/// Under the shared SCC law both refuse, and the failure names `close`, the
/// pass that produced the graph.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_cycle_a_dfs_walk_would_mask_is_refused_by_engine_and_verifier_alike() {
    let source = MapSource(BTreeMap::from([
        (
            "spec://org.demo/lib/contract/v#r".to_string(),
            "#use spec://org.demo/lib/contract/x#r\n#use spec://org.demo/lib/impl/w#r\n# V {#v}\n"
                .to_string(),
        ),
        (
            "spec://org.demo/lib/contract/x#r".to_string(),
            "#use spec://org.demo/lib/contract/u#r\n# X {#x}\n".to_string(),
        ),
        (
            "spec://org.demo/lib/impl/w#r".to_string(),
            "#use spec://org.demo/lib/contract/u#r\n# W {#w}\n".to_string(),
        ),
        (
            "spec://org.demo/lib/contract/u#r".to_string(),
            "#use spec://org.demo/lib/contract/v#r\n# U {#u}\n".to_string(),
        ),
    ]));
    let seed = crate::SpecAddress::parse("spec://org.demo/lib/contract/v#r").unwrap();
    let plan = ArtifactPlan::compatibility(seed, StaticCompileMode::Plain);
    let error = compile_artifact_prefix(plan, &source)
        .expect_err("the component holding the non-contract node is illegal");
    let rendered = error.to_string();
    assert!(
        rendered.contains("impl/w"),
        "the offending node is named: {rendered}"
    );
}

/// Cross-occurrence fence reversibility, end to end through the real compiler.
///
/// The shape is the one `link::fence_tests` already declares supported: an
/// occurrence whose body leaves a Markdown fence open, and a following
/// occurrence that closes it. The compiler writes its own `vibe:end`/`vibe:begin`
/// framing *between* those two bodies, so a reader that treats a carried fence
/// as hiding everything swallows both control lines and returns one merged
/// block instead of two. `decompile` reads compiler-owned structure through a
/// carried fence, so the emitted lane still splits into its exact blocks.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn an_occurrence_that_leaves_a_fence_open_still_decompiles_into_exact_blocks() {
    let open = "spec://org.demo/lib/open#root";
    let entry = "spec://org.demo/lib/entry#root";
    let source = MapSource(BTreeMap::from([
        (open.to_string(), "# Open {#open}\n```\n".to_string()),
        (
            entry.to_string(),
            format!("#use {open}\n# Entry {{#entry}}\n```\ntail\n"),
        ),
    ]));
    let seed = crate::SpecAddress::parse(entry).unwrap();
    let compiled = crate::pipeline::compile_static(&seed, &source)
        .expect("an occurrence may leave a fence open for the next one");

    let blocks = crate::markers::decompile(&compiled);
    let keys: Vec<&str> = blocks.iter().map(|block| block.key.as_str()).collect();
    assert_eq!(
        keys,
        [open, entry],
        "both blocks survive the carried fence: {compiled}"
    );
    assert!(
        blocks[0].body.contains("# Open {#open}") && blocks[0].body.trim_end().ends_with("```"),
        "the opening block keeps its own body: {:?}",
        blocks[0].body
    );
    assert!(
        blocks[1].body.contains("# Entry {#entry}") && blocks[1].body.contains("tail"),
        "the closing block keeps its own body: {:?}",
        blocks[1].body
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER")]
fn a_legal_contract_only_use_cycle_compiles_green_under_verify_each() {
    let source = MapSource(BTreeMap::from([
        (
            "spec://org.demo/lib/contract/a#r".to_string(),
            "#use spec://org.demo/lib/contract/b#r\n# A {#a}\ncontract a\n".to_string(),
        ),
        (
            "spec://org.demo/lib/contract/b#r".to_string(),
            "#use spec://org.demo/lib/contract/a#r\n# B {#b}\ncontract b\n".to_string(),
        ),
    ]));
    let seed = crate::SpecAddress::parse("spec://org.demo/lib/contract/a#r").unwrap();
    let plan = ArtifactPlan::compatibility(seed, StaticCompileMode::Plain);
    let closure = compile_artifact_prefix(plan, &source)
        .expect("the contract-only forward-declaration cycle stays legal");
    let _ = closure.context();
}
