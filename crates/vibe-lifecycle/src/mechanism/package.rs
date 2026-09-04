//! The package phase's mechanism executor — §6.0.2's half of the one
//! wiring.
//!
//! The build executor's exact sibling, and deliberately built from the
//! same parts: one resolver ([`resolve_mechanism`]), one record writer,
//! one containment cell. There is no `if builtin` shortcut here either —
//! the executor asks who services the target's logical key and only then
//! looks at the selected row's handler, so a host that routes
//! `package:static-skill` to a plugin gets the plugin's refusal and
//! demonstrably NOT a static skill.
//!
//! Two things are this phase's own:
//!
//! 1. **inputs are resolved before a provider sees them.** A consumed
//!    artifact is found through the engine's own record and re-proven; see
//!    [`inputs`];
//! 2. **the output directory is prepared by the engine.** A target's
//!    distributable lands in `target/vibe-package/<target-id>/`, and the
//!    engine empties that directory first — otherwise a stale file from a
//!    previous run would enter a fresh directory digest and the record
//!    would describe a tree nobody produced.
//!
//! [`resolve_mechanism`]: vibe_extension_registry::resolve_mechanism

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use std::path::Path;

use specmark::spec;
use vibe_core::manifest::{ArtifactPackageTarget, ExtensionHandler, MechanismRoutes};
use vibe_extension_registry::{
    MechanismRegistry, MechanismSelection, SelectionStep, resolve_mechanism,
};
use vibe_safefs::Project;

mod error;
mod inputs;
pub(crate) mod protocol;
pub(crate) mod static_file;

pub use error::PackageError;

use super::client_projection::ClientProjectionProvider;
use super::client_projection::client::ProjectionClient;
use super::contain::{forward_slashed, relative_to};
use super::order::{GraphNode, OrderFault, Unresolved, dag_order};
use super::plugin::AgentPluginProvider;
use super::record::{
    RecordFreshness, RecordInputs, build_record, config_digest, sanitize, write_record,
};
use super::skill::StaticSkillProvider;
use super::zip::WindowsZipProvider;
use super::{
    BUILTIN_AGENT_PLUGIN_NAME, BUILTIN_CLAUDE_PLUGIN_PROJECTION_NAME,
    BUILTIN_CODEX_PLUGIN_PROJECTION_NAME, BUILTIN_OPENCODE_PLUGIN_PROJECTION_NAME,
    BUILTIN_STATIC_FILE_NAME, BUILTIN_STATIC_SKILL_NAME, BUILTIN_WINDOWS_ZIP_NAME,
    DEFAULT_PACKAGE_ROOT, PackageProvider, PackageTargetRequest,
};
use inputs::resolve_inputs;
use protocol::{PackagePlan, StagedArtifact};
use static_file::StaticFileProvider;

/// Everything one package-phase execution needs, and nothing more.
///
/// The same shape as [`BuildExecution`](crate::BuildExecution), minus the
/// offline posture: neither packaging provider can reach the network under
/// any configuration, so a member saying whether it may would be a lie
/// with two legal values.
///
/// ```
/// use std::path::Path;
/// use vibe_core::manifest::MechanismRoutes;
/// use vibe_lifecycle::{PackageExecution, collect_mechanisms, execute_package_targets};
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
/// let execution = PackageExecution {
///     project_root: Path::new("."),
///     targets: &[],
///     registry: &registry,
///     routes: &routes,
///     package_root: PackageExecution::default_package_root(),
///     created_at: "2026-08-30T00:00:00Z",
/// };
///
/// // A project that declares no package target packages nothing and says so.
/// assert!(execute_package_targets(&execution).unwrap().is_empty());
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
#[derive(Debug, Clone, Copy)]
pub struct PackageExecution<'a> {
    /// The selected project's absolute root.
    pub project_root: &'a Path,
    /// The landed `[[artifacts.package]]` rows, in declaration order.
    pub targets: &'a [ArtifactPackageTarget],
    /// The collected mechanism plane of this world.
    pub registry: &'a MechanismRegistry,
    /// The host's `[mechanisms]` routes.
    pub routes: &'a MechanismRoutes,
    /// The engine-owned package output root, relative to `project_root`.
    /// [`PackageExecution::default_package_root`] is the shipped value.
    pub package_root: &'a str,
    /// The run's RFC 3339 clock value, stamped into every record.
    pub created_at: &'a str,
}

