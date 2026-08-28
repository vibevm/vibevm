//! The surface-neutral install inputs.
//!
//! Split from `inputs.rs` along the responsibility seam that cell already
//! carries: input NORMALISATION (the one manifest snapshot, the one workspace
//! load, the one canonical root) versus the surface's projected ARGUMENTS.
//! They change for different reasons, and keeping them together pushed the
//! normalisation cell past its line budget.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use specmark::spec;
use vibe_core::manifest::SpecFormat;
use vibe_core::user_config::SlotIntegrity;

/// The surface-neutral install inputs one run consumes.
///
/// Deliberately NOT the surface's argument type. Registry, solver and
/// source-preference flags never enter — their one consumer is the
/// package-source factory the surface owns. Neither does any source-mutation
/// grammar: `--git`/`--tag`/`--branch`/`--rev`/`--git-auth`/`--git-token-env`
/// are the surface's vocabulary with the surface's exit codes, and they reach
/// the core only as an [`crate::ports::InstallManifestMutation`] applied at its
/// one position. What remains is exactly what the shared core itself reads.
///
/// ```
/// use vibe_orchestrator::InstallInputs;
/// let inputs = InstallInputs::default();
/// assert!(inputs.packages.is_empty());
/// assert!(!inputs.exact);
/// ```
#[derive(Debug, Clone, Default)]
#[spec(documents = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail")]
pub struct InstallInputs {
    /// Zero or more surface-supplied package references, unparsed.
    pub packages: Vec<String>,
    /// Explicitly requested features.
    pub features: Vec<String>,
    /// Whether `[features].default` activation is suppressed.
    pub no_default_features: bool,
    /// Whether every non-private feature is activated.
    pub all_features: bool,
    /// The caller's language override.
    pub language: Option<String>,
    /// Whether resolved versions are pinned exactly.
    pub exact: bool,
}

/// The narrow execution policy the shared core reads.
///
/// The surface's complete user-configuration value deliberately does NOT cross:
/// it carries global provider, model, endpoint and credential settings, and the
/// core needs exactly three decided answers out of it. The surface loads that
/// config once, at the point it always did, and hands the answers down.
///
/// ```
/// use vibe_orchestrator::InstallPolicy;
/// let policy = InstallPolicy::default();
/// assert!(!policy.offline);
/// assert!(policy.spec_format_default.is_none());
/// ```
#[derive(Debug, Clone, Copy, Default)]
#[spec(documents = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail")]
pub struct InstallPolicy {
    /// The ALREADY-RESOLVED offline posture — PROP-010 §2.5's whole ladder
    /// (surface flags > `VIBE_OFFLINE` > user config) collapsed by the surface
    /// into one boolean, so no rung is re-derived here.
    pub offline: bool,
    /// PROP-011 §2.3 — the materialise-diff strategy.
    pub slot_integrity: SlotIntegrity,
    /// PROP-045 — the operator default the selected manifest's own pin wins
    /// over. Combined with the manifest at the position it always was.
    pub spec_format_default: Option<SpecFormat>,
}
