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
//! picks the slot version, and absent one a single installed version is taken.
//! A lockfile-backed selection (kind + version from `vibe.lock`) is the layer
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
    #[error("package `{0}` has several installed versions; address must pin `@version`")]
    AmbiguousVersion(String),
    #[error("document `{doc_path}` not found under `{base}`")]
    DocNotFound { doc_path: String, base: String },
    #[error("document id `{id}` is ambiguous ({count} files match) under `{dir}`")]
    AmbiguousDoc {
        id: String,
        count: usize,
        dir: String,
    },
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
            Authority::Package { name, version, .. } => {
                Ok(self.package_slot(name, version.as_deref())?.join("spec"))
            }
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

    /// Find a package's materialised slot: `vibedeps/<kind>-<name>/<version>`.
    /// The address carries no `kind`, so the slot is matched by the `-<name>`
    /// suffix (kind + name is unique).
    fn package_slot(&self, name: &str, version: Option<&str>) -> Result<PathBuf, ResolveError> {
        let vibedeps = self.ws_root.join("vibedeps");
        let suffix = format!("-{name}");
        let slot_dir = read_dir_or_empty(&vibedeps)
            .map(|e| e.path())
            .find(|p| {
                p.is_dir()
                    && p.file_name()
                        .and_then(|s| s.to_str())
                        .is_some_and(|n| n.ends_with(&suffix))
            })
            .ok_or_else(|| ResolveError::PackageSlotNotFound(name.to_string()))?;

        match version {
            Some(v) => Ok(slot_dir.join(v)),
            None => {
                let mut versions: Vec<PathBuf> = read_dir_or_empty(&slot_dir)
                    .map(|e| e.path())
                    .filter(|p| p.is_dir())
                    .collect();
                match versions.len() {
                    0 => Err(ResolveError::PackageSlotNotFound(name.to_string())),
                    1 => Ok(versions.pop().unwrap()),
                    _ => Err(ResolveError::AmbiguousVersion(name.to_string())),
                }
            }
        }
    }
}

/// Resolve a doc-path (relative to a `spec/` root) to a `.md` file, inverting
/// the `PROP-NNN` / `FEAT-NNN` truncation by a prefix-scan.
fn resolve_doc(base_spec: &Path, doc_path: &str) -> Result<PathBuf, ResolveError> {
    let (dir, last) = match doc_path.rsplit_once('/') {
        Some((d, l)) => (base_spec.join(d), l),
        None => (base_spec.to_path_buf(), doc_path),
    };

    if is_id_stem(last) {
        let mut matches: Vec<PathBuf> = read_dir_or_empty(&dir)
            .map(|e| e.path())
            .filter(|p| id_file_matches(p, last))
            .collect();
        match matches.len() {
            0 => Err(ResolveError::DocNotFound {
                doc_path: doc_path.to_string(),
                base: base_spec.display().to_string(),
            }),
            1 => Ok(matches.pop().unwrap()),
            n => Err(ResolveError::AmbiguousDoc {
                id: last.to_string(),
                count: n,
                dir: dir.display().to_string(),
            }),
        }
    } else {
        let candidate = base_spec.join(format!("{doc_path}.md"));
        if candidate.is_file() {
            Ok(candidate)
        } else {
            Err(ResolveError::DocNotFound {
                doc_path: doc_path.to_string(),
                base: base_spec.display().to_string(),
            })
        }
    }
}

