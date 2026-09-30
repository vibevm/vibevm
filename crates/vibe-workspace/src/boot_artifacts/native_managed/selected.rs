//! Install-only native-managed reader with exact selected slot witnesses.

use std::path::Path;

use vibe_core::manifest::SpecFormat;
use vibe_spec::SelfCoordinate;

use crate::WorkspaceError;
use crate::boot::EffectiveBoot;
use crate::extension_world::{OwnerNativeCompileProvider, OwnerRuntimeView};

use super::{
    OwnerManagedStaticCompile, OwnerNativeCompileMode, compile_static_owner_managed_using,
    finalize_compiler_pending_artifact,
};
use crate::boot_artifacts::SelectedSlotWitness;

#[allow(
    clippy::too_many_arguments,
    reason = "one selected install-owner read boundary"
)]
pub(crate) fn compile_static_owner_managed_selected<P: OwnerNativeCompileProvider>(
    boot: &EffectiveBoot,
    workspace_root: &Path,
    self_coord: &SelfCoordinate,
    spec_format: SpecFormat,
    owner: OwnerRuntimeView<'_>,
    mode: OwnerNativeCompileMode<'_>,
    provider: Option<&mut P>,
    witnesses: &[SelectedSlotWitness],
) -> Result<Option<OwnerManagedStaticCompile>, WorkspaceError> {
    compile_static_owner_managed_using(
        boot,
        workspace_root,
        self_coord,
        spec_format,
        owner,
        mode,
        provider,
        None,
        None,
        Some(witnesses),
        finalize_compiler_pending_artifact,
    )
}
