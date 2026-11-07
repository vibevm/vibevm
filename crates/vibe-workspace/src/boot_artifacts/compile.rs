//! The single fallible artifact preparation/compile boundary, without publication.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#artifacts");
use super::*;

/// [`render_static_with_spec_format`] through one not-yet-acquired trace
/// occurrence. `acquisition` is the borrowed run plus the base identity a
/// traced caller minted for THIS artifact; the occurrence itself is acquired
/// at the compile boundary inside. `None` is the historical untraced path,
/// byte-for-byte.
///
/// `plans` is the lane owner's retained compile plan (R4 architecture §5.1:
/// activation authority follows the artifact being written), already lowered
/// by the caller that holds the world. The per-unit path passes THAT
/// package's view; a caller with no world passes
/// [`CompilePlans::transforms_only`].
pub(crate) fn render_static_observed(
    boot: &EffectiveBoot,
    workspace_root: &Path,
    self_coord: &SelfCoordinate,
    spec_format: SpecFormat,
    sources: Option<&DeclaredBootSources>,
    acquisition: Option<&ScopeAcquisition<'_>>,
    plans: CompilePlans,
) -> Result<Option<String>, WorkspaceError> {
    let compiled = compile_static_artifact_with_plans(
        boot,
        workspace_root,
        self_coord,
        spec_format,
        sources,
        acquisition,
        plans,
        None,
        compile_artifact,
    )?;
    compiled
        .map(|compiled| {
            String::from_utf8(compiled.artifact.into_bytes()).map_err(|error| {
                WorkspaceError::InlineCompile {
                    lane: static_path(spec_format).to_string(),
                    reason: format!(
                        "the built-in STATIC backend returned non-UTF-8 bytes: {error}"
                    ),
                }
            })
        })
        .transpose()
}

/// Compile one node's static lane exactly once, traced or not.
///
/// The trace law of this seam (R3.4): no static entries means no scope AND no
/// compiler, exactly as before; an acquisition present means the occurrence is
/// Acquires tracing only after fallible preparation and immediately before
/// the compiler. The injected compiler is used only by characterization tests;
/// observed/traced paths retain the real built-in schedule.
#[allow(clippy::too_many_arguments)]
#[cfg(test)]
pub(crate) fn compile_static_artifact_with(
    boot: &EffectiveBoot,
    workspace_root: &Path,
    self_coord: &SelfCoordinate,
    spec_format: SpecFormat,
    acquisition: Option<&ScopeAcquisition<'_>>,
    transforms: TransformPlan,
    observer: Option<std::sync::Arc<dyn vibe_spec::CompileObserver>>,
    compiler: impl FnOnce(
        ArtifactPlan,
        &BootSectionSource,
    ) -> Result<EmittedArtifact, ArtifactCompileError>,
) -> Result<Option<StaticCompile>, WorkspaceError> {
    compile_static_artifact_with_plans(
        boot,
        workspace_root,
        self_coord,
        spec_format,
        None,
        acquisition,
        CompilePlans::transforms_only(transforms),
        observer,
        compiler,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn compile_static_artifact_with_plans(
    boot: &EffectiveBoot,
    workspace_root: &Path,
    self_coord: &SelfCoordinate,
    spec_format: SpecFormat,
    sources: Option<&DeclaredBootSources>,
    acquisition: Option<&ScopeAcquisition<'_>>,
    plans: CompilePlans,
    observer: Option<std::sync::Arc<dyn vibe_spec::CompileObserver>>,
    compiler: impl FnOnce(
        ArtifactPlan,
        &BootSectionSource,
    ) -> Result<EmittedArtifact, ArtifactCompileError>,
) -> Result<Option<StaticCompile>, WorkspaceError> {
    let entries: Vec<_> = boot.static_entries().collect();
    if entries.is_empty() {
        return Ok(None);
    }
    let source =
        BootSectionSource::new(workspace_root, self_coord, boot, spec_format, sources, None)?;
    let (inputs, providers) = inputs::build_with_providers_bound(
        entries,
        workspace_root,
        self_coord,
        static_path(spec_format),
        sources,
        None,
        &source,
    )?;
    let target = if matches!(spec_format, SpecFormat::Xml) {
        ArtifactTarget::StaticXml
    } else {
        ArtifactTarget::StaticMarkdown
    };
    let plan = plans.attach_to(
        ArtifactPlan::static_lane(
            target,
            static_path(spec_format),
            layout_paths::vibedeps(""),
            inputs,
        )
        .map_err(|error| WorkspaceError::InlineCompile {
            lane: static_path(spec_format).to_string(),
            reason: error.to_string(),
        })?,
    );
    // THE COMPILE BOUNDARY. Every fallible preparation above has completed;
    // the next thing that happens is the compiler. Only now is the occurrence
    // acquired — and a refusal to declare it simply compiles untraced.
    let scope = acquisition.and_then(ScopeAcquisition::acquire);
    // The ONE compiler call: through the observer when analyzing, through
    // the acquired sink when tracing, through the historical path when
    // neither. Its result — never a second compile — supplies the scope's
    // terminal word, the observer's evidence and the artifact.
    let compiled = match (observer, &scope) {
        (Some(observer), _) => vibe_spec::compile_artifact_observed(plan, &source, observer),
        (None, Some(scope)) => compile_artifact_traced(plan, &source, scope),
        (None, None) => compiler(plan, &source),
    };
    match compiled {
        Ok(artifact) => {
            if let Some(scope) = &scope {
                scope.complete_lossy(&artifact.output_fingerprint());
            }
            Ok(Some(StaticCompile {
                artifact,
                providers,
            }))
        }
        Err(error) => {
            if let Some(scope) = &scope {
                scope.fail_lossy(&error.to_string());
            }
            let reason = match error {
                ArtifactCompileError::Input { input, source }
                    if input.kind == ArtifactInputType::Normal =>
                {
                    format!(
                        "compiling the normal package `{}` closure (PROP-035 §8): {source}",
                        input.origin
                    )
                }
                error => error.to_string(),
            };
            Err(WorkspaceError::InlineCompile {
                lane: static_path(spec_format).to_string(),
                reason,
            })
        }
    }
}
