//! Verified pass-tier adapter over the one compiler-native manager.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE");

use std::collections::BTreeMap;
use std::marker::PhantomData;

use crate::compiler::pass::{IrPayload, Pass, PassName};
use crate::compiler::transform::native_identity::NativeHandlerIdentity;
use crate::compiler::transform::native_manager::{
    CompilerNativeInvoker, NativeEntry, NativeManagerError, NativeRuntime, execute,
};
use serde_json::Value;

use super::plan::PassEntry;

/// One pass-tier native invocation with a compile-time carrier contract.
pub(super) struct NativePass<'invoke, Input, Output> {
    invoker: &'invoke dyn CompilerNativeInvoker,
    key: vibe_core::manifest::ExtensionKey,
    order: u32,
    config: BTreeMap<String, Option<Value>>,
    implementation: crate::compiler::transform::native_identity::CompilerNativeImplementationDigest,
    name: PassName,
    marker: PhantomData<fn(Input) -> Output>,
}

impl<'invoke, Input, Output> NativePass<'invoke, Input, Output> {
    pub(super) fn from_entry(
        entry: &PassEntry,
        invoker: &'invoke dyn CompilerNativeInvoker,
        name: PassName,
    ) -> Result<Self, NativePassBuildError> {
        let identity = NativeHandlerIdentity::from_handler(entry.handler())?
            .ok_or(NativePassBuildError::NotNative)?;
        let config = project_config(entry.config())?;
        Ok(Self {
            invoker,
            key: entry.key().clone(),
            order: entry.ordinal(),
            config,
            implementation: identity.digest(),
            name,
            marker: PhantomData,
        })
    }
}

impl<Input, Output> Pass for NativePass<'_, Input, Output>
where
    Input: IrPayload,
    Output: IrPayload,
{
    type Input = Input;
    type Output = Output;
    type Error = NativePassError;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, input: Input) -> Result<Output, Self::Error> {
        let output = execute(
            NativeEntry::new_pass(
                NativeRuntime::new(self.invoker, None),
                &self.key,
                self.order,
                &self.config,
                self.implementation,
                &self.name,
                Output::SHAPE,
            ),
            input.into_any(),
        )?
        .into_parts()
        .0;
        Output::try_from_any(output).map_err(|_| NativePassError::WrongOutput)
    }
}

#[derive(Debug, thiserror::Error)]
pub(super) enum NativePassBuildError {
    #[error("pass-tier execution requires a native handler")]
    NotNative,
    #[error(transparent)]
    Identity(
        #[from]
        crate::compiler::transform::native_identity::CompilerNativeImplementationDigestError,
    ),
    #[error("the effective pass configuration has no JSON projection: {0}")]
    Config(serde_json::Error),
}

fn project_config(
    config: Option<&vibe_core::manifest::ExtensionConfig>,
) -> Result<BTreeMap<String, Option<Value>>, NativePassBuildError> {
    let Some(config) = config else {
        return Ok(BTreeMap::new());
    };
    config
        .as_table()
        .iter()
        .map(|(key, value)| {
            serde_json::to_value(value)
                .map(|value| (key.clone(), Some(value)))
                .map_err(NativePassBuildError::Config)
        })
        .collect()
}

#[derive(Debug, thiserror::Error)]
pub(super) enum NativePassError {
    #[error(transparent)]
    Manager(#[from] NativeManagerError),
    #[error("the native manager returned a carrier outside the pass's typed output")]
    WrongOutput,
}
