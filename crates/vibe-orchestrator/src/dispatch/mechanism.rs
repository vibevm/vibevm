//! The engine-owned mechanism fences: where a COMPLETE default-phase plan
//! executes its declared `[[artifacts.build]]` and `[[artifacts.package]]`
//! targets inside the one contribution walk. Engine work fires before that
//! phase's own rows; partial install epochs pass no targets and arm no fence.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use specmark::spec;
use vibe_core::manifest::{
    ArtifactBuildTarget, ArtifactPackageTarget, ArtifactsSection, BinaryDecl, DeployTarget,
    Manifest, MechanismRoutes, TargetOs, build_target_for_binary,
};
use vibe_lifecycle::native::{NativeBuildExecution, NativePlatform};
use vibe_lifecycle::{
    BuildExecution, ClientExecutables, DeployExecution, DeploySelection, MechanismRegistry,
    PackageExecution, Phase, deploy_state_home, execute_build_targets, execute_deploy_targets,
    execute_package_targets,
};

use crate::RitualPlan;
use crate::install::NativeInstallContext;

/// Everything one deploy dispatch's COMMAND SURFACE resolved, travelling as
/// data — §7.0.5's "travels as data" and §6.3.0.6's "Home and executable
/// authority are injected", which are one surface act and arrive together.
///
/// They are one value rather than three optional parameters because they
/// are decided in one cell, at one moment, and a dispatch that carried a
/// selection without a home (or the reverse) would be a run whose deploy
/// half is half-resolved — exactly the ambiguity `Option<Option<_>>` at the
/// flag boundary already taught this codebase to avoid.
///
/// ```
/// use std::path::PathBuf;
/// use vibe_orchestrator::DeployAuthority;
/// use vibe_lifecycle::{ClientExecutable, ClientExecutables, DeploySelection};
///
/// let authority = DeployAuthority {
///     selection: DeploySelection {
///         profile: "local".into(),
///         targets: vec!["local-helper".into()],
///     },
///     user_home: PathBuf::from("/home/u"),
///     clients: ClientExecutables {
///         claude: ClientExecutable::Resolved {
///             command: "claude".into(),
///             path: PathBuf::from("/opt/bin/claude"),
///         },
///         codex: ClientExecutable::Missing { command: "codex".into() },
///         opencode: ClientExecutable::Missing { command: "opencode".into() },
///     },
/// };
/// assert_eq!(authority.selection.profile, "local");
/// assert!(authority.user_home.ends_with("u"));
/// // Nothing below this value may search a path: a member is a resolved
/// // absolute executable or a named absence, never a command word.
/// assert!(authority.clients.claude.resolved_path().is_some());
/// ```
#[derive(Debug, Clone)]
pub struct DeployAuthority {
    /// The profile selection, resolved once by the layer that owns flags.
    pub selection: DeploySelection,
    /// The invoking user's home.
    pub user_home: PathBuf,
    /// The client executables this run may invoke.
    pub clients: ClientExecutables,
}

/// Everything the mechanism half of one dispatch needs.
///
/// A named value rather than a positional call, because most of it is
/// borrowed from three different places and two members of the same type
/// could otherwise be swapped without the compiler noticing.
pub(crate) struct MechanismTargets<'a> {
    /// The selected project's absolute root.
    pub(crate) project_root: &'a Path,
    /// The build target set, ALREADY lowered — authored
    /// `[[artifacts.build]]` rows plus every legacy `[[binary]]` row
    /// projected into one ([`lower_binaries`]). The executor therefore has
    /// no legacy case: it walks build targets.
    pub(crate) build: &'a [ArtifactBuildTarget],
    /// The declared `[[artifacts.package]]` rows.
    pub(crate) package: &'a [ArtifactPackageTarget],
    /// The declared `[[deploy.target]]` rows.
    pub(crate) deploy_targets: &'a [DeployTarget],
    /// The mechanism plane of the world this plan was collected from.
    pub(crate) registry: &'a MechanismRegistry,
    /// The host's `[mechanisms]` routes.
    pub(crate) routes: &'a MechanismRoutes,
    /// Enabled native rows from the exact registry epoch, in its one order.
    pub(crate) native_candidates: &'a [vibe_lifecycle::ExtensionRegistryRow],
    /// Platform selected before dispatch; complete chains recover it from the
    /// install epoch, while isolated build slices observe it at their surface.
    pub(crate) native_platform: Option<NativePlatform>,
    /// Exact install epoch and sealed replay, moved into this dispatch once.
    pub(crate) native: Option<NativeInstallContext>,
    pub(crate) native_mechanisms: vibe_lifecycle::native::NativeMechanismPlan,
    /// Command-observed OS whose projection produced package/deploy slices.
    pub(crate) target_os: TargetOs,
    /// The run's effective offline posture.
    pub(crate) offline: bool,
    /// The run's injected instant, in the RFC 3339 spelling every record
    /// carries. Nothing below a surface reads a clock.
    pub(crate) created_at: &'a str,
    /// The deploy half, present exactly when the command layer resolved a
    /// profile selection (§7.0.5). `None` is a dispatch that carries no
    /// selection, and the deploy fence then arms nothing at all.
    pub(crate) deploy: Option<&'a DeployCarriage>,
}

