//! Error attribution for the built-in schedule (T6b): the one place a
//! pass-manager failure becomes an [`ArtifactCompileError`].
//!
//! ONE transform-first classifier serves the document, prefix/lane and
//! emitted paths: a `PassFailed` box is downcast to the private transform
//! error before any per-name or generic string arm runs, and a
//! `VerificationFailed` is attributed through the schedule-owned exact
//! transform pass-name set, never by parsing the rendered name. The
//! historical per-pass ladders and their panics for impossible cross-type
//! faults are unchanged below those arms. Converting a [`TransformError`]
//! into the public artifact family happens HERE and in the driver — the
//! transform cell itself names no builtin type.

use crate::use_graph::UseGraphError;

use super::super::assemble::{ASSEMBLE_PASS_NAME, AssemblePassError};
use super::super::close::CLOSE_PASS_NAME;
use super::super::embed::{EMBED_PASS_NAME, EmbedPassError};
use super::super::ir::ArtifactPlan;
use super::super::link::{LINK_PASS_NAME, LinkPassError};
use super::super::merge::{MERGE_PASS_NAME, MergePassError};
use super::super::pass::{PassName, PassSegmentError};
use super::super::pipeline::CompilerPipelineError;
use super::super::transform::fault::TransformError;
use super::super::worklist::ErrorOwners;
use super::{ArtifactCompileError, BuiltinSchedule, transform_public};

impl BuiltinSchedule {
    /// Whether one pass name belongs to this schedule's transform wrappers.
    ///
    /// The empty plan owns an empty set — no allocation, no match.
    fn is_transform_pass(&self, pass: &PassName) -> bool {
        self.transform_names.iter().any(|name| name == pass)
    }

    /// The ONE shared transform-first classifier: extract the typed
    /// transform fault from a pipeline failure, handing the error back
    /// untouched when it is not one. A `PassFailed` box downcasts to the
    /// private error; a `VerificationFailed` classifies through the exact
    /// schedule-owned name set.
    pub(super) fn transform_fault(
        &self,
        error: CompilerPipelineError,
    ) -> Result<TransformError, CompilerPipelineError> {
        match error {
            CompilerPipelineError::Segment(PassSegmentError::PassFailed { pass, source }) => {
                match source.downcast::<TransformError>() {
                    Ok(boxed) => Ok(*boxed),
                    Err(source) => Err(CompilerPipelineError::Segment(
                        PassSegmentError::PassFailed { pass, source },
                    )),
                }
            }
            CompilerPipelineError::Segment(PassSegmentError::VerificationFailed {
                pass,
                source,
                ..
            }) if self.is_transform_pass(&pass) => {
                Ok(TransformError::Verification { pass, source })
            }
            error => Err(error),
        }
    }

    /// Attribute one document-segment failure crossed through fallible
    /// discovery: a transform fault keeps its typed identity, everything else
    /// keeps the manager spelling it always had.
    pub(super) fn document_error(&self, error: CompilerPipelineError) -> ArtifactCompileError {
        match self.transform_fault(error) {
            Ok(transform) => transform_public(transform),
            Err(CompilerPipelineError::Segment(PassSegmentError::PassFailed { pass, source })) => {
                ArtifactCompileError::Pass {
                    pass: pass.to_string(),
                    reason: source.to_string(),
                }
            }
            Err(other) => ArtifactCompileError::Manager {
                reason: other.to_string(),
            },
        }
    }

    /// The whole-artifact ladder for the closure/lane prefix runs: transform
    /// faults first, then the historical per-pass downcasts with their
    /// impossible-cross-type panics unchanged.
    pub(super) fn map_artifact_result<T>(
        &self,
        result: Result<T, CompilerPipelineError>,
    ) -> Result<T, ArtifactCompileError> {
        match result {
            Ok(value) => Ok(value),
            Err(error) => match self.transform_fault(error) {
                Ok(transform) => Err(transform_public(transform)),
                Err(CompilerPipelineError::Segment(PassSegmentError::PassFailed {
                    pass,
                    source,
                })) => Self::attribute_prefix_pass_failure(pass, source),
                Err(CompilerPipelineError::Segment(PassSegmentError::VerificationFailed {
                    pass,
                    source,
                    ..
                })) => panic!("inter-pass verification rejected `{pass}` output: {source}"),
                Err(CompilerPipelineError::Segment(PassSegmentError::InputVerification {
                    input,
                    source,
                })) => {
                    panic!("inter-pass verification rejected the {input:?} segment input: {source}")
                }
                Err(CompilerPipelineError::GatherVerification { source }) => {
                    panic!(
                        "inter-pass verification rejected the gather-documents boundary: {source}"
                    )
                }
                Err(error) => panic!("the private built-in artifact schedule is invalid: {error}"),
            },
        }
    }

