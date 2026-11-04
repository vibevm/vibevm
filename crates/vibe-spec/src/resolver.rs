//! The file resolver — `spec://` authority + doc-path → a file on disk
//! (PROP-035 §6, the `doc_path → file` half).
//!
//! It resolves against the **materialised** tree, as the specmap engine does:
//! the host project's authored `spec/`, and each package's `vibedeps/` slot —
//! never `packages/` (the authoring source). The lossy `PROP-NNN` / `FEAT-NNN`
//! truncation is inverted by a directory prefix-scan (the id number is unique
//! within a directory, an invariant), so `…/PROP-042` finds
//! `PROP-042-example-thing.md`.
//!
//! Version / slot selection is deliberately thin here: an explicit `@version`
//! picks the slot version, and an absent one resolves to the **freshest**
//! installed version (semver-newest; the owner's optional-version rule,
//! B-028 2026-08-04) — no pin is required when several are installed. A
//! lockfile-backed selection (kind + version from `vibe.lock`) is the layer
//! above; this resolver only needs the workspace root and the project's self
//! coordinate.
//!
//! **B-031 — the host is a package coordinate.** The root project is addressed
//! as `spec://<group>/<name>/…` (its *self coordinate*), not by a reserved
//! host token. A `SelfCoordinate` whose `group` is `None` names a project that
//! declares no self coordinate — its authored `spec/` is then unreachable by
//! address. An undotted `spec://<host>/…` authority (the legacy form) parses
//! but never resolves: it errors with [`ResolveError::LegacyHostAuthority`].

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::address::{Authority, SpecAddress};

/// The root project's own `<group>/<name>` coordinate — what a
/// `spec://<group>/<name>/…` address must name to resolve against the
/// project's authored `spec/` tree (B-031: the host is a package coordinate,
/// not a reserved host token).
///
/// `group` is optional: a project with no `group` declares no self coordinate,
/// so its authored tree is unreachable by `spec://` address. Both halves are
/// plain strings — the group is validated at the manifest layer (PROP-008);
/// the resolver only matches them against the parsed address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfCoordinate {
    /// The reverse-DNS group half (e.g. `org.vibevm.core`).
    pub group: Option<String>,
    /// The name half (e.g. `vibevm`).
    pub name: String,
}

impl SelfCoordinate {
    /// Build a self coordinate from its halves.
    pub fn new(group: Option<String>, name: String) -> Self {
        Self { group, name }
    }
}

/// Resolves `spec://` addresses to files under a workspace root.
#[derive(Debug, Clone)]
pub struct FileResolver {
    /// Where the SELF coordinate's authored specs live. Equal to `ws_root` for
    /// the ordinary workspace resolver; a caller that already knows the exact
    /// root of the package it is resolving inside (an executing extension
    /// provider, a workspace member node) pins it here instead, so no slot
    /// search — and therefore no "freshest installed version" guess — can
    /// substitute a different instance of the same coordinate.
    self_root: PathBuf,
    ws_root: PathBuf,
    self_coord: SelfCoordinate,
    /// When present, the ONLY coordinates that resolve, each pinned to an
    /// exact root. A caller that already knows which instance of every package
    /// its world selected hands that map here, and the slot scan below — which
    /// answers with the semver-freshest *installed* version — is never reached.
    /// A coordinate absent from the map refuses rather than falling back.
    selected: Option<BTreeMap<(String, String), SelectedPackage>>,
}

/// One row of an already-selected world: the exact version a lock chose and
/// the root it was materialised at.
///
/// Both halves are load-bearing. The root is what removes the slot scan; the
/// version is what lets an explicit `@version` in an address be *checked*
/// rather than dropped. Silently discarding a pin is the failure this type
/// exists to make impossible: an author who wrote `@2.0.0` and got 1.0.0's
/// bytes has no way to notice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedPackage {
    pub version: String,
    pub root: PathBuf,
}

impl SelectedPackage {
    pub fn new(version: impl Into<String>, root: impl Into<PathBuf>) -> Self {
        Self {
            version: version.into(),
            root: root.into(),
        }
    }
}

