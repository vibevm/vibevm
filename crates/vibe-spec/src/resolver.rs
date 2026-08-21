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

use std::fs;
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
    ws_root: PathBuf,
    self_coord: SelfCoordinate,
}

/// Why an address does not resolve to a file.
#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
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
}

impl FileResolver {
    /// A resolver rooted at `ws_root`, treating `self_coord` as the project's
    /// own `<group>/<name>` authority — the coordinate a `spec://` address
    /// must name to reach the authored `spec/` tree (B-031).
    pub fn new(ws_root: impl Into<PathBuf>, self_coord: SelfCoordinate) -> Self {
        Self {
            ws_root: ws_root.into(),
            self_coord,
        }
    }

    /// Resolve an address to the file that holds its document. Ignores the
    /// anchor / revision — those address a node *within* the returned file
    /// (see [`DocTree`](crate::DocTree)).
    pub fn resolve_file(&self, addr: &SpecAddress) -> Result<PathBuf, ResolveError> {
        // A pattern (a `*` in the package name) names a set, not a file. Refuse
        // it loudly rather than falling through to the suffix lookup, which would
        // report PackageSlotNotFound — "not installed" where the truth is "this
        // address must be expanded first" (B-056, law 7).
        if glob::is_pattern(addr) {
            return Err(ResolveError::PatternNotExpanded {
                given: addr.raw.clone(),
            });
        }
        let base_spec = self.spec_root(&addr.authority)?;
        resolve_doc(&base_spec, &addr.doc_path)
    }

