//! Bounded Install entries, pure preflight and read-only environment.

use super::*;
use vibe_core::manifest::{CURRENT_SCHEMA_VERSION, Lockfile};
use vibe_lifecycle::process::{PackageTreeWitness, capture_selected_manifest_lock};

/// Read-only, exact local Install comparison bytes for a typed question.
/// The eventual submit must repeat these checks under its own incumbent lease.
pub(in crate::machine_submission) struct LocalInstallPreflight {
    pub canonical_request: Vec<u8>,
    pub canonical_plan: Vec<u8>,
    pub source: PackageTreeWitness,
}

pub(in crate::machine_submission) fn preflight_one_package_local_install_under_lease(
    leased: &LeasedDefaultLifecycle,
    factory: &dyn BoundOnePackageSource,
    target_os: TargetOs,
) -> Result<LocalInstallPreflight> {
    let lease = leased.retain_lease();
    let source = factory.source_witness().clone();
    factory.recheck_source_binding(&source)?;
    recheck_package_tree_witness(&source)?;
    let selected_root = leased.selected_root();
    let (snapshot, witness) = SelectedManifest::read_witnessed(selected_root)?;
    let selection = snapshot.prepare();
    ensure!(
        selection.loaded_root() == Some(lease.root())
            && selection
                .loaded_workspace()
                .is_some_and(|world| world.iter_nodes().count() == 1)
            && !selection.request(false)
            && selected_under_lease(&selection, &lease)? == ".",
        "local Install question requires one untraced root node"
    );
    require_one_package_fixture_world(&selection, &witness, &source)?;
    ensure!(
        LifecycleStateStore::peek_with_lease(&lease)?.is_none(),
        "local Install question refuses prior lifecycle state"
    );
    let request = DefaultLifecycleRequest {
        requested: Phase::Install,
        force: false,
        agent_mode: RunAgentMode::Cli,
        assume_yes: true,
        trace_flag: false,
        install_inputs: Default::default(),
        policy: crate::install::InstallPolicy {
            offline: true,
            slot_integrity: SlotIntegrity::Verify,
            ..Default::default()
        },
    };
    let canonical_request = canonical_one_package_fixture_request(&source, target_os)?;
    let canonical_plan = canonical_lifecycle_pre_effect_plan(&LifecyclePreEffectPlanInput {
        lease_root: lease.root(),
        selected_root,
        selected_node: ".",
        witness: &witness,
        workspace_witness: &witness,
        package_source: Some(&source),
        script_source: None,
        prompt_source: None,
        requested: Phase::Install,
        agent_mode: &request.agent_mode,
        trace_flag: false,
        effective_trace: false,
        force: false,
        assume_yes: true,
        policy: &request.policy,
        install_inputs: &request.install_inputs,
    })?;
    factory.recheck_source_binding(&source)?;
    recheck_package_tree_witness(&source)?;
    Ok(LocalInstallPreflight {
        canonical_request,
        canonical_plan,
        source,
    })
}

#[cfg(test)]
pub(super) const FIXTURE_PIN: &str = "flow:org.example/fixture@=1.1.0";
#[cfg(test)]
pub(super) const FIXTURE_FILES: &[&str] = &["vibe.toml", "README.md"];
#[cfg(test)]
pub(super) const FIXTURE_PROJECT_MANIFEST: &str = "[project]\nname = 'consumer'\nversion = '1.1.0'\n[requires.packages]\n'flow:org.example/fixture' = '=1.1.0'\n";
#[cfg(test)]
pub(super) const FIXTURE_PACKAGE_MANIFEST: &str =
    "[package]\ngroup = 'org.example'\nname = 'fixture'\nkind = 'flow'\nversion = '1.1.0'\n";

/// Internal admission of one exact offline source. The host's bound factory
/// proves the served tree; typed machine approval is a separate requirement
/// before this route can be exposed to a wire adapter.
#[allow(clippy::too_many_arguments)]
pub(super) fn submit_one_package_local_install(
    path: &Path,
    factory: &dyn BoundOnePackageSource,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
    request: DefaultLifecycleRequest,
    target_os: TargetOs,
    observed_at: Timestamp,
) -> Result<SubmissionOutcome<RetainedLifecycleResult>> {
    submit_protected_lifecycle_with_fixture(
        path,
        namespace,
        scope,
        key,
        request,
        target_os,
        observed_at,
        Some(FixtureSourcePort {
            factory,
            after_stage: None,
            confirm: None,
        }),
    )
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn submit_one_package_fixture_install_with_hook<'a>(
    path: &Path,
    source_root: &'a Path,
    factory: &'a dyn BoundOnePackageSource,
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
        factory.source_witness().root == source_root,
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
    let pin = &source.package_pin;
    let version_spec = pin.version.to_string();
    let version = version_spec
        .strip_prefix('=')
        .context("local source pin is not exact")?;
    ensure!(
        pin.group.is_some() && pin.kind.is_some() && !version.is_empty(),
        "local source pin needs an exact qualified kind"
    );
    let project = manifest
        .project
        .as_ref()
        .context("local Install requires a project")?;
    ensure!(
        project.group.is_none()
            && project.spec_format.is_none()
            && project.sources.is_empty()
            && project.authors.is_empty()
            && !project.name.is_empty()
            && !project.version.is_empty(),
        "one-package local Install requires a simple project identity"
    );
    let mut expected_project =
        vibe_core::manifest::Manifest::parse_str("[project]\nname = 'bound'\nversion = '1.1.0'\n")?;
    expected_project.project = Some(project.clone());
    expected_project.requires.packages.push(pin.clone());
    ensure!(
        manifest == &expected_project,
        "one-package local Install refuses unreviewed root manifest declarations"
    );
    require_empty_lock(witness)?;
    let source_files = capture_selected_manifest_lock(&source.root)?;
    ensure!(
        source_files.lock.is_none(),
        "local source lock is not allowed"
    );
    let source_manifest = source
        .files
        .iter()
        .find(|file| file.relative_path == "vibe.toml")
        .context("source witness lacks package manifest")?;
    ensure!(
        source_manifest.bytes == source_files.manifest.size
            && source_manifest.sha256 == source_files.manifest.sha256,
        "local source manifest differs from tree witness"
    );
    let source_text = std::str::from_utf8(&source_files.manifest.bytes)?;
    let package_manifest = vibe_core::manifest::Manifest::parse_str(source_text)?;
    let package = package_manifest.require_package()?;
    let expected_package = vibe_core::manifest::Manifest::parse_str(&format!(
        "[package]\ngroup = '{}'\nname = '{}'\nkind = '{}'\nversion = '{}'\n",
        pin.group.as_ref().expect("checked group"),
        pin.name,
        pin.kind.as_ref().expect("checked kind"),
        version,
    ))?;
    ensure!(
        package.as_package_ref()? == source.package_pin && package_manifest == expected_package,
        "local package carries unreviewed declarations"
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

pub(super) fn require_empty_lock(witness: &SelectedManifestLockWitness) -> Result<()> {
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
