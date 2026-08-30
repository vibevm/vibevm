//! The build phase's mechanism executor — §5.0.2's wiring.
//!
//! "The build phase walks the landed `[[artifacts.build]]` targets in the
//! A1 DAG order; per target the logical key comes from `target.mechanism`,
//! the exact pin from the target's own `provider` member, the routes from
//! the host manifest, and selection is R8-MECHANISM's `resolve_mechanism`
//! — one law, no second resolver."
//!
//! That last clause is the load-bearing one. There is no `if builtin`
//! shortcut anywhere in this file: the executor asks the resolver who
//! services the key, and only then looks at the selected row's handler. A
//! host that routes `build:cargo` to a plugin gets the plugin's refusal
//! (the transport is a later atom) and demonstrably NOT a Cargo build —
//! which is the whole proof that routing is real.
//!
//! Records are written by this engine, not by the provider, to the
//! engine-owned `.vibe/state/artifacts/` of the selected project.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use std::path::Path;

use specmark::spec;
use vibe_core::manifest::{ArtifactBuildTarget, ExtensionHandler, MechanismRoutes};
use vibe_extension_registry::{
    MechanismRegistry, MechanismSelection, SelectionStep, resolve_mechanism,
};
use vibe_wire::generated::artifact_record::ArtifactShape;

mod error;

pub use error::BuildError;

use super::cargo::{BuildPlan, CargoProvider, SelectedArtifact, ToolchainIdentity};
use super::order::{GraphNode, OrderFault, Unresolved, dag_order};
use super::record::{
    RecordFreshness, RecordInputs, build_record, config_digest, sanitize, write_record,
};
use super::{BUILTIN_CARGO_NAME, BuildProvider, BuildTargetRequest, DEFAULT_BUILD_ROOT};

/// Everything one build-phase execution needs, and nothing more.
///
/// The registry and the routes arrive already collected — this executor
/// resolves, it does not discover — and the clock arrives injected, so a
/// run stamped twice with one value produces byte-identical records.
///
/// ```
/// use std::path::Path;
/// use vibe_core::manifest::MechanismRoutes;
/// use vibe_extension_registry::collect_mechanisms;
/// use vibe_lifecycle::{BuildExecution, execute_build_targets};
/// # use vibe_lifecycle::{ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider};
/// # use vibe_core::manifest::ExtensionsControl;
/// # let world = ExtensionWorld {
/// #     installed: Vec::new(),
/// #     host: HostExtensionSource {
/// #         provider: HostProvider {
/// #             identity: HostIdentity::ungrouped_project("demo"),
/// #             root: std::path::PathBuf::from("."),
/// #             version: "0.1.0".into(),
/// #             kind: None,
/// #             content_hash: None,
/// #         },
/// #         declarations: Vec::new(),
/// #         controls: ExtensionsControl::default(),
/// #         mechanisms: Vec::new(),
/// #     },
/// #     effective_stack: None,
/// # };
/// let registry = collect_mechanisms(&world).unwrap();
/// let routes = MechanismRoutes::default();
/// let execution = BuildExecution {
///     project_root: Path::new("."),
///     targets: &[],
///     registry: &registry,
///     routes: &routes,
///     build_root: "target",
///     offline: true,
///     created_at: "2026-08-30T00:00:00Z",
/// };
///
/// // A project that declares no build target builds nothing and says so.
/// assert!(execute_build_targets(&execution).unwrap().is_empty());
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
#[derive(Debug, Clone, Copy)]
pub struct BuildExecution<'a> {
    /// The selected project's absolute root.
    pub project_root: &'a Path,
    /// The landed `[[artifacts.build]]` rows, in declaration order.
    pub targets: &'a [ArtifactBuildTarget],
    /// The collected mechanism plane of this world.
    pub registry: &'a MechanismRegistry,
    /// The host's `[mechanisms]` routes.
    pub routes: &'a MechanismRoutes,
    /// The engine-owned build output root, relative to `project_root`.
    /// [`BuildExecution::default_build_root`] is the shipped value.
    pub build_root: &'a str,
    /// The run's effective offline posture.
    pub offline: bool,
    /// The run's RFC 3339 clock value, stamped into every record.
    pub created_at: &'a str,
}

