//! The M0 local-directory registry backend — [`LocalRegistry`].
//!
//! Laid out `<root>/<group>/<name>/v<version>/` per `VIBEVM-SPEC.md`
//! §8.2 — the same on-disk shape a [`GitMonorepoRegistry`](crate::GitMonorepoRegistry)
//! clone delegates to. Split out of the crate root so the cell behind
//! the `Registry` seam lives in one file-set with one registration
//! point.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");

use std::fs;
use std::path::{Path, PathBuf};

use specmark::cell;
use vibe_core::manifest::Manifest;
use vibe_core::{Group, PackageRef, VersionSpec};

use crate::{CachedPackage, Registry, RegistryError, ResolvedPackage, compute_content_hash, store};

/// The M0 local-directory registry backend, laid out
/// `<root>/<group>/<name>/v<version>/` per `VIBEVM-SPEC.md` §8.2.
///
/// The blessed path — open the root, resolve a pkgref, fetch into the
/// machine-global store (touches the filesystem at every step):
///
/// ```no_run
/// use std::path::Path;
/// use vibe_core::PackageRef;
/// use vibe_registry::{LocalRegistry, store};
///
/// let registry = LocalRegistry::new("path/to/registry").unwrap();
/// let pkgref = PackageRef::parse("org.vibevm/wal@^0.1").unwrap();
/// let resolved = registry.resolve(&pkgref).unwrap();
/// let cached = registry
///     .fetch(&resolved, &store::store_root().unwrap())
///     .unwrap();
/// assert!(cached.content_hash.starts_with("sha256:"));
/// ```
#[derive(Clone)]
#[cell(seam = "Registry", variant = "local")]
pub struct LocalRegistry {
    root: PathBuf,
    pub(crate) candidate_owner: std::sync::Arc<()>,
    /// The invocation's template layer, when one was installed
    /// (PROP-075 `#TEMPLATE-DOMAIN-SEAM`'s fourth call site). `None`
    /// leaves every method behaving exactly as it did before PROP-075,
    /// which is what a library caller and a unit test get.
    templates: Option<std::sync::Arc<crate::templates::TemplateLayer>>,
}

impl LocalRegistry {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, RegistryError> {
        let root = root.into();
        if !root.is_dir() {
            return Err(RegistryError::MissingRoot(root));
        }
        Ok(LocalRegistry {
            root,
            candidate_owner: std::sync::Arc::new(()),
            templates: crate::templates::active(),
        })
    }

    /// The same registry over an explicit template layer — the seam a
    /// test drives instead of installing one process-wide.
    ///
    /// ```no_run
    /// use std::path::Path;
    /// use std::sync::Arc;
    /// use std::time::Duration;
    /// use vibe_registry::LocalRegistry;
    /// use vibe_registry::templates::{Posture, TemplateLayer};
    ///
    /// let layer = Arc::new(TemplateLayer::http(
    ///     Path::new("/home/u/.vibe"),
    ///     Duration::from_secs(900),
    ///     Posture::Cached,
    /// ));
    /// let registry = LocalRegistry::new("path/to/registry")?.with_templates(Some(layer));
    /// assert!(registry.template_layer().is_some());
    /// # Ok::<(), vibe_registry::RegistryError>(())
    /// ```
    pub fn with_templates(
        mut self,
        templates: Option<std::sync::Arc<crate::templates::TemplateLayer>>,
    ) -> Self {
        self.templates = templates;
        self
    }

