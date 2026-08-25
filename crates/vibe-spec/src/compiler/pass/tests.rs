use std::convert::Infallible;

use specmark::verifies;

use super::*;
use crate::{DocTree, SpecAddress};

use crate::compiler::ir::{
    ArtifactId, ClosureContribution, ClosureDocument, ClosureNodeId, ContributionMeta,
    DocumentAddress, EmittedIr, SourceFormatId,
};

fn name(value: &str) -> PassName {
    PassName::new(value).unwrap()
}

fn source(text: &str) -> SourceIr {
    SourceIr::new(
        DocumentAddress::Spec(SpecAddress::parse("spec://org.demo/pkg/boot/entry#root").unwrap()),
        SourceFormatId::new("markdown").unwrap(),
        text,
    )
}

fn closure() -> ClosureIr {
    let node = ClosureNodeId(0);
    ClosureIr {
        artifact: ArtifactId::new("static-markdown").unwrap(),
        nodes: vec![ClosureDocument {
            address: DocumentAddress::Spec(
                SpecAddress::parse("spec://org.demo/pkg/boot/entry#root").unwrap(),
            ),
            origin: "org.demo/pkg".to_string(),
            tree: DocTree::parse("BODY\n"),
        }],
        edges: Vec::new(),
        contributions: vec![ClosureContribution::Normal {
            meta: ContributionMeta {
                origin: "org.demo/pkg".to_string(),
                path: "vibedeps/org.demo.pkg/1.0.0/boot/entry.md".to_string(),
            },
            seed: node,
            emission_order: vec![node],
        }],
        renames: Vec::new(),
        pending_sources: None,
    }
}

struct ParseForTest {
    name: PassName,
}

impl Pass for ParseForTest {
    type Input = SourceIr;
    type Output = DocumentIr;
    type Error = Infallible;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, input: SourceIr) -> Result<DocumentIr, Infallible> {
        let tree = DocTree::parse(input.text());
        Ok(DocumentIr::new(input, tree))
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn heterogeneous_segment_keeps_typed_boundaries_behind_erasure() {
    let mut segment = PassSegment::default();
    segment
        .push(ParseForTest {
            name: name("parse-test"),
        })
        .unwrap();
    segment
        .push(IdentityPass::<DocumentIr>::new(name("document-identity")))
        .unwrap();

    let descriptors: Vec<PassDescriptor> = segment.descriptors().collect();
    assert_eq!(descriptors[0].input, SourceIr::SHAPE);
    assert_eq!(descriptors[0].output, DocumentIr::SHAPE);
    assert_eq!(descriptors[1].input, DocumentIr::SHAPE);

    let AnyIr::Document(document) = segment
        .run(AnyIr::Source(source("# Entry {#root}\nBODY\n")))
        .unwrap()
    else {
        panic!("parse plus identity must return one document")
    };
    assert_eq!(document.source().text(), "# Entry {#root}\nBODY\n");
    assert_eq!(
        document.tree().text(document.tree().root()),
        "# Entry {#root}\nBODY"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn identity_preserves_artifact_structure_and_non_utf8_emitted_bytes() {
    let expected = closure();
    let mut closure_segment = PassSegment::default();
    closure_segment
        .push(IdentityPass::<ClosureIr>::new(name("closure-identity")))
        .unwrap();
    let AnyIr::Closure(actual) = closure_segment
        .run(AnyIr::Closure(expected.clone()))
        .unwrap()
    else {
        panic!("closure identity changed the IR level")
    };
    assert_eq!(actual, expected);

    let expected = EmittedIr {
        artifact: ArtifactId::new("opaque-backend").unwrap(),
        bytes: vec![0, 0xff, b'\n'],
    };
    let mut emitted_segment = PassSegment::default();
    emitted_segment
        .push(IdentityPass::<EmittedIr>::new(name("emitted-identity")))
        .unwrap();
    let AnyIr::Emitted(actual) = emitted_segment
        .run(AnyIr::Emitted(expected.clone()))
        .unwrap()
    else {
        panic!("emitted identity changed the IR level")
    };
    assert_eq!(actual, expected);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn segment_rejects_duplicate_names_incompatible_chains_and_wrong_inputs() {
    let mut duplicate = PassSegment::default();
    duplicate
        .push(IdentityPass::<SourceIr>::new(name("same")))
        .unwrap();
    let error = duplicate
        .push(IdentityPass::<SourceIr>::new(name("same")))
        .unwrap_err();
    assert!(matches!(error, PassSegmentError::DuplicateName { .. }));

    let mut broken = PassSegment::default();
    broken
        .push(IdentityPass::<SourceIr>::new(name("source")))
        .unwrap();
    let error = broken
        .push(IdentityPass::<ClosureIr>::new(name("closure")))
        .unwrap_err();
    assert!(matches!(error, PassSegmentError::BrokenChain { .. }));

    let mut wrong = PassSegment::default();
    wrong
        .push(IdentityPass::<SourceIr>::new(name("source-only")))
        .unwrap();
    let error = wrong.run(AnyIr::Closure(closure())).unwrap_err();
    assert!(matches!(
        error,
        PassSegmentError::WrongInput {
            ref pass,
            expected: IrShape {
                level: IrLevel::Source,
                cardinality: IrCardinality::Document,
            },
            actual: IrShape {
                level: IrLevel::Closure,
                cardinality: IrCardinality::Artifact,
            },
        } if pass.as_str() == "source-only"
    ));
}

struct LyingOutputPass;

impl DynPass for LyingOutputPass {
    fn descriptor(&self) -> PassDescriptor {
        PassDescriptor {
            name: name("lying-output"),
            input: SourceIr::SHAPE,
            output: SourceIr::SHAPE,
        }
    }

    fn run_erased(&self, _input: AnyIr) -> Result<AnyIr, PassSegmentError> {
        Ok(AnyIr::Closure(closure()))
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn erased_output_shape_is_checked_even_after_a_lying_adapter() {
    let mut segment = PassSegment::default();
    segment.push_dyn(Box::new(LyingOutputPass)).unwrap();
    let error = segment
        .run(AnyIr::Source(source("# Entry {#root}\n")))
        .unwrap_err();
    assert!(matches!(
        error,
        PassSegmentError::WrongOutput {
            ref pass,
            expected: IrShape {
                level: IrLevel::Source,
                cardinality: IrCardinality::Document,
            },
            actual: IrShape {
                level: IrLevel::Closure,
                cardinality: IrCardinality::Artifact,
            },
        } if pass.as_str() == "lying-output"
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-LEVELS")]
fn documents_is_an_artifact_batch_at_the_document_level() {
    assert_eq!(
        Documents::SHAPE,
        IrShape::new(IrLevel::Document, IrCardinality::Artifact)
    );
    assert_ne!(Documents::SHAPE, DocumentIr::SHAPE);
    assert!(Documents::new(Vec::new()).is_empty());
}

#[test]
fn blank_pass_name_is_rejected() {
    assert_eq!(PassName::new(""), Err(PassNameError));
    assert_eq!(PassName::new(" \t"), Err(PassNameError));
}
