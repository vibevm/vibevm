//! The install pass's data surface — the resolved-dep record, the
//! `verify` spot-check seam, and the outcome report. Split from
//! `install.rs` at the PROP-011 §2.3 spot-check landing (the file crossed
//! the 600-line budget); the orchestration stays in the parent module.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-011#materialise-diff");

use std::path::{Path, PathBuf};

use vibe_core::manifest::{Manifest, SpecFormat};
use vibe_core::{Group, PackageKind};

use crate::hooks::HookReport;

/// A resolved, fetched dependency ready to materialise — the minimum the
/// install orchestrator needs, decoupled from the registry's richer
/// `CachedPackage`.
#[derive(Debug, Clone)]
pub struct ResolvedDep {
    /// The package's `kind` — metadata; used only for its `vibedeps/` slot
    /// directory name, never for identity (PROP-008 §2.3).
    pub kind: PackageKind,
    /// Reverse-FQDN group — with `name`, the `(group, name)` identity.
    pub group: Group,
    pub name: String,
    pub version: semver::Version,
    /// On-disk directory holding the package's fetched content tree — the
    /// source `vibedeps` materialisation copies verbatim.
    pub content_dir: PathBuf,
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
    /// package-authoring shape). Such a source is a working tree the author
    /// edits in place, so its `vibedeps/` slot is **never** presence-trusted by
    /// the PROP-011 §2.3 fast path: it is re-materialised every install
    /// (PROP-011 §2.6). `false` for immutable remote-registry sources and for
    /// boot-only re-derivations from disk. `in-place` (PROP-022) packages take
    /// the separate in-place branch and never reach the skip this flag guards.
    pub source_mutable: bool,
}

/// The verdict of a `slot_integrity = verify` spot-check on a present
/// slot (PROP-011 §2.3/§5.2) — produced by the caller-supplied
/// [`SlotVerifier`] seam, consumed by the materialise pass below.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotCheck {
    /// The slot's tree hashes to the recorded `content_hash` — the fast
    /// path may accept it without copying.
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
/// materialise pass calls it for a present, immutable slot **only** under
/// [`SlotIntegrity::Verify`], handing the resolved dep and the slot's
/// absolute path; the implementation hashes the slot and compares against
/// the hash the resolution records.
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
/// // `apply_resolution_with` performs for every present immutable slot.
/// fn consult(v: &dyn SlotVerifier) -> bool {
///     let _ = v; // hash + compare happens against a real slot in production
///     true
/// }
/// assert!(consult(&AlwaysDiverged));
/// ```
pub trait SlotVerifier {
    /// The fetched source tree's recipe-labelled content hash, used as the
    /// immutable input identity in a transformed slot manifest.
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

/// What [`apply_resolution`] did — for the caller to report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallOutcome {
    /// `vibedeps/` slot paths freshly materialised this run — a new or
    /// version-bumped dependency whose content was copied, or a present
    /// slot whose `verify` spot-check diverged and was overwritten.
    pub materialised: Vec<String>,
    /// `vibedeps/` slot paths skipped — already present for the resolved
    /// version, trusted and not re-copied (PROP-011 §2.3). Under
    /// `trust-presence` (the default) that trust is presence alone; under
    /// `verify` a slot lands here only after its `content_hash` checked
    /// out — and never when no [`SlotVerifier`] was supplied, which keeps
    /// the always-re-copy discipline.
    pub skipped: Vec<String>,
    /// One warn line per `verify`-mode slot whose hash diverged from the
    /// recorded one (naming the package and both hashes) — the slot was
    /// re-materialised; these lines are the record of why. Empty under
    /// `trust-presence` and on a clean `verify` pass.
    pub integrity_warnings: Vec<String>,
    /// `vibedeps/` slot paths pruned — present before, absent from this
    /// resolution (a version bump, or a dropped dependency).
    pub pruned: Vec<String>,
    /// `rel_path` of every node whose boot artifacts were regenerated.
    pub nodes_regenerated: Vec<String>,
    /// Structured reports from the `pre-install` hooks that ran this install
    /// (PROP-020 §2.1) — one per freshly-materialised package that declares a
    /// `pre-install` script. Empty when no package declares hooks or hook
    /// running was not requested (`hooks = None`). Each report is `ran` /
    /// `skipped-needs-consent`; the CLI renders them so a skipped hook is
    /// surfaced, never silent.
    pub hook_reports: Vec<HookReport>,
}
