//! Registered source-format identities at the compiler's parse seat.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#PASS-FRONTEND");

use std::collections::BTreeMap;

use crate::compiler::ir::{DocumentIr, SourceFormatId, SourceIr};
use crate::compiler::pass::{
    AnyIr, DynPass, IrPayload, Pass, PassDescriptor, PassName, PassSegment, PassSegmentError,
};
use crate::compiler::pipeline::{CompilerPipeline, CompilerPipelineError};
use crate::compiler::trace::CompileTraceSink;
use crate::compiler::verify::IrVerifier;

use super::catalog::{CatalogPass, PassCatalogError};
use super::plan::PassEntry;

const BUILTIN_FORMATS: [&str; 3] = ["markdown", "md", "xml"];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FrontendCatalog {
    formats: BTreeMap<String, CatalogPass>,
}

#[derive(Default)]
pub(crate) struct FrontendSeats {
    formats: BTreeMap<String, FrontendSeat>,
}

enum FrontendSeat {
    Deferred {
        format: String,
        name: PassName,
    },
    #[cfg(test)]
    Installed(Box<TestFrontendFactory>),
}

#[cfg(test)]
type TestFrontendFactory =
    dyn Fn(&str) -> Result<PassSegment<'static>, PassSegmentError> + Send + Sync;

impl FrontendSeats {
    pub(crate) fn deferred(catalog: &FrontendCatalog) -> Self {
        Self {
            formats: catalog
                .formats
                .iter()
                .map(|(format, binding)| {
                    (
                        format.clone(),
                        FrontendSeat::Deferred {
                            format: format.clone(),
                            name: binding.descriptor().name.clone(),
                        },
                    )
                })
                .collect(),
        }
    }

    pub(crate) fn physical_formats(&self) -> impl Iterator<Item = &str> {
        self.formats.keys().map(String::as_str)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.formats.is_empty()
    }

    pub(crate) fn validate(
        &self,
        pipeline: &CompilerPipeline<'_>,
    ) -> Result<(), CompilerPipelineError> {
        for seat in self.formats.values() {
            match seat {
                FrontendSeat::Deferred { format, name } => {
                    let parser = deferred_segment(format, name, "")?;
                    pipeline.validate_frontend_parser(&parser)?;
                }
                #[cfg(test)]
                FrontendSeat::Installed(factory) => {
                    pipeline.validate_frontend_parser(&factory("")?)?
                }
            }
        }
        Ok(())
    }

    pub(crate) fn run(
        &self,
        pipeline: &CompilerPipeline<'_>,
        source: SourceIr,
        physical_stem: &str,
        trace: Option<&dyn CompileTraceSink>,
    ) -> Result<DocumentIr, CompilerPipelineError> {
        let Some(seat) = self.formats.get(source.format().as_str()) else {
            return pipeline.run_document_traced(source, trace);
        };
        match seat {
            FrontendSeat::Deferred { format, name } => {
                let parser = deferred_segment(format, name, physical_stem)?;
                pipeline.run_document_with_parser(source, &parser, trace)
            }
            #[cfg(test)]
            FrontendSeat::Installed(factory) => {
                let parser = factory(physical_stem)?;
                pipeline.run_document_with_parser(source, &parser, trace)
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn replace_for_test<F, P>(&mut self, format: &str, factory: F)
    where
        F: Fn(&str) -> P + Send + Sync + 'static,
        P: Pass<Input = SourceIr, Output = DocumentIr> + 'static,
    {
        self.formats.insert(
            format.to_owned(),
            FrontendSeat::Installed(Box::new(move |stem| {
                let mut seat = PassSegment::default();
                seat.push(factory(stem))?;
                Ok(seat)
            })),
        );
    }
}

fn deferred_segment(
    format: &str,
    name: &PassName,
    physical_stem: &str,
) -> Result<PassSegment<'static>, PassSegmentError> {
    let mut parser = PassSegment::default();
    parser.push(DeferredFrontendPass {
        format: format.to_owned(),
        physical_stem: physical_stem.to_owned(),
        name: name.clone(),
    })?;
    Ok(parser)
}

struct DeferredFrontendPass {
    format: String,
    physical_stem: String,
    name: PassName,
}

impl Pass for DeferredFrontendPass {
    type Input = SourceIr;
    type Output = DocumentIr;
    type Error = FrontendProviderDeferred;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, _input: SourceIr) -> Result<DocumentIr, Self::Error> {
        Err(FrontendProviderDeferred {
            format: self.format.clone(),
            physical_stem: self.physical_stem.clone(),
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error(
    "frontend format `{format}` at physical stem `{physical_stem}` awaits R6.5-B2 provider pre-admission"
)]
struct FrontendProviderDeferred {
    format: String,
    physical_stem: String,
}

#[derive(Default)]
pub(crate) struct DocumentPipeline<'pass> {
    source: PassSegment<'pass>,
    parser: PassSegment<'pass>,
    document: PassSegment<'pass>,
}

impl<'pass> DocumentPipeline<'pass> {
    pub(crate) fn push<P: Pass + 'pass>(&mut self, pass: P) -> Result<(), CompilerPipelineError> {
        let next = P::Input::SHAPE;
        if let Some(previous) = self.last_descriptor()
            && previous.output != next
        {
            return Err(PassSegmentError::BrokenChain {
                previous: previous.name,
                previous_output: previous.output,
                next: pass.name().clone(),
                next_input: next,
            }
            .into());
        }
        match (next, P::Output::SHAPE) {
            (SourceIr::SHAPE, SourceIr::SHAPE) => self.source.push(pass)?,
            (SourceIr::SHAPE, DocumentIr::SHAPE) => self.parser.push(pass)?,
            (DocumentIr::SHAPE, DocumentIr::SHAPE) => self.document.push(pass)?,
            _ => {
                return Err(CompilerPipelineError::ScheduleBoundary {
                    boundary: "document parse seat",
                    expected: DocumentIr::SHAPE,
                    actual: Some(P::Output::SHAPE),
                });
            }
        }
        Ok(())
    }

    pub(crate) fn descriptors(&self) -> impl Iterator<Item = PassDescriptor> + '_ {
        self.source
            .descriptors()
            .chain(self.parser.descriptors())
            .chain(self.document.descriptors())
    }

    fn last_descriptor(&self) -> Option<PassDescriptor> {
        self.descriptors().last()
    }

    pub(crate) fn run(
        &self,
        source: SourceIr,
        parser: Option<&PassSegment<'_>>,
        verifier: Option<IrVerifier>,
        trace: Option<&dyn CompileTraceSink>,
    ) -> Result<DocumentIr, CompilerPipelineError> {
        let parser = parser.unwrap_or(&self.parser);
        self.validate(parser)?;
        let source = self
            .source
            .run_traced(AnyIr::Source(source), verifier, trace)?;
        let document = parser.run_traced(source, verifier, trace)?;
        let output = self.document.run_traced(document, verifier, trace)?;
        match output {
            AnyIr::Document(document) => Ok(document),
            other => Err(CompilerPipelineError::UnexpectedCarrier {
                boundary: "document segment output",
                expected: DocumentIr::SHAPE,
                actual: other.shape(),
            }),
        }
    }

    pub(crate) fn validate(&self, parser: &PassSegment<'_>) -> Result<(), CompilerPipelineError> {
        super::super::pipeline::validate_segment_endpoints(
            super::super::pipeline::DOCUMENT_ENDPOINTS,
            parser.first_input(),
            parser.last_output(),
        )?;
        super::super::pipeline::validate_segment_endpoints(
            super::super::pipeline::DOCUMENT_ENDPOINTS,
            self.source.first_input().or_else(|| parser.first_input()),
            self.document.last_output().or_else(|| parser.last_output()),
        )
    }

    pub(crate) fn validate_default(&self) -> Result<(), CompilerPipelineError> {
        self.validate(&self.parser)
    }

    pub(crate) fn take_passes(&mut self) -> Vec<Box<dyn DynPass + 'pass>> {
        std::mem::take(&mut self.source)
            .into_passes()
            .into_iter()
            .chain(std::mem::take(&mut self.parser).into_passes())
            .chain(std::mem::take(&mut self.document).into_passes())
            .collect()
    }