impl PackageExecution<'_> {
    /// The shipped engine-owned package output root.
    ///
    /// ```
    /// assert_eq!(
    ///     vibe_lifecycle::PackageExecution::default_package_root(),
    ///     "target/vibe-package",
    /// );
    /// ```
    #[must_use]
    pub const fn default_package_root() -> &'static str {
        DEFAULT_PACKAGE_ROOT
    }
}

/// One distributable a package target produced, as the engine recorded it.
///
/// ```
/// use vibe_lifecycle::PackagedArtifact;
///
/// let packaged = PackagedArtifact {
///     id: "demo.skill".into(),
///     path_absolute: "C:/w/target/vibe-package/demo/SKILL.md".into(),
///     path_relative: "target/vibe-package/demo/SKILL.md".into(),
///     digest: "0".repeat(64),
///     bytes: 512,
///     files: 1,
///     record: ".vibe/state/artifacts/demo.skill.json".into(),
/// };
/// assert_eq!(packaged.files, 1);
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackagedArtifact {
    pub id: String,
    pub path_absolute: String,
    pub path_relative: String,
    /// 64 lowercase hex — the file's SHA-256, or the canonical tree
    /// digest of a directory distributable.
    pub digest: String,
    pub bytes: u64,
    /// How many files the digest covers.
    pub files: usize,
    /// Project-relative path of the written artifact record.
    pub record: String,
}

/// What one executed package target did, including the routing decision.
///
/// ```
/// use vibe_lifecycle::PackageOutcome;
///
/// let outcome = PackageOutcome {
///     target: "demo-skill".into(),
///     mechanism: "package:static-skill".into(),
///     provider: "org.vibevm/vibe#static-skill".into(),
///     via: "the shipped builtin default".into(),
///     displaced_default: None,
///     produced: Vec::new(),
/// };
/// assert!(outcome.displaced_default.is_none());
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageOutcome {
    pub target: String,
    pub mechanism: String,
    pub provider: String,
    /// Which of §3.1's steps selected the provider.
    pub via: String,
    /// The builtin default this selection displaced, if any.
    pub displaced_default: Option<String>,
    pub produced: Vec<PackagedArtifact>,
}

