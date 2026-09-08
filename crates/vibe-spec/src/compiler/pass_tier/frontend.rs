//! Registered source-format identities at the compiler's parse seat.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#PASS-FRONTEND");

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use crate::compiler::ir::{DocumentAddress, DocumentIr, SourceFormatId, SourceIr};
use crate::compiler::pass::{
    AnyIr, DynPass, IrPayload, Pass, PassDescriptor, PassName, PassSegment, PassSegmentError,
};
use crate::compiler::pipeline::{CompilerPipeline, CompilerPipelineError};
use crate::compiler::trace::CompileTraceSink;
use crate::compiler::verify::IrVerifier;

use super::catalog::{CatalogPass, PassCatalogError};
use super::native::{NativePass, NativePassAdmission};
use super::plan::PassEntry;

pub(crate) trait FormatAdmission<E>:
    Fn(&std::path::Path, &str, &str) -> Result<(), E>
{
}
impl<E, T> FormatAdmission<E> for T where T: Fn(&std::path::Path, &str, &str) -> Result<(), E> {}

pub(crate) struct AdmittedSource<'source, Source, Admit> {
    source: &'source Source,
    active_formats: &'source [String],
    admit: &'source Admit,
}

impl<'source, Source, Admit> AdmittedSource<'source, Source, Admit> {
    pub(crate) const fn new(
        source: &'source Source,
        active_formats: &'source [String],
        admit: &'source Admit,
    ) -> Self {
        Self {
            source,
            active_formats,
            admit,
        }
    }
}

impl<Source: crate::SectionSource, Admit> AdmittedSource<'_, Source, Admit> {
    pub(crate) fn load<E>(
        &self,
        address: &crate::SpecAddress,
    ) -> Result<Result<crate::embed::ResolvedSource, String>, E>
    where
        Admit: FormatAdmission<E>,
    {
        let metadata = match self.source.source_metadata(address, self.active_formats) {
            Ok(metadata) => metadata,
            Err(error) => return Ok(Err(error)),
        };
        if metadata.format() != "markdown" {
            (self.admit)(
                metadata.origin(),
                metadata.format(),
                metadata.physical_stem(),
            )?;
        }
        Ok(self.source.read_resolved_source(address, metadata))
    }

    pub(crate) fn expand_pattern(
        &self,
        address: &crate::SpecAddress,
    ) -> Result<Vec<crate::SpecAddress>, String> {
        self.source.expand_pattern(address)
    }
}

pub(crate) fn physical_stem(source: &SourceIr) -> String {
    match source.address() {
        DocumentAddress::Spec(address) => address
            .doc_path
            .rsplit('/')
            .next()
            .unwrap_or(&address.doc_path)
            .to_owned(),
        DocumentAddress::StaticEntry { path, .. } => std::path::Path::new(path)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or(path)
            .to_owned(),
    }
}

const BUILTIN_FORMATS: [&str; 3] = ["markdown", "md", "xml"];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FrontendCatalog {
    formats: BTreeMap<String, CatalogPass>,
}

#[derive(Default)]
pub(crate) struct FrontendSeats<'invoke> {
    invoker: Option<&'invoke dyn crate::compiler::transform::native_manager::CompilerNativeInvoker>,
    formats: BTreeMap<String, FrontendSeat>,
    admitted: Mutex<BTreeSet<vibe_core::manifest::ExtensionKey>>,
}

enum FrontendSeat {
    Native {
        admission: NativePassAdmission,
        name: PassName,
    },
    #[cfg(test)]
    Installed(Box<TestFrontendFactory>),
}

#[cfg(test)]
type TestFrontendFactory =
    dyn Fn(&str) -> Result<PassSegment<'static>, PassSegmentError> + Send + Sync;

