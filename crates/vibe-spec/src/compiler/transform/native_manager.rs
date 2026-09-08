//! Borrowed compiler-native invocation and manager-owned reply admission.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-NATIVE-ONLY");
use super::config::{ConfigDatetime, ConfigOffset, ConfigTable, ConfigValue};
use super::emitted_reconstruction;
use super::lane_admission;
use super::native_identity::CompilerNativeImplementationDigest;
use super::native_policy::CompilerNativePolicyError;
use super::native_policy::session::{NativePolicySession, UnavailableDisposition};
use super::plan::TransformConfig;
use crate::compiler::ir::{IrCardinality, IrLevel, IrShape};
use crate::compiler::pass::{AnyIr, PassName};
use crate::compiler::verify::IrVerifier;
use crate::compiler::wire;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt;
use vibe_core::lifecycle::CompilePoint;
use vibe_core::manifest::ExtensionKey;
use vibe_wire::behaviour::native_compile::{IrCarrier, NativeCompileError, validate_reply};
use vibe_wire::generated::native::e1::compile_reply::CompileReply;
use vibe_wire::generated::shared::Ir;
mod backend;
pub(crate) use backend::{NativeBackendEntry, NativeBackendError, execute_backend};

const DIAGNOSTIC_BYTES: usize = 256;
pub struct CompilerNativeCall<'call> {
    key: &'call ExtensionKey,
    point: CompilePoint,
    order: u32,
    config: &'call BTreeMap<String, Option<Value>>,
    implementation: CompilerNativeImplementationDigest,
    frontend_physical_stem: Option<&'call str>,
    backend: Option<&'call str>,
    payload: Ir,
}

impl<'call> CompilerNativeCall<'call> {
    #[cfg(any(test, feature = "test-support"))]
    #[doc(hidden)]
    pub fn new_for_test(
        key: &'call ExtensionKey,
        point: CompilePoint,
        order: u32,
        config: &'call BTreeMap<String, Option<Value>>,
        implementation: CompilerNativeImplementationDigest,
        payload: Ir,
    ) -> Self {
        Self {
            key,
            point,
            order,
            config,
            implementation,
            frontend_physical_stem: None,
            backend: None,
            payload,
        }
    }
    #[cfg(any(test, feature = "test-support"))]
    #[doc(hidden)]
    pub fn with_frontend_physical_stem_for_test(mut self, stem: &'call str) -> Self {
        self.frontend_physical_stem = Some(stem);
        self
    }
    #[cfg(any(test, feature = "test-support"))]
    #[doc(hidden)]
    pub fn with_backend_for_test(mut self, backend: &'call str) -> Self {
        self.backend = Some(backend);
        self
    }
    pub fn key(&self) -> &ExtensionKey {
        self.key
    }
    pub const fn point(&self) -> CompilePoint {
        self.point
    }
    pub const fn order(&self) -> u32 {
        self.order
    }
    pub fn config(&self) -> &BTreeMap<String, Option<Value>> {
        self.config
    }
    pub const fn implementation(&self) -> CompilerNativeImplementationDigest {
        self.implementation
    }
    pub fn frontend_physical_stem(&self) -> Option<&str> {
        self.frontend_physical_stem
    }
    pub fn backend(&self) -> Option<&str> {
        self.backend
    }

    pub fn payload(&self) -> &Ir {
        &self.payload
    }

    pub fn into_payload(self) -> Ir {
        self.payload
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilerNativeInvokerErrorKind {
    BuildableSourceUnavailable,
    InvocationFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilerNativeInvokerError {
    kind: CompilerNativeInvokerErrorKind,
    detail: String,
}

impl CompilerNativeInvokerError {
    pub fn new(kind: CompilerNativeInvokerErrorKind, detail: impl AsRef<str>) -> Self {
        Self {
            kind,
            detail: bounded(detail.as_ref()),
        }
    }

    pub const fn kind(&self) -> CompilerNativeInvokerErrorKind {
        self.kind
    }
}

impl fmt::Display for CompilerNativeInvokerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}: {}", self.kind, self.detail)
    }
}