/// Execute every declared package target in dependency order.
///
/// The canonical use is on [`PackageExecution`]. Order is the A1 graph's
/// among package targets; an input naming a BUILD output constrains
/// nothing here, because the build phase already ran and left its record.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
pub fn execute_package_targets(
    execution: &PackageExecution<'_>,
) -> Result<Vec<PackageOutcome>, PackageError> {
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
    execution: &PackageExecution<'_>,
    target: &ArtifactPackageTarget,
) -> Result<PackageOutcome, PackageError> {
    let selection = resolve_mechanism(
        execution.registry,
        &target.mechanism,
        target.provider.as_ref(),
        execution.routes,
    )?;
    let row = selection.row();
    let pin = row.pin().to_string();
    let key = target.mechanism.to_string();
    let provider: Builtin = match row.handler() {
        ExtensionHandler::Builtin { name } if name == BUILTIN_STATIC_FILE_NAME => {
            Builtin::StaticFile(StaticFileProvider)
        }
        ExtensionHandler::Builtin { name } if name == BUILTIN_STATIC_SKILL_NAME => {
            Builtin::StaticSkill(StaticSkillProvider)
        }
        ExtensionHandler::Builtin { name } if name == BUILTIN_AGENT_PLUGIN_NAME => {
            Builtin::AgentPlugin(AgentPluginProvider)
        }
        ExtensionHandler::Builtin { name } if name == BUILTIN_WINDOWS_ZIP_NAME => {
            Builtin::WindowsZip(WindowsZipProvider)
        }
        // §6.3's three projections: three distinct rows, three distinct
        // pins, ONE implementation parameterised by the client. The arms
        // stay separate because the registry rows are separate — a table
        // lookup here would hide which row selected what.
        ExtensionHandler::Builtin { name } if name == BUILTIN_CLAUDE_PLUGIN_PROJECTION_NAME => {
            Builtin::ClientProjection(ClientProjectionProvider::new(ProjectionClient::Claude))
        }
        ExtensionHandler::Builtin { name } if name == BUILTIN_CODEX_PLUGIN_PROJECTION_NAME => {
            Builtin::ClientProjection(ClientProjectionProvider::new(ProjectionClient::Codex))
        }
        ExtensionHandler::Builtin { name } if name == BUILTIN_OPENCODE_PLUGIN_PROJECTION_NAME => {
            Builtin::ClientProjection(ClientProjectionProvider::new(ProjectionClient::OpenCode))
        }
        ExtensionHandler::Builtin { name } => {
            return Err(PackageError::UnknownBuiltinProvider {
                key,
                pin,
                name: name.clone(),
            });
        }
        handler => {
            return Err(PackageError::TransportNotLanded {
                key,
                pin,
                kind: handler.kind().to_string(),
            });
        }
    };
    let resolved = resolve_inputs(execution.project_root, target)?;
    let request = PackageTargetRequest {
        target,
        project_root: execution.project_root,
        package_root: execution.package_root,
        inputs: &resolved,
    };
    let plan = provider.plan(&request)?;
    let fingerprint = provider.fingerprint(&request, &plan)?;
    if !provider.prepares_output() {
        prepare_output_dir(&request)?;
    }
    let staged = provider.apply(&request, &plan)?;
    let produced = record_all(
        execution,
        &request,
        &provider,
        &plan,
        &fingerprint.digest,
        fingerprint.counted,
        &staged,
        &pin,
    )?;
    Ok(PackageOutcome {
        target: target.id.clone(),
        mechanism: key,
        provider: pin,
        via: selection.via().to_string(),
        displaced_default: displaced(&selection),
        produced,
    })
}

/// The builtin package-role adapters, behind one dispatch.
///
/// The builtin set is closed, so this exhaustive enum is its dispatch map.
enum Builtin {
    StaticFile(StaticFileProvider),
    StaticSkill(StaticSkillProvider),
    AgentPlugin(AgentPluginProvider),
    WindowsZip(WindowsZipProvider),
    ClientProjection(ClientProjectionProvider),
}

impl Builtin {
    /// Static-file opens and proves its source before resetting the output
    /// directory, so its one streaming safefs operation owns that ordering.
    const fn prepares_output(&self) -> bool {
        matches!(self, Self::StaticFile(_))
    }
}

impl PackageProvider for Builtin {
    fn descriptor(&self) -> super::ProviderDescriptor {
        match self {
            Self::StaticFile(provider) => provider.descriptor(),
            Self::StaticSkill(provider) => provider.descriptor(),
            Self::AgentPlugin(provider) => provider.descriptor(),
            Self::WindowsZip(provider) => provider.descriptor(),
            Self::ClientProjection(provider) => provider.descriptor(),
        }
    }

    fn plan(
        &self,
        request: &PackageTargetRequest<'_>,
    ) -> Result<PackagePlan, super::MechanismError> {
        match self {
            Self::StaticFile(provider) => provider.plan(request),
            Self::StaticSkill(provider) => provider.plan(request),
            Self::AgentPlugin(provider) => provider.plan(request),
            Self::WindowsZip(provider) => provider.plan(request),
            Self::ClientProjection(provider) => provider.plan(request),
        }
    }

    fn fingerprint(
        &self,
        request: &PackageTargetRequest<'_>,
        plan: &PackagePlan,
    ) -> Result<protocol::PackageFingerprint, super::MechanismError> {
        match self {
            Self::StaticFile(provider) => provider.fingerprint(request, plan),
            Self::StaticSkill(provider) => provider.fingerprint(request, plan),
            Self::AgentPlugin(provider) => provider.fingerprint(request, plan),
            Self::WindowsZip(provider) => provider.fingerprint(request, plan),
            Self::ClientProjection(provider) => provider.fingerprint(request, plan),
        }
    }

