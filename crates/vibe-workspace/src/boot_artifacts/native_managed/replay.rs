use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use vibe_core::manifest::SpecFormat;
use vibe_spec::{FsSectionSource, SelfCoordinate, finalize_compiler_pending_artifact};

use crate::WorkspaceError;
use crate::boot::EffectiveBoot;
use crate::extension_world::{OwnerNativeCompileProvider, OwnerRuntimeView};

use super::{
    OwnerManagedStaticCompile, OwnerNativeCompileMode, compile_static_owner_managed_using,
};

#[allow(clippy::too_many_arguments)]
pub(crate) fn compile_static_owner_managed_with_source<P: OwnerNativeCompileProvider>(
    boot: &EffectiveBoot,
    workspace_root: &Path,
    self_coord: &SelfCoordinate,
    spec_format: SpecFormat,
    owner: OwnerRuntimeView<'_>,
    source: &FsSectionSource,
    overlay: &BTreeMap<PathBuf, Arc<[u8]>>,
    provider: Option<&mut P>,
) -> Result<Option<OwnerManagedStaticCompile>, WorkspaceError> {
    compile_static_owner_managed_using(
        boot,
        workspace_root,
        self_coord,
        spec_format,
        owner,
        OwnerNativeCompileMode::Plain,
        provider,
        Some(source),
        Some(overlay),
        finalize_compiler_pending_artifact,
    )
}