/// The deploy half of one dispatch's targets, assembled ONCE.
///
/// §7.0.5 puts profile resolution in the command layer that owns flags and
/// says the result "travels as data". [`DeployAuthority`] is that data,
/// arriving already resolved; the other members are what the engine adds
/// around it and could not be flags: where user deployment state lives
/// (§7.0.3) and which project's receipts these are.
pub(crate) struct DeployCarriage {
    /// The resolved selection, as the command layer decided it.
    pub(crate) selection: DeploySelection,
    /// The invoking user's home, as the command layer resolved it —
    /// §6.3.0.6. It is NOT derived from `settings_root`: `$VIBE_SETTINGS`
    /// relocates that root anywhere, while a client destination hangs off
    /// the home itself.
    pub(crate) user_home: PathBuf,
    /// The client executables this run may invoke, injected whole by the
    /// command layer. No cell below this one may look for a client.
    pub(crate) clients: ClientExecutables,
    /// The absolute deployment state home — `state/deployments` under the
    /// vibevm settings directory.
    pub(crate) state_home: PathBuf,
    /// The absolute vibevm settings directory that state home hangs off,
    /// carried beside it — §7.1.0 ruling 2. A user-scope deploy provider
    /// reconciles a destination inside this root (`bin/`, `store/`), and
    /// it is resolved in exactly the one place the state home already is,
    /// so no cell below this surface ever calls `settings_dir()`.
    pub(crate) settings_root: PathBuf,
    /// The project identity every intent and receipt is keyed under.
    pub(crate) project: String,
    /// The package identity, when a deployment comes from one package
    /// rather than the selected node. No atom produces one yet — a
    /// dependency-declared deploy target is the case the member exists
    /// for — so it is honestly absent rather than a second spelling of the
    /// project.
    pub(crate) package: Option<String>,
}

impl DeployCarriage {
    /// Assemble the carriage around one already-resolved authority.
    ///
    /// The settings directory is resolved HERE and nowhere below: the
    /// executor takes its state home as a parameter precisely so no engine
    /// cell reads the operator's home. The user home and the client
    /// executables are not resolved here at all — §6.3.0.6 puts that in the
    /// command surface, and they arrive inside [`DeployAuthority`].
    pub(crate) fn assemble(authority: DeployAuthority, manifest: &Manifest) -> Result<Self> {
        let settings = vibe_core::settings::settings_dir().ok_or_else(|| {
            anyhow::anyhow!(
                "the vibevm settings directory could not be resolved, so deployment intents and \
                 receipts have nowhere to live (violates \
                 spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: set \
                 `$VIBE_SETTINGS`, or make a home directory resolvable, then rerun)"
            )
        })?;
        Ok(Self {
            selection: authority.selection,
            user_home: authority.user_home,
            clients: authority.clients,
            state_home: deploy_state_home(&settings),
            settings_root: settings,
            project: project_identity(manifest),
            package: None,
        })
    }
}