impl std::error::Error for CompilerNativeInvokerError {}

/// The narrow loader/artifact-independent native invocation seam.
pub trait CompilerNativeInvoker: Send + Sync {
    /// Admit one payload-free frontend row before its selected source is read.
    fn admit_frontend(
        &self,
        _key: &ExtensionKey,
        _order: u32,
        _config: &BTreeMap<String, Option<Value>>,
        _implementation: CompilerNativeImplementationDigest,
    ) -> Result<(), CompilerNativeInvokerError> {
        Err(CompilerNativeInvokerError::new(
            CompilerNativeInvokerErrorKind::InvocationFailed,
            "the compiler-native invoker does not admit frontend catalogs",
        ))
    }

    /// Admit one payload-free backend row before any artifact source is read.
    fn admit_backend(
        &self,
        _key: &ExtensionKey,
        _order: u32,
        _config: &BTreeMap<String, Option<Value>>,
        _implementation: CompilerNativeImplementationDigest,
        _backend: &str,
    ) -> Result<(), CompilerNativeInvokerError> {
        Err(CompilerNativeInvokerError::new(
            CompilerNativeInvokerErrorKind::InvocationFailed,
            "the compiler-native invoker does not admit backend catalogs",
        ))
    }

    fn invoke(&self, call: CompilerNativeCall<'_>) -> Result<Vec<u8>, CompilerNativeInvokerError>;
}

/// Borrowed invocation authority shared by one resolved native row.
#[derive(Clone, Copy)]
pub(crate) struct NativeRuntime<'entry> {
    invoker: &'entry dyn CompilerNativeInvoker,
    policy: Option<&'entry NativePolicySession>,
}

impl<'entry> NativeRuntime<'entry> {
    pub(crate) const fn new(
        invoker: &'entry dyn CompilerNativeInvoker,
        policy: Option<&'entry NativePolicySession>,
    ) -> Self {
        Self { invoker, policy }
    }
}

/// One resolved native schedule row, borrowed for exactly one invocation.
pub(crate) struct NativeEntry<'entry> {
    runtime: NativeRuntime<'entry>,
    key: &'entry ExtensionKey,
    point: CompilePoint,
    order: u32,
    config: Option<&'entry TransformConfig>,
    projected_config: Option<&'entry BTreeMap<String, Option<Value>>>,
    implementation: CompilerNativeImplementationDigest,
    pass: &'entry PassName,
    expected: Option<IrShape>,
    frontend_physical_stem: Option<&'entry str>,
}

