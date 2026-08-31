//! The engine-owned mechanism fences: where a COMPLETE default-phase plan
//! executes its declared `[[artifacts.build]]` and `[[artifacts.package]]`
//! targets, INSIDE the one contribution walk (§6.0.2's wiring).
//!
//! ## Why the fences live here and not above the dispatch
//!
//! §2 is the architecture's PRIMARY law — the nine-phase line, with
//! `generate` owning deterministic derived source and `build` producing code
//! artifacts *from* that source. An executor placed above this walk would run
//! the mechanism build before every `phase:generate` contribution, which
//! inverts the one edge §2 exists to fix. The contribution walk is the only
//! place in the engine that already visits phases in the requested chain's
//! order, so it is the only place both edges can hold at once.
//!
//! ## Where inside a phase, and why
//!
//! An engine-owned action fires **before that phase's own contributions** —
//! the identical position, and the identical reason, as the verify boundary
//! next door ("BEFORE the first verify-or-later row, and therefore before any
//! verify contribution is dispatched"). §3 spells the difference the position
//! encodes: "an ordinary `phase:build` contribution adds a task to the
//! ritual; a build-role mechanism can service one or more declarative build
//! targets". The declared targets ARE the phase's artifact production; a
//! contribution is an addition to it. Firing first is what lets a
//! `phase:build` or `phase:package` contribution observe or consume the
//! artifact — and its A2 record — that its own phase just produced. The
//! reverse order would make every such contribution run before the artifact
//! it might read exists, and there would be no in-phase reading of §4's
//! "later phases consume records" at all.
//!
//! Both fences therefore straddle the verify gate exactly as the phase line
//! does: build, then verify, then package.
//!
//! ## Why the targets are a parameter and not a plan field
//!
//! The same reason the verify boundary's permission is: the dispatcher is
//! entered from TWO epochs and only one owns the complete chain. The
//! post-durability install callback dispatches a `[validate, install]` plan
//! while carrying the outer command's `metadata.chain`, which for
//! `vibe package` already names every phase through `package`. A fence that
//! read the chain alone would fire there and build a project's artifacts
//! during its prerequisite install. `None` is every partial epoch saying "not
//! here, whatever the chain says".

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use specmark::spec;
use vibe_core::manifest::{
    ArtifactBuildTarget, ArtifactPackageTarget, ArtifactsSection, BinaryDecl, DeployTarget,
    Manifest, MechanismRoutes, build_target_for_binary,
};
use vibe_lifecycle::native::{NativeBuildExecution, NativePlatform, build_native_sources};
use vibe_lifecycle::{
    BuildExecution, ClientExecutables, DeployExecution, DeploySelection, MechanismRegistry,
    PackageExecution, Phase, deploy_state_home, execute_build_targets, execute_deploy_targets,
    execute_package_targets,
};

use crate::RitualPlan;

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
    targets: &'targets MechanismTargets<'targets>,
    build: Option<usize>,
    package: Option<usize>,
    deploy: Option<usize>,
}

impl<'targets> Fences<'targets> {
    /// Arm every fence for one plan, or nothing at all for a partial epoch.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
    pub(super) fn arm(
        targets: Option<&'targets MechanismTargets<'targets>>,
        plan: &RitualPlan,
        chain: &[String],
    ) -> Option<Self> {
        let targets = targets?;
        Some(Self {
            targets,
            build: fence(plan, chain, Phase::Build),
            package: fence(plan, chain, Phase::Package),
            // §7.0.5: "The fence arms only when the dispatch carries a
            // resolved selection AND that epoch's plan reaches `deploy`."
            // The two conditions are separate for a reason — a chain that
            // reaches deploy without a selection is `vibe deploy` on a
            // project that declares no deploy section, which must stay the
            // historical no-op rather than become a refusal.
            deploy: targets
                .deploy
                .and_then(|_| fence(plan, chain, Phase::Deploy)),
        })
    }

    /// Fire the build fence if this index is where it is armed.
    ///
    /// Called at the top of every iteration and once more with the plan's
    /// length, so every armed index from `0` to `len` is visited exactly once
    /// and a fence can be neither skipped nor fired twice.
    pub(super) fn fire_build(&mut self, index: usize) -> Result<()> {
        if self.build != Some(index) {
            return Ok(());
        }
        self.build = None;
        let candidates = self.targets.native_candidates.iter().collect::<Vec<_>>();
        let platform = NativePlatform::current().context("selecting the native build platform")?;
        build_native_sources(&NativeBuildExecution {
            candidates: &candidates,
            selected_project_root: self.targets.project_root,
            registry: self.targets.registry,
            routes: self.targets.routes,
            platform,
            offline: self.targets.offline,
            created_at: self.targets.created_at,
        })
        .context("building enabled native source extensions at the build fence")?;
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

    /// Fire the package fence if this index is where it is armed.
    pub(super) fn fire_package(&mut self, index: usize) -> Result<()> {
        if self.package != Some(index) {
            return Ok(());
        }
        self.package = None;
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

    /// Fire the deploy fence if this index is where it is armed.
    ///
    /// The third member of §6.0's fence family, armed at the deploy
    /// phase's own-contribution boundary with the identical position and
    /// the identical reason: a `phase:deploy` contribution that wants to
    /// observe what the deployment did runs after it, and the phase line
    /// puts deploy last, so this fires after the package fence and after
    /// the verify gate between them.
    pub(super) fn fire_deploy(&mut self, index: usize) -> Result<()> {
        if self.deploy != Some(index) {
            return Ok(());
        }
        self.deploy = None;
        let Some(carriage) = self.targets.deploy else {
            return Ok(());
        };
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
