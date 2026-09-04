//! The mechanism EXECUTION seam — §5.0 of the packages-2026-09 build/
//! package/deploy architecture, the first executing mechanism.
//!
//! R8-MECHANISM landed the plane, the routes and the selection law; nothing
//! selected a provider because nothing executed one. This module is the
//! other half: the build phase walks the landed `[[artifacts.build]]`
//! targets, asks the ONE resolver ([`resolve_mechanism`]) who services each
//! target's logical key, and runs the selected provider.
//!
//! Three decisions of §5.0 shape everything here and are implemented, not
//! re-decided:
//!
//! 1. **Execution lives beside the phase machine.** The engine owns
//!    ordering, paths, artifact identities, state and narration (§3.2);
//!    a provider owns only the transformation. Builtin providers are
//!    IN-PROCESS implementations of one crate-internal trait
//!    ([`BuildProvider`]) mirroring the four §3.2 operations a build
//!    provider needs — `plan`, `fingerprint`, `apply`, `verify`. There is
//!    deliberately no in-process copy of the out-of-process envelope:
//!    serialising a request to hand it to a function in the same process
//!    is theater with no second process to justify it.
//! 2. **A resolved NON-builtin refuses typed.** The script/binary/native
//!    provider transport is a later atom, so a routed-away build target
//!    says exactly that instead of pretending
//!    ([`BuildError::TransportNotLanded`]).
//! 3. **Records are the engine's.** A provider reports what it produced;
//!    the engine writes `.vibe/state/artifacts/<output-id>.json` after
//!    validating it through the A2 behaviour cell.
//!
//! Nothing in this module reads the operator's settings home, and no
//! secret can reach a provider: the Cargo adapter builds its argv from the
//! target's own config and inherits only the environment the toolchain
//! already needs.
//!
//! [`resolve_mechanism`]: vibe_extension_registry::resolve_mechanism

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE");

use std::path::{Path, PathBuf};

use vibe_core::manifest::{ArtifactBuildTarget, ArtifactKind, ArtifactPackageTarget};

pub(crate) mod build;
pub(crate) mod cargo;
pub(crate) mod client_projection;
pub(crate) mod contain;
pub(crate) mod deploy;
pub(crate) mod error;
pub(crate) mod order;
pub(crate) mod package;
pub(crate) mod plugin;
pub(crate) mod record;
pub(crate) mod skill;
pub(crate) mod vibebin;
pub(crate) mod zip;

pub use build::{
    BuildError, BuildExecution, BuildOutcome, ProducedArtifact, execute_build_targets,
};
pub use client_projection::ClientProjectionError;
pub use deploy::{
    ClientExecutable, ClientExecutables, DEPLOY_STATE_DIR, DeployError, DeployExecution,
    DeployOutcome, DeployPlanReport, DeployResourcePlan, DeploySelection, DeployStatus,
    DeployedResource, DeploymentRow, RemovalOutcome, deploy_state_home, execute_deploy_targets,
    list_deployments, plan_deploy_targets, undeploy_targets,
};
pub use error::{DeployProviderError, MechanismError};
pub use package::{
    PackageError, PackageExecution, PackageOutcome, PackagedArtifact, execute_package_targets,
};
pub use record::{ARTIFACT_RECORD_DIR, RecordError};

// The deploy role's protocol lives beside its value types; it is named
// here so every use site keeps spelling it `crate::mechanism::…`.
pub(crate) use deploy::protocol::{DeployProvider, DeployTargetRequest};
use package::protocol::{
    PackageFingerprint, PackagePlan, ResolvedInput, StagedArtifact, VerifiedPackageArtifact,
};

/// The engine-owned build output root, project-relative.
///
/// §3.2 is explicit that a provider "cannot mint an unscoped output path":
/// the engine chooses where a build writes, passes it as `--target-dir`,
/// and therefore always knows the produced artifact's project-relative
/// identity. Cargo still owns everything *inside* that directory — its
/// incremental machinery is untouched.
pub(crate) const DEFAULT_BUILD_ROOT: &str = "target";

/// The engine-owned package output root, project-relative — §6.0.6's
/// "distributables land under the engine-owned package root
/// `target/vibe-package/<target-id>/…`".
///
/// The same law as the build root and for the same sentence of §3.2: a
/// packaging provider is TOLD where its distributable goes and proves
/// afterwards that what it produced is really there, so no provider can
/// mint an identity the engine then has to trust.
pub(crate) const DEFAULT_PACKAGE_ROOT: &str = "target/vibe-package";

