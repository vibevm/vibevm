//! The install pass's data surface — the resolved-dep record, the
//! `verify` spot-check seam, and the outcome report. Split from
//! `install.rs` at the PROP-011 §2.3 spot-check landing (the file crossed
//! the 600-line budget); the orchestration stays in the parent module.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-011#materialise-diff");

use std::path::{Path, PathBuf};

use vibe_core::manifest::{Manifest, SpecFormat};
use vibe_core::{ContentHash, Group, PackageKind};

use crate::hooks::HookReport;

/// A resolved, fetched dependency ready to materialise — the minimum the
/// install orchestrator needs, decoupled from the registry's richer
/// `CachedPackage`.
#[derive(Debug, Clone)]
pub struct ResolvedDep {
    /// The package's `kind` — metadata; used only for its dependency slot
    /// directory name, never for identity (PROP-008 §2.3).
    pub kind: PackageKind,
    /// Reverse-FQDN group — with `name`, the `(group, name)` identity.
    pub group: Group,
    pub name: String,
    pub version: semver::Version,
    /// On-disk directory holding the package's fetched content tree — the
    /// source `vibedeps` materialisation copies verbatim.
    pub content_dir: PathBuf,
    /// Fetched shippable-tree identity. Boot-only reconstruction may carry
    /// `None`; every path that materialises a slot requires `Some`.
    pub source_hash: Option<ContentHash>,
    /// The package's parsed manifest (its `vibe.toml`) — read for the
    /// `[boot_snippet]` contribution.
    pub manifest: Manifest,
    /// `(group, name)` of every package this one directly requires — the
    /// edges of the dependency-boot topological order.
    pub requires: Vec<(Group, String)>,
    /// Visibility rule that admitted this package into the consumer's
    /// effective set: `root-edge`, `public-chain`, or `friends-chain`.
    pub admitted_by: Option<String>,
    /// Coordinate of the node whose path-scoped override admitted the
    /// decisive edge, when an override changed its access.
    pub via_override: Option<String>,
    /// `true` iff the package came from a mutable local `file://` source — an
    /// in-repo / local-directory registry (`--registry <path>`, the
    /// package-authoring shape). Version presence alone cannot vouch for such a
    /// working tree. The PROP-011 §2.6 fast path instead requires a valid slot
    /// record whose source hash matches this resolution's freshly-fetched
    /// [`source_hash`](Self::source_hash); unchanged sources can then skip,
    /// while changed ones reconcile only their payload diff. `false` for
    /// immutable remote-registry sources and boot-only disk re-derivations.
    /// `in-place` (PROP-022) packages take their separate update branch.
    pub source_mutable: bool,
    /// Exact change evidence for an already-placed `in-place` slot. `Some`
    /// comes only from the registry's post-confirmation git refresh; `None`
    /// is every copy/hardlink dependency and every fresh clone still awaiting
    /// placement.
    pub in_place_changed: Option<bool>,
}

/// The verdict of a `slot_integrity = verify` spot-check on a present
/// slot (PROP-011 §2.3/§5.2) — produced by the caller-supplied
/// [`SlotVerifier`] seam, consumed by the materialise pass below.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotCheck {
    /// The slot's record and payload verify, or its legacy tree hashes to the
    /// recorded `content_hash`; the fast path may accept it without copying.
    Verified,
    /// The slot's tree diverges from the recorded `content_hash` —
    /// re-materialise it and warn, naming the package and both hashes.
    Diverged {
        /// The hash the resolution records for this package (the lockfile
        /// pin) — the wire form, `sha256:…` / `sha256-tree/1:…`.
        expected: String,
        /// The hash the present slot actually computed to.
        actual: String,
    },
    /// A transformed slot's typed identity record or derived tree is stale.
    /// The reason names the exact field/check that failed.
    DivergedDetail { reason: String },
    /// The check could not run — no recorded hash to compare against, or
    /// the slot could not be hashed. Falls back to re-materialising, the
    /// pre-spot-check `verify` discipline, with no warn.
    Unverifiable,
}