/// The selected node's own identity, in the one spelling the extension
/// plane already renders hosts by.
fn project_identity(manifest: &Manifest) -> String {
    if let Some(package) = &manifest.package {
        return format!("{}/{}", package.group, package.name);
    }
    if let Some(project) = &manifest.project {
        return match &project.group {
            Some(group) => format!("{group}/{}", project.name),
            None => project.name.clone(),
        };
    }
    // A pure virtual workspace declares no deploy target (it declares no
    // provider identity at all), so this is the honest placeholder rather
    // than a name it does not have.
    "<workspace>".to_owned()
}

/// The build target set one dispatch executes: the authored
/// `[[artifacts.build]]` rows, then every legacy `[[binary]]` row lowered
/// through the R8-CARGO projection.
///
/// §7.0.7: "The same assembly that arms the fences lowers legacy
/// `[[binary]]` rows through the R8-CARGO projection into the build target
/// set; an id collision between a lowered row and an authored
/// `[[artifacts.build]]` row is a typed refusal (two claimants for one
/// identity), never a silent merge."
///
/// The collision check is over BOTH identities a target owns — its own id
/// and every output id it declares — because either one being claimed
/// twice makes the artifact graph ambiguous, and the projection mints both
/// from the binary's `name`.
///
/// Each lowered row JOINS the claimed set as it is added, so the law holds
/// among the lowered rows too. A manifest cannot reach that case (names
/// are unique within a package), but this function is handed a slice, and
/// a law that only held against authored rows would be a law about where
/// the duplicate came from rather than about the identity.
pub(crate) fn lower_binaries(
    artifacts: Option<&ArtifactsSection>,
    binaries: &[BinaryDecl],
) -> Result<Vec<ArtifactBuildTarget>> {
    let authored = artifacts.map_or(&[] as &[ArtifactBuildTarget], |section| &section.build);
    if binaries.is_empty() {
        return Ok(authored.to_vec());
    }
    let mut claimed: BTreeSet<&str> = BTreeSet::new();
    for target in authored {
        claimed.insert(target.id.as_str());
        for output in &target.outputs {
            claimed.insert(output.id.as_str());
        }
    }
    let mut lowered = authored.to_vec();
    for binary in binaries {
        if claimed.contains(binary.name.as_str()) {
            bail!(
                "the legacy `[[binary]]` row `{}` lowers into a build target whose identity an \
                 authored `[[artifacts.build]]` row already claims; two claimants for one \
                 identity are never merged \
                 (violates spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY; fix: \
                 rename the `[[binary]]`, or drop the authored row that duplicates it — a \
                 `[[binary]]` IS a build target after lowering, so declaring both is declaring \
                 the same producer twice)",
                binary.name,
            );
        }
        // The projection mints the target id AND its one output id from
        // `name`, so claiming the name claims both.
        claimed.insert(binary.name.as_str());
        lowered.push(build_target_for_binary(binary));
    }
    Ok(lowered)
}

/// The two fences of one dispatch, each armed at the execution index its
/// phase begins at — or at the end of the plan when the phase selected no
/// contribution at all.
///
/// That last case is the point: a project with zero `phase:package`
/// contributions still packages its declared targets, exactly as a project
/// with zero verify contributions still gets its verify member.
pub(super) struct Fences<'targets> {
    targets: MechanismTargets<'targets>,
    prepared_native: vibe_lifecycle::native::PreparedNativeMechanisms,
    build: Option<usize>,
    package: Option<usize>,
    deploy: Option<usize>,
}

impl<'targets> Fences<'targets> {
    /// Arm every fence for one plan, or nothing at all for a partial epoch.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
    pub(super) fn arm(
        targets: Option<MechanismTargets<'targets>>,
        plan: &RitualPlan,
        chain: &[String],
    ) -> Option<Self> {
        let targets = targets?;
        let build = fence(plan, chain, Phase::Build);
        let package = fence(plan, chain, Phase::Package);
        let deploy = targets
            .deploy
            .and_then(|_| fence(plan, chain, Phase::Deploy));
        Some(Self {
            targets,
            prepared_native: Default::default(),
            build,
            package,
            deploy,
        })
    }