/// Why an address does not resolve to a file.
#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    /// A source locator or held observation could not preserve its boundary.
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#URI-OBSERVED-SOURCE-READ: source `{path}` unavailable ({reason}); fix surface: restore a regular source below the selected root and resolve it again"
    )]
    SourceObservation { path: String, reason: String },
    /// A filesystem-bound spelling violates the shared declarant path law.
    #[error(
        "violates REQ spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#URI-LOOKUP-PATHS: invalid {field} `{value}` ({}); fix surface: use a portable relative document path or single slot/version/suffix component",
        .reason.reason()
    )]
    InvalidLookupPath {
        field: &'static str,
        value: String,
        reason: vibe_core::manifest::DeclarantPathFault,
    },
    /// A legacy undotted `spec://<host>/…` authority (B-031). Such an authority
    /// is grammar-legal but never resolves — the host is a package coordinate.
    /// `hint` names the resolver's actual self coordinate (or notes its
    /// absence) so a caller can rewrite the address; `given` is the token as
    /// written.
    #[error("{hint}")]
    LegacyHostAuthority { given: String, hint: String },
    /// A self-coordinate address carrying an `@version` — the self coordinate
    /// is unversioned, so a version pin on it is an error, never silently
    /// dropped.
    #[error(
        "the self coordinate `spec://{self_group}/{self_name}` is unversioned; \
         drop the `@{version}` from the address"
    )]
    SelfCoordinateVersioned {
        self_group: String,
        self_name: String,
        version: String,
    },
    #[error("no installed vibedeps slot for package `{0}`")]
    PackageSlotNotFound(String),
    /// The resolver was given an exact selected world and the address names a
    /// coordinate that world does not contain. Refusing is the point: falling
    /// back to a scan would answer with an instance the lock never chose.
    #[error(
        "package `{given}` is not in this resolver's selected world; \
         only coordinates the lock selected can be reached"
    )]
    UnselectedPackage { given: String },
    /// The address pinned a version the selected world did not choose.
    /// Refusing names both numbers, because the interesting question is
    /// always "which one did I get" and silence answers it wrongly.
    #[error(
        "address pins `{given}@{requested}`, but the selected world holds \
         `{given}@{selected}`; the pin and the lock disagree"
    )]
    SelectedVersionMismatch {
        given: String,
        requested: String,
        selected: String,
    },
    #[error("document `{doc_path}` not found under `{base}`")]
    DocNotFound { doc_path: String, base: String },
    #[error("document id `{id}` is ambiguous ({count} files match) under `{dir}`")]
    AmbiguousDoc {
        id: String,
        count: usize,
        dir: String,
    },
    /// A `spec://` address whose package name carries a glob `*` — a pattern,
    /// not a coordinate (B-056). Point resolution cannot name a single file
    /// from a pattern: it would fall through to the name-suffix lookup and
    /// report [`PackageSlotNotFound`](Self::PackageSlotNotFound), mis-stating
    /// "not installed" where the truth is "this address names a set; expand it
    /// first with [`FileResolver::expand_pattern`]." `given` is the address as
    /// written.
    #[error(
        "address `{given}` is a pattern (its package name has `*`); expand it with `expand_pattern` first"
    )]
    PatternNotExpanded { given: String },
    /// One logical document found in both serialisations — `X.md` and `X.xml`
    /// beside each other. The mixed tree holds each document in ONE form
    /// (PROP-045 ##TARGET-MIXED); a pair is a split brain, reported with both
    /// paths, never resolved by guessing which half to read.
    #[error(
        "`{}` and `{}` are one logical document in two forms — one document, one \
         form; delete one of the pair or rename one",
        .markdown.display(),
        .xml.display()
    )]
    PairCollision { markdown: PathBuf, xml: PathBuf },
    /// An active custom frontend made more than one physical form eligible for
    /// the same logical document.
    #[error("document `{doc_path}` has multiple active physical forms: {files}")]
    ActiveFormatCollision { doc_path: String, files: String },
}

impl FileResolver {
    /// A resolver rooted at `ws_root`, treating `self_coord` as the project's
    /// own `<group>/<name>` authority — the coordinate a `spec://` address
    /// must name to reach the authored `spec/` tree (B-031).
    pub fn new(ws_root: impl Into<PathBuf>, self_coord: SelfCoordinate) -> Self {
        let ws_root = ws_root.into();
        Self {
            self_root: ws_root.clone(),
            ws_root,
            self_coord,
            selected: None,
        }
    }

    /// A resolver with separate self and dependency roots.
    ///
    /// `self_root` is where `self_coord`'s own authored specs live; `ws_root`
    /// remains the selected workspace world every OTHER coordinate resolves
    /// against. Pinning the two apart is what lets a caller resolve a document
    /// inside one exact instance of a package — the version and directory that
    /// is actually executing — while cross-package references still go through
    /// the ordinary installed world.
    ///
    /// ```
    /// use vibe_spec::{FileResolver, SelfCoordinate};
    /// let resolver = FileResolver::with_roots(
    ///     "/abs/vibedeps/org.demo.tools/1.0.0",
    ///     "/abs/workspace",
    ///     SelfCoordinate::new(Some("org.demo".into()), "tools".into()),
    /// );
    /// assert_eq!(resolver.self_root(), std::path::Path::new("/abs/vibedeps/org.demo.tools/1.0.0"));
    /// ```
    pub fn with_roots(
        self_root: impl Into<PathBuf>,
        ws_root: impl Into<PathBuf>,
        self_coord: SelfCoordinate,
    ) -> Self {
        Self {
            self_root: self_root.into(),
            ws_root: ws_root.into(),
            self_coord,
            selected: None,
        }
    }