impl BuildExecution<'_> {
    /// The shipped engine-owned build output root.
    ///
    /// ```
    /// assert_eq!(vibe_lifecycle::BuildExecution::default_build_root(), "target");
    /// ```
    #[must_use]
    pub const fn default_build_root() -> &'static str {
        DEFAULT_BUILD_ROOT
    }
}

/// One artifact a build target produced, as the engine recorded it.
///
/// ```
/// use vibe_lifecycle::ProducedArtifact;
///
/// let produced = ProducedArtifact {
///     id: "vibe-helper.exe".into(),
///     path_absolute: "C:/w/target/debug/vibe-helper.exe".into(),
///     path_relative: "target/debug/vibe-helper.exe".into(),
///     digest: "0".repeat(64),
///     bytes: 4096,
///     fresh: false,
///     record: ".vibe/state/artifacts/vibe-helper.exe.json".into(),
/// };
/// assert_eq!(produced.digest.len(), 64);
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProducedArtifact {
    pub id: String,
    pub path_absolute: String,
    pub path_relative: String,
    /// 64 lowercase hex over the produced bytes.
    pub digest: String,
    pub bytes: u64,
    /// Cargo's own freshness verdict for this artifact — §5.0.5's
    /// "the record notes Cargo's own `fresh` verdict".
    pub fresh: bool,
    /// Project-relative path of the written artifact record.
    pub record: String,
}

/// What one executed build target did, including the routing decision a
/// narration has to show (§3.1: "the logical key, selected provider,
/// displaced default …").
///
/// ```
/// use vibe_lifecycle::BuildOutcome;
///
/// let outcome = BuildOutcome {
///     target: "vibe-helper".into(),
///     mechanism: "build:cargo".into(),
///     provider: "org.vibevm/vibe#cargo".into(),
///     via: "the shipped builtin default".into(),
///     displaced_default: None,
///     produced: Vec::new(),
/// };
/// assert!(outcome.displaced_default.is_none());
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildOutcome {
    pub target: String,
    pub mechanism: String,
    pub provider: String,
    /// Which of §3.1's steps selected the provider.
    pub via: String,
    /// The builtin default this selection displaced, if any.
    pub displaced_default: Option<String>,
    pub produced: Vec<ProducedArtifact>,
}

/// Execute every declared build target in dependency order.
///
/// The canonical use is on [`BuildExecution`]. Order is the A1 graph's:
/// a target that consumes another's output runs after it, and targets
/// that do not constrain each other keep declaration order, so two runs of
/// one manifest execute in one sequence.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
pub fn execute_build_targets(
    execution: &BuildExecution<'_>,
) -> Result<Vec<BuildOutcome>, BuildError> {
    let mut outcomes = Vec::with_capacity(execution.targets.len());
    for index in order(execution.targets)? {
        let Some(target) = execution.targets.get(index) else {
            continue;
        };
        outcomes.push(execute_one(execution, target)?);
    }
    Ok(outcomes)
}

/// Resolve one target's provider through the ONE law, then run it.
fn execute_one(
    execution: &BuildExecution<'_>,
    target: &ArtifactBuildTarget,
) -> Result<BuildOutcome, BuildError> {
    let selection = resolve_mechanism(
        execution.registry,
        &target.mechanism,
        target.provider.as_ref(),
        execution.routes,
    )?;
    let row = selection.row();
    let pin = row.pin().to_string();
    let key = target.mechanism.to_string();
    match row.handler() {
        ExtensionHandler::Builtin { name } if name == BUILTIN_CARGO_NAME => {}
        ExtensionHandler::Builtin { name } => {
            return Err(BuildError::UnknownBuiltinProvider {
                key,
                pin,
                name: name.clone(),
            });
        }
        handler => {
            return Err(BuildError::TransportNotLanded {
                key,
                pin,
                kind: handler.kind().to_string(),
            });
        }
    }
    let request = BuildTargetRequest {
        target,
        project_root: execution.project_root,
        build_root: execution.build_root,
        offline: execution.offline,
    };
    let provider = CargoProvider;
    let plan = provider.plan(&request)?;
    let toolchain = provider.fingerprint(&request)?;
    let selected = provider.apply(&request, &plan)?;
    let produced = record_all(
        execution, &request, &provider, &plan, &toolchain, &selected, &pin,
    )?;
    Ok(BuildOutcome {
        target: target.id.clone(),
        mechanism: key,
        provider: pin,
        via: selection.via().to_string(),
        displaced_default: displaced(&selection),
        produced,
    })
}