    /// Fire the build fence once at its armed execution index.
    pub(super) fn fire_build(&mut self, index: usize) -> Result<()> {
        if self.build != Some(index) {
            return Ok(());
        }
        self.build.take();
        let native = self.targets.native.take();
        let platform = self
            .targets
            .native_platform
            .context("build fence has no preselected native platform")?;
        self.prepared_native = super::native_mechanism::prepare(
            native.as_ref(),
            self.targets.native_candidates,
            std::mem::take(&mut self.targets.native_mechanisms),
            self.targets.project_root,
            self.targets.registry,
            self.targets.routes,
            platform,
            self.targets.offline,
            self.targets.created_at,
        )?;
        if let Some(native) = native.filter(|native| !native.replay_is_empty()) {
            let mut factory = platform.replay_factory();
            native
                .into_carriage()
                .replay(&mut factory)
                .context("converging pending compiler-native boot artifacts")?;
        }
        execute_build_targets(&BuildExecution {
            project_root: self.targets.project_root,
            targets: self.targets.build,
            registry: self.targets.registry,
            routes: self.targets.routes,
            build_root: BuildExecution::default_build_root(),
            offline: self.targets.offline,
            created_at: self.targets.created_at,
        })
        .context("executing the declared [[artifacts.build]] targets")?;
        Ok(())
    }

    /// Fire the package fence at its armed execution index.
    pub(super) fn fire_package(&mut self, index: usize) -> Result<()> {
        if self.package != Some(index) {
            return Ok(());
        }
        self.package = None;
        anyhow::ensure!(
            self.targets.package.iter().all(|target| target
                .when
                .as_ref()
                .is_none_or(|when| when.applies_to(self.targets.target_os))),
            "internal: package fence received a target inactive on the injected host OS"
        );
        execute_package_targets(&PackageExecution {
            project_root: self.targets.project_root,
            targets: self.targets.package,
            registry: self.targets.registry,
            routes: self.targets.routes,
            package_root: PackageExecution::default_package_root(),
            created_at: self.targets.created_at,
        })
        .context("executing the declared [[artifacts.package]] targets")?;
        Ok(())
    }

    /// Fire the deploy fence at its armed execution index.
    pub(super) fn fire_deploy(&mut self, index: usize) -> Result<()> {
        if self.deploy != Some(index) {
            return Ok(());
        }
        self.deploy = None;
        let Some(carriage) = self.targets.deploy else {
            return Ok(());
        };
        let _prepared_native = &self.prepared_native;
        execute_deploy_targets(&DeployExecution {
            project_root: self.targets.project_root,
            targets: self.targets.deploy_targets,
            selection: &carriage.selection,
            registry: self.targets.registry,
            routes: self.targets.routes,
            state_home: &carriage.state_home,
            settings_root: &carriage.settings_root,
            user_home: &carriage.user_home,
            clients: &carriage.clients,
            project: &carriage.project,
            package: carriage.package.as_deref(),
            created_at: self.targets.created_at,
        })
        .context("executing the selected [[deploy.target]] rows")?;
        Ok(())
    }
}

type BuildRowSignature = Vec<(
    String,
    vibe_core::manifest::ExtensionHandler,
    Option<vibe_core::manifest::ExtensionConfig>,
)>;

pub(super) struct PreflightBuildGroup<'a> {
    pub(super) source: (
        String,
        PathBuf,
        vibe_lifecycle::native::NativeArtifactRecordRoot,
        String,
    ),
    signature: BuildRowSignature,
    pub(super) provider_pin: String,
    pub(super) candidates: Vec<&'a vibe_lifecycle::ExtensionRegistryRow>,
    pub(super) registry: &'a MechanismRegistry,
    pub(super) routes: &'a MechanismRoutes,
}

pub(super) fn preflight_all_owner_native_sources<'a>(
    native: &'a NativeInstallContext,
    project_root: &Path,
    platform: NativePlatform,
    offline: bool,
    created_at: &str,
) -> Result<Vec<PreflightBuildGroup<'a>>> {
    preflight_all_owner_native_sources_with(
        native,
        project_root,
        platform,
        offline,
        created_at,
        |_, execution| {
            platform
                .resolved_build_provider_pin(execution)
                .map_err(Into::into)
        },
    )
}

