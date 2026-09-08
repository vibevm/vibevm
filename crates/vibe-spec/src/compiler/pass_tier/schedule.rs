//! Resolve one pass plan against the immutable builtin schedule transactionally.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#PASS-TRANSFORM");

use vibe_core::manifest::{ExtensionIrLevel as Level, ExtensionPassKind as Kind};

use crate::compiler::ir::{ClosureIr, DocumentIr, Documents, EmittedIr, LaneIr, SourceIr};
use crate::compiler::pass::{IrPayload, Pass, PassName};
use crate::compiler::pipeline::PassTierVerifierCapability;
use crate::compiler::pipeline::{CompilerPipeline, PipelineEdit, ScheduleItem};
use crate::compiler::transform::native_manager::CompilerNativeInvoker;

use super::execution::{PassTierCompileError, PassTierExecutionError};
use super::native::NativePass;
use super::plan::{PassEntry, PassPlacement, PassPlan};

pub(crate) struct PassExecutionAuthority<'invoke> {
    invoker: Option<&'invoke dyn CompilerNativeInvoker>,
    verifier: PassTierVerifierCapability,
}

impl<'invoke> PassExecutionAuthority<'invoke> {
    pub(crate) const fn verified(
        invoker: Option<&'invoke dyn CompilerNativeInvoker>,
        verifier: PassTierVerifierCapability,
    ) -> Self {
        Self { invoker, verifier }
    }
}

pub(crate) struct PassSchedule;

impl PassSchedule {
    /// Check live verification, resolve every entry, then apply one edit batch.
    pub(crate) fn install<'invoke>(
        plan: &PassPlan,
        pipeline: &mut CompilerPipeline<'invoke>,
        authority: PassExecutionAuthority<'invoke>,
    ) -> Result<(), PassTierCompileError> {
        if plan.is_empty() {
            return Ok(());
        }
        if !pipeline.has_pass_tier_verifier(&authority.verifier) {
            return Err(PassTierExecutionError::VerifierCapability.into());
        }
        let snapshot = pipeline.schedule();
        let mut edits = Vec::with_capacity(plan.len());
        for entry in plan.entries() {
            edits.push(resolve_entry(entry, &authority, &snapshot)?);
        }
        pipeline
            .apply_builtin_edits(edits)
            .map_err(|source| PassTierExecutionError::Schedule { source })?;
        Ok(())
    }
}

fn resolve_entry<'invoke>(
    entry: &PassEntry,
    authority: &PassExecutionAuthority<'invoke>,
    snapshot: &[ScheduleItem],
) -> Result<PipelineEdit<'invoke>, PassTierCompileError> {
    let name = PassName::new(format!("pass:{}", entry.key().as_str())).map_err(|source| {
        PassTierExecutionError::Name {
            key: entry.key().clone(),
            source,
        }
    })?;
    let pass = entry.declaration();
    match pass.kind {
        Kind::Transform => match pass.level {
            Some(Level::Source) => typed::<SourceIr, SourceIr>(entry, authority, name),
            Some(Level::Document) => typed::<DocumentIr, DocumentIr>(entry, authority, name),
            Some(Level::Closure) => typed::<ClosureIr, ClosureIr>(entry, authority, name),
            Some(Level::Lane) => typed::<LaneIr, LaneIr>(entry, authority, name),
            Some(Level::Emitted) => typed::<EmittedIr, EmittedIr>(entry, authority, name),
            None => invalid_shape(entry, "transform requires `level`"),
        },
        Kind::Lowering if pass.replace.is_some() && pass.from.is_none() && pass.to.is_none() => {
            replacement(entry, authority, name, snapshot)
        }
        Kind::Lowering => match (pass.from, pass.to) {
            (Some(Level::Source), Some(Level::Document)) => {
                typed::<SourceIr, DocumentIr>(entry, authority, name)
            }
            (Some(Level::Document), Some(Level::Closure)) => {
                typed::<Documents, ClosureIr>(entry, authority, name)
            }
            (Some(Level::Closure), Some(Level::Lane)) => {
                typed::<ClosureIr, LaneIr>(entry, authority, name)
            }
            (Some(Level::Lane), Some(Level::Emitted)) => {
                typed::<LaneIr, EmittedIr>(entry, authority, name)
            }
            _ => invalid_shape(
                entry,
                "lowering requires one adjacent `from` -> `to` transition",
            ),
        },
        Kind::Frontend => typed::<SourceIr, DocumentIr>(entry, authority, name),
        Kind::Backend => typed::<LaneIr, EmittedIr>(entry, authority, name),
    }
}

