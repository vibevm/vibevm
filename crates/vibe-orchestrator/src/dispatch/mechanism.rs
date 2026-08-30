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

use std::path::Path;

use anyhow::{Context, Result};
use specmark::spec;
use vibe_core::manifest::{
    ArtifactBuildTarget, ArtifactPackageTarget, ArtifactsSection, MechanismRoutes,
};
use vibe_lifecycle::{
    BuildExecution, MechanismRegistry, PackageExecution, Phase, execute_build_targets,
    execute_package_targets,
};

use crate::RitualPlan;

/// Everything the mechanism half of one dispatch needs.
///
/// A named value rather than six arguments, because five of the six are
/// borrowed from three different places and a positional call would let two
/// of them be swapped without the compiler noticing.
pub(crate) struct MechanismTargets<'a> {
    /// The selected project's absolute root.
    pub(crate) project_root: &'a Path,
    /// The selected manifest's declared artifact graph, when it declares one.
    pub(crate) artifacts: Option<&'a ArtifactsSection>,
    /// The mechanism plane of the world this plan was collected from.
    pub(crate) registry: &'a MechanismRegistry,
    /// The host's `[mechanisms]` routes.
    pub(crate) routes: &'a MechanismRoutes,
    /// The run's effective offline posture.
    pub(crate) offline: bool,
    /// The run's injected instant, in the RFC 3339 spelling every record
    /// carries. Nothing below a surface reads a clock.
    pub(crate) created_at: &'a str,
}

impl MechanismTargets<'_> {
    /// The declared build rows, or none.
    fn build(&self) -> &[ArtifactBuildTarget] {
        self.artifacts
            .map_or(&[] as &[ArtifactBuildTarget], |section| &section.build)
    }

    /// The declared package rows, or none.
    fn package(&self) -> &[ArtifactPackageTarget] {
        self.artifacts
            .map_or(&[] as &[ArtifactPackageTarget], |section| &section.package)
    }
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
}

impl<'targets> Fences<'targets> {
    /// Arm both fences for one plan, or nothing at all for a partial epoch.
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
        execute_build_targets(&BuildExecution {
            project_root: self.targets.project_root,
            targets: self.targets.build(),
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
            targets: self.targets.package(),
            registry: self.targets.registry,
            routes: self.targets.routes,
            package_root: PackageExecution::default_package_root(),
            created_at: self.targets.created_at,
        })
        .context("executing the declared [[artifacts.package]] targets")?;
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