impl<'invoke> FrontendSeats<'invoke> {
    pub(crate) fn native(
        catalog: &FrontendCatalog,
        invoker: Option<
            &'invoke dyn crate::compiler::transform::native_manager::CompilerNativeInvoker,
        >,
    ) -> Result<Self, FrontendAdmissionError> {
        let formats = catalog
            .formats
            .iter()
            .map(|(format, binding)| {
                let name = binding.descriptor().name.clone();
                Ok((
                    format.clone(),
                    FrontendSeat::Native {
                        admission: NativePassAdmission::from_entry(binding.entry()).map_err(
                            |error| FrontendAdmissionError {
                                pass: Some(name.clone()),
                                reason: error.to_string(),
                            },
                        )?,
                        name,
                    },
                ))
            })
            .collect::<Result<_, FrontendAdmissionError>>()?;
        Ok(Self {
            invoker,
            formats,
            admitted: Mutex::new(BTreeSet::new()),
        })
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
                FrontendSeat::Native { name, .. } => {
                    let parser = descriptor_segment(name)?;
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

    pub(crate) fn admit(&self, format: &str) -> Result<(), FrontendAdmissionError> {
        let Some(seat) = self.formats.get(format) else {
            return Err(FrontendAdmissionError {
                pass: None,
                reason: format!("custom source format `{format}` has no selected frontend seat"),
            });
        };
        let (admission, name) = match seat {
            FrontendSeat::Native { admission, name } => (admission, name),
            #[cfg(test)]
            FrontendSeat::Installed(_) => return Ok(()),
        };
        let admitted = self.admitted.lock().map_err(|_| FrontendAdmissionError {
            pass: Some(name.clone()),
            reason: "frontend admission state is unavailable".to_owned(),
        })?;
        if admitted.contains(admission.key()) {
            return Ok(());
        }
        drop(admitted);
        let invoker = self.invoker.ok_or_else(|| FrontendAdmissionError {
            pass: Some(name.clone()),
            reason: "frontend provider pre-admission requires a compiler-native invoker".to_owned(),
        })?;
        admission
            .admit(invoker)
            .map_err(|error| FrontendAdmissionError {
                pass: Some(name.clone()),
                reason: error.to_string(),
            })?;
        self.admitted
            .lock()
            .map_err(|_| FrontendAdmissionError {
                pass: Some(name.clone()),
                reason: "frontend admission state is unavailable".to_owned(),
            })?
            .insert(admission.key().clone());
        Ok(())
    }

    pub(crate) fn run(
        &self,
        pipeline: &CompilerPipeline<'_>,
        source: SourceIr,
        physical_stem: &str,
        trace: Option<&dyn CompileTraceSink>,
    ) -> Result<DocumentIr, CompilerPipelineError> {
        let format = source.format().as_str();
        let Some(seat) = self.formats.get(format) else {
            if format == "markdown" {
                return pipeline.run_document_traced(source, trace);
            }
            return Err(PassSegmentError::PassFailed {
                pass: PassName::new(format!("frontend:{format}"))
                    .expect("a source format is nonblank"),
                source: Box::new(FrontendNotAdmitted),
            }
            .into());
        };
        match seat {
            FrontendSeat::Native { admission, name } => {
                if !self
                    .admitted
                    .lock()
                    .is_ok_and(|admitted| admitted.contains(admission.key()))
                {
                    return Err(PassSegmentError::PassFailed {
                        pass: name.clone(),
                        source: Box::new(FrontendNotAdmitted),
                    }
                    .into());
                }
                let invoker = self.invoker.expect("admission requires an invoker");
                let mut parser = PassSegment::default();
                parser.push(
                    NativePass::<SourceIr, DocumentIr>::from_admission(
                        admission.clone(),
                        invoker,
                        name.clone(),
                    )
                    .with_frontend_physical_stem(physical_stem),
                )?;
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

fn descriptor_segment(name: &PassName) -> Result<PassSegment<'static>, PassSegmentError> {
    let mut parser = PassSegment::default();
    parser.push(FrontendDescriptorPass(name.clone()))?;
    Ok(parser)
}

struct FrontendDescriptorPass(PassName);

impl Pass for FrontendDescriptorPass {
    type Input = SourceIr;
    type Output = DocumentIr;
    type Error = FrontendProviderDeferred;

    fn name(&self) -> &PassName {
        &self.0
    }

    fn run(&self, _input: SourceIr) -> Result<DocumentIr, Self::Error> {
        Err(FrontendProviderDeferred)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("frontend descriptor cannot execute")]
struct FrontendProviderDeferred;

#[derive(Debug, thiserror::Error)]
#[error("custom frontend reached execution without metadata pre-admission")]
struct FrontendNotAdmitted;

#[derive(Debug)]
pub(crate) struct FrontendAdmissionError {
    pub(crate) pass: Option<PassName>,
    pub(crate) reason: String,
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