fn replacement<'invoke>(
    entry: &PassEntry,
    authority: &PassExecutionAuthority<'invoke>,
    name: PassName,
    snapshot: &[ScheduleItem],
) -> Result<PipelineEdit<'invoke>, PassTierCompileError> {
    let PassPlacement::Replace(anchor) = entry.placement() else {
        return invalid_shape(
            entry,
            "a lowering without `from`/`to` must replace a builtin",
        );
    };
    let descriptor = snapshot.iter().find_map(|item| match item {
        ScheduleItem::Pass(pass) if pass.name.as_str() == anchor => Some(pass),
        _ => None,
    });
    let Some(descriptor) = descriptor else {
        return Err(PassTierExecutionError::ReplacementAnchor {
            key: entry.key().clone(),
            anchor: anchor.clone(),
        }
        .into());
    };
    match (descriptor.input, descriptor.output) {
        (input, output) if input == SourceIr::SHAPE && output == DocumentIr::SHAPE => {
            typed::<SourceIr, DocumentIr>(entry, authority, name)
        }
        (input, output) if input == Documents::SHAPE && output == ClosureIr::SHAPE => {
            typed::<Documents, ClosureIr>(entry, authority, name)
        }
        (input, output) if input == ClosureIr::SHAPE && output == ClosureIr::SHAPE => {
            typed::<ClosureIr, ClosureIr>(entry, authority, name)
        }
        (input, output) if input == ClosureIr::SHAPE && output == LaneIr::SHAPE => {
            typed::<ClosureIr, LaneIr>(entry, authority, name)
        }
        (input, output) if input == LaneIr::SHAPE && output == EmittedIr::SHAPE => {
            typed::<LaneIr, EmittedIr>(entry, authority, name)
        }
        _ => invalid_shape(
            entry,
            "replacement anchor has an unsupported compiler IR shape",
        ),
    }
}

fn typed<'invoke, Input, Output>(
    entry: &PassEntry,
    authority: &PassExecutionAuthority<'invoke>,
    name: PassName,
) -> Result<PipelineEdit<'invoke>, PassTierCompileError>
where
    Input: IrPayload,
    Output: IrPayload,
{
    let invoker = authority
        .invoker
        .ok_or_else(|| PassTierExecutionError::MissingInvoker {
            key: entry.key().clone(),
        })?;
    let pass = NativePass::<Input, Output>::from_entry(entry, invoker, name).map_err(|error| {
        PassTierExecutionError::NativeExecution {
            key: entry.key().clone(),
            detail: error.to_string(),
        }
    })?;
    place(entry, pass)
}

fn place<'invoke>(
    entry: &PassEntry,
    pass: impl Pass + 'invoke,
) -> Result<PipelineEdit<'invoke>, PassTierCompileError> {
    let anchor = match entry.placement() {
        PassPlacement::Before(anchor)
        | PassPlacement::After(anchor)
        | PassPlacement::Replace(anchor) => {
            PassName::new(anchor.clone()).map_err(|source| PassTierExecutionError::Name {
                key: entry.key().clone(),
                source,
            })?
        }
        PassPlacement::Intrinsic => {
            return Err(PassTierExecutionError::IntrinsicPlacement {
                key: entry.key().clone(),
            }
            .into());
        }
    };
    Ok(match entry.placement() {
        PassPlacement::Before(_) => PipelineEdit::before(anchor, pass),
        PassPlacement::After(_) => PipelineEdit::after(anchor, pass),
        PassPlacement::Replace(_) => PipelineEdit::replace(anchor, pass),
        PassPlacement::Intrinsic => unreachable!("intrinsic placement returned above"),
    })
}

fn invalid_shape<T>(entry: &PassEntry, detail: &'static str) -> Result<T, PassTierCompileError> {
    Err(PassTierExecutionError::Shape {
        key: entry.key().clone(),
        detail,
    }
    .into())
}
