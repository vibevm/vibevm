//! Surface-neutral lifecycle orchestration — the one shared application
//! service both the CLI and a hosted MCP surface execute a default-lifecycle
//! chain through.
//!
//! It owns selected-world loading and plan construction, run-identity
//! selection, the validate/install barrier and slot continuation, phase
//! dispatch and removed-row reconciliation, package-binding composition, the
//! report-neutral success/park/failure values, and the command-level compile
//! [`trace`] owner and its consuming funnel. The neutral package-source and
//! registry-cell composition lives in the separate `vibe-package-source`
//! crate, which implements this crate's package-source port; argument
//! grammar, surface short-name qualification, provider construction,
//! terminal and JSON rendering, and the registered report families stay in
//! the surfaces, behind [`ports`].
//!
//! Provider, model and credential configuration is not a planning fact and
//! cannot cross this boundary: the surface injects an already-built
//! [`vibe_lifecycle::AgentBackend`], and a hosted surface injects one that
//! parks instead of paying.

#![forbid(unsafe_code)]

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM");

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use specmark::spec;
use vibe_agent_projection::pkgskill::ProjectSkillBinding;
use vibe_lifecycle::{ExecutablePlan, MechanismRegistry, Phase};
use vibe_wire::generated::lifecycle::e1::context::{Project, World};

pub mod failure;
pub mod ports;
pub mod trace;
pub mod values;

mod callback;
mod command;
mod dispatch;
mod install;
mod phase;
mod plan;
mod prelude;
mod world;

pub use callback::after_durable_world_stage;
pub use command::{
    DefaultLifecyclePorts, DefaultLifecycleRequest, LeasedDefaultLifecycle,
    PreparedDefaultLifecycle, lease_default_lifecycle, prepare_default_lifecycle,
};
pub use dispatch::{DeployAuthority, dispatch_plan_untracked};
pub use install::{
    InstallDisposition, InstallExecution, InstallInputs, InstallPolicy, InstallRun,
    InstallRunContext, PreparedSelection, ProvenSelection, ResumeOutcome, ResumeRequest,
    ResumedInstall, SelectedManifest, WorldCallbackOutcome, WorldCallbackSummary, acquire_lease,
    execute_prepared, generated_by, lease_root, own_resume, prefixed, provisional_world,
    resolve_project_root, resolve_spec_format, resume_slot_continuation, selected_node_manifest,
};
pub use phase::{PhaseOutcome, PhaseRun, run_phases};
pub use plan::{planned_contribution, provider_and_version, tier_name};
pub use prelude::{RunPrelude, run_prelude};
pub use world::{
    LoadedRegistry, PACKAGE_SKILL_RECONCILE_KEY, PACKAGE_SKILL_RECOVER_KEY, inspect,
    plan_clean_prepared, plan_default_prepared,
};

/// One owned, surface-neutral lifecycle plan.
///
/// It contains only selected execution and world facts — nine fields, and no
/// tenth. In particular NO manifest rides here: a complete `Manifest` carries
/// `[llm]` provider/model/credential configuration, so storing one would smuggle
/// exactly the seam this boundary exists to keep out. A surface reads its own
/// snapshot, derives its own configuration from it, and injects an already-built
/// backend. The structural RED below destructures every field with no `..`, so a
/// hidden tenth carrier is a compile error rather than a review question.
///
/// The ninth field is R8-PACKAGE's, and it is the same genre as the eight: the
/// mechanism plane is a collected fact of the selected world, taken off the one
/// snapshot the extension registry came from. It carries provider identities and
/// handler kinds — never configuration, never credentials — so the fence the
/// count enforces is unchanged in meaning.
///
/// The fields are PRIVATE. Every one of them is an invariant this crate
/// establishes at collection time and every dispatch entry point then trusts —
/// `workspace_root` is checked against the lease, `executions` is canonically
/// ordered, `package_bindings`/`package_desired_keys` are the reconciliation
/// pair. A surface that could mutate any of them could hand a gate a root it
/// already passed and then run against a different one. What a surface really
/// needs is presentation, so that is what it gets: read-only borrows.
///
/// ```
/// use vibe_lifecycle::Phase;
/// use vibe_orchestrator::RitualPlan;
///
/// fn count(plan: &RitualPlan) -> usize {
///     plan.count_for(Phase::Build)
/// }
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
#[derive(Debug)]
pub struct RitualPlan {
    /// Canonically ordered executable contributions.
    pub(crate) executions: ExecutablePlan,
    /// Non-fatal collection notices in deterministic order.
    pub(crate) notices: Vec<String>,
    /// Selected-project facts carried into handler envelopes.
    pub(crate) project: Project,
    /// Effective installed-world facts carried into handler envelopes.
    pub(crate) world: World,
    /// The mechanism plane of the same world — the provider rows the build
    /// and package executors resolve their declared targets against.
    pub(crate) mechanisms: MechanismRegistry,
    /// Canonical workspace root which owns lifecycle state.
    pub(crate) workspace_root: PathBuf,
    /// Planned project-skill bindings keyed by execution identity.
    pub(crate) package_bindings: BTreeMap<String, ProjectSkillBinding>,
    /// Complete desired project-skill execution-key set.
    pub(crate) package_desired_keys: BTreeSet<String>,
    /// Whether the requested chain contains the package phase.
    pub(crate) package_phase_planned: bool,
}