/// §3.2's effect classes — what scope an operation can touch.
///
/// The four §3.2 vocabularies below are closed sets, and each is spelled
/// in full even though the one build-role provider that exists today
/// declares a single member of each. Narrowing a vocabulary to the members
/// today's providers happen to use would make the type a snapshot of the
/// current provider set rather than a property of the plane, and the very
/// next provider — a deploy one, reconciling `user` scope irreversibly —
/// would have to reopen it. The unused members are the vocabulary, not
/// spare code.
#[allow(dead_code)] // closed §3.2 vocabulary; deploy-role providers declare the rest
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EffectClass {
    /// Only the selected workspace tree.
    Workspace,
    /// The invoking user's home state.
    User,
    /// A remote destination.
    Remote,
    /// Machine-wide state.
    System,
}

impl EffectClass {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::User => "user",
            Self::Remote => "remote",
            Self::System => "system",
        }
    }
}

/// Whether an operation can reach the network.
#[allow(dead_code)] // closed §3.2 vocabulary; a packaging provider declares `never`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NetworkUse {
    /// Never, under any configuration.
    Never,
    /// Only when the run is not offline — the honest answer for Cargo,
    /// which fetches registry dependencies unless `offline`/`frozen` say
    /// otherwise. The fixture path pins offline, so it reaches nothing.
    WhenNotOffline,
}

impl NetworkUse {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Never => "never",
            Self::WhenNotOffline => "when-online",
        }
    }
}

/// Whether an operation needs elevated privilege before it can apply.
#[allow(dead_code)] // closed §3.2 vocabulary; a system-scope deploy provider declares `elevated`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PrivilegeNeed {
    None,
    Elevated,
}

impl PrivilegeNeed {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Elevated => "elevated",
        }
    }
}

/// Whether an applied operation can be undone. §3.2: "An installer that
/// cannot roll back says so before apply; VibeVM never implies a
/// transaction it cannot provide." A build produces artifacts inside an
/// engine-owned output root and reconciles no destination, so the question
/// does not apply to it — which is a different answer from "no".
#[allow(dead_code)] // closed §3.2 vocabulary; deploy providers declare the other two
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reversibility {
    NotApplicable,
    Reversible,
    Irreversible,
}

impl Reversibility {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::NotApplicable => "n/a",
            Self::Reversible => "reversible",
            Self::Irreversible => "irreversible",
        }
    }
}

/// One §3.2 provider operation.
#[allow(dead_code)] // closed §3.2 vocabulary; `remove`/`recover` are deploy-role operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProviderOperation {
    Plan,
    Fingerprint,
    Apply,
    Verify,
    Remove,
    Recover,
}

impl ProviderOperation {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Plan => "plan",
            Self::Fingerprint => "fingerprint",
            Self::Apply => "apply",
            Self::Verify => "verify",
            Self::Remove => "remove",
            Self::Recover => "recover",
        }
    }
}

/// What a provider declares about itself before it is asked to do
/// anything — §3.2's descriptor list, as far as a build-role provider
/// needs it.
///
/// Every member is read: [`ProviderDescriptor::supports`] gates the
/// declared output kinds, and the rest are rendered into the artifact
/// record's evidence summary, so a record says under what declared posture
/// its artifact was produced.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ProviderDescriptor {
    /// The reserved provider identity this descriptor belongs to.
    pub(crate) key: &'static str,
    /// The artifact kinds the provider can produce.
    pub(crate) kinds: &'static [ArtifactKind],
    pub(crate) effect: EffectClass,
    pub(crate) network: NetworkUse,
    pub(crate) privilege: PrivilegeNeed,
    pub(crate) reversibility: Reversibility,
    /// The §3.2 operations this provider implements. A build provider
    /// implements four; `remove`/`recover` are deploy-only by the
    /// architecture's own sentence.
    pub(crate) operations: &'static [ProviderOperation],
}

impl ProviderDescriptor {
    /// Whether the provider can produce one declared artifact kind.
    pub(crate) fn supports(&self, kind: ArtifactKind) -> bool {
        self.kinds.contains(&kind)
    }

    /// The compact posture summary a record's evidence carries. Control-
    /// free and non-blank by construction, which is what the A2 free-text
    /// law asks of it.
    pub(crate) fn posture(&self) -> String {
        format!(
            "provider {} effect={} network={} privilege={} reversibility={} ops={}",
            self.key,
            self.effect.as_str(),
            self.network.as_str(),
            self.privilege.as_str(),
            self.reversibility.as_str(),
            self.operations
                .iter()
                .map(|operation| operation.as_str())
                .collect::<Vec<_>>()
                .join("+"),
        )
    }
}

