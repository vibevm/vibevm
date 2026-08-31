//! Per-stage execution dispatch for resolved builtin/native transform rows.
//!
//! The schedule cell owns ordering and selector placement. This sibling owns
//! only what happens after a row has been selected for execution: builtin
//! behavior dispatch or the borrowed native-manager call, followed by the
//! existing manager-side lane/emitted admission laws.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-NATIVE-ONLY");

use std::sync::Arc;

use vibe_core::lifecycle::CompilePoint;
use vibe_core::manifest::ExtensionKey;

use crate::compiler::ir::{DocumentIr, EmittedArtifact, LaneIr, SourceIr};
use crate::compiler::pass::{AnyIr, PassName};

use super::behavior::TransformBehavior;
use super::emitted_reconstruction;
use super::fault::TransformError;
use super::lane_admission::{self, LaneAdmissionError};
use super::native_identity::CompilerNativeImplementationDigest;
use super::native_manager::{self, CompilerNativeInvoker, NativeEntry, NativeManagerError};
use super::plan::{TransformConfig, TransformStage};
use super::plan_validate::BoundedPreview;

#[derive(Clone)]
pub(super) enum TransformExecution<'invoke> {
    Builtin(Arc<dyn TransformBehavior>),
    Native {
        key: ExtensionKey,
        invoker: &'invoke dyn CompilerNativeInvoker,
        digest: CompilerNativeImplementationDigest,
    },
}

impl<'invoke> TransformExecution<'invoke> {
    pub(super) fn from_behavior(behavior: Arc<dyn TransformBehavior>) -> Self {
        Self::Builtin(behavior)
    }

    pub(super) fn native(
        key: ExtensionKey,
        invoker: &'invoke dyn CompilerNativeInvoker,
        digest: CompilerNativeImplementationDigest,
    ) -> Self {
        Self::Native {
            key,
            invoker,
            digest,
        }
    }

    pub(super) fn run_source(
        &self,
        name: &PassName,
        preview: &BoundedPreview,
        order: u32,
        config: Option<&TransformConfig>,
        input: SourceIr,
    ) -> Result<SourceIr, TransformError> {
        match self {
            Self::Builtin(behavior) => {
                behavior
                    .run_source(config, input)
                    .map_err(|source| TransformError::Behavior {
                        preview: preview.clone(),
                        order,
                        stage: TransformStage::Source,
                        source,
                    })
            }
            Self::Native {
                key,
                invoker,
                digest,
            } => {
                let output = native_manager::execute(
                    NativeEntry::new(
                        *invoker,
                        key,
                        CompilePoint::Source,
                        order,
                        config,
                        *digest,
                        name,
                    ),
                    AnyIr::Source(input),
                )
                .map_err(|source| native_fault(preview, order, TransformStage::Source, source))?;
                match output {
                    AnyIr::Source(output) => Ok(output),
                    _ => Err(internal(preview, order, CompilePoint::Source)),
                }
            }
        }
    }

    pub(super) fn run_document(
        &self,
        name: &PassName,
        preview: &BoundedPreview,
        order: u32,
        config: Option<&TransformConfig>,
        input: DocumentIr,
    ) -> Result<DocumentIr, TransformError> {
        match self {
            Self::Builtin(behavior) => {
                behavior
                    .run_document(config, input)
                    .map_err(|source| TransformError::Behavior {
                        preview: preview.clone(),
                        order,
                        stage: TransformStage::Document,
                        source,
                    })
            }
            Self::Native {
                key,
                invoker,
                digest,
            } => {
                let output = native_manager::execute(
                    NativeEntry::new(
                        *invoker,
                        key,
                        CompilePoint::Document,
                        order,
                        config,
                        *digest,
                        name,
                    ),
                    AnyIr::Document(input),
                )
                .map_err(|source| native_fault(preview, order, TransformStage::Document, source))?;
                match output {
                    AnyIr::Document(output) => Ok(output),
                    _ => Err(internal(preview, order, CompilePoint::Document)),
                }
            }
        }
    }

