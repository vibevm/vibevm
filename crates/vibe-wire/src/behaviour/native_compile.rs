//! Transport-neutral admission for the compiler-native request/reply exchange.
//!
//! The generated types own structural decoding. This module owns the small
//! relational boundary JTD cannot state: envelope/schema epochs, the point to
//! carrier table, and preservation of carrier/level/cardinality across an
//! `ok` exchange. It performs no loading, invocation, lifecycle work, or
//! compiler-domain conversion.

use std::fmt;

use crate::generated::native::e1::{compile_reply, compile_request};
use crate::generated::shared::Ir;

/// Epoch accepted by both compiler-native roots.
pub const ENVELOPE_EPOCH: u32 = 1;

/// Maximum bytes copied from an unknown wire scalar into a diagnostic.
pub const DIAGNOSTIC_SCALAR_BYTES: usize = 128;

/// A bounded diagnostic copy of an untrusted wire scalar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticScalar {
    value: String,
    truncated: bool,
}

impl DiagnosticScalar {
    fn from_wire(value: &str) -> Self {
        let mut bounded = String::new();
        let mut truncated = false;
        for character in value.chars() {
            let printable = if character.is_control() {
                '\u{fffd}'
            } else {
                character
            };
            if bounded.len() + printable.len_utf8() > DIAGNOSTIC_SCALAR_BYTES {
                truncated = true;
                break;
            }
            bounded.push(printable);
        }
        Self {
            value: bounded,
            truncated,
        }
    }

    /// The bounded prefix copied into the diagnostic.
    pub fn as_str(&self) -> &str {
        &self.value
    }

    /// Whether the original scalar had bytes beyond [`Self::as_str`].
    pub fn truncated(&self) -> bool {
        self.truncated
    }
}

impl fmt::Display for DiagnosticScalar {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.value)?;
        if self.truncated {
            formatter.write_str("…")?;
        }
        Ok(())
    }
}

/// The five admitted compiler extension points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeCompilePoint {
    Source,
    Document,
    Lane,
    Emitted,
    Pass,
}

impl NativeCompilePoint {
    fn parse(value: &str) -> Result<Self, NativeCompileError> {
        match value {
            "compile:source" => Ok(Self::Source),
            "compile:document" => Ok(Self::Document),
            "compile:lane" => Ok(Self::Lane),
            "compile:emitted" => Ok(Self::Emitted),
            "compile:pass" => Ok(Self::Pass),
            _ => Err(NativeCompileError::UnsupportedPoint {
                point: DiagnosticScalar::from_wire(value),
            }),
        }
    }

    /// Exact wire spelling of this admitted point.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Source => "compile:source",
            Self::Document => "compile:document",
            Self::Lane => "compile:lane",
            Self::Emitted => "compile:emitted",
            Self::Pass => "compile:pass",
        }
    }
}

impl fmt::Display for NativeCompilePoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Canonical compiler-IR discriminator carried by a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrCarrier {
    SourceDocument,
    DocumentDocument,
    DocumentsArtifact,
    ClosureArtifact,
    LaneArtifact,
    EmittedArtifact,
}

impl IrCarrier {
    /// Exact `shape` discriminator spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SourceDocument => "source-document",
            Self::DocumentDocument => "document-document",
            Self::DocumentsArtifact => "documents-artifact",
            Self::ClosureArtifact => "closure-artifact",
            Self::LaneArtifact => "lane-artifact",
            Self::EmittedArtifact => "emitted-artifact",
        }
    }
}

impl fmt::Display for IrCarrier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Canonical compiler-IR level, projected without a compiler-domain dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrLevel {
    Source,
    Document,
    Closure,
    Lane,
    Emitted,
}

/// Canonical compiler-IR cardinality.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrCardinality {
    Document,
    Artifact,
}

/// The three identity scalars an `ok` reply must preserve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IrShape {
    pub carrier: IrCarrier,
    pub level: IrLevel,
    pub cardinality: IrCardinality,
}

/// Which half of the exchange carried an invalid epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileWireSide {
    Request,
    Reply(CompileReplyStatus),
}

/// Bounded reply status vocabulary used in diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileReplyStatus {
    Ok,
    Skip,
    Fail,
}

impl CompileReplyStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Skip => "skip",
            Self::Fail => "fail",
        }
    }
}

/// Typed admission failures. No variant contains a payload or an unbounded
/// point/status/carrier scalar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeCompileError {
    Envelope {
        side: CompileWireSide,
        found: u32,
    },
    IrSchema {
        side: CompileWireSide,
        found: u32,
    },
    UnsupportedPoint {
        point: DiagnosticScalar,
    },
    StageCarrier {
        point: NativeCompilePoint,
        carrier: IrCarrier,
    },
    ExchangeShape {
        request: IrShape,
        reply: IrShape,
    },
}

impl fmt::Display for NativeCompileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Envelope { side, found } => {
                write!(
                    formatter,
                    "native compile {side} envelope {found}, expected 1"
                )
            }
            Self::IrSchema { side, found } => {
                write!(
                    formatter,
                    "native compile {side} IR schema {found}, expected 1"
                )
            }
            Self::UnsupportedPoint { point } => {
                write!(formatter, "unsupported native compile point `{point}`")
            }
            Self::StageCarrier { point, carrier } => {
                write!(
                    formatter,
                    "native compile point `{point}` rejects carrier `{carrier}`"
                )
            }
            Self::ExchangeShape { request, reply } => write!(
                formatter,
                "native compile ok reply changed shape {:?}/{:?}/{:?} to {:?}/{:?}/{:?}",
                request.carrier,
                request.level,
                request.cardinality,
                reply.carrier,
                reply.level,
                reply.cardinality
            ),
        }
    }
}