/// One build target as the engine hands it to a provider.
///
/// The provider receives the declared target and the engine's own paths,
/// and nothing else: no ambient environment, no settings home, no run
/// credentials. `build_root` is engine-owned (§3.2) — the provider is told
/// where to write, it does not choose.
#[derive(Debug, Clone, Copy)]
pub(crate) struct BuildTargetRequest<'a> {
    pub(crate) target: &'a ArtifactBuildTarget,
    /// The selected project's absolute root.
    pub(crate) project_root: &'a Path,
    /// The engine-owned build output root, relative to `project_root`.
    pub(crate) build_root: &'a str,
    /// The run's effective offline posture, folded with the target's own
    /// `offline` config member — either one forbids the network.
    pub(crate) offline: bool,
}

impl BuildTargetRequest<'_> {
    /// The absolute directory the provider runs in — `project_root` joined
    /// with the target's declarant-relative `workdir`.
    pub(crate) fn workdir(&self) -> PathBuf {
        if self.target.workdir == "." {
            return self.project_root.to_path_buf();
        }
        let mut path = self.project_root.to_path_buf();
        for segment in self.target.workdir.split('/') {
            path.push(segment);
        }
        path
    }

    /// The absolute engine-owned build output root.
    pub(crate) fn target_dir(&self) -> PathBuf {
        let mut path = self.project_root.to_path_buf();
        for segment in self.build_root.split('/') {
            path.push(segment);
        }
        path
    }
}

/// The in-process builtin provider protocol for the build role — §3.2's
/// operations, minus the two the architecture reserves for deploy.
///
/// Crate-internal on purpose: the out-of-process transport (script/binary/
/// native envelopes) is a later atom, and publishing this trait now would
/// freeze an in-process shape as the provider contract before the real one
/// exists.
pub(crate) trait BuildProvider {
    /// What this provider declares about itself.
    fn descriptor(&self) -> ProviderDescriptor;

    /// Validate the target's config, resolve its declared inputs and
    /// outputs, and report the argv this provider WOULD run. Pure: it
    /// spawns nothing, creates nothing, and writes nothing.
    fn plan(&self, request: &BuildTargetRequest<'_>) -> Result<cargo::BuildPlan, MechanismError>;

    /// The provider/toolchain portion of the freshness digest. It takes
    /// the request because a Rust toolchain is resolved per directory —
    /// asking `cargo` for its version anywhere but the target's own
    /// workdir can name a different toolchain than the one that builds.
    fn fingerprint(
        &self,
        request: &BuildTargetRequest<'_>,
    ) -> Result<cargo::ToolchainIdentity, MechanismError>;

    /// Perform the declared transformation and report what it produced.
    fn apply(
        &self,
        request: &BuildTargetRequest<'_>,
        plan: &cargo::BuildPlan,
    ) -> Result<Vec<cargo::SelectedArtifact>, MechanismError>;

    /// Independently prove one produced artifact exists and digest it.
    fn verify(
        &self,
        request: &BuildTargetRequest<'_>,
        selected: &cargo::SelectedArtifact,
    ) -> Result<cargo::VerifiedArtifact, MechanismError>;
}

/// One package target as the engine hands it to a provider.
///
/// The difference from its build sibling is the one §6.0.2 names: a
/// package target's inputs are already RESOLVED when the provider sees
/// them. An input naming a build output was found through the engine's own
/// `.vibe/state/artifacts/` record and re-proven against the file; an
/// input naming a workspace path was contained and digested. A provider
/// therefore never reads the engine's state, never guesses an artifact
/// path, and cannot be handed a stale one.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PackageTargetRequest<'a> {
    pub(crate) target: &'a ArtifactPackageTarget,
    /// The selected project's absolute root.
    pub(crate) project_root: &'a Path,
    /// The engine-owned package output root, relative to `project_root`.
    pub(crate) package_root: &'a str,
    /// This target's declared inputs, already resolved and proven.
    pub(crate) inputs: &'a [ResolvedInput],
}

impl PackageTargetRequest<'_> {
    /// The absolute engine-owned output directory of THIS target —
    /// `<project>/<package root>/<target id>`.
    pub(crate) fn output_dir(&self) -> PathBuf {
        let mut path = self.project_root.to_path_buf();
        for segment in self.package_root.split('/') {
            path.push(segment);
        }
        path.push(&self.target.id);
        path
    }

    /// The same directory's project-relative, forward-slashed identity.
    pub(crate) fn output_dir_relative(&self) -> String {
        format!("{}/{}", self.package_root, self.target.id)
    }
}