/// Verify, record and publish every artifact one target produced.
#[allow(clippy::too_many_arguments)]
fn record_all(
    execution: &BuildExecution<'_>,
    request: &BuildTargetRequest<'_>,
    provider: &CargoProvider,
    plan: &BuildPlan,
    toolchain: &ToolchainIdentity,
    selected: &[SelectedArtifact],
    pin: &str,
) -> Result<Vec<ProducedArtifact>, BuildError> {
    let descriptor = provider.descriptor();
    let fingerprint = config_digest(
        &request.target.mechanism,
        pin,
        &plan.build_argv,
        &plan.inputs,
    );
    let mut produced = Vec::with_capacity(selected.len());
    for artifact in selected {
        let verified = provider.verify(request, artifact)?;
        let evidence = sanitize(&format!(
            "{}; {}; {}; cargo-fresh={}; network={}; sha256 over {} byte(s) at {}",
            descriptor.posture(),
            toolchain.cargo,
            toolchain.rustc,
            artifact.fresh,
            plan.network,
            verified.bytes,
            verified.path_relative,
        ));
        // The freshness triple is the honest one for a provider-fresh
        // target (§4.1, §5.0.5): `inputs` is ABSENT, because Cargo owns
        // inputs the engine does not model and a fabricated input digest
        // would be a claim the engine cannot support; `config` and
        // `toolchain` are present, because the engine really did hash the
        // target's config and the provider really did report its
        // toolchain identity.
        let record = build_record(&RecordInputs {
            target: &request.target.id,
            mechanism: &request.target.mechanism,
            provider_key: pin,
            provider_version: None,
            provider_hash: None,
            output_id: &verified.output_id,
            kind: artifact.kind,
            shape: ArtifactShape::File,
            digest: &verified.digest,
            path_absolute: &verified.path_absolute,
            path_relative: &verified.path_relative,
            freshness: RecordFreshness {
                inputs: None,
                config: Some(&fingerprint),
                toolchain: Some(&toolchain.digest),
            },
            platform: toolchain.host.as_deref(),
            media_type: None,
            created_at: execution.created_at,
            evidence,
        })?;
        let path = write_record(execution.project_root, &record)?;
        produced.push(ProducedArtifact {
            id: verified.output_id,
            path_absolute: verified.path_absolute,
            path_relative: verified.path_relative,
            digest: verified.digest,
            bytes: verified.bytes,
            fresh: artifact.fresh,
            record: path,
        });
    }
    Ok(produced)
}

/// The displaced builtin default, when a replacement really replaced one.
fn displaced(selection: &MechanismSelection<'_>) -> Option<String> {
    match selection.via() {
        SelectionStep::BuiltinDefault => None,
        SelectionStep::TargetPin | SelectionStep::HostRoute => selection
            .displaced_default()
            .map(|row| row.pin().to_string()),
    }
}

/// The build graph, in dependency order.
///
/// The walk itself is the shared one; what is stated here is the build
/// role's own decision about an unresolved input. A build input naming an
/// artifact no build target produces is an ERROR: the build graph is
/// closed under itself, and the phase-forward law gives it no other
/// producer to have come from.
fn order(targets: &[ArtifactBuildTarget]) -> Result<Vec<usize>, BuildError> {
    dag_order(targets, Unresolved::Refuse).map_err(|fault| match fault {
        OrderFault::Cycle { cycle } => BuildError::Cycle { cycle },
        OrderFault::UnknownInput { target, input } => BuildError::UnknownInput { target, input },
    })
}

impl GraphNode for ArtifactBuildTarget {
    fn id(&self) -> &str {
        &self.id
    }

    fn outputs(&self) -> &[vibe_core::manifest::ArtifactOutput] {
        &self.outputs
    }

    fn inputs(&self) -> Option<&[vibe_core::manifest::ArtifactInput]> {
        self.inputs.as_deref()
    }
}

#[cfg(test)]
#[path = "build/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "build/e2e_tests.rs"]
mod e2e_tests;