/// The `verify`-mode slot spot-check seam (PROP-011 §2.3/§5.2). The
/// materialise pass calls it for an identity-current present slot **only**
/// under [`SlotIntegrity::Verify`], handing the resolved dep and the slot's
/// absolute path. For mutable sources, identity-current means the valid record
/// already matches the freshly-fetched source hash. New slots verify their
/// typed record and recorded payload;
/// legacy slots retain their historical tree-hash checks.
///
/// A seam, not a call, because this crate deliberately depends on neither
/// hash crate: `compute_content_hash` lives in `vibe-registry` (and its
/// parity-locked port in `vibe-index`), and `vibe-install` — which does
/// depend on `vibe-registry` — supplies the implementation. Callers that
/// pass no verifier keep the shipped `verify` behaviour (re-materialise
/// every slot), which is exactly what `vibe reinstall --force` and
/// `vibe update` ask for.
/// ```
/// use std::path::Path;
/// use vibe_workspace::install::{ResolvedDep, SlotCheck, SlotVerifier};
///
/// /// The canonical shape: hash the slot, compare to the recorded pin.
/// /// This stub stands in for vibe-install's registry-backed verifier.
/// struct AlwaysDiverged;
/// impl SlotVerifier for AlwaysDiverged {
///     fn verify_slot(&self, _dep: &ResolvedDep, _slot_abs: &Path) -> SlotCheck {
///         SlotCheck::Diverged {
///             expected: "sha256:aa".into(),
///             actual: "sha256:bb".into(),
///         }
///     }
/// }
///
/// // The materialise pass consumes the seam through &dyn — the wiring
/// // `apply_resolution_with` performs for every eligible present slot.
/// fn consult(v: &dyn SlotVerifier) -> bool {
///     let _ = v; // hash + compare happens against a real slot in production
///     true
/// }
/// assert!(consult(&AlwaysDiverged));
/// ```
pub trait SlotVerifier {
    /// The verifier's fetched expected source identity. Materialisation reads
    /// [`ResolvedDep::source_hash`] directly; this lookup exists only inside
    /// verifier implementations that retain a fetched-set index.
    fn source_hash<'a>(&'a self, _dep: &ResolvedDep) -> Option<&'a str> {
        None
    }

    /// Hash `slot_abs` and compare it against the hash recorded for `dep`.
    fn verify_slot(&self, dep: &ResolvedDep, slot_abs: &Path) -> SlotCheck;

    /// Verify the slot under its effective representation. Implementations
    /// that know only legacy mixed slots retain the old hash check by default.
    fn verify_slot_for_format(
        &self,
        dep: &ResolvedDep,
        slot_abs: &Path,
        _spec_format: SpecFormat,
    ) -> SlotCheck {
        self.verify_slot(dep, slot_abs)
    }
}

/// One-shot authority to run exactly the post-install hooks selected by one
/// materialisation pass.
///
/// The plan binds the workspace root to owned clones of the eligible resolved
/// dependencies. It has no public constructor or fields and is consumed by
/// [`super::run_post_install_hooks`], so a caller cannot combine slot labels
/// from one install with a different resolution or workspace.
#[derive(Debug)]
pub struct PostInstallPlan {
    workspace_root: PathBuf,
    eligible_deps: Vec<ResolvedDep>,
}

impl PostInstallPlan {
    pub(super) fn new(workspace_root: &Path, eligible_deps: Vec<ResolvedDep>) -> Option<Self> {
        (!eligible_deps.is_empty()).then(|| Self {
            workspace_root: workspace_root.to_path_buf(),
            eligible_deps,
        })
    }

    pub(super) fn into_parts(self) -> (PathBuf, Vec<ResolvedDep>) {
        (self.workspace_root, self.eligible_deps)
    }
}

/// What [`apply_resolution`] did — for the caller to report.
#[derive(Debug)]
pub struct InstallOutcome {
    /// Dependency-slot paths materialised this run — a new or version-bumped
    /// dependency, or a present slot whose recorded footprint was reconciled.
    pub materialised: Vec<String>,
    /// Dependency-slot paths skipped — already present for the resolved
    /// version and source identity, trusted without materialising (PROP-011
    /// §2.3). Under `trust-presence` (the default), immutable sources need
    /// current representation while mutable sources also need a matching
    /// source hash in their valid record. Under `verify`, either kind lands
    /// here only after its payload checked out — and never when no
    /// [`SlotVerifier`] was supplied, preserving the rematerialise discipline.
    pub skipped: Vec<String>,
    /// One warn line per `verify`-mode slot whose hash diverged from the
    /// recorded one (naming the package and both hashes) — the slot was
    /// re-materialised; these lines are the record of why. Empty under
    /// `trust-presence` and on a clean `verify` pass.
    pub integrity_warnings: Vec<String>,
    /// Dependency-slot paths pruned — present before, absent from this
    /// resolution (a version bump, or a dropped dependency).
    pub pruned: Vec<String>,
    /// `rel_path` of every node whose boot artifacts were regenerated.
    pub nodes_regenerated: Vec<String>,
    /// One-shot post-install authority, deliberately excluded from the public
    /// reporting surface.
    pub(super) post_install_plan: Option<PostInstallPlan>,
    /// Structured reports from the `pre-install` hooks that ran this install
    /// (PROP-020 §2.1) — one per payload-changing package that declares a
    /// `pre-install` script. Empty when no package declares hooks or hook
    /// running was not requested (`hooks = None`). Each report is `ran` /
    /// `skipped-needs-consent`; the CLI renders them so a skipped hook is
    /// surfaced, never silent.
    pub hook_reports: Vec<HookReport>,
}

impl InstallOutcome {
    /// Take the post-install plan produced by this install, at most once.
    ///
    /// `None` means either no dependency changed eligible payload or the plan
    /// was already taken. The returned value is consumed by
    /// [`super::run_post_install_hooks`].
    pub fn take_post_install_plan(&mut self) -> Option<PostInstallPlan> {
        self.post_install_plan.take()
    }
}