    /// This registry's template layer, if it has one.
    pub fn template_layer(&self) -> Option<&std::sync::Arc<crate::templates::TemplateLayer>> {
        self.templates.as_ref()
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// List every version available for `<group>/<name>`, sorted ascending.
    pub fn list_versions(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Vec<semver::Version>, RegistryError> {
        let dir = self.root.join(group.as_str()).join(name);
        if !dir.is_dir() {
            return Err(RegistryError::UnknownPackage {
                group: group.clone(),
                name: name.to_owned(),
            });
        }
        let mut versions = Vec::new();
        let entries = fs::read_dir(&dir).map_err(|source| RegistryError::Io {
            path: dir.clone(),
            source,
        })?;
        for entry in entries {
            let entry = entry.map_err(|source| RegistryError::Io {
                path: dir.clone(),
                source,
            })?;
            if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                continue;
            }
            let n = entry.file_name();
            let s = n.to_string_lossy();
            let Some(ver_str) = s.strip_prefix('v') else {
                continue;
            };
            match semver::Version::parse(ver_str) {
                Ok(v) => versions.push(v),
                Err(_) => {
                    return Err(RegistryError::BadVersionDir {
                        path: entry.path(),
                        name: s.into_owned(),
                    });
                }
            }
        }
        versions.sort();
        // **Call site 1 / 4 of PROP-075 `#TEMPLATE-DOMAIN-SEAM`**: the
        // published versions, minus the template revisions, unioned with
        // the versions the template's source lists.
        if let Some(layer) = &self.templates {
            return layer
                .union_versions(&self.root, group, name, versions)
                .map_err(RegistryError::from);
        }
        Ok(versions)
    }

    /// Enumerate every `group` that publishes a package of the bare
    /// `name` — the candidate set short-name resolution (PROP-008
    /// §2.6) walks. A local-directory registry is laid out
    /// `<root>/<group>/<name>/v<version>/`, so the candidates are the
    /// top-level entries whose directory name parses as a [`Group`]
    /// and that carry a `<name>/` subdirectory. De-duplication is
    /// structural — one directory per group. A non-group or
    /// non-package top-level entry is skipped silently; the result is
    /// sorted. `len() > 1` is a collision (PROP-008 §2.7).
    ///
    /// Unlike a remote git registry — which needs a PROP-005 index to
    /// be enumerated cheaply — a local directory is itself the
    /// enumeration, so this needs no index.
    pub fn candidate_groups(&self, name: &str) -> Result<Vec<Group>, RegistryError> {
        let mut groups: Vec<Group> = Vec::new();
        let entries = fs::read_dir(&self.root).map_err(|source| RegistryError::Io {
            path: self.root.clone(),
            source,
        })?;
        for entry in entries {
            let entry = entry.map_err(|source| RegistryError::Io {
                path: self.root.clone(),
                source,
            })?;
            if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                continue;
            }
            let Ok(group) = Group::parse(&entry.file_name().to_string_lossy()) else {
                continue;
            };
            if entry.path().join(name).is_dir() {
                groups.push(group);
            }
        }
        groups.sort();
        Ok(groups)
    }

    /// The groups publishing a package of this bare name, as the
    /// registry's own index answers them (PROP-002 `#registry-index`):
    /// the persisted `_index/by-name/<name>.json` while its listing
    /// matches the tree — a retired name then answers with nothing —
    /// otherwise the directory listing of [`Self::candidate_groups`],
    /// which is what the projection of a tree without journal facts
    /// holds.
    ///
    /// These are not short-name resolution candidates: since the
    /// owner's 2026-11-12 ruling only a spelling the registry ISSUED
    /// resolves (PROP-008 §2.6 `##ISSUED-NAMES-ONLY`), and this list
    /// serves the refusal's hint.
    pub fn indexed_groups_publishing(&self, name: &str) -> Result<Vec<Group>, RegistryError> {
        Ok(self.indexed_name_lookup(name)?.groups)
    }