    pub(crate) fn from_passes(
        passes: Vec<Box<dyn DynPass + 'pass>>,
    ) -> Result<Self, PassSegmentError> {
        let mut source = Vec::new();
        let mut parser = Vec::new();
        let mut document = Vec::new();
        for pass in passes {
            let shape = pass.descriptor();
            match (shape.input, shape.output) {
                (input, output) if input == SourceIr::SHAPE && output == SourceIr::SHAPE => {
                    source.push(pass);
                }
                (input, output) if input == SourceIr::SHAPE && output == DocumentIr::SHAPE => {
                    parser.push(pass);
                }
                (input, output) if input == DocumentIr::SHAPE && output == DocumentIr::SHAPE => {
                    document.push(pass);
                }
                _ => unreachable!("validated document chain has a closed parse seat"),
            }
        }
        Ok(Self {
            source: PassSegment::from_passes(source)?,
            parser: PassSegment::from_passes(parser)?,
            document: PassSegment::from_passes(document)?,
        })
    }
}

impl FrontendCatalog {
    pub(super) fn register(&mut self, entry: &PassEntry) -> Result<(), PassCatalogError> {
        let formats = entry
            .declaration()
            .formats
            .as_ref()
            .filter(|formats| !formats.is_empty())
            .ok_or_else(|| PassCatalogError::MissingFormats {
                key: entry.key().clone(),
            })?;
        let binding = CatalogPass::new(entry, SourceIr::SHAPE, DocumentIr::SHAPE)?;
        for authored in formats {
            let format = SourceFormatId::new(authored.clone()).map_err(|_| {
                PassCatalogError::InvalidFormat {
                    key: entry.key().clone(),
                    format: bounded(authored),
                }
            })?;
            if !is_physical_extension(format.as_str()) {
                return Err(PassCatalogError::InvalidFormat {
                    key: entry.key().clone(),
                    format: bounded(authored),
                });
            }
            if BUILTIN_FORMATS.contains(&format.as_str()) {
                return Err(PassCatalogError::BuiltinFormat {
                    key: entry.key().clone(),
                    format: format.as_str().to_owned(),
                });
            }
            if let Some(first) = self.formats.get(format.as_str()) {
                return Err(PassCatalogError::DuplicateFormat {
                    format: format.as_str().to_owned(),
                    first: first.key().clone(),
                    second: entry.key().clone(),
                });
            }
            self.formats
                .insert(format.as_str().to_owned(), binding.clone());
        }
        Ok(())
    }

    pub(crate) fn get(&self, format: &str) -> Option<&CatalogPass> {
        self.formats.get(format)
    }

    pub(crate) fn len(&self) -> usize {
        self.formats.len()
    }

    pub(crate) fn physical_formats(&self) -> impl Iterator<Item = &str> {
        self.formats.keys().map(String::as_str)
    }
}

fn is_physical_extension(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn bounded(value: &str) -> String {
    let mut characters = value.chars();
    let mut result = characters.by_ref().take(80).collect::<String>();
    if characters.next().is_some() {
        result.push('…');
    }
    result
}