    fn apply(
        &self,
        request: &PackageTargetRequest<'_>,
        plan: &PackagePlan,
    ) -> Result<Vec<StagedArtifact>, super::MechanismError> {
        match self {
            Self::StaticFile(provider) => provider.apply(request, plan),
            Self::StaticSkill(provider) => provider.apply(request, plan),
            Self::AgentPlugin(provider) => provider.apply(request, plan),
            Self::WindowsZip(provider) => provider.apply(request, plan),
            Self::ClientProjection(provider) => provider.apply(request, plan),
        }
    }

    fn verify(
        &self,
        request: &PackageTargetRequest<'_>,
        staged: &StagedArtifact,
    ) -> Result<protocol::VerifiedPackageArtifact, super::MechanismError> {
        match self {
            Self::StaticFile(provider) => provider.verify(request, staged),
            Self::StaticSkill(provider) => provider.verify(request, staged),
            Self::AgentPlugin(provider) => provider.verify(request, staged),
            Self::WindowsZip(provider) => provider.verify(request, staged),
            Self::ClientProjection(provider) => provider.verify(request, staged),
        }
    }
}

/// Empty and recreate the target's own engine-owned output directory.
///
/// The engine does this, not the provider: §3.2 gives the engine scratch
/// paths, and determinism demands it. A directory digest over a tree that
/// still holds a previous run's file would describe something nobody
/// produced, and the record would be a true statement about a wrong thing.
/// A link occupying the path is refused rather than followed — removing
/// through one would delete somebody else's tree.
fn prepare_output_dir(request: &PackageTargetRequest<'_>) -> Result<(), PackageError> {
    let refuse = |reason: String| PackageError::OutputRoot {
        target: request.target.id.clone(),
        path: request.output_dir_relative(),
        reason,
    };
    let project =
        Project::open(request.project_root).map_err(|error| refuse(format!("{error:#}")))?;
    project
        .reset_dir(&request.output_dir_relative())
        .map(|_| ())
        .map_err(|error| refuse(format!("{error:#}")))
}

/// Verify, record and publish every distributable one target produced.
#[allow(clippy::too_many_arguments)]
fn record_all(
    execution: &PackageExecution<'_>,
    request: &PackageTargetRequest<'_>,
    provider: &Builtin,
    plan: &PackagePlan,
    inputs_digest: &str,
    counted: usize,
    staged: &[StagedArtifact],
    pin: &str,
) -> Result<Vec<PackagedArtifact>, PackageError> {
    let descriptor = provider.descriptor();
    let config_fingerprint = config_digest(
        &request.target.mechanism,
        pin,
        std::slice::from_ref(&plan.summary),
        &plan.inputs,
    );
    let mut produced = Vec::with_capacity(staged.len());
    for artifact in staged {
        let verified = provider.verify(request, artifact)?;
        // The input ORIGINS are named, not merely counted: §6.0.2's whole
        // point is that a consumed artifact came from the engine's own
        // record rather than a guessed path, and evidence that cannot say
        // which happened cannot corroborate it.
        let evidence = sanitize(&format!(
            "{}; {}; engine-fresh over {counted} declared input(s) [{}]; {} file(s) covering {} \
             byte(s) at {}",
            descriptor.posture(),
            plan.summary,
            origins(request),
            verified.files,
            verified.bytes,
            verified.path_relative,
        ));
        // Engine-fresh, and the record says so by PRESENCE: §4.1 admits
        // it "only when the complete input set is closed and hashable",
        // and every engine-fresh package provider's input set is exactly
        // that (§§6.1–6.3, §7.0.8). `toolchain` is absent because no toolchain
        // took part — the transformation is this engine's own.
        let record = build_record(&RecordInputs {
            target: &request.target.id,
            mechanism: &request.target.mechanism,
            provider_key: pin,
            provider_version: None,
            provider_hash: None,
            output_id: &verified.output_id,
            kind: artifact.kind,
            shape: artifact.shape.clone(),
            digest: &verified.digest,
            path_absolute: &verified.path_absolute,
            path_relative: &verified.path_relative,
            freshness: RecordFreshness {
                inputs: Some(inputs_digest),
                config: Some(&config_fingerprint),
                toolchain: None,
            },
            platform: None,
            media_type: artifact.media_type.as_deref(),
            created_at: execution.created_at,
            evidence,
        })?;
        let path = write_record(execution.project_root, &record)?;
        produced.push(PackagedArtifact {
            id: verified.output_id,
            path_absolute: verified.path_absolute,
            path_relative: verified.path_relative,
            digest: verified.digest,
            bytes: verified.bytes,
            files: verified.files,
            record: path,
        });
    }
    Ok(produced)
}

