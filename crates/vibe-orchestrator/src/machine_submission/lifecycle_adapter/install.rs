//! Empty-world Install entry, pure preflight and read-only environment.

use super::*;
use vibe_core::manifest::{CURRENT_SCHEMA_VERSION, Lockfile};
use vibe_lifecycle::process::{PackageTreeWitness, capture_selected_manifest_lock};

pub(super) const FIXTURE_PIN: &str = "flow:org.example/fixture@=1.1.0";
pub(super) const FIXTURE_FILES: &[&str] = &["vibe.toml", "README.md"];
pub(super) const FIXTURE_PROJECT_MANIFEST: &str = "[project]\nname = 'consumer'\nversion = '1.1.0'\n[requires.packages]\n'flow:org.example/fixture' = '=1.1.0'\n";
pub(super) const FIXTURE_PACKAGE_MANIFEST: &str =
    "[package]\ngroup = 'org.example'\nname = 'fixture'\nkind = 'flow'\nversion = '1.1.0'\n";

/// Test harness admission only. Production PackageSourceFactory has no
/// provenance method; this claim is checked against the selected source path
/// here and backed by the controlled fixture implementation in tests.
#[cfg(test)]
pub(crate) trait ControlledFixtureFactory: PackageSourceFactory {
    fn serving_source_root(&self) -> &Path;
}

/// Test-only admission of one exact offline source. The source factory is a
/// controlled test port; no production surface may infer its provenance from
/// this fixture's manifest or from the tree witness alone.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn submit_one_package_fixture_install<F: ControlledFixtureFactory>(
    path: &Path,
    source_root: &Path,
    factory: &F,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    request: DefaultLifecycleRequest,
    target_os: TargetOs,
    observed_at: Timestamp,
) -> Result<SubmissionOutcome<RetainedLifecycleResult>> {
    submit_one_package_fixture_install_with_hook(
        path,
        source_root,
        factory,
        namespace,
        scope,
        key,
        request,
        target_os,
        observed_at,
        None,
        None,
    )
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn submit_one_package_fixture_install_with_hook<'a, F: ControlledFixtureFactory>(
    path: &Path,
    source_root: &'a Path,
    factory: &'a F,
    namespace: &'a SubmissionNamespace,
    scope: &'a SubmissionScope,
    key: &'a str,
    request: DefaultLifecycleRequest,
    target_os: TargetOs,
    observed_at: Timestamp,
    after_stage: Option<&'a dyn Fn()>,
    confirm: Option<&'a dyn ConfirmGate>,
) -> Result<SubmissionOutcome<RetainedLifecycleResult>> {
    ensure!(
        factory.serving_source_root() == source_root,
        "controlled fixture source root differs from witnessed root"
    );
    submit_protected_lifecycle_with_fixture(
        path,
        namespace,
        scope,
        key,
        request,
        target_os,
        observed_at,
        Some(FixtureSourcePort {
            source_root,
            factory,
            after_stage,
            confirm,
        }),
    )
}

pub(super) fn require_one_package_fixture_world(
    selection: &PreparedSelection,
    witness: &SelectedManifestLockWitness,
    source: &PackageTreeWitness,
) -> Result<()> {
    let manifest = selection
        .parsed_ref()
        .context("selected manifest did not parse")?;
    let pin = vibe_core::PackageRef::parse(FIXTURE_PIN)?;
    ensure!(source.package_pin == pin, "fixture source pin differs");
    let expected_project = vibe_core::manifest::Manifest::parse_str(FIXTURE_PROJECT_MANIFEST)?;
    ensure!(
        manifest == &expected_project,
        "one-package fixture refuses unreviewed root manifest declarations"
    );
    require_empty_lock(witness)?;
    let source_files = capture_selected_manifest_lock(&source.root)?;
    ensure!(
        source_files.lock.is_none(),
        "fixture source lock is not allowed"
    );
    let source_manifest = source
        .files
        .iter()
        .find(|file| file.relative_path == "vibe.toml")
        .context("source witness lacks package manifest")?;
    ensure!(
        source_manifest.bytes == source_files.manifest.size
            && source_manifest.sha256 == source_files.manifest.sha256,
        "fixture source manifest differs from tree witness"
    );
    let source_text = std::str::from_utf8(&source_files.manifest.bytes)?;
    let package_manifest = vibe_core::manifest::Manifest::parse_str(source_text)?;
    let package = package_manifest.require_package()?;
    let expected_package = vibe_core::manifest::Manifest::parse_str(FIXTURE_PACKAGE_MANIFEST)?;
    ensure!(
        package.as_package_ref()? == source.package_pin && package_manifest == expected_package,
        "fixture package carries unreviewed declarations"
    );
    Ok(())
}

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
    require_empty_lock(witness)
}

fn require_empty_lock(witness: &SelectedManifestLockWitness) -> Result<()> {
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
struct AcceptOneFixture;
impl ConfirmGate for AcceptOneFixture {
    fn confirm_install(&self, packages: usize) -> Result<()> {
        ensure!(
            packages == 1,
            "fixture confirmation expected exactly one package"
        );
        Ok(())
    }
}

pub(super) fn one_package_fixture_ports<'a>(
    factory: &'a dyn PackageSourceFactory,
    confirm: Option<&'a dyn ConfirmGate>,
) -> DefaultLifecyclePorts<'a> {
    let agent: Arc<dyn AgentBackend> = Arc::new(vibe_lifecycle::NoAgentBackend);
    DefaultLifecyclePorts {
        observer: &SilentRun,
        install_observer: &SilentInstall,
        confirm_gate: confirm.unwrap_or(&AcceptOneFixture),
        sources: factory,
        environment: &ReadOnlyEmptyEnvironment,
        manifest_mutation: &NoManifestMutation,
        agent,
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
