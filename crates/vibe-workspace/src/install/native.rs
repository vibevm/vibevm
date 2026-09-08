//! Opaque native install epoch, all-owner build plan, and replay continuation.

use super::{ResolvedDep, Workspace, WorkspaceError, bootgen, validate_redirect_blocks};
use crate::extension_world::{OwnerRuntimeEpoch, OwnerRuntimeId};
use vibe_core::manifest::SpecFormat;

pub struct NativeInstallCarriage {
    epoch: OwnerRuntimeEpoch,
    build_owners: Box<[OwnerRuntimeId]>,
    replay: bootgen::replay_prepare::BootReplaySet,
}

impl NativeInstallCarriage {
    pub(crate) fn new(
        epoch: OwnerRuntimeEpoch,
        replay: bootgen::replay_prepare::BootReplaySet,
    ) -> Result<Self, WorkspaceError> {
        let mut build_owners = Vec::new();
        for (rel, runtime) in epoch.lowered().nodes() {
            if !runtime.rows()?.native().is_empty() {
                build_owners.push(OwnerRuntimeId::Node { rel: rel.clone() });
            }
        }
        for (provider, runtime) in epoch.lowered().units() {
            if !runtime.rows()?.native().is_empty() {
                build_owners.push(OwnerRuntimeId::Unit {
                    provider: provider.clone(),
                });
            }
        }
        Ok(Self {
            epoch,
            build_owners: build_owners.into_boxed_slice(),
            replay,
        })
    }

    #[must_use]
    pub const fn epoch(&self) -> &OwnerRuntimeEpoch {
        &self.epoch
    }

    #[must_use]
    pub fn build_owners(&self) -> &[OwnerRuntimeId] {
        &self.build_owners
    }

    pub fn replay<F>(self, factory: &mut F) -> Result<(), NativeInstallReplayError>
    where
        F: crate::extension_world::CompilerNativeReplayFactory,
    {
        let prepared =
            bootgen::replay_prepare::prepare_boot_replay(self.replay, &self.epoch, factory)
                .map_err(NativeInstallReplayError::prepare)?;
        bootgen::replay_publish::publish_boot_replay(prepared)
            .map(|_| ())
            .map_err(NativeInstallReplayError::publish)
    }

    #[must_use]
    pub const fn replay_is_empty(&self) -> bool {
        matches!(self.replay, bootgen::replay_prepare::BootReplaySet::Empty)
    }

    #[cfg(test)]
    pub(crate) const fn replay_is_empty_for_test(&self) -> bool {
        self.replay_is_empty()
    }
}

#[derive(Debug)]
pub struct NativeInstallReplayError {
    inner: NativeInstallReplayErrorInner,
}

#[derive(Debug)]
enum NativeInstallReplayErrorInner {
    Prepare(WorkspaceError),
    Publish(bootgen::replay_publish::BootReplayPublishFailure),
}

impl NativeInstallReplayError {
    fn prepare(source: WorkspaceError) -> Self {
        Self {
            inner: NativeInstallReplayErrorInner::Prepare(source),
        }
    }

    fn publish(source: bootgen::replay_publish::BootReplayPublishFailure) -> Self {
        Self {
            inner: NativeInstallReplayErrorInner::Publish(source),
        }
    }
}

impl std::fmt::Display for NativeInstallReplayError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.inner {
            NativeInstallReplayErrorInner::Prepare(source) => {
                write!(
                    formatter,
                    "preparing native compiler replay failed: {source}"
                )
            }
            NativeInstallReplayErrorInner::Publish(source) => source.fmt(formatter),
        }
    }
}

impl std::error::Error for NativeInstallReplayError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.inner {
            NativeInstallReplayErrorInner::Prepare(source) => Some(source),
            NativeInstallReplayErrorInner::Publish(source) => Some(source),
        }
    }
}

pub struct NativeInstallOutcome {
    pub outcome: super::InstallOutcome,
    pub carriage: NativeInstallCarriage,
}

#[allow(clippy::too_many_arguments)]
pub fn regenerate_boot_from_traced_native<F, P>(
    workspace: &Workspace,
    resolution: &[ResolvedDep],
    world: crate::extension_world::ExtensionWorldEpoch,
    spec_format: SpecFormat,
    trace: Option<&crate::compile_trace::TraceRun>,
    lowering: crate::extension_world::OwnerRuntimeLowering,
    run: crate::extension_world::OwnerRuntimeRunFacts,
    make_provider: &mut F,
) -> Result<(Vec<String>, NativeInstallCarriage), WorkspaceError>
where
    F: FnMut(
        std::collections::BTreeMap<OwnerRuntimeId, vibe_spec::CompilerNativePolicy>,
    ) -> Result<P, WorkspaceError>,
    P: crate::extension_world::OwnerNativeCompileProvider,
{
    validate_redirect_blocks(workspace)?;
    bootgen::regenerate_boot_from_traced_native_prepared(
        workspace,
        resolution,
        world,
        spec_format,
        trace,
        lowering,
        run,
        make_provider,
    )
}
