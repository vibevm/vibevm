//! Empty-world Install entry, pure preflight and read-only environment.

use super::*;
use vibe_core::manifest::{CURRENT_SCHEMA_VERSION, Lockfile};

/// An internal empty-world Install run. The preflight admits no dependency or
/// authored provider, and its registry environment is constructed read-only.
#[allow(clippy::too_many_arguments)]
pub(crate) fn submit_empty_world_install(
    path: &Path,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    request: DefaultLifecycleRequest,
    target_os: TargetOs,
    observed_at: Timestamp,
) -> Result<SubmissionOutcome<RetainedLifecycleResult>> {
    submit_protected_lifecycle(
        path,
        namespace,
        scope,
        key,
        request,
        target_os,
        observed_at,
        Phase::Install,
    )
}

pub(super) fn require_empty_install_world(
    selection: &PreparedSelection,
    witness: &SelectedManifestLockWitness,
) -> Result<()> {
    let manifest = selection
        .parsed_ref()
        .context("selected manifest did not parse")?;
    ensure!(
        manifest.project.is_some()
            && manifest.package.is_none()
            && manifest.requires.is_empty()
            && manifest.requires_any.is_empty()
            && manifest.embedded_sources.is_empty()
            && manifest.extensions.is_empty()
            && manifest.extension_controls.is_empty()
            && manifest.mechanism_decls.is_empty()
            && manifest.mechanism_routes.is_empty()
            && manifest.hooks.is_empty()
            && manifest.artifacts.is_none()
            && manifest.binaries.is_empty()
            && manifest.mcp_servers.is_empty()
            && manifest.run_commands.is_empty()
            && manifest.deploy.is_none()
            && manifest.conditional_deps.is_empty()
            && manifest.registries.is_empty()
            && manifest.mirrors.is_empty()
            && manifest.overrides.is_empty()
            && manifest.override_table.is_none()
            && manifest.application_source.is_none()
            && manifest.llm.is_none(),
        "protected empty-world install refuses requirements and provider declarations"
    );
    if let Some(lock) = &witness.lock {
        let text = std::str::from_utf8(&lock.bytes).context("selected lock is not UTF-8")?;
        let parsed: Lockfile = toml::from_str(text).context("selected lock is malformed")?;
        ensure!(
            parsed.meta.schema_version == CURRENT_SCHEMA_VERSION
                && parsed.packages.is_empty()
                && parsed.meta.root_dependencies.is_empty(),
            "protected empty-world install refuses preexisting lock dependencies or schema"
        );
    }
    Ok(())
}

/// A fully in-memory registry snapshot. Empty-world install does not build a
/// package source, so it cannot consult an ambient registry or seed defaults.
struct ReadOnlyEmptyEnvironment;
impl RegistryEnvironment for ReadOnlyEmptyEnvironment {
    fn prepare(&self) -> Result<RegistryEnvironmentSnapshot> {
        Ok(RegistryEnvironmentSnapshot {
            embedded_root: None,
            global: vibe_core::GlobalRegistryConfig::default(),
        })
    }
}
pub(super) fn empty_install_ports<'a>() -> DefaultLifecyclePorts<'a> {
    let agent: Arc<dyn AgentBackend> = Arc::new(vibe_lifecycle::NoAgentBackend);
    DefaultLifecyclePorts {
        observer: &SilentRun,
        install_observer: &SilentInstall,
        confirm_gate: &RejectInstall,
        sources: &RejectInstall,
        environment: &ReadOnlyEmptyEnvironment,
        manifest_mutation: &NoManifestMutation,
        agent,
    }
}