/// Does a file stem equal `id` or start with `id-` (the descriptive-slug form)?
fn id_file_matches(path: &Path, id: &str) -> bool {
    let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
        return false;
    };
    let Some(stem) = name.strip_suffix(".md") else {
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

#[cfg(test)]
mod tests {
    use super::*;

    // ----- B-031: the host is a package coordinate (Т1–Т6) ------------------

    /// The self coordinate the host project carries since B-031.
    fn host_coord() -> SelfCoordinate {
        SelfCoordinate::new(Some("org.vibevm.core".into()), "vibevm".into())
    }

    #[test]
    fn t1_self_coordinate_resolves_to_the_authored_spec_tree() {
        // Т1: `spec://<self_group>/<self_name>/…` resolves under ws_root/spec,
        // ahead of any vibedeps/ slot lookup (B-031 — the self-match is first).
        let ws = tempfile::TempDir::new().unwrap();
        let doc = ws.path().join("spec/common/TARGET.md");
        fs::create_dir_all(doc.parent().unwrap()).unwrap();
        fs::write(&doc, "# Target\n").unwrap();
        let r = FileResolver::new(ws.path(), host_coord());
        let addr = SpecAddress::parse("spec://org.vibevm.core/vibevm/common/TARGET").unwrap();
        let file = r.resolve_file(&addr).unwrap();
        assert!(file.ends_with("TARGET.md"), "{file:?}");
    }

    #[test]
    fn t2_legacy_host_authority_names_the_self_coordinate_and_b031() {
        // Т2: the old reserved host token no longer resolves; the error points
        // at the actual self coordinate and cites B-031.
        let r = FileResolver::new(Path::new("."), host_coord());
        let addr = SpecAddress::parse("spec://vibevm/common/PROP-000#commits").unwrap();
        let err = r.resolve_file(&addr).unwrap_err();
        let ResolveError::LegacyHostAuthority { given, hint } = &err else {
            panic!("expected LegacyHostAuthority, got {err:?}");
        };
        assert_eq!(given, "vibevm");
        assert!(hint.contains("org.vibevm.core/vibevm"), "{hint}");
        assert!(hint.contains("B-031"), "{hint}");
    }

    #[test]
    fn t3_any_undotted_authority_never_resolves() {
        // Т3: a fixture-style undotted authority (`spec://demo/…`) parses but
        // never resolves — the same legacy-host error as the real token.
        let r = FileResolver::new(Path::new("."), host_coord());
        let addr = SpecAddress::parse("spec://demo/x/y#z").unwrap();
        let err = r.resolve_file(&addr).unwrap_err();
        assert!(matches!(err, ResolveError::LegacyHostAuthority { .. }));
        // The hint still points at the self coordinate, not at `demo`.
        let hint = err.to_string();
        assert!(hint.contains("org.vibevm.core/vibevm"), "{hint}");
    }

    #[test]
    fn t4_a_non_self_package_resolves_to_its_vibedeps_slot() {
        // Т4: a package coordinate that is NOT the self coordinate falls through
        // to the vibedeps/ slot lookup, unchanged from before B-031.
        let ws = tempfile::TempDir::new().unwrap();
        let doc = ws
            .path()
            .join("vibedeps/flow-demo/1.0.0/spec/contract/API.md");
        fs::create_dir_all(doc.parent().unwrap()).unwrap();
        fs::write(&doc, "# API\n").unwrap();
        let r = FileResolver::new(ws.path(), host_coord());
        let addr = SpecAddress::parse("spec://org.vibevm.demo/demo/contract/API#root").unwrap();
        let file = r.resolve_file(&addr).unwrap();
        assert!(file.ends_with("API.md"), "{file:?}");
    }

    #[test]
    fn t5_a_groupless_project_has_no_self_coordinate() {
        // Т5: a project with no `group` declares no self coordinate. Its own
        // name in package form does NOT resolve to spec/ (it falls through to a
        // vibedeps slot lookup that finds nothing), and an undotted authority
        // errors "no self coordinate".
        let ws = tempfile::TempDir::new().unwrap();
        let coord = SelfCoordinate::new(None, "solo".into());
        let r = FileResolver::new(ws.path(), coord);

        // Package form of the project's own name → slot lookup, not spec/.
        let pkg = SpecAddress::parse("spec://org.foo/solo/x/y").unwrap();
        assert!(matches!(
            r.resolve_file(&pkg).unwrap_err(),
            ResolveError::PackageSlotNotFound(_)
        ));

        // Undotted authority → "no self coordinate".
        let undotted = SpecAddress::parse("spec://solo/x/y").unwrap();
        let err = r.resolve_file(&undotted).unwrap_err();
        assert!(err.to_string().contains("no self coordinate"), "{}", err);
    }

    #[test]
    fn t6_a_versioned_self_coordinate_is_an_error() {
        // Т6 (У2): a self-coordinate address carrying `@version` is an error —
        // the self coordinate is unversioned, so the pin is never dropped.
        let r = FileResolver::new(Path::new("."), host_coord());
        let addr = SpecAddress::parse("spec://org.vibevm.core/vibevm@0.1/common/TARGET").unwrap();
        let err = r.resolve_file(&addr).unwrap_err();
        let ResolveError::SelfCoordinateVersioned {
            self_group,
            self_name,
            version,
        } = &err
        else {
            panic!("expected SelfCoordinateVersioned, got {err:?}");
        };
        assert_eq!(self_group, "org.vibevm.core");
        assert_eq!(self_name, "vibevm");
        assert_eq!(version, "0.1");
    }

    #[test]
    fn id_stem_recognition() {
        assert!(is_id_stem("PROP-042"));
        assert!(is_id_stem("FEAT-7"));
        assert!(!is_id_stem("PROP"));
        assert!(!is_id_stem("PROP-"));
        assert!(!is_id_stem("README"));
        assert!(!is_id_stem("PROP-00x"));
        assert!(!is_id_stem("DESIGN-1")); // only PROP / FEAT truncate
    }

    #[test]
    fn id_file_match() {
        assert!(id_file_matches(
            Path::new("PROP-042-example-thing.md"),
            "PROP-042"
        ));
        assert!(id_file_matches(Path::new("PROP-042.md"), "PROP-042"));
        // A different number sharing a prefix does not match.
        assert!(!id_file_matches(Path::new("PROP-0420-x.md"), "PROP-042"));
        assert!(!id_file_matches(
            Path::new("PROP-042-example.txt"),
            "PROP-042"
        ));
    }
}
