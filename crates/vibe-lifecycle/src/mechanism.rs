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

use vibe_core::manifest::{ArtifactBuildTarget, ArtifactKind};

pub(crate) mod build;
pub(crate) mod cargo;
pub(crate) mod error;
pub(crate) mod record;

pub use build::{
    BuildError, BuildExecution, BuildOutcome, ProducedArtifact, execute_build_targets,
};
pub use error::MechanismError;
pub use record::ARTIFACT_RECORD_DIR;

/// The engine-owned build output root, project-relative.
///
/// §3.2 is explicit that a provider "cannot mint an unscoped output path":
/// the engine chooses where a build writes, passes it as `--target-dir`,
/// and therefore always knows the produced artifact's project-relative
/// identity. Cargo still owns everything *inside* that directory — its
/// incremental machinery is untouched.
pub(crate) const DEFAULT_BUILD_ROOT: &str = "target";

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

/// The reserved identity of the one builtin build provider — §3's
/// `org.vibevm/vibe#cargo`, "not a privileged branch outside the
/// registry".
pub(crate) const BUILTIN_CARGO_PIN: &str = "org.vibevm/vibe#cargo";

/// The `handler = { kind = "builtin", name = … }` spelling of the same row.
pub(crate) const BUILTIN_CARGO_NAME: &str = "cargo";