/// The in-process builtin provider protocol for the package role.
///
/// The SAME four §3.2 operations as [`BuildProvider`], and deliberately
/// not a second protocol shape: §6.0.1 rules "one seam, two roles". What
/// differs is only what an operation is given — a package provider is
/// handed resolved inputs and an output directory instead of a workdir and
/// a build root — because that is a difference between the two ROLES, not
/// between two protocols.
pub(crate) trait PackageProvider {
    /// What this provider declares about itself.
    fn descriptor(&self) -> ProviderDescriptor;

    /// Validate the target's config, resolve its declared outputs, and
    /// report what this provider WOULD produce.
    ///
    /// READ-ONLY, which §6.3.0.11 states as the law for every plan on this
    /// plane: "Plan and verify use read-only probes only." The three §6
    /// producing providers need no probe at all and take none; a §6.3
    /// client projection probes the canonical tree it was handed, because
    /// its capability report — which requested component is missing, which
    /// member this client cannot express — is a fact about that tree, and a
    /// plan that could not state it would promise a projection apply would
    /// then refuse. No operation here creates or writes anything.
    fn plan(&self, request: &PackageTargetRequest<'_>) -> Result<PackagePlan, MechanismError>;

    /// The freshness fingerprint over the target's COMPLETE closed input
    /// set — §4.1's engine freshness, which both packaging providers are
    /// entitled to because their inputs really are closed and hashable
    /// (§6.0.3, §6.0.4). It reads the declared entry documents, so it is
    /// not pure; it writes nothing.
    fn fingerprint(
        &self,
        request: &PackageTargetRequest<'_>,
        plan: &PackagePlan,
    ) -> Result<PackageFingerprint, MechanismError>;

    /// Produce the distributable inside the engine-owned output directory.
    fn apply(
        &self,
        request: &PackageTargetRequest<'_>,
        plan: &PackagePlan,
    ) -> Result<Vec<StagedArtifact>, MechanismError>;

    /// Independently prove one produced distributable and digest it.
    fn verify(
        &self,
        request: &PackageTargetRequest<'_>,
        staged: &StagedArtifact,
    ) -> Result<VerifiedPackageArtifact, MechanismError>;
}

/// The reserved identity of the one builtin build provider — §3's
/// `org.vibevm/vibe#cargo`, "not a privileged branch outside the
/// registry".
pub(crate) const BUILTIN_CARGO_PIN: &str = "org.vibevm/vibe#cargo";

/// The `handler = { kind = "builtin", name = … }` spelling of the same row.
pub(crate) const BUILTIN_CARGO_NAME: &str = "cargo";

/// The reserved identity of the §6.1 packaging provider.
pub(crate) const BUILTIN_STATIC_SKILL_PIN: &str = "org.vibevm/vibe#static-skill";

/// The `handler = { kind = "builtin", name = … }` spelling of the same row.
pub(crate) const BUILTIN_STATIC_SKILL_NAME: &str = "static-skill";

/// The reserved identity of the §6.2 packaging provider.
pub(crate) const BUILTIN_AGENT_PLUGIN_PIN: &str = "org.vibevm/vibe#agent-plugin";

/// The `handler = { kind = "builtin", name = … }` spelling of the same row.
pub(crate) const BUILTIN_AGENT_PLUGIN_NAME: &str = "agent-plugin";

/// The reserved identity of the §7.0.8 packaging provider.
pub(crate) const BUILTIN_WINDOWS_ZIP_PIN: &str = "org.vibevm/vibe#windows-zip";

/// The `handler = { kind = "builtin", name = … }` spelling of the same row.
pub(crate) const BUILTIN_WINDOWS_ZIP_NAME: &str = "windows-zip";

/// The reserved identity of §13.1's opaque static-file packager.
pub(crate) const BUILTIN_STATIC_FILE_PIN: &str = "org.vibevm/vibe#static-file";

/// The `handler = { kind = "builtin", name = … }` spelling of the same row.
pub(crate) const BUILTIN_STATIC_FILE_NAME: &str = "static-file";

/// The reserved identity of §6.3's Claude projection provider.
///
/// The three pins below carry §6.3.0.2's deliberate lesson: a provider id
/// is not a logical name. `#claude-plugin-projection` services
/// `package:claude-plugin`, because the reserved owner already keys a
/// DEPLOY row `#claude-plugin` that installs what this one projects.
pub(crate) const BUILTIN_CLAUDE_PLUGIN_PROJECTION_PIN: &str =
    "org.vibevm/vibe#claude-plugin-projection";

/// The `handler = { kind = "builtin", name = … }` spelling of the same row.
pub(crate) const BUILTIN_CLAUDE_PLUGIN_PROJECTION_NAME: &str = "claude-plugin-projection";

/// The reserved identity of §6.3's Codex projection provider.
pub(crate) const BUILTIN_CODEX_PLUGIN_PROJECTION_PIN: &str =
    "org.vibevm/vibe#codex-plugin-projection";

/// The `handler = { kind = "builtin", name = … }` spelling of the same row.
pub(crate) const BUILTIN_CODEX_PLUGIN_PROJECTION_NAME: &str = "codex-plugin-projection";

/// The reserved identity of §6.3's OpenCode projection provider.
pub(crate) const BUILTIN_OPENCODE_PLUGIN_PROJECTION_PIN: &str =
    "org.vibevm/vibe#opencode-plugin-projection";

/// The `handler = { kind = "builtin", name = … }` spelling of the same row.
pub(crate) const BUILTIN_OPENCODE_PLUGIN_PROJECTION_NAME: &str = "opencode-plugin-projection";

/// The reserved identity of the §7.1 deploy provider — the ONE deploy
/// builtin, and since R8-VIBE-BIN a provider that really runs.
///
/// The executor matches on the handler NAME (the row's own spelling); the
/// pin is what the provider's own descriptor answers under, so a receipt
/// records the exact identity that reconciled the destination.
pub(crate) const BUILTIN_VIBE_BIN_PIN: &str = "org.vibevm/vibe#vibe-bin";

/// The `handler = { kind = "builtin", name = … }` spelling of the same row.
pub(crate) const BUILTIN_VIBE_BIN_NAME: &str = "vibe-bin";

/// The reserved identity of §13.1's receipt-owned opt launcher provider.
pub(crate) const BUILTIN_VIBE_OPT_LAUNCHER_PIN: &str = "org.vibevm/vibe#vibe-opt-launcher";

/// The `handler = { kind = "builtin", name = … }` spelling of the same row.
pub(crate) const BUILTIN_VIBE_OPT_LAUNCHER_NAME: &str = "vibe-opt-launcher";

/// The reserved identity of §6.3.0.5's Claude standalone-skill deploy row.
///
/// The three skill rows share ONE closed provider parameterised by
/// [`SkillClient`](deploy::skill::SkillClient); their ids and names are
/// the registry's own spellings, restated here so the dispatch and the
/// descriptor answer under one constant each.
pub(crate) const BUILTIN_CLAUDE_SKILL_PIN: &str = "org.vibevm/vibe#claude-skill";

/// The `handler = { kind = "builtin", name = … }` spelling of the same row.
pub(crate) const BUILTIN_CLAUDE_SKILL_NAME: &str = "claude-skill";

/// The reserved identity of §6.3.0.5's Codex standalone-skill deploy row.
pub(crate) const BUILTIN_CODEX_SKILL_PIN: &str = "org.vibevm/vibe#codex-skill";

/// The `handler = { kind = "builtin", name = … }` spelling of the same row.
pub(crate) const BUILTIN_CODEX_SKILL_NAME: &str = "codex-skill";

/// The reserved identity of §6.3.0.5's OpenCode standalone-skill deploy
/// row.
pub(crate) const BUILTIN_OPENCODE_SKILL_PIN: &str = "org.vibevm/vibe#opencode-skill";

/// The `handler = { kind = "builtin", name = … }` spelling of the same row.
pub(crate) const BUILTIN_OPENCODE_SKILL_NAME: &str = "opencode-skill";

/// The reserved identities and handler names of §6.3's client-plugin
/// destination providers.
pub(crate) const BUILTIN_CLAUDE_PLUGIN_PIN: &str = "org.vibevm/vibe#claude-plugin";
pub(crate) const BUILTIN_CLAUDE_PLUGIN_NAME: &str = "claude-plugin";
pub(crate) const BUILTIN_CODEX_PLUGIN_PIN: &str = "org.vibevm/vibe#codex-plugin";
pub(crate) const BUILTIN_CODEX_PLUGIN_NAME: &str = "codex-plugin";
pub(crate) const BUILTIN_OPENCODE_PLUGIN_PIN: &str = "org.vibevm/vibe#opencode-plugin";
pub(crate) const BUILTIN_OPENCODE_PLUGIN_NAME: &str = "opencode-plugin";
