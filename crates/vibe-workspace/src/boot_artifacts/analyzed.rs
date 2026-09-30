//! The R4.3 analyzer's compile half (`boot_artifacts`' file-length
//! split): the observed compile spelling and the compile result that
//! carries its attribution side. Everything here delegates to the one
//! compile boundary in the parent cell; nothing writes.

use std::path::Path;

use vibe_core::manifest::SpecFormat;
use vibe_spec::{CompileObserver, CompilePlans, DocumentProvider, EmittedArtifact, TransformPlan};

use crate::errors::WorkspaceError;

use super::{
    EffectiveBoot, SelectedSlotWitness, SelfCoordinate, compile_artifact,
    compile_static_artifact_with_plans,
};

/// One static lane's compile result: the emitted artifact BESIDE the
/// typed provider that declared each input, in input order — the
/// attribution side only the analyzer entry reads; every write path
/// takes the artifact and drops the providers.
pub(crate) struct StaticCompile {
    pub(crate) artifact: EmittedArtifact,
    pub(crate) providers: Vec<Option<DocumentProvider>>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn compile_static_analyzed_selected(
    boot: &EffectiveBoot,
    workspace_root: &Path,
    self_coord: &SelfCoordinate,
    spec_format: SpecFormat,
    transforms: TransformPlan,
    observer: Option<std::sync::Arc<dyn CompileObserver>>,
    witnesses: &[SelectedSlotWitness],
) -> Result<Option<StaticCompile>, WorkspaceError> {
    compile_static_artifact_with_plans(
        boot,
        workspace_root,
        self_coord,
        spec_format,
        Some(witnesses),
        None,
        CompilePlans::transforms_only(transforms),
        observer,
        compile_artifact,
    )
}