#[cfg(test)]
pub(super) fn preflight_with_test_pins(
    native: &NativeInstallContext,
    project_root: &Path,
    platform: NativePlatform,
    mut pin: impl FnMut(&vibe_workspace::extension_world::OwnerRuntimeId) -> String,
) -> Result<()> {
    preflight_all_owner_native_sources_with(
        native,
        project_root,
        platform,
        true,
        "2026-09-08T00:00:00Z",
        |owner, _| Ok(pin(owner)),
    )
    .map(|_| ())
}

fn preflight_all_owner_native_sources_with<'a>(
    native: &'a NativeInstallContext,
    project_root: &Path,
    platform: NativePlatform,
    offline: bool,
    created_at: &str,
    mut resolve_pin: impl FnMut(
        &vibe_workspace::extension_world::OwnerRuntimeId,
        &NativeBuildExecution<'_>,
    ) -> Result<String>,
) -> Result<Vec<PreflightBuildGroup<'a>>> {
    let (epoch, owners) = native.build_parts();
    let mut groups: Vec<PreflightBuildGroup<'a>> = Vec::new();
    for owner in owners {
        let view = match owner {
            vibe_workspace::extension_world::OwnerRuntimeId::Node { rel } => epoch.node(rel)?,
            vibe_workspace::extension_world::OwnerRuntimeId::Unit { provider } => {
                epoch.unit(provider)?
            }
        };
        let runtime = view.runtime();
        let rows = runtime.rows()?;
        let all_candidates = rows.native().to_vec();
        let execution = NativeBuildExecution {
            candidates: &all_candidates,
            selected_project_root: project_root,
            registry: runtime.mechanisms(),
            routes: runtime.routes(),
            platform,
            offline,
            created_at,
        };
        let owner_groups =
            vibe_lifecycle::native::project_native_source_groups(&all_candidates, platform)?;
        if owner_groups.is_empty() {
            continue;
        }
        let provider_pin = resolve_pin(owner, &execution)
            .with_context(|| format!("preflighting build provider for retained owner {owner:?}"))?;
        for projection in owner_groups {
            let source = (
                projection.provider,
                projection.provider_root,
                projection.record_root,
                projection.crate_dir,
            );
            let candidates = projection.candidates;
            let signature = candidates
                .iter()
                .map(|row| {
                    (
                        row.key().as_str().to_owned(),
                        row.declaration().handler.clone(),
                        row.effective_config().cloned(),
                    )
                })
                .collect::<Vec<_>>();
            if let Some(previous) = groups.iter().find(|group| group.source == source) {
                anyhow::ensure!(
                    previous.signature == signature && previous.provider_pin == provider_pin,
                    "native source group `{}/{}` has conflicting effective rows or build provider across retained owners",
                    source.0,
                    source.3,
                );
                continue;
            }
            groups.push(PreflightBuildGroup {
                source,
                signature,
                provider_pin: provider_pin.clone(),
                candidates,
                registry: runtime.mechanisms(),
                routes: runtime.routes(),
            });
        }
    }
    for group in &groups {
        platform
            .admit_build_provider(&NativeBuildExecution {
                candidates: &group.candidates,
                selected_project_root: project_root,
                registry: group.registry,
                routes: group.routes,
                platform,
                offline,
                created_at,
            })
            .with_context(|| {
                format!(
                    "admitting build provider {} for native source group {}/{}",
                    group.provider_pin, group.source.0, group.source.3
                )
            })?;
    }
    Ok(groups)
}

/// Where one phase's own contributions begin, or `None` when the requested
/// chain never reaches that phase.
///
/// Rank, not string order, and for the reason the verify boundary states: a
/// phase's position is a fact about the REQUESTED chain, and a lexical
/// comparison would call `test` later than `package`.
fn fence(plan: &RitualPlan, chain: &[String], phase: Phase) -> Option<usize> {
    let at = rank(chain, phase.as_str())?;
    let first = plan
        .executions
        .iter()
        .position(|execution| rank(chain, &execution.phase).is_some_and(|other| other >= at));
    Some(first.unwrap_or(plan.executions.len()))
}

/// A phase's position in the chain this run was asked for.
fn rank(chain: &[String], phase: &str) -> Option<usize> {
    chain.iter().position(|spelling| spelling == phase)
}