    pub(super) fn run_lane(
        &self,
        name: &PassName,
        preview: &BoundedPreview,
        order: u32,
        config: Option<&TransformConfig>,
        input: LaneIr,
    ) -> Result<LaneIr, TransformError> {
        match self {
            Self::Builtin(behavior) => {
                let witness = lane_admission::witness(&input);
                let output = behavior.run_lane(config, input).map_err(|source| {
                    TransformError::Behavior {
                        preview: preview.clone(),
                        order,
                        stage: TransformStage::Lane,
                        source,
                    }
                })?;
                lane_admission::admit(&witness, &output)
                    .map_err(|source| lane_fault(preview, order, source))?;
                Ok(output)
            }
            Self::Native {
                key,
                invoker,
                digest,
            } => {
                let output = native_manager::execute(
                    NativeEntry::new(
                        *invoker,
                        key,
                        CompilePoint::Lane,
                        order,
                        config,
                        *digest,
                        name,
                    ),
                    AnyIr::Lane(input),
                )
                .map_err(|source| native_fault(preview, order, TransformStage::Lane, source))?;
                match output {
                    AnyIr::Lane(output) => Ok(output),
                    _ => Err(internal(preview, order, CompilePoint::Lane)),
                }
            }
        }
    }

    pub(super) fn run_emitted(
        &self,
        name: &PassName,
        preview: &BoundedPreview,
        order: u32,
        config: Option<&TransformConfig>,
        input: EmittedArtifact,
    ) -> Result<EmittedArtifact, TransformError> {
        match self {
            Self::Builtin(behavior) => {
                let bytes = input.bytes().to_vec();
                let output = behavior.run_emitted(config, bytes).map_err(|source| {
                    TransformError::Behavior {
                        preview: preview.clone(),
                        order,
                        stage: TransformStage::Emitted,
                        source,
                    }
                })?;
                Ok(emitted_reconstruction::reconstruct(input, output, name))
            }
            Self::Native {
                key,
                invoker,
                digest,
            } => {
                let output = native_manager::execute(
                    NativeEntry::new(
                        *invoker,
                        key,
                        CompilePoint::Emitted,
                        order,
                        config,
                        *digest,
                        name,
                    ),
                    AnyIr::Emitted(input),
                )
                .map_err(|source| native_fault(preview, order, TransformStage::Emitted, source))?;
                match output {
                    AnyIr::Emitted(output) => Ok(output),
                    _ => Err(internal(preview, order, CompilePoint::Emitted)),
                }
            }
        }
    }
}

fn lane_fault(preview: &BoundedPreview, order: u32, source: LaneAdmissionError) -> TransformError {
    match source {
        LaneAdmissionError::Intrinsic(source) => TransformError::LaneIntrinsic {
            preview: preview.clone(),
            order,
            stage: TransformStage::Lane,
            source,
        },
        LaneAdmissionError::Transition(source) => TransformError::LaneTransition {
            preview: preview.clone(),
            order,
            stage: TransformStage::Lane,
            source,
        },
    }
}

fn native_fault(
    preview: &BoundedPreview,
    order: u32,
    stage: TransformStage,
    source: NativeManagerError,
) -> TransformError {
    TransformError::Native {
        preview: preview.clone(),
        order,
        stage,
        source,
    }
}

fn internal(preview: &BoundedPreview, order: u32, point: CompilePoint) -> TransformError {
    native_fault(
        preview,
        order,
        match point {
            CompilePoint::Source => TransformStage::Source,
            CompilePoint::Document => TransformStage::Document,
            CompilePoint::Lane => TransformStage::Lane,
            CompilePoint::Emitted => TransformStage::Emitted,
            CompilePoint::Pass => unreachable!("compile:pass is outside the staged plan"),
        },
        NativeManagerError::InternalCarrier { point },
    )
}
