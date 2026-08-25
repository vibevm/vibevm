//! The single declared compiler schedule and its cardinality barrier.
//!
//! Source/document passes run once for every addressed document. Their owned
//! outputs cross [`GatherDocuments`]—a scheduler operation, never a named
//! compiler pass—then closure/lane/emitted passes run once for the artifact.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR");

use std::collections::BTreeSet;

use super::ir::{
    ClosureIr, DocumentIr, Documents, EmittedIr, IrCardinality, IrLevel, IrShape, SourceIr,
};
use super::pass::{
    AnyIr, IrPayload, Pass, PassDescriptor, PassName, PassSegment, PassSegmentError,
};

const SOURCE_DOCUMENT: IrShape = IrShape::new(IrLevel::Source, IrCardinality::Document);
const DOCUMENT_DOCUMENT: IrShape = IrShape::new(IrLevel::Document, IrCardinality::Document);
const DOCUMENT_ARTIFACT: IrShape = IrShape::new(IrLevel::Document, IrCardinality::Artifact);
const CLOSURE_ARTIFACT: IrShape = IrShape::new(IrLevel::Closure, IrCardinality::Artifact);
const EMITTED_ARTIFACT: IrShape = IrShape::new(IrLevel::Emitted, IrCardinality::Artifact);

/// The typed cardinality boundary between per-document and per-artifact work.
///
/// It has no name and cannot be targeted by pass ordering: gathering values is
/// execution mechanics, not a sixth IR level or an extension point.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct GatherDocuments;

impl GatherDocuments {
    pub(crate) fn run(self, documents: Vec<DocumentIr>) -> Documents {
        Documents::new(documents)
    }
}

/// One item of the declared schedule, including its non-pass barrier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScheduleItem {
    Pass(PassDescriptor),
    GatherDocuments,
}

/// One complete schedule: document segment → gather → artifact segment.
#[derive(Default)]
pub(crate) struct CompilerPipeline {
    document: PassSegment,
    gather: GatherDocuments,
    artifact: PassSegment,
    pass_names: BTreeSet<PassName>,
}

impl CompilerPipeline {
    pub(crate) fn push_document<P: Pass>(&mut self, pass: P) -> Result<(), CompilerPipelineError> {
        let name = pass.name().clone();
        Self::ensure_segment_cardinality::<P>("document", IrCardinality::Document, &name)?;
        self.ensure_name_free(&name)?;
        self.document.push(pass)?;
        self.pass_names.insert(name);
        Ok(())
    }

    pub(crate) fn push_artifact<P: Pass>(&mut self, pass: P) -> Result<(), CompilerPipelineError> {
        let name = pass.name().clone();
        Self::ensure_segment_cardinality::<P>("artifact", IrCardinality::Artifact, &name)?;
        self.ensure_name_free(&name)?;
        self.artifact.push(pass)?;
        self.pass_names.insert(name);
        Ok(())
    }

    fn ensure_name_free(&self, name: &PassName) -> Result<(), CompilerPipelineError> {
        if self.pass_names.contains(name) {
            Err(CompilerPipelineError::DuplicateName { pass: name.clone() })
        } else {
            Ok(())
        }
    }

    fn ensure_segment_cardinality<P: Pass>(
        segment: &'static str,
        expected: IrCardinality,
        pass: &PassName,
    ) -> Result<(), CompilerPipelineError> {
        let input = P::Input::SHAPE;
        let output = P::Output::SHAPE;
        if input.cardinality == expected && output.cardinality == expected {
            Ok(())
        } else {
            Err(CompilerPipelineError::WrongSegmentCardinality {
                segment,
                pass: pass.clone(),
                expected,
                input,
                output,
            })
        }
    }

    /// The one declared schedule in execution order.
    pub(crate) fn schedule(&self) -> Vec<ScheduleItem> {
        self.document
            .descriptors()
            .map(ScheduleItem::Pass)
            .chain(std::iter::once(ScheduleItem::GatherDocuments))
            .chain(self.artifact.descriptors().map(ScheduleItem::Pass))
            .collect()
    }

    /// Run the declared schedule with the accepted cardinality law.
    pub(crate) fn run(&self, sources: Vec<SourceIr>) -> Result<EmittedIr, CompilerPipelineError> {
        self.validate_boundaries()?;

        let documents = self.run_documents_unchecked(sources)?;
        let output = self.artifact.run(AnyIr::Documents(documents))?;
        match output {
            AnyIr::Emitted(emitted) => Ok(emitted),
            other => Err(CompilerPipelineError::UnexpectedCarrier {
                boundary: "artifact segment output",
                expected: EMITTED_ARTIFACT,
                actual: other.shape(),
            }),
        }
    }