    /// A resolver whose dependency arm is an exact, already-selected world.
    ///
    /// `selected` maps `(group, name)` to the materialised root the caller's
    /// lock chose. Every non-self coordinate resolves through it and nothing
    /// else: an unselected coordinate answers
    /// [`ResolveError::UnselectedPackage`], and no `vibedeps` directory is
    /// scanned, so "freshest installed" cannot substitute an instance the lock
    /// did not choose.
    ///
    /// An explicit `@version` on such an address is **checked, never
    /// dropped**: it must equal the selected version or the address refuses
    /// with both numbers named.
    ///
    /// ```
    /// use std::collections::BTreeMap;
    /// use vibe_spec::{FileResolver, SelectedPackage, SelfCoordinate, SpecAddress};
    /// let mut selected = BTreeMap::new();
    /// selected.insert(
    ///     ("org.demo".to_string(), "b".to_string()),
    ///     SelectedPackage::new("1.0.0", "/abs/b-1.0"),
    /// );
    /// let resolver = FileResolver::with_selected_world(
    ///     "/abs/a-1.0",
    ///     "/abs/workspace",
    ///     selected,
    ///     SelfCoordinate::new(Some("org.demo".into()), "a".into()),
    /// );
    /// let unselected = SpecAddress::parse("spec://org.demo/c/doc").unwrap();
    /// assert!(resolver.resolve_file(&unselected).is_err());
    /// let wrong_pin = SpecAddress::parse("spec://org.demo/b@2.0.0/doc").unwrap();
    /// assert!(resolver.resolve_file(&wrong_pin).is_err());
    /// ```
    pub fn with_selected_world(
        self_root: impl Into<PathBuf>,
        ws_root: impl Into<PathBuf>,
        selected: BTreeMap<(String, String), SelectedPackage>,
        self_coord: SelfCoordinate,
    ) -> Self {
        Self {
            self_root: self_root.into(),
            ws_root: ws_root.into(),
            self_coord,
            selected: Some(selected),
        }
    }

    /// The root this resolver's self coordinate resolves against.
    #[must_use]
    pub fn self_root(&self) -> &Path {
        &self.self_root
    }

    /// Resolve an address to the file that holds its document. Ignores the
    /// anchor / revision — those address a node *within* the returned file
    /// (see [`DocTree`](crate::DocTree)).
    pub fn resolve_file(&self, addr: &SpecAddress) -> Result<PathBuf, ResolveError> {
        self.resolve_source_file(addr, &[])
            .map(|resolved| resolved.path)
    }

    pub(crate) fn resolve_source_file(
        &self,
        addr: &SpecAddress,
        active_formats: &[String],
    ) -> Result<ResolvedSourceFile, ResolveError> {
        self.locate_source(addr, active_formats)
            .map(|source| source.resolved)
    }

    /// Whether a parsed `<group>/<name>` is this resolver's own coordinate. A
    /// groupless self coordinate matches nothing — it has no package form.
    fn is_self(&self, group: &str, name: &str) -> bool {
        match &self.self_coord.group {
            Some(g) => g == group && self.self_coord.name == name,
            None => false,
        }
    }

    /// The rewrite hint for a legacy undotted authority: name the resolver's
    /// actual self coordinate, or — when the project declares none — say so.
    fn legacy_host_hint(&self, given: &str) -> String {
        match &self.self_coord.group {
            Some(group) => format!(
                "`spec://{given}/…` no longer resolves: the host is a package at \
                 `spec://{group}/{name}/…` since B-031 — rewrite the address",
                name = self.self_coord.name
            ),
            None => "this workspace declares no self coordinate; \
                     undotted authorities never resolve"
                .to_string(),
        }
    }
}

/// The document-lookup half: doc-path → file, the layout roots it resolves
/// against, and the canonical doc-path both physical layouts fold onto. Split
/// out of this file along its own responsibility seam — everything above
/// answers "which `spec/` root does this authority mean", everything there
/// answers "which file inside a root does this doc-path mean".
mod lookup;
mod observation;
mod paths;
pub use lookup::canonical_doc_path;
#[cfg(test)]
pub(crate) use lookup::{specs_root_under, vibedeps_root_under};
pub(crate) use observation::SourceLocation;

#[derive(Debug)]
pub(crate) struct ResolvedSourceFile {
    pub(crate) path: PathBuf,
    pub(crate) format: String,
    pub(crate) physical_stem: String,
}
// The layout roots stay reachable under their historical path for the tests
// that assert both physical layouts fold onto one address.
#[cfg(test)]
pub(crate) use lookup::{
    LEGACY_SPECS_ROOT, LEGACY_VIBEDEPS_ROOT, NEW_SPECS_ROOT, NEW_VIBEDEPS_ROOT,
};
#[cfg(test)]
use lookup::{id_file_matches, is_id_stem};

/// Version ordering for the freshest-installed rule (B-028): `newest` selects
/// the semver-newest installed version for an unpinned address.
mod version_order;

/// Pattern expansion (B-056): a `*` in a package name enumerates the installed
/// set. The `is_pattern` predicate is re-exported at the crate root for the
/// pipeline (its next consumer).
mod glob;
pub use glob::is_pattern;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod path_tests;