    /// Pick the highest version that satisfies `req`.
    pub fn resolve(&self, pkgref: &PackageRef) -> Result<ResolvedPackage, RegistryError> {
        let group = pkgref
            .group
            .as_ref()
            .ok_or_else(|| RegistryError::UnqualifiedPkgref(pkgref.to_string()))?;
        // **A channel lowers to an exact pin before anything selects.**
        // PROP-075 `#CHANNEL-SYNTAX` names the entry paths; lowering at
        // the one seam every one of them passes through is strictly
        // stronger — no entry path can forget — and leaves the solver
        // untouched, because an unlowered channel still matches nothing.
        let lowered = self.lower(pkgref)?;
        let pkgref = lowered.as_ref().unwrap_or(pkgref);
        let versions = self.list_versions(group, pkgref.name.as_str())?;
        let picked = match &pkgref.version {
            VersionSpec::Latest => {
                // Latest stable = highest version with no pre-release segment.
                versions.iter().rev().find(|v| v.pre.is_empty()).cloned()
            }
            VersionSpec::Req(req) => versions
                .iter()
                .rev()
                .find(|v| req.matches(v) && v.pre.is_empty())
                .or_else(|| versions.iter().rev().find(|v| req.matches(v)))
                .cloned(),
            // An unlowered channel picks nothing (PROP-075
            // `#CHANNEL-SYNTAX`): `lower_channel` runs at the entry path,
            // and a registry that was reached without it refuses.
            VersionSpec::Channel(_) => None,
        };
        let Some(version) = picked else {
            return Err(RegistryError::NoMatchingVersion {
                group: group.clone(),
                name: pkgref.name.to_string(),
                req: pkgref.version.requirement_label(),
            });
        };
        // **Call site 3 of PROP-075 `#TEMPLATE-DOMAIN-SEAM`**: a version
        // the template produces and the registry does not publish is
        // materialised as an ordinary package directory, and every caller
        // above this gets the `ResolvedPackage` it already expects.
        // `#TEMPLATE-CONCRETE-WINS`: a version in both sets never reaches
        // the renderer at all.
        if let Some(layer) = &self.templates
            && let Some(resolved) = layer
                .directory_materialise(&self.root, group, pkgref.name.as_str(), &version)
                .map_err(RegistryError::from)?
        {
            return Ok(resolved);
        }
        let source_dir = self
            .root
            .join(group.as_str())
            .join(pkgref.name.as_str())
            .join(format!("v{version}"));
        Ok(ResolvedPackage {
            group: group.clone(),
            name: pkgref.name.to_string(),
            version,
            source_dir,
        })
    }

    /// Insert a resolved package into the machine-global store at
    /// `<store_root>/<group>/<name>/v<version>/`, write-once
    /// (PROP-010 §2.7), and return a `CachedPackage` with manifest and
    /// content hash populated. The content hash is computed over the
    /// source tree — the recipe's exclude set skips `.git` and build
    /// output, so it equals the hash of the entry the store will hold.
    /// An already-present entry is returned untouched (our code never
    /// rewrites one); its manifest is read back so the
    /// `CachedPackage` describes the bytes that will materialise.
    pub fn fetch(
        &self,
        resolved: &ResolvedPackage,
        store_root: &Path,
    ) -> Result<CachedPackage, RegistryError> {
        let content_hash = compute_content_hash(&resolved.source_dir)?;
        let inserted = store::insert_current_at(
            store_root,
            &resolved.source_dir,
            &resolved.group,
            &resolved.name,
            &resolved.version,
            &content_hash,
        )?;
        let reused = matches!(&inserted, store::InsertOutcome::AlreadyPresent(_));
        let cache_dir = inserted.into_entry();

        let manifest_path = cache_dir.join(Manifest::FILENAME);
        let manifest = Manifest::read(&manifest_path)?;
        if manifest.package.is_none() {
            return Err(RegistryError::MalformedMeta {
                path: manifest_path.clone(),
                reason: "registry package manifest must carry a [package] table".to_string(),
            });
        }

        // Build a source URI. On local-registry M0 we just encode the absolute
        // source path. We intentionally do NOT include the store path.
        let source_uri = source_uri_for_local(&resolved.source_dir);
        let noted = self
            .templates
            .as_ref()
            .map(|layer| layer.noted(&resolved.group, &resolved.name, &resolved.version))
            .unwrap_or_default();

        Ok(CachedPackage {
            // The lock record of a rendered version, when this
            // invocation rendered one (PROP-075
            // `#TEMPLATE-LOCK-RECORD`). `None` for every concretely
            // published package, which is what keeps a lock with no
            // template byte-identical.
            template: noted.template,
            via_channel: noted.via_channel,
            locked_selection: None,
            route_publication: vibe_core::RoutePublication::Local,
            recorded_source_kind: None,
            store_reused: Some(reused),
            acquisition: Some(vibe_wire::generated::shared::SourceAcquisition::Directory),
            resolved: resolved.clone(),
            cache_dir,
            manifest,
            content_hash,
            source_uri,
            // Nothing redirected this fetch (`PROP-066#provenance`).
            fetched_from: None,
            // `LocalRegistry` (`--registry <path>`) is the M0-shape and
            // does not participate in the per-package / multi-registry
            // model; it leaves the lockfile-v2 provenance fields blank.
            registry_name: None,
            source_ref: None,
            resolved_commit: None,
            overridden: false,
            is_git_source: false,
            is_path_source: false,
            is_embedded: false,
            is_local: false,
            via_redirect: None,
        })
    }
}