impl RitualPlan {
    /// Count contributions selected for one canonical default phase.
    ///
    /// The struct-level example demonstrates this query for `Phase::Build`.
    #[must_use]
    #[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENGINE-ALGORITHM")]
    pub fn count_for(&self, phase: Phase) -> usize {
        self.executions.count_for(phase.as_str())
    }

    /// The canonically ordered contributions, for a surface to render.
    ///
    /// See [`Self::count_for`] for the query form.
    #[must_use]
    pub fn executions(&self) -> &ExecutablePlan {
        &self.executions
    }

    /// The collection notices, for a surface to render.
    ///
    /// See [`Self::count_for`].
    #[must_use]
    pub fn notices(&self) -> &[String] {
        &self.notices
    }

    /// The selected-project facts, for a surface to render.
    ///
    /// See [`Self::count_for`].
    #[must_use]
    pub fn project(&self) -> &Project {
        &self.project
    }

    /// The canonical workspace root this plan was collected over — the value a
    /// surface's own lease gate checks against.
    ///
    /// See [`Self::count_for`].
    #[must_use]
    pub fn workspace_root(&self) -> &std::path::Path {
        &self.workspace_root
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

    /// A12 widened the value-only atom into the shared application service;
    /// A13 added the trace funnel. The set is still EXACT, and still contains
    /// no surface, provider or transport edge: the whole point of the
    /// extraction is that neither CLI nor MCP can be reached from here.
    ///
    /// Two edges go beyond the accepted nine, and both are accepted for a
    /// named, load-bearing reason:
    ///
    /// * `toml` — the moved package-skill preset builder constructs
    ///   `ExtensionConfig::from_table(toml::Table)`, a `vibe-core` public type
    ///   whose crate that crate does not re-export;
    /// * `chrono` — the moved trace funnel parses the lifecycle's own recorded
    ///   RFC 3339 start into the trace epoch's instant, and
    ///   `vibe_wire ..shared::Timestamp` IS `chrono::DateTime<Utc>`. Nothing
    ///   here reads a clock: every instant the funnel writes is injected.
    #[test]
    fn the_application_service_has_exactly_the_accepted_lower_dependencies() {
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
            "anyhow",
            "chrono",
            "specmark",
            "toml",
            "vibe-agent-projection",
            "vibe-core",
            "vibe-install",
            "vibe-lifecycle",
            "vibe-resolver",
            "vibe-wire",
            "vibe-workspace",
        ]);
        assert_eq!(
            actual, expected,
            "A12/A13 carry the shared algorithm only: no CLI, MCP or LLM edge"
        );
        // The exactness above is the fence; this states the INTENT it exists
        // for, so a widening that happened to keep the set exact is still
        // caught by name.
        for forbidden in ["vibe-cli", "vibe-mcp", "vibe-llm", "dialoguer", "console"] {
            assert!(
                !actual.contains(forbidden),
                "`{forbidden}` is a surface or provider edge and can never be a normal dependency",
            );
        }
    }

    /// The plan carries NINE fields, and the compiler proves it.
    ///
    /// Destructured with no `..`, so a tenth field — a manifest, a config, any
    /// carrier that could smuggle provider settings below the surface — is a
    /// compile error here rather than a review question. This is the structural
    /// half of the fence; the source scan below is the textual half.
    ///
    /// The count moved from eight to nine at R8-PACKAGE, deliberately and once:
    /// `run_phases` executes the declared `[[artifacts.build]]` and
    /// `[[artifacts.package]]` targets, and selection needs the mechanism plane
    /// of the same world snapshot the executions were planned from. Collecting
    /// it a second time inside the phase run would be a second world — the
    /// exact retry the prepared-selection bundle exists to forbid.
    #[test]
    fn the_shared_plan_carries_exactly_its_nine_neutral_fields() {
        fn destructure(plan: super::RitualPlan) {
            let super::RitualPlan {
                executions,
                notices,
                project,
                world,
                mechanisms,
                workspace_root,
                package_bindings,
                package_desired_keys,
                package_phase_planned,
            } = plan;
            let _ = (
                executions,
                notices,
                project,
                world,
                mechanisms,
                workspace_root,
                package_bindings,
                package_desired_keys,
                package_phase_planned,
            );
        }
        let _ = destructure;
    }

    /// The dependency set is a necessary but not sufficient fence: a provider
    /// seam can also arrive as a stored type, a config field or a borrowed CLI
    /// vocabulary, and a RENDERING seam can arrive as a borrowed context, a
    /// clap identity or a printing call. This reads the crate's own PRODUCTION
    /// sources.
    ///
    /// It deliberately does NOT forbid `Manifest` outright — world collection
    /// and the install core legitimately parse manifests. What it forbids is
    /// the named seams: the whole user config, the provider section, the
    /// surface's own source-mutation grammar, and — since A13 moved the trace
    /// funnel down — every presentation symbol the funnel deliberately left
    /// behind, so "the adapter followed the values down" is a red rather than a
    /// review question.
    #[test]
    fn no_production_source_names_a_config_provider_or_surface_seam() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut offenders: Vec<String> = Vec::new();
        let mut stack = vec![root];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                    continue;
                }
                // Test cells legitimately construct fixtures with wider
                // vocabulary; the fence is on production source.
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                if name == "tests.rs" || name.ends_with("_tests.rs") {
                    continue;
                }
                if path
                    .components()
                    .any(|part| part.as_os_str() == std::ffi::OsStr::new("tests"))
                {
                    continue;
                }
                let body = std::fs::read_to_string(&path).unwrap();
                // Spelled in halves on purpose: this checker's own source must
                // not contain the strings it forbids, or it would report
                // itself and never fail for a real offender.
                for needle in [
                    concat!("vibe", "_cli"),
                    concat!("vibe", "_mcp"),
                    concat!("vibe", "_llm"),
                    concat!("Llm", "Section"),
                    concat!(".", "llm"),
                    concat!("User", "Config"),
                    concat!("git", "_auth"),
                    concat!("git", "_token_env"),
                    concat!("credential", "_helper"),
                    // A13: presentation stays in the surface. A borrowed
                    // rendering context, a clap identity, a JSON emission or a
                    // deferred-plan flush below this line would mean the
                    // adapter followed the values down.
                    // (`dialoguer` and `console` are fenced by NAME in the
                    // dependency red above; they are deliberately not needles
                    // here, because that red's own list spells them literally
                    // and a source scan would report this file.)
                    concat!("output", "::Context"),
                    concat!("clap", "::"),
                    concat!("emit", "_json"),
                    concat!("flush", "_json_plans"),
                    concat!("discard", "_json_plans"),
                    concat!("is", "_quiet"),
                    concat!("is", "_json"),
                    concat!("quiet", "_suffix"),
                    concat!("render", "_human"),
                ] {
                    if body.contains(needle) {
                        offenders.push(format!("{} names `{needle}`", path.display()));
                    }
                }
            }
        }
        assert!(offenders.is_empty(), "{offenders:#?}");
    }
}
