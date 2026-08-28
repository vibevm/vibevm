//! Surface-neutral lifecycle orchestration.
//!
//! This first extraction owns the canonical planned-run value. Execution,
//! filesystem loading and surface presentation move behind it in later R7.4
//! atoms; provider/model/credential configuration never enters this crate.

#![forbid(unsafe_code)]

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use specmark::spec;
use vibe_agent_projection::pkgskill::ProjectSkillBinding;
use vibe_lifecycle::{ExecutablePlan, Phase};
use vibe_wire::generated::lifecycle::e1::context::{Project, World};

/// One owned, surface-neutral lifecycle plan.
///
/// It contains only selected execution and world facts. In particular, LLM
/// provider/model/credential configuration is not a planning fact and cannot
/// cross this boundary.
///
/// ```
/// use std::collections::{BTreeMap, BTreeSet};
/// use std::path::PathBuf;
/// use vibe_lifecycle::{ExecutablePlan, Phase};
/// use vibe_orchestrator::RitualPlan;
/// use vibe_wire::generated::lifecycle::e1::context::{Project, World};
///
/// let plan = RitualPlan {
///     executions: ExecutablePlan::default(),
///     notices: Vec::new(),
///     project: Project {
///         kind: "project".into(),
///         manifest: "vibe.toml".into(),
///         name: "demo".into(),
///         root: ".".into(),
///         spec_roots: Vec::new(),
///         version: "0.1.0".into(),
///     },
///     world: World {
///         deps_root: "vibevm/vibedeps".into(),
///         lockfile: "vibe.lock".into(),
///         packages: Vec::new(),
///     },
///     workspace_root: PathBuf::from("."),
///     package_bindings: BTreeMap::new(),
///     package_desired_keys: BTreeSet::new(),
///     package_phase_planned: false,
/// };
/// assert_eq!(plan.count_for(Phase::Build), 0);
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
#[derive(Debug)]
pub struct RitualPlan {
    /// Canonically ordered executable contributions.
    pub executions: ExecutablePlan,
    /// Non-fatal collection notices in deterministic order.
    pub notices: Vec<String>,
    /// Selected-project facts carried into handler envelopes.
    pub project: Project,
    /// Effective installed-world facts carried into handler envelopes.
    pub world: World,
    /// Canonical workspace root which owns lifecycle state.
    pub workspace_root: PathBuf,
    /// Planned project-skill bindings keyed by execution identity.
    pub package_bindings: BTreeMap<String, ProjectSkillBinding>,
    /// Complete desired project-skill execution-key set.
    pub package_desired_keys: BTreeSet<String>,
    /// Whether the requested chain contains the package phase.
    pub package_phase_planned: bool,
}

impl RitualPlan {
    /// Count contributions selected for one canonical default phase.
    ///
    /// The struct-level example constructs an empty plan and demonstrates this
    /// query for `Phase::Build`.
    #[must_use]
    #[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
    pub fn count_for(&self, phase: Phase) -> usize {
        self.executions.count_for(phase.as_str())
    }
}

/// One planned contribution retained after the source registry is dropped.
///
/// ```
/// use vibe_orchestrator::PlannedExecution;
/// let none: Option<PlannedExecution> = None;
/// assert!(none.is_none());
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
pub type PlannedExecution = vibe_lifecycle::ExecutableContribution;

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    #[test]
    fn the_value_only_atom_has_exactly_the_four_lower_dependencies() {
        let manifest: toml::Table = toml::from_str(include_str!("../Cargo.toml")).unwrap();
        let dependencies = manifest
            .get("dependencies")
            .and_then(toml::Value::as_table)
            .unwrap();
        let actual = dependencies
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let expected = BTreeSet::from([
            "specmark",
            "vibe-agent-projection",
            "vibe-lifecycle",
            "vibe-wire",
        ]);
        assert_eq!(
            actual, expected,
            "A11 carries values only: no CLI, MCP, LLM, install or workspace edge"
        );
    }
}
