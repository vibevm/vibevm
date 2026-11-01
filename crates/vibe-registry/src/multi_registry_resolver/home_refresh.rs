//! The user home's refresh — `vibe registry sync -g`'s walk over the
//! per-package clones a registry's cache bucket already holds.
//!
//! The project form walks a lockfile; a user home has none. What it has
//! is the cache itself: every `<cache>/<hash>/packages/<group>.<name>/clone/`
//! a global install (or any earlier resolution through the same registry)
//! left behind. Those are exactly the clones the next global install
//! reads, so walking the buckets — rather than some list of packages kept
//! on the side — refreshes the set that can actually go stale, and needs
//! no record the cache does not already carry.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");

use super::*;

/// What [`MultiRegistryResolver::refresh_cached_clones`] did for one
/// registry: the clones it brought to the newest release tag, the ones it
/// left and why, and whether the registry's index answered its handshake
/// when this resolver was opened (the handshake is re-asked on every open,
/// so opening the resolver IS the index half of the refresh).
#[derive(Debug, Clone)]
pub struct RegistryRefresh {
    pub registry: String,
    pub registry_ref: String,
    pub index: bool,
    pub clones: Vec<RefreshedEntry>,
    pub skipped: Vec<SkippedEntry>,
}

impl MultiRegistryResolver {
    /// Refresh every per-package clone cached for each git registry of
    /// this resolver to the package's newest release tag (the newest
    /// stable `v<semver>`, or the newest of all when only prereleases
    /// exist).
    ///
    /// A package the registry no longer serves, or one with no release
    /// tag, is reported as skipped: the cache outlived its source, which
    /// is a fact about the cache and not a failure of the refresh. Any
    /// other error — network, auth — stops the walk, as the lockfile
    /// form does, so a half-refreshed home is never reported as done.
    pub fn refresh_cached_clones(&self) -> Result<Vec<RegistryRefresh>, RegistryError> {
        let mut out = Vec::new();
        for reg in &self.registries {
            if reg.is_single_package() {
                continue;
            }
            let mut refresh = RegistryRefresh {
                registry: reg.name().to_string(),
                registry_ref: reg.org_ref().to_string(),
                index: reg.index_client().is_some(),
                clones: Vec::new(),
                skipped: Vec::new(),
            };
            for (group, name) in cached_packages(&reg.cache_dir().join("packages"))? {
                let skip = |reason: String| SkippedEntry {
                    group: group.clone(),
                    name: name.clone(),
                    reason,
                };
                let versions = match reg.list_versions(&group, &name) {
                    Ok(versions) => versions,
                    Err(RegistryError::UnknownPackage { .. }) => {
                        refresh.skipped.push(skip(format!(
                            "registry `{}` no longer serves this package",
                            reg.name()
                        )));
                        continue;
                    }
                    Err(other) => return Err(other),
                };
                let Some(version) = newest_release(&versions) else {
                    refresh
                        .skipped
                        .push(skip("the package has no `v<semver>` tag".to_string()));
                    continue;
                };
                let refname = format!("v{version}");
                reg.refresh_package(&group, &name, &refname)?;
                refresh.clones.push(RefreshedEntry {
                    group,
                    name,
                    via: RefreshedVia::Registry(reg.name().to_string()),
                    refname,
                });
            }
            out.push(refresh);
        }
        Ok(out)
    }
}

/// The `(group, name)` of every clone under a bucket's `packages/`, in
/// name order so two runs report the same list. The directory name is
/// `<group>.<name>`; a package name carries no dot (PROP-008), so the
/// last dot is the split. A directory that does not read as one, or holds
/// no clone, is not ours to refresh and is passed over.
fn cached_packages(packages: &Path) -> Result<Vec<(Group, String)>, RegistryError> {
    let entries = match std::fs::read_dir(packages) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => {
            return Err(RegistryError::Io {
                path: packages.to_path_buf(),
                source,
            });
        }
    };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        if !entry.path().join("clone").join(".git").exists() {
            continue;
        }
        let dir = entry.file_name().to_string_lossy().into_owned();
        let Some((group, name)) = dir.rsplit_once('.') else {
            continue;
        };
        if let Ok(group) = Group::parse(group) {
            found.push((group, name.to_string()));
        }
    }
    found.sort_by(|a, b| (a.0.as_str(), &a.1).cmp(&(b.0.as_str(), &b.1)));
    Ok(found)
}

/// The newest stable version, or the newest of all when every version is
/// a prerelease — the same choice an unconstrained install makes.
fn newest_release(versions: &[semver::Version]) -> Option<&semver::Version> {
    versions
        .iter()
        .filter(|v| v.pre.is_empty())
        .max()
        .or_else(|| versions.iter().max())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newest_release_prefers_stable_over_a_newer_prerelease() {
        let versions: Vec<semver::Version> = ["1.1.1", "1.1.2", "1.2.0-rc.1"]
            .iter()
            .map(|v| v.parse().unwrap())
            .collect();
        assert_eq!(newest_release(&versions).unwrap().to_string(), "1.1.2");
        let only_pre: Vec<semver::Version> = vec!["2.0.0-rc.1".parse().unwrap()];
        assert_eq!(newest_release(&only_pre).unwrap().to_string(), "2.0.0-rc.1");
        assert!(newest_release(&[]).is_none());
    }

    #[test]
    fn cached_packages_reads_group_and_name_and_passes_over_the_rest() {
        let bucket = tempfile::tempdir().unwrap();
        for dir in ["org.vibevm.world.wal", "org.example.vim", "not-a-package"] {
            std::fs::create_dir_all(bucket.path().join(dir).join("clone/.git")).unwrap();
        }
        std::fs::create_dir_all(bucket.path().join("org.example.empty")).unwrap();
        let found: Vec<String> = cached_packages(bucket.path())
            .unwrap()
            .into_iter()
            .map(|(g, n)| format!("{g}/{n}"))
            .collect();
        assert_eq!(found, ["org.example/vim", "org.vibevm.world/wal"]);
        assert!(
            cached_packages(&bucket.path().join("absent"))
                .unwrap()
                .is_empty()
        );
    }
}
