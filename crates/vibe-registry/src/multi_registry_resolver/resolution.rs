//! The resolver's resolution types — `MultiResolution` (a resolved
//! package with provenance) and `ResolvedPathDep` (a path-source
//! declaration with the on-disk location already computed). Split from
//! `mod.rs` along the types seam when the store-backed resolution work
//! outgrew the combined file; `mod.rs` re-exports both, so the
//! historical `multi_registry_resolver::MultiResolution` paths stay
//! valid.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");

use vibe_core::VersionSpec;
use vibe_core::{Group, PackageKind};

use crate::ResolvedPackage;

/// A resolved package with provenance — which registry served it, the
/// URL / ref recorded in the lockfile, and whether the resolution
/// short-circuited via an override.
#[derive(Debug, Clone)]
pub struct MultiResolution {
    pub resolved: ResolvedPackage,
    /// Name of the `[[registry]]` that served this package. `None` for
    /// override-resolved and git-source entries.
    pub registry_name: Option<String>,
    /// What goes into lockfile `source_url`.
    pub source_url: String,
    /// What goes into lockfile `source_ref` — typically the version tag
    /// (`v0.3.0`) for registry resolutions, or the override's / git-source
    /// `tag`/`branch`/`rev` value.
    pub source_ref: Option<String>,
    pub overridden: bool,
    /// True when this package was resolved via a `[requires.packages]`
    /// git-source declaration (PROP-002 §2.4.1) rather than through
    /// the registry walk or `[[override]]`. Lockfile maps this to
    /// `source_kind = "git"`.
    pub is_git_source: bool,
    /// True when this package was resolved via a `[requires.packages]`
    /// path-source declaration (PROP-007 §2.5) — a package in a local
    /// directory, typically a sibling workspace member — rather than the
    /// registry walk, `[[override]]`, or git-source. Lockfile maps this
    /// to `source_kind = "path"`, and `source_url` then carries the
    /// member's path relative to the workspace root, not a URL.
    pub is_path_source: bool,
    /// When this package was resolved via a registry stub that
    /// redirected to an external URL (PROP-002 §2.4.2), the **stub**
    /// URL is recorded here while `source_url` carries the **target**
    /// URL. `None` for non-redirected resolutions.
    pub via_redirect: Option<String>,
    /// True when this resolution came from the **machine store**
    /// (PROP-010 §2.6) — either the offline posture (versions and
    /// manifests read from the store, no network at all) or the
    /// availability fallback (a cache hit outranks a registry that no
    /// longer lists the version). `resolved.source_dir` is then the
    /// store entry, and the fetch path short-circuits to serve the
    /// entry's bytes instead of walking sources.
    pub from_store: bool,
    /// Auth regime declared in the redirect's `[redirect].auth`. Only
    /// meaningful when `via_redirect.is_some()`; for non-redirected
    /// resolutions the registry's own auth applies via `registry_name`
    /// → registry lookup. The fetch path uses this to synthesise a
    /// target-side `GitPerPackageRegistry` with the right auth without
    /// re-fetching the redirect marker.
    pub redirect_target_auth: vibe_core::manifest::AuthKind,
    /// Env-var name when `redirect_target_auth = TokenEnv`. `None`
    /// otherwise.
    pub redirect_target_token_env: Option<String>,
}

/// A `[requires.packages]` path-source declaration (PROP-007 §2.5) with
/// the on-disk location already computed by the caller. A path-source
/// dependency is a package living in a local directory — typically a
/// sibling workspace member — so there is no registry walk and no git
/// clone: the source is a directory the resolver reads and copies.
///
/// The resolver does **no** filesystem path arithmetic. The caller (the
/// workspace layer, a later milestone) resolves `PathPackageDep.path`
/// against the declaring manifest's directory, canonicalises it, and
/// hands the absolute `package_dir` plus the workspace-relative
/// `workspace_rel` in already-computed. The resolver just consumes them.
#[derive(Debug, Clone)]
pub struct ResolvedPathDep {
    /// Optional `kind` prefix carried by the pkgref key (PROP-008 §2.4).
    /// Metadata only — never used to resolve; `(group, name)` is identity.
    pub kind: Option<PackageKind>,
    /// Reverse-FQDN group — a manifest pkgref is always qualified.
    pub group: Group,
    pub name: String,
    /// Optional dual-form version constraint from `{ path, version }`.
    /// When present, the package's own `[package].version` must satisfy
    /// it; mismatch is a hard error — same shape as the git-source
    /// version check.
    pub version: Option<VersionSpec>,
    /// Absolute directory where the dependency package lives. The caller
    /// resolves `PathPackageDep.path` against the declaring manifest's
    /// directory and canonicalises it; the resolver just consumes it.
    pub package_dir: std::path::PathBuf,
    /// `package_dir` relative to the workspace absolute root,
    /// forward-slashed. Recorded verbatim as the lockfile `source_url`
    /// for this entry — a portable relative path, never a URL, never
    /// absolute.
    pub workspace_rel: String,
}