    /// Run the declared per-document schedule through its gather boundary.
    ///
    /// R3 migrates the built-ins one at a time. This is the executable seam for
    /// that progression: while only `parse` has moved, its `SourceIr ->
    /// DocumentIr` result still crosses the one explicit [`Documents`] gather
    /// before the legacy artifact phases continue. Once `close` moves, the full
    /// [`CompilerPipeline::run`] path takes over without changing cardinality.
    pub(crate) fn run_documents(
        &self,
        sources: Vec<SourceIr>,
    ) -> Result<Documents, CompilerPipelineError> {
        self.validate_document_boundaries()?;
        self.run_documents_unchecked(sources)
    }

    /// Run the per-document segment for one newly discovered addressed source.
    ///
    /// Close discovers a finite worklist from parsed directives, so the full
    /// vector does not exist before the first parse invocation. Every call
    /// still traverses this manager's declared document segment; gathering is
    /// explicit and happens exactly once after discovery.
    pub(crate) fn run_document(
        &self,
        source: SourceIr,
    ) -> Result<DocumentIr, CompilerPipelineError> {
        self.validate_document_boundaries()?;
        self.run_document_unchecked(source)
    }

    /// Cross the one scheduler-owned document/artifact cardinality boundary.
    pub(crate) fn gather_documents(&self, documents: Vec<DocumentIr>) -> Documents {
        self.gather.run(documents)
    }

    /// Run the declared artifact prefix through the named `close` lowering.
    pub(crate) fn run_to_closure(
        &self,
        documents: Documents,
    ) -> Result<ClosureIr, CompilerPipelineError> {
        self.expect_boundary(
            "artifact segment input",
            DOCUMENT_ARTIFACT,
            self.artifact.first_input(),
        )?;
        self.expect_boundary(
            "artifact close output",
            CLOSURE_ARTIFACT,
            self.artifact.last_output(),
        )?;
        let output = self.artifact.run(AnyIr::Documents(documents))?;
        match output {
            AnyIr::Closure(closure) => Ok(closure),
            other => Err(CompilerPipelineError::UnexpectedCarrier {
                boundary: "artifact close output",
                expected: CLOSURE_ARTIFACT,
                actual: other.shape(),
            }),
        }
    }

    fn run_documents_unchecked(
        &self,
        sources: Vec<SourceIr>,
    ) -> Result<Documents, CompilerPipelineError> {
        let mut documents = Vec::with_capacity(sources.len());
        for source in sources {
            documents.push(self.run_document_unchecked(source)?);
        }

        Ok(self.gather.run(documents))
    }

    fn run_document_unchecked(
        &self,
        source: SourceIr,
    ) -> Result<DocumentIr, CompilerPipelineError> {
        let output = self.document.run(AnyIr::Source(source))?;
        match output {
            AnyIr::Document(document) => Ok(document),
            other => Err(CompilerPipelineError::UnexpectedCarrier {
                boundary: "document segment output",
                expected: DOCUMENT_DOCUMENT,
                actual: other.shape(),
            }),
        }
    }

    fn validate_boundaries(&self) -> Result<(), CompilerPipelineError> {
        self.validate_document_boundaries()?;
        self.expect_boundary(
            "artifact segment input",
            DOCUMENT_ARTIFACT,
            self.artifact.first_input(),
        )?;
        self.expect_boundary(
            "artifact segment output",
            EMITTED_ARTIFACT,
            self.artifact.last_output(),
        )
    }

    fn validate_document_boundaries(&self) -> Result<(), CompilerPipelineError> {
        self.expect_boundary(
            "document segment input",
            SOURCE_DOCUMENT,
            self.document.first_input(),
        )?;
        self.expect_boundary(
            "document segment output",
            DOCUMENT_DOCUMENT,
            self.document.last_output(),
        )
    }

    fn expect_boundary(
        &self,
        boundary: &'static str,
        expected: IrShape,
        actual: Option<IrShape>,
    ) -> Result<(), CompilerPipelineError> {
        if actual == Some(expected) {
            Ok(())
        } else {
            Err(CompilerPipelineError::ScheduleBoundary {
                boundary,
                expected,
                actual,
            })
        }
    }
}

/// Why the declared schedule could not be built or executed.
#[derive(Debug, thiserror::Error)]
pub(crate) enum CompilerPipelineError {
    #[error("compiler schedule contains duplicate pass name `{pass}`")]
    DuplicateName { pass: PassName },
    #[error(
        "compiler pass `{pass}` cannot enter the {segment} segment: both sides must have {expected:?} cardinality, got {input:?} -> {output:?}"
    )]
    WrongSegmentCardinality {
        segment: &'static str,
        pass: PassName,
        expected: IrCardinality,
        input: IrShape,
        output: IrShape,
    },
    #[error(transparent)]
    Segment(#[from] PassSegmentError),
    #[error("{boundary} must be {expected:?}, got {actual:?}")]
    ScheduleBoundary {
        boundary: &'static str,
        expected: IrShape,
        actual: Option<IrShape>,
    },
    #[error("{boundary} must be {expected:?}, got {actual:?}")]
    UnexpectedCarrier {
        boundary: &'static str,
        expected: IrShape,
        actual: IrShape,
    },
}

#[cfg(test)]
mod tests;
