//! Publication around one prepared owner-managed artifact and its continuation.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION");
use super::*;

#[allow(clippy::too_many_arguments)]
#[cfg(test)]
pub(crate) fn write_boot_artifacts_owner_managed<P: OwnerNativeCompileProvider>(
    node_dir: &Path,
    node_rel: &str,
    workspace_root: &Path,
    self_coord: &SelfCoordinate,
    boot: &EffectiveBoot,
    spec_format: SpecFormat,
    trace: Option<&TraceRun>,
    owner: OwnerRuntimeView<'_>,
    provider: Option<&mut P>,
) -> Result<(WrittenArtifacts, Option<OwnerNativeCompileContinuation>), WorkspaceError> {
    write_boot_artifacts_owner_managed_using(
        node_dir,
        node_rel,
        workspace_root,
        self_coord,
        boot,
        spec_format,
        None,
        trace,
        owner,
        provider,
        finalize_compiler_pending_artifact,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn write_boot_artifacts_owner_managed_with_sources<P: OwnerNativeCompileProvider>(
    node_dir: &Path,
    node_rel: &str,
    workspace_root: &Path,
    self_coord: &SelfCoordinate,
    boot: &EffectiveBoot,
    spec_format: SpecFormat,
    sources: Option<&DeclaredBootSources>,
    trace: Option<&TraceRun>,
    owner: OwnerRuntimeView<'_>,
    provider: Option<&mut P>,
    prompt: Option<NodePrompt<'_>>,
) -> Result<(WrittenArtifacts, Option<OwnerNativeCompileContinuation>), WorkspaceError> {
    write_boot_artifacts_owner_managed_using(
        node_dir,
        node_rel,
        workspace_root,
        self_coord,
        boot,
        spec_format,
        sources,
        trace,
        owner,
        provider,
        finalize_compiler_pending_artifact,
        prompt,
    )
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn write_boot_artifacts_owner_managed_with_finalizer<P: OwnerNativeCompileProvider>(
    node_dir: &Path,
    node_rel: &str,
    workspace_root: &Path,
    self_coord: &SelfCoordinate,
    boot: &EffectiveBoot,
    spec_format: SpecFormat,
    trace: Option<&TraceRun>,
    owner: OwnerRuntimeView<'_>,
    provider: Option<&mut P>,
    finalizer: PendingFinalizer,
) -> Result<(WrittenArtifacts, Option<OwnerNativeCompileContinuation>), WorkspaceError> {
    write_boot_artifacts_owner_managed_using(
        node_dir,
        node_rel,
        workspace_root,
        self_coord,
        boot,
        spec_format,
        None,
        trace,
        owner,
        provider,
        finalizer,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn write_boot_artifacts_owner_managed_using<P: OwnerNativeCompileProvider>(
    node_dir: &Path,
    node_rel: &str,
    workspace_root: &Path,
    self_coord: &SelfCoordinate,
    boot: &EffectiveBoot,
    spec_format: SpecFormat,
    sources: Option<&DeclaredBootSources>,
    trace: Option<&TraceRun>,
    owner: OwnerRuntimeView<'_>,
    provider: Option<&mut P>,
    finalizer: PendingFinalizer,
    prompt: Option<NodePrompt<'_>>,
) -> Result<(WrittenArtifacts, Option<OwnerNativeCompileContinuation>), WorkspaceError> {
    let control = provider
        .as_ref()
        .and_then(|provider| provider.cancellation())
        .cloned();
    let has_static = boot.static_entries().next().is_some();
    let acquisition = trace
        .filter(|_| has_static)
        .map(|run| ScopeAcquisition::node(run, node_rel, spec_format));
    let mode = match acquisition.as_ref() {
        Some(acquisition) => OwnerNativeCompileMode::Traced(acquisition),
        None => OwnerNativeCompileMode::Plain,
    };
    let index_text = render_index_with_spec_format(boot, None, spec_format)?;
    let compiled = compile_static_owner_managed_using(
        boot,
        workspace_root,
        self_coord,
        spec_format,
        sources,
        owner,
        mode,
        provider,
        None,
        None,
        finalizer,
    )?;
    let boot_dir = node_dir.join(layout::current_boot_dir());
    let index = boot_dir.join(INDEX_FILE);
    let static_path = boot_dir.join(static_file(spec_format));
    let stale_path = boot_dir.join(stale_static_file(spec_format));
    let static_bytes = compiled
        .as_ref()
        .map(|compiled| compiled.artifact().bytes());
    if let Some(sources) = sources {
        sources.recheck_node(node_dir)?;
    }
    // One generation, one set of router facts — the same rule as the builtin
    // seam, which is why the facts are a value and not a second rendering.
    // The Codex truncation limit this node declares — the block is measured
    // against the declaration rather than against a constant, because the
    // limit is the harness's setting and a project may raise it
    // (`[system_prompt.harness.codex] doc_max_bytes`).
    let codex_limit = prompt.map_or(redirect::PROJECT_DOC_MAX_BYTES, |prompt| {
        prompt
            .manifest
            .system_prompt
            .as_ref()
            .map_or(redirect::PROJECT_DOC_MAX_BYTES, |section| {
                section.codex_doc_max_bytes()
            })
    });
    let router = redirect::RouterFacts {
        node_id: redirect::router_node_id(prompt.map(|prompt| prompt.manifest), self_coord),
        static_hash12: static_bytes.map(crate::system_prompt::segment_hash12),
        spec_format,
    };
    // The same composition hook as the builtin seam, through the same
    // shared function: the seam is chosen once per regeneration, so a node
    // reaches exactly one of the two and both must publish the prefix
    // (PROP-076 `#SYS-LANE-VERBATIM`, `#SYS-TRANSACTION-ROLES`).
    let publication = super::super::prompt_publication(
        node_dir,
        boot,
        spec_format,
        sources,
        prompt,
        static_bytes,
        index_text.as_bytes(),
        &router,
    )?;
    let redirects = transaction::write_controlled_with_selectors(
        transaction::ArtifactWrite {
            index_path: &index,
            index_bytes: index_text.as_bytes(),
            static_path: &static_path,
            static_bytes,
            stale_path: &stale_path,
            prompt: super::super::prompt_payload(&publication),
        },
        control.as_ref(),
        |transaction| {
            redirect::write_redirect_blocks_with_transaction(
                node_dir,
                &router,
                codex_limit,
                transaction,
            )
        },
    )?;
    let static_lane = compiled.as_ref().map(|_| static_path);
    let native = compiled
        .map(OwnerManagedStaticCompile::into_parts)
        .and_then(|(_, native, _)| native);
    Ok((
        WrittenArtifacts {
            index,
            static_lane,
            redirects,
        },
        native,
    ))
}