    /// The per-name downcast ladder for a prefix/lane pass failure: the
    /// historical arms exactly, with the assemble impossible-failure panic
    /// and the old private-schedule panic for any unknown pass — a string
    /// `Pass` error is not this path's posture.
    fn attribute_prefix_pass_failure<T>(
        pass: PassName,
        source: Box<dyn std::error::Error + Send + Sync>,
    ) -> Result<T, ArtifactCompileError> {
        match pass.as_str() {
            CLOSE_PASS_NAME => {
                let error = source.downcast::<UseGraphError>().unwrap_or_else(|source| {
                    panic!("the close pass returned an unexpected error type: {source}")
                });
                Err(ArtifactCompileError::Compile(
                    crate::pipeline::CompileError::UseGraph(*error),
                ))
            }
            MERGE_PASS_NAME => {
                let error = source
                    .downcast::<MergePassError>()
                    .unwrap_or_else(|source| {
                        panic!("the merge pass returned an unexpected error type: {source}")
                    });
                Err(ArtifactCompileError::Compile(error.into_compile_error()))
            }
            EMBED_PASS_NAME => {
                let error = source
                    .downcast::<EmbedPassError>()
                    .unwrap_or_else(|source| {
                        panic!("the embed pass returned an unexpected error type: {source}")
                    });
                Err(ArtifactCompileError::Compile(error.into_compile_error()))
            }
            LINK_PASS_NAME => {
                let error = source.downcast::<LinkPassError>().unwrap_or_else(|source| {
                    panic!("the link pass returned an unexpected error type: {source}")
                });
                Err(ArtifactCompileError::Compile(error.into_compile_error()))
            }
            ASSEMBLE_PASS_NAME => {
                let error = source
                    .downcast::<AssemblePassError>()
                    .unwrap_or_else(|source| {
                        panic!("the assemble pass returned an unexpected error type: {source}")
                    });
                panic!("the built-in assemble pass rejected validated compiler state: {error}");
            }
            _ => panic!(
                "the private built-in artifact schedule is invalid: `{pass}` failed: {source}"
            ),
        }
    }

    /// The per-name downcast ladder for an emitted-path pass failure,
    /// attributing engine faults to their owning plan input exactly as
    /// before the transform family existed.
    pub(super) fn attribute_emit_pass_failure(
        &self,
        pass: PassName,
        source: Box<dyn std::error::Error + Send + Sync>,
        plan: &ArtifactPlan,
        owners: &ErrorOwners,
    ) -> Result<super::super::ir::EmittedArtifact, ArtifactCompileError> {
        match pass.as_str() {
            CLOSE_PASS_NAME => match source.downcast::<UseGraphError>() {
                Ok(error) => Err(super::driver::attribute_compile_error(
                    crate::pipeline::CompileError::UseGraph(*error),
                    plan,
                    owners,
                    None,
                )),
                Err(source) => unexpected_pass_error(&pass, source),
            },
            MERGE_PASS_NAME => match source.downcast::<MergePassError>() {
                Ok(error) => Err(super::driver::attribute_compile_error(
                    error.into_compile_error(),
                    plan,
                    owners,
                    None,
                )),
                Err(source) => unexpected_pass_error(&pass, source),
            },
            EMBED_PASS_NAME => match source.downcast::<EmbedPassError>() {
                Ok(error) => Err(super::driver::attribute_compile_error(
                    error.into_compile_error(),
                    plan,
                    owners,
                    None,
                )),
                Err(source) => unexpected_pass_error(&pass, source),
            },
            LINK_PASS_NAME => match source.downcast::<LinkPassError>() {
                Ok(error) => {
                    let input = match error.as_ref() {
                        LinkPassError::AmbiguousShortLink { contribution, .. } => {
                            Some(*contribution)
                        }
                        _ => None,
                    };
                    Err(super::driver::attribute_compile_error(
                        error.into_compile_error(),
                        plan,
                        owners,
                        input,
                    ))
                }
                Err(source) => unexpected_pass_error(&pass, source),
            },
            _ => match source.downcast::<super::super::emit::EmitPassError>() {
                Ok(error) => Err(ArtifactCompileError::Backend {
                    pass: pass.to_string(),
                    reason: error.to_string(),
                }),
                Err(source) => Err(ArtifactCompileError::Pass {
                    pass: pass.to_string(),
                    reason: source.to_string(),
                }),
            },
        }
    }
}

fn unexpected_pass_error<T>(
    pass: &PassName,
    source: Box<dyn std::error::Error + Send + Sync>,
) -> Result<T, ArtifactCompileError> {
    Err(ArtifactCompileError::Pass {
        pass: pass.to_string(),
        reason: source.to_string(),
    })
}