#[path = "local_registry/templates.rs"]
mod templates;

impl Registry for LocalRegistry {
    fn list_versions(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Vec<semver::Version>, RegistryError> {
        LocalRegistry::list_versions(self, group, name)
    }
    fn resolve(&self, pkgref: &PackageRef) -> Result<ResolvedPackage, RegistryError> {
        LocalRegistry::resolve(self, pkgref)
    }
    fn fetch(
        &self,
        resolved: &ResolvedPackage,
        store_root: &Path,
    ) -> Result<CachedPackage, RegistryError> {
        LocalRegistry::fetch(self, resolved, store_root)
    }
}

pub(crate) fn source_uri_for_local(path: &Path) -> String {
    let mut s = path.to_string_lossy().replace('\\', "/");
    // Ensure a leading `/` on Windows paths like `C:/Users/…` so the URI has
    // the `file:///C:/Users/...` shape.
    if !s.starts_with('/') {
        s.insert(0, '/');
    }
    format!("file://{s}")
}

/// The short-name half of a local-directory registry: what a bare
/// spelling finds in it, candidates and registry-offered aliases alike.
mod short_name;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    /// The canonical group every fixture package in these tests belongs to.
    fn org() -> Group {
        Group::parse("org.vibevm").unwrap()
    }

    fn make_fixture_registry() -> (tempfile::TempDir, PathBuf) {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();

        // org.vibevm/wal/v0.1.0
        let v1 = root.join("org.vibevm/wal/v0.1.0");
        fs::create_dir_all(&v1).unwrap();
        fs::write(
            v1.join("vibe.toml"),
            r#"[package]
group = "org.vibevm"
name = "wal"
kind = "flow"
version = "0.1.0"
description = "WAL v0.1.0"
"#,
        )
        .unwrap();
        fs::write(v1.join("README.md"), "# wal 0.1.0\n").unwrap();

        // org.vibevm/wal/v0.2.0
        let v2 = root.join("org.vibevm/wal/v0.2.0");
        fs::create_dir_all(&v2).unwrap();
        fs::write(
            v2.join("vibe.toml"),
            r#"[package]
group = "org.vibevm"
name = "wal"
kind = "flow"
version = "0.2.0"
description = "WAL v0.2.0"
"#,
        )
        .unwrap();
        fs::write(v2.join("README.md"), "# wal 0.2.0\n").unwrap();

        (dir, root)
    }

    #[test]
    fn lists_versions_sorted() {
        let (_guard, root) = make_fixture_registry();
        let reg = LocalRegistry::new(root).unwrap();
        let versions = reg.list_versions(&org(), "wal").unwrap();
        assert_eq!(
            versions.iter().map(|v| v.to_string()).collect::<Vec<_>>(),
            vec!["0.1.0", "0.2.0"]
        );
    }

    #[test]
    fn resolves_latest() {
        let (_guard, root) = make_fixture_registry();
        let reg = LocalRegistry::new(root).unwrap();
        let pkgref = PackageRef::parse("org.vibevm/wal").unwrap();
        let r = reg.resolve(&pkgref).unwrap();
        assert_eq!(r.version.to_string(), "0.2.0");
    }

    #[test]
    fn resolves_exact_version() {
        let (_guard, root) = make_fixture_registry();
        let reg = LocalRegistry::new(root).unwrap();
        let pkgref = PackageRef::parse("org.vibevm/wal@0.1.0").unwrap();
        let r = reg.resolve(&pkgref).unwrap();
        assert_eq!(r.version.to_string(), "0.1.0");
    }

    #[test]
    fn resolves_range_to_highest_match() {
        let (_guard, root) = make_fixture_registry();
        let reg = LocalRegistry::new(root).unwrap();
        let pkgref = PackageRef::parse("org.vibevm/wal@^0.1").unwrap();
        let r = reg.resolve(&pkgref).unwrap();
        // ^0.1 → >=0.1.0, <0.2.0 — so only 0.1.0 qualifies.
        assert_eq!(r.version.to_string(), "0.1.0");
    }