impl std::error::Error for NativeCompileError {}

impl fmt::Display for CompileWireSide {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Request => formatter.write_str("request"),
            Self::Reply(status) => write!(formatter, "reply({})", status.as_str()),
        }
    }
}

/// Admit one structurally decoded request and return its canonical shape.
pub fn validate_request(
    request: &compile_request::CompileRequest,
) -> Result<IrShape, NativeCompileError> {
    if request.envelope != ENVELOPE_EPOCH {
        return Err(NativeCompileError::Envelope {
            side: CompileWireSide::Request,
            found: request.envelope,
        });
    }
    let shape = validate_ir(&request.payload, CompileWireSide::Request)?;
    let point = NativeCompilePoint::parse(&request.point)?;
    let accepted = match point {
        NativeCompilePoint::Source => shape.carrier == IrCarrier::SourceDocument,
        NativeCompilePoint::Document => shape.carrier == IrCarrier::DocumentDocument,
        NativeCompilePoint::Lane => shape.carrier == IrCarrier::LaneArtifact,
        NativeCompilePoint::Emitted => shape.carrier == IrCarrier::EmittedArtifact,
        NativeCompilePoint::Pass => true,
    };
    if !accepted {
        return Err(NativeCompileError::StageCarrier {
            point,
            carrier: shape.carrier,
        });
    }
    Ok(shape)
}

/// Admit one structurally decoded reply. `None` is the typed skip/fail shape.
pub fn validate_reply(
    reply: &compile_reply::CompileReply,
) -> Result<Option<IrShape>, NativeCompileError> {
    match reply {
        compile_reply::CompileReply::Ok(value) => {
            validate_reply_envelope(value.envelope, CompileReplyStatus::Ok)?;
            validate_ir(
                &value.payload,
                CompileWireSide::Reply(CompileReplyStatus::Ok),
            )
            .map(Some)
        }
        compile_reply::CompileReply::Skip(value) => {
            validate_reply_envelope(value.envelope, CompileReplyStatus::Skip)?;
            Ok(None)
        }
        compile_reply::CompileReply::Fail(value) => {
            validate_reply_envelope(value.envelope, CompileReplyStatus::Fail)?;
            Ok(None)
        }
    }
}

/// Validate both halves and require an `ok` reply to preserve the request's
/// carrier, level, and cardinality. Skip/fail cannot carry a payload by type.
pub fn validate_exchange(
    request: &compile_request::CompileRequest,
    reply: &compile_reply::CompileReply,
) -> Result<(), NativeCompileError> {
    let request_shape = validate_request(request)?;
    if let Some(reply_shape) = validate_reply(reply)?
        && request_shape != reply_shape
    {
        return Err(NativeCompileError::ExchangeShape {
            request: request_shape,
            reply: reply_shape,
        });
    }
    Ok(())
}

fn validate_reply_envelope(
    envelope: u32,
    status: CompileReplyStatus,
) -> Result<(), NativeCompileError> {
    if envelope != ENVELOPE_EPOCH {
        return Err(NativeCompileError::Envelope {
            side: CompileWireSide::Reply(status),
            found: envelope,
        });
    }
    Ok(())
}

fn validate_ir(value: &Ir, side: CompileWireSide) -> Result<IrShape, NativeCompileError> {
    let (ir_schema, shape) = match value {
        Ir::SourceDocument(value) => (
            value.ir_schema,
            IrShape {
                carrier: IrCarrier::SourceDocument,
                level: IrLevel::Source,
                cardinality: IrCardinality::Document,
            },
        ),
        Ir::DocumentDocument(value) => (
            value.ir_schema,
            IrShape {
                carrier: IrCarrier::DocumentDocument,
                level: IrLevel::Document,
                cardinality: IrCardinality::Document,
            },
        ),
        Ir::DocumentsArtifact(value) => (
            value.ir_schema,
            IrShape {
                carrier: IrCarrier::DocumentsArtifact,
                level: IrLevel::Document,
                cardinality: IrCardinality::Artifact,
            },
        ),
        Ir::ClosureArtifact(value) => (
            value.ir_schema,
            IrShape {
                carrier: IrCarrier::ClosureArtifact,
                level: IrLevel::Closure,
                cardinality: IrCardinality::Artifact,
            },
        ),
        Ir::LaneArtifact(value) => (
            value.ir_schema,
            IrShape {
                carrier: IrCarrier::LaneArtifact,
                level: IrLevel::Lane,
                cardinality: IrCardinality::Artifact,
            },
        ),
        Ir::EmittedArtifact(value) => (
            value.ir_schema,
            IrShape {
                carrier: IrCarrier::EmittedArtifact,
                level: IrLevel::Emitted,
                cardinality: IrCardinality::Artifact,
            },
        ),
    };
    if ir_schema != 1 {
        return Err(NativeCompileError::IrSchema {
            side,
            found: ir_schema,
        });
    }
    Ok(shape)
}