impl<'entry> NativeEntry<'entry> {
    pub(crate) fn new(
        runtime: NativeRuntime<'entry>,
        key: &'entry ExtensionKey,
        point: CompilePoint,
        order: u32,
        config: Option<&'entry TransformConfig>,
        implementation: CompilerNativeImplementationDigest,
        pass: &'entry PassName,
    ) -> Self {
        Self {
            runtime,
            key,
            point,
            order,
            config,
            projected_config: None,
            implementation,
            pass,
            expected: None,
            frontend_physical_stem: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_pass(
        runtime: NativeRuntime<'entry>,
        key: &'entry ExtensionKey,
        order: u32,
        projected_config: &'entry BTreeMap<String, Option<Value>>,
        implementation: CompilerNativeImplementationDigest,
        pass: &'entry PassName,
        expected: IrShape,
        frontend_physical_stem: Option<&'entry str>,
    ) -> Self {
        Self {
            runtime,
            key,
            point: CompilePoint::Pass,
            order,
            config: None,
            projected_config: Some(projected_config),
            implementation,
            pass,
            expected: Some(expected),
            frontend_physical_stem,
        }
    }
}

/// One locally attributed native-manager refusal. No variant retains raw
/// reply bytes, an unbounded plugin message, or a returned carrier payload.
#[derive(Debug, thiserror::Error)]
pub(crate) enum NativeManagerError {
    #[error("the effective configuration cannot enter the native call: {0}")]
    Config(#[from] ConfigProjectionError),
    #[error("the manager could not project the canonical request IR: {detail}")]
    Request { detail: String },
    #[error("the native invoker refused: {0}")]
    Invoker(#[from] CompilerNativeInvokerError),
    #[error("the compiler-native pending policy refused: {0}")]
    Policy(#[from] CompilerNativePolicyError),
    #[error("the strict native reply reader refused: {detail}")]
    ReplyReader { detail: String },
    #[error("the native reply exchange refused: {0}")]
    Exchange(#[from] NativeCompileError),
    #[error("the native handler returned `fail`{message}")]
    Fail { message: BoundedMessage },
    #[error("the native reply returned carrier `{actual}`, expected `{expected}`")]
    Carrier {
        expected: IrCarrier,
        actual: IrCarrier,
    },
    #[error("the returned canonical IR was refused locally: {detail}")]
    ReturnedIr { detail: String },
    #[error("the returned carrier violated the manager transition law: {detail}")]
    Transition { detail: String },
    #[error("the native manager received an impossible carrier for {point:?}")]
    InternalCarrier { point: CompilePoint },
}

/// One manager-admitted carrier and whether the native handler executed.
pub(crate) struct NativeManagerOutput {
    carrier: AnyIr,
    executed: bool,
}

impl NativeManagerOutput {
    pub(crate) fn into_parts(self) -> (AnyIr, bool) {
        (self.carrier, self.executed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BoundedMessage(Option<String>);

impl fmt::Display for BoundedMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Some(message) => write!(formatter, ": {message}"),
            None => Ok(()),
        }
    }
}

impl BoundedMessage {
    fn new(message: Option<&str>) -> Self {
        Self(message.map(bounded))
    }
}

pub(crate) fn execute(
    entry: NativeEntry<'_>,
    input: AnyIr,
) -> Result<NativeManagerOutput, NativeManagerError> {
    let NativeEntry {
        runtime,
        key,
        point,
        order,
        config,
        projected_config,
        implementation,
        pass,
        expected,
        frontend_physical_stem,
    } = entry;
    let NativeRuntime { invoker, policy } = runtime;
    let owned_config;
    let projected = match projected_config {
        Some(projected) => projected,
        None => {
            owned_config = execution_config(config.map(TransformConfig::as_table))?;
            &owned_config
        }
    };
    let verifier = IrVerifier;
    let witness = if point == CompilePoint::Pass {
        None
    } else {
        Some(verifier.witness(&input).map_err(transition)?)
    };
    let payload = wire::encode_generated(&input).map_err(returned_request)?;
    let raw = match invoker.invoke(CompilerNativeCall {
        key,
        point,
        order,
        config: projected,
        implementation,
        frontend_physical_stem,
        backend: None,
        payload,
    }) {
        Ok(raw) => raw,
        Err(error)
            if error.kind() == CompilerNativeInvokerErrorKind::BuildableSourceUnavailable
                && policy.is_some() =>
        {
            let Some(policy) = policy else {
                return Err(NativeManagerError::Invoker(error));
            };
            match policy.unavailable(order, key, point, config, implementation)? {
                UnavailableDisposition::Hard => return Err(NativeManagerError::Invoker(error)),
                UnavailableDisposition::ContinueOriginal => {
                    return Ok(NativeManagerOutput {
                        carrier: input,
                        executed: false,
                    });
                }
            }
        }
        Err(error) => return Err(NativeManagerError::Invoker(error)),
    };
    let reply: CompileReply = wire::json::from_strict_slice(&raw).map_err(reply_reader)?;
    let shape = validate_reply(&reply)?;
    let output = match reply {
        CompileReply::Skip(_) => input,
        CompileReply::Fail(reply) => Err(NativeManagerError::Fail {
            message: BoundedMessage::new(reply.message.as_deref()),
        })?,
        CompileReply::Ok(reply) => {
            let Some(shape) = shape else {
                return Err(NativeManagerError::InternalCarrier { point });
            };
            let actual = shape.carrier;
            let expected_carrier = expected_carrier(point, expected);
            if actual != expected_carrier {
                return Err(NativeManagerError::Carrier {
                    expected: expected_carrier,
                    actual,
                });
            }
            let returned = wire::decode_generated(&reply.payload).map_err(returned_ir)?;
            let final_value = admit_stage(point, input, returned, pass)?;
            if let Some(witness) = &witness {
                verifier.verify(&final_value).map_err(transition)?;
                verifier
                    .verify_transition(witness, &final_value)
                    .map_err(transition)?;
            }
            final_value
        }
    };
    if let Some(expected) = expected {
        let actual = output.shape();
        if actual != expected {
            return Err(NativeManagerError::Carrier {
                expected: carrier_for_shape(expected),
                actual: carrier_for_shape(actual),
            });
        }
    }
    if let Some(policy) = policy {
        policy.success(order, key, point, config, implementation)?;
    }
    Ok(NativeManagerOutput {
        carrier: output,
        executed: true,
    })
}

fn expected_carrier(point: CompilePoint, pass_output: Option<IrShape>) -> IrCarrier {
    match point {
        CompilePoint::Source => IrCarrier::SourceDocument,
        CompilePoint::Document => IrCarrier::DocumentDocument,
        CompilePoint::Lane => IrCarrier::LaneArtifact,
        CompilePoint::Emitted => IrCarrier::EmittedArtifact,
        CompilePoint::Pass => {
            let Some(pass_output) = pass_output else {
                unreachable!("the test-only pass-tier entry supplies its output shape")
            };
            carrier_for_shape(pass_output)
        }
    }
}

fn carrier_for_shape(shape: IrShape) -> IrCarrier {
    match (shape.level, shape.cardinality) {
        (IrLevel::Source, IrCardinality::Document) => IrCarrier::SourceDocument,
        (IrLevel::Document, IrCardinality::Document) => IrCarrier::DocumentDocument,
        (IrLevel::Document, IrCardinality::Artifact) => IrCarrier::DocumentsArtifact,
        (IrLevel::Closure, IrCardinality::Artifact) => IrCarrier::ClosureArtifact,
        (IrLevel::Lane, IrCardinality::Artifact) => IrCarrier::LaneArtifact,
        (IrLevel::Emitted, IrCardinality::Artifact) => IrCarrier::EmittedArtifact,
        _ => unreachable!("the compiler has no wire carrier for {shape:?}"),
    }
}

fn admit_stage(
    point: CompilePoint,
    original: AnyIr,
    returned: AnyIr,
    pass: &PassName,
) -> Result<AnyIr, NativeManagerError> {
    if point == CompilePoint::Pass {
        return Ok(returned);
    }
    match (point, original, returned) {
        (CompilePoint::Source, AnyIr::Source(_), value @ AnyIr::Source(_))
        | (CompilePoint::Document, AnyIr::Document(_), value @ AnyIr::Document(_)) => Ok(value),
        (CompilePoint::Lane, AnyIr::Lane(original), AnyIr::Lane(returned)) => {
            let witness = lane_admission::witness(&original);
            lane_admission::admit(&witness, &returned).map_err(|source| {
                NativeManagerError::Transition {
                    detail: wire::bounded::debug_within(source, DIAGNOSTIC_BYTES),
                }
            })?;
            Ok(AnyIr::Lane(returned))
        }
        (CompilePoint::Emitted, AnyIr::Emitted(original), AnyIr::Emitted(returned)) => {
            Ok(AnyIr::Emitted(emitted_reconstruction::reconstruct(
                original,
                returned.bytes().to_vec(),
                pass,
            )))
        }
        (point, _, _) => Err(NativeManagerError::InternalCarrier { point }),
    }
}

fn returned_request(source: wire::IrWireError) -> NativeManagerError {
    NativeManagerError::Request {
        detail: wire::bounded::debug_within(source, DIAGNOSTIC_BYTES),
    }
}

fn reply_reader(source: wire::IrWireError) -> NativeManagerError {
    NativeManagerError::ReplyReader {
        detail: wire::bounded::debug_within(source, DIAGNOSTIC_BYTES),
    }
}

fn returned_ir(source: wire::IrWireError) -> NativeManagerError {
    NativeManagerError::ReturnedIr {
        detail: wire::bounded::debug_within(source, DIAGNOSTIC_BYTES),
    }
}

fn transition(source: impl fmt::Debug) -> NativeManagerError {
    NativeManagerError::Transition {
        detail: wire::bounded::debug_within(source, DIAGNOSTIC_BYTES),
    }
}

fn bounded(value: &str) -> String {
    let mut kept = String::new();
    for character in value.chars() {
        let character = if character.is_control() {
            '\u{fffd}'
        } else {
            character
        };
        if kept.len() + character.len_utf8() > DIAGNOSTIC_BYTES {
            kept.push('…');
            break;
        }
        kept.push(character);
    }
    kept
}

/// Project plan configuration onto the generated lifecycle execution map.
/// Plan absence and authored-empty deliberately both produce the mandatory
/// empty map; every present value otherwise keeps its JSON-visible spelling.
fn execution_config(
    table: Option<&ConfigTable>,
) -> Result<BTreeMap<String, Option<Value>>, ConfigProjectionError> {
    table
        .into_iter()
        .flat_map(|table| table.iter())
        .map(|(key, value)| Ok((key.clone(), Some(project_value(value)?))))
        .collect()
}

fn project_value(value: &ConfigValue) -> Result<Value, ConfigProjectionError> {
    Ok(match value {
        ConfigValue::String(value) => Value::String(value.clone()),
        ConfigValue::Integer(value) => Value::Number((*value).into()),
        ConfigValue::Float(value) => serde_json::Number::from_f64(value.value())
            .map(Value::Number)
            .ok_or(ConfigProjectionError::NonFiniteFloat)?,
        ConfigValue::Boolean(value) => Value::Bool(*value),
        ConfigValue::Datetime(value) => Value::String(render_datetime(value)),
        ConfigValue::Array(values) => Value::Array(
            values
                .iter()
                .map(project_value)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        ConfigValue::Table(table) => Value::Object(
            table
                .iter()
                .map(|(key, value)| Ok((key.clone(), project_value(value)?)))
                .collect::<Result<serde_json::Map<_, _>, _>>()?,
        ),
    })
}

fn render_datetime(value: &ConfigDatetime) -> String {
    let mut rendered = String::new();
    if let Some(date) = value.date() {
        rendered.push_str(&format!(
            "{:04}-{:02}-{:02}",
            date.year(),
            date.month(),
            date.day()
        ));
    }
    if let Some(time) = value.time() {
        if value.date().is_some() {
            rendered.push('T');
        }
        rendered.push_str(&format!(
            "{:02}:{:02}:{:02}",
            time.hour(),
            time.minute(),
            time.second()
        ));
        if time.nanosecond() != 0 {
            let fraction = format!("{:09}", time.nanosecond());
            rendered.push('.');
            rendered.push_str(fraction.trim_end_matches('0'));
        }
    }
    match value.offset() {
        Some(ConfigOffset::Z) => rendered.push('Z'),
        Some(ConfigOffset::Custom { minutes }) => {
            let sign = if minutes < 0 { '-' } else { '+' };
            let absolute = minutes.unsigned_abs();
            rendered.push(sign);
            rendered.push_str(&format!("{:02}:{:02}", absolute / 60, absolute % 60));
        }
        None => {}
    }
    rendered
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ConfigProjectionError {
    #[error("a non-finite TOML float has no exact JSON execution-config value")]
    NonFiniteFloat,
}