    #[test]
    fn unknown_package_errors_clearly() {
        let (_guard, root) = make_fixture_registry();
        let reg = LocalRegistry::new(root).unwrap();
        let pkgref = PackageRef::parse("org.vibevm/nope").unwrap();
        let err = reg.resolve(&pkgref).unwrap_err();
        assert!(matches!(err, RegistryError::UnknownPackage { .. }));
    }

    #[test]
    fn candidate_groups_single_group_one_match() {
        // PROP-008 §2.6 short-name resolution: the standard fixture
        // carries only `org.vibevm/wal`, so `wal` has one candidate.
        let (_guard, root) = make_fixture_registry();
        let reg = LocalRegistry::new(root).unwrap();
        assert_eq!(reg.candidate_groups("wal").unwrap(), vec![org()]);
    }

    #[test]
    fn candidate_groups_absent_name_is_empty() {
        let (_guard, root) = make_fixture_registry();
        let reg = LocalRegistry::new(root).unwrap();
        assert!(reg.candidate_groups("nope").unwrap().is_empty());
    }

    #[test]
    fn candidate_groups_collision_lists_every_group_sorted() {
        // A short-name collision (PROP-008 §2.7): two groups each
        // publish a `wal`. The standard fixture gives `org.vibevm/wal`;
        // add `com.acme/wal` alongside. The result is sorted.
        let (_guard, root) = make_fixture_registry();
        let acme = root.join("com.acme/wal/v0.1.0");
        fs::create_dir_all(&acme).unwrap();
        fs::write(acme.join("README.md"), "# acme wal\n").unwrap();
        let reg = LocalRegistry::new(root).unwrap();
        assert_eq!(
            reg.candidate_groups("wal").unwrap(),
            vec![Group::parse("com.acme").unwrap(), org()]
        );
    }

    #[test]
    fn candidate_groups_skips_non_group_directories() {
        // A top-level entry whose name is not a valid group (here:
        // uppercase) is skipped, never an error.
        let (_guard, root) = make_fixture_registry();
        fs::create_dir_all(root.join("NotAGroup/wal/v0.1.0")).unwrap();
        let reg = LocalRegistry::new(root).unwrap();
        assert_eq!(reg.candidate_groups("wal").unwrap(), vec![org()]);
    }

    #[test]
    fn no_matching_version_errors() {
        let (_guard, root) = make_fixture_registry();
        let reg = LocalRegistry::new(root).unwrap();
        let pkgref = PackageRef::parse("org.vibevm/wal@^9.0").unwrap();
        let err = reg.resolve(&pkgref).unwrap_err();
        assert!(matches!(err, RegistryError::NoMatchingVersion { .. }));
    }

    #[test]
    fn fetch_populates_cache() {
        let (_guard, root) = make_fixture_registry();
        let reg = LocalRegistry::new(root).unwrap();
        let cache_dir = tempdir().unwrap();
        let pkgref = PackageRef::parse("org.vibevm/wal@0.2.0").unwrap();
        let resolved = reg.resolve(&pkgref).unwrap();
        let cached = reg.fetch(&resolved, cache_dir.path()).unwrap();
        assert!(cached.cache_dir.join("vibe.toml").exists());
        assert!(cached.cache_dir.join("README.md").exists());
        assert_eq!(cached.package_meta().version.to_string(), "0.2.0");
        assert!(cached.content_hash.starts_with("sha256:"));
        assert!(cached.source_uri.starts_with("file://"));
    }

    #[test]
    fn content_hash_is_stable() {
        let (_guard, root) = make_fixture_registry();
        let reg = LocalRegistry::new(root).unwrap();
        let cache_a = tempdir().unwrap();
        let cache_b = tempdir().unwrap();
        let pkgref = PackageRef::parse("org.vibevm/wal@0.2.0").unwrap();
        let resolved = reg.resolve(&pkgref).unwrap();
        let a = reg.fetch(&resolved, cache_a.path()).unwrap();
        let b = reg.fetch(&resolved, cache_b.path()).unwrap();
        assert_eq!(a.content_hash, b.content_hash);
    }
}