/// How many declared inputs came from a record and how many from the
/// workspace — the evidence's own witness for §6.0.2's law.
fn origins(request: &PackageTargetRequest<'_>) -> String {
    let recorded = request
        .inputs
        .iter()
        .filter(|input| input.origin.recorded_kind().is_some())
        .count();
    let workspace = request.inputs.len() - recorded;
    format!(
        "{}={recorded} {}={workspace}",
        protocol::InputOrigin::RECORD_SPELLING,
        protocol::InputOrigin::WORKSPACE_SPELLING,
    )
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

/// The package graph, in dependency order.
///
/// The walk itself is the shared one; what is stated here is the package
/// role's own decision about an unresolved input. A consumed artifact that
/// no package target here produces is NOT an error — it is a build output,
/// and the input resolver finds it in the engine's record or refuses BY
/// NAME there, where the refusal can say which record was missing.
fn order(targets: &[ArtifactPackageTarget]) -> Result<Vec<usize>, PackageError> {
    dag_order(targets, Unresolved::Defer).map_err(|fault| match fault {
        OrderFault::Cycle { cycle } => PackageError::Cycle { cycle },
        // Unreachable under `Defer`, and a refusal rather than a panic for
        // exactly that reason.
        OrderFault::UnknownInput { target, input } => {
            PackageError::InputNotRecorded { target, input }
        }
    })
}

impl GraphNode for ArtifactPackageTarget {
    fn id(&self) -> &str {
        &self.id
    }

    fn produces(&self) -> Vec<&str> {
        self.outputs
            .iter()
            .map(|output| output.id.as_str())
            .collect()
    }

    fn consumes(&self) -> Vec<&str> {
        self.inputs
            .iter()
            .flatten()
            .filter_map(vibe_core::manifest::ArtifactInput::artifact_ref)
            .collect()
    }
}

/// The project-relative identity of one produced path, or the containment
/// refusal that says why the engine will not mint one.
pub(crate) fn contained_identity(
    request: &PackageTargetRequest<'_>,
    output_id: &str,
    absolute: &Path,
) -> Result<(String, String), super::MechanismError> {
    let outside = || super::MechanismError::PackageOutsideRoot {
        target: request.target.id.clone(),
        output: output_id.to_owned(),
        path: super::error::preview(&forward_slashed(absolute)),
        package_root: request.output_dir_relative(),
    };
    if relative_to(absolute, &request.output_dir()).is_none() && absolute != request.output_dir() {
        return Err(outside());
    }
    let relative = relative_to(absolute, request.project_root).ok_or_else(outside)?;
    Ok((forward_slashed(absolute), relative))
}

// The fixture home the whole package-phase suite shares — including the
// two provider-law cells, which is why it is `pub(crate)` under the test
// cfg rather than private to this module.
#[cfg(test)]
#[path = "package/support.rs"]
pub(crate) mod support;

#[cfg(test)]
#[path = "package/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "package/e2e_tests.rs"]
mod e2e_tests;

#[cfg(test)]
#[path = "package/chain_tests.rs"]
mod chain_tests;