    /// The `spec/` root an authority resolves against.
    ///
    /// The self-coordinate match is the **first** arm (B-031): a
    /// `spec://<self_group>/<self_name>/…` address lands in the authored
    /// `spec/` tree before any `vibedeps/` slot is considered. Any other
    /// package coordinate falls through to slot lookup; an undotted host
    /// authority never resolves.
    fn spec_root(&self, authority: &Authority) -> Result<PathBuf, ResolveError> {
        match authority {
            Authority::Package {
                group,
                name,
                version: None,
            } if self.is_self(group, name) => Ok(self.ws_root.join("spec")),
            Authority::Package {
                group,
                name,
                version: Some(v),
            } if self.is_self(group, name) => Err(ResolveError::SelfCoordinateVersioned {
                self_group: group.clone(),
                self_name: name.clone(),
                version: v.clone(),
            }),
            Authority::Package {
                group,
                name,
                version,
            } => Ok(self
                .package_slot(group, name, version.as_deref())?
                .join("spec")),
            Authority::Host(h) => Err(ResolveError::LegacyHostAuthority {
                given: h.clone(),
                hint: self.legacy_host_hint(h),
            }),
        }
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

    /// Find a package's materialised slot: `vibedeps/<group>.<name>/<version>`
    /// (PROP-022 §2.1 — the slot is keyed by identity, so the address's own
    /// `<group>/<name>` names the directory exactly; no suffix scan). An
    /// explicit `@version` names the slot version; an absent one resolves to
    /// the **freshest installed** version (semver-newest; the owner's
    /// optional-version rule, B-028 2026-08-04). A directory whose name does
    /// not look like a version (it does not start with a digit) is ignored as
    /// a candidate — if none remain, the slot is treated as not installed.
    /// Lockfile-backed selection is the layer above.
    fn package_slot(
        &self,
        group: &str,
        name: &str,
        version: Option<&str>,
    ) -> Result<PathBuf, ResolveError> {
        let slot_dir = self
            .ws_root
            .join("vibedeps")
            .join(format!("{group}.{name}"));
        if !slot_dir.is_dir() {
            return Err(ResolveError::PackageSlotNotFound(name.to_string()));
        }

        match version {
            Some(v) => Ok(slot_dir.join(v)),
            None => {
                // The freshest installed version: collect names that look like
                // versions (a version directory starts with its major number —
                // a stray non-version folder is not a candidate, B-028 У1) and
                // take the semver-newest. Zero candidates ⇒ not installed.
                let candidates: Vec<String> = read_dir_or_empty(&slot_dir)
                    .filter_map(|e| {
                        let p = e.path();
                        if !p.is_dir() {
                            return None;
                        }
                        let n = p.file_name()?.to_str()?;
                        n.chars()
                            .next()
                            .is_some_and(|c| c.is_ascii_digit())
                            .then_some(n.to_string())
                    })
                    .collect();
                match version_order::newest(candidates.iter().map(String::as_str)) {
                    Some(v) => Ok(slot_dir.join(v)),
                    None => Err(ResolveError::PackageSlotNotFound(name.to_string())),
                }
            }
        }
    }
}

/// Resolve a doc-path (relative to a `spec/` root) to a spec-source file —
/// either serialisation (`.md` or `.xml`; a document's address does not
/// depend on its form, PROP-045 ##ADDRESSING-UNCHANGED) — inverting the
/// `PROP-NNN` / `FEAT-NNN` truncation by a prefix-scan. `X.md` + `X.xml`
/// beside each other is a [`ResolveError::PairCollision`]: one document,
/// one form, and the resolver never guesses which half of a split brain
/// to read.
fn resolve_doc(base_spec: &Path, doc_path: &str) -> Result<PathBuf, ResolveError> {
    let (dir, last) = match doc_path.rsplit_once('/') {
        Some((d, l)) => (base_spec.join(d), l),
        None => (base_spec.to_path_buf(), doc_path),
    };

    if is_id_stem(last) {
        let matches: Vec<PathBuf> = read_dir_or_empty(&dir)
            .map(|e| e.path())
            .filter(|p| id_file_matches(p, last))
            .collect();
        if let Some((md, xml)) = pair_among(&matches) {
            return Err(ResolveError::PairCollision { markdown: md, xml });
        }
        match matches.as_slice() {
            [] => Err(ResolveError::DocNotFound {
                doc_path: doc_path.to_string(),
                base: base_spec.display().to_string(),
            }),
            [one] => Ok(one.clone()),
            many => Err(ResolveError::AmbiguousDoc {
                id: last.to_string(),
                count: many.len(),
                dir: dir.display().to_string(),
            }),
        }
    } else {
        let md = base_spec.join(format!("{doc_path}.md"));
        let xml = base_spec.join(format!("{doc_path}.xml"));
        match (md.is_file(), xml.is_file()) {
            (true, true) => Err(ResolveError::PairCollision { markdown: md, xml }),
            (true, false) => Ok(md),
            (false, true) => Ok(xml),
            (false, false) => Err(ResolveError::DocNotFound {
                doc_path: doc_path.to_string(),
                base: base_spec.display().to_string(),
            }),
        }
    }
}

/// The first same-stem `.md` + `.xml` pair among `files`, if any — the
/// one-document-one-form law over an already-filtered candidate list.
fn pair_among(files: &[PathBuf]) -> Option<(PathBuf, PathBuf)> {
    for a in files {
        if !is_md(a) {
            continue;
        }
        for b in files {
            if is_xml(b) && a.file_stem() == b.file_stem() {
                return Some((a.clone(), b.clone()));
            }
        }
    }
    None
}

fn is_md(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()) == Some("md")
}

fn is_xml(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()) == Some("xml")
}

/// Does a file stem (either serialisation's extension stripped) equal `id`
/// or start with `id-` (the descriptive-slug form)?
fn id_file_matches(path: &Path, id: &str) -> bool {
    let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
        return false;
    };
    let Some(stem) = name
        .strip_suffix(".md")
        .or_else(|| name.strip_suffix(".xml"))
    else {
        return false;
    };
    stem == id
        || stem
            .strip_prefix(id)
            .is_some_and(|rest| rest.starts_with('-'))
}

/// A `PROP-NNN` / `FEAT-NNN` id stem (the truncated doc-path tail).
fn is_id_stem(s: &str) -> bool {
    let Some((kind, num)) = s.split_once('-') else {
        return false;
    };
    (kind == "PROP" || kind == "FEAT") && !num.is_empty() && num.bytes().all(|b| b.is_ascii_digit())
}

/// Iterate a directory's entries, yielding nothing if it is unreadable or
/// absent (the resolver degrades to "not found", never panics).
fn read_dir_or_empty(dir: &Path) -> impl Iterator<Item = fs::DirEntry> {
    fs::read_dir(dir).into_iter().flatten().flatten()
}

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
