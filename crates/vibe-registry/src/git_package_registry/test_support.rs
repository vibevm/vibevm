//! Shared fixtures for the per-package registry's submodule tests —
//! the canned [`GitBackend`] fake plus registry constructors.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");

pub(crate) use fixtures::*;

/// The fixtures live behind their own `#[cfg(test)]` marker: fact
/// extraction is per-file, and the no-unwrap rule scopes test code by
/// the enclosing `#[cfg(test)]` item — the marker keeps these fakes
/// reading as test code now that they live outside the parent module's
/// inline `mod test_support`.
#[cfg(test)]
mod fixtures {
    use std::collections::{HashMap, HashSet};
    use std::sync::Mutex;

    use super::super::*;

    /// Test-only `GitBackend` that serves a pre-seeded set of tags and
    /// archive-fetched files per `(url, ref, path)`, and on `bootstrap`
    /// copies a fixture directory into the destination clone.
    #[derive(Default)]
    pub(crate) struct FakeBackend {
        pub(crate) tags: Mutex<HashMap<String, Vec<String>>>,
        pub(crate) files: Mutex<HashMap<(String, String, String), Vec<u8>>>,
        pub(crate) bootstrap_seeds: Mutex<HashMap<String, PathBuf>>,
        /// URLs that should make `update` fail with `RefNotFound`. Used
        /// to test the "primary's working clone is now stuck on a tag
        /// the remote no longer carries; fall through to mirror"
        /// scenario — the mirror walk must wipe the local clone and
        /// re-bootstrap from the next URL when `update` fails.
        pub(crate) update_fail_urls: Mutex<HashSet<String>>,
        pub(crate) bootstrap_calls: Mutex<u32>,
        pub(crate) update_calls: Mutex<u32>,
        /// Recorded bootstrap-call URLs, in order. Tests assert on
        /// this when verifying primary-then-mirror dispatch.
        pub(crate) bootstrap_urls: Mutex<Vec<String>>,
    }

    impl FakeBackend {
        pub(crate) fn seed_tags(&self, url: impl Into<String>, tags: Vec<String>) {
            self.tags.lock().unwrap().insert(url.into(), tags);
        }
        pub(crate) fn seed_file(
            &self,
            url: impl Into<String>,
            refname: impl Into<String>,
            path: impl Into<String>,
            bytes: Vec<u8>,
        ) {
            self.files
                .lock()
                .unwrap()
                .insert((url.into(), refname.into(), path.into()), bytes);
        }
        pub(crate) fn seed_bootstrap(&self, url: impl Into<String>, source_dir: PathBuf) {
            self.bootstrap_seeds
                .lock()
                .unwrap()
                .insert(url.into(), source_dir);
        }
        /// Wire `update(dest, refname)` to fail with `RefNotFound` when
        /// the clone at `dest` was last bootstrapped from `url`. Used
        /// to drive the "wipe and fall through" branch of
        /// `bootstrap_or_update_at`.
        #[allow(dead_code)]
        pub(crate) fn fail_update_for_url(&self, url: impl Into<String>) {
            self.update_fail_urls.lock().unwrap().insert(url.into());
        }
        pub(crate) fn bootstrap_count(&self) -> u32 {
            *self.bootstrap_calls.lock().unwrap()
        }
        pub(crate) fn update_count(&self) -> u32 {
            *self.update_calls.lock().unwrap()
        }
        pub(crate) fn bootstrap_urls(&self) -> Vec<String> {
            self.bootstrap_urls.lock().unwrap().clone()
        }
    }

    impl GitBackend for FakeBackend {
        fn bootstrap(&self, url: &str, _refname: &str, dest: &Path) -> Result<(), GitError> {
            *self.bootstrap_calls.lock().unwrap() += 1;
            self.bootstrap_urls.lock().unwrap().push(url.to_string());
            let seed = self
                .bootstrap_seeds
                .lock()
                .unwrap()
                .get(url)
                .cloned()
                .ok_or_else(|| GitError::RepoNotFound {
                    url: url.to_string(),
                })?;
            fs::create_dir_all(dest).unwrap();
            for entry in walkdir::WalkDir::new(&seed)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let rel = entry.path().strip_prefix(&seed).unwrap();
                if rel.as_os_str().is_empty() {
                    continue;
                }
                let target = dest.join(rel);
                if entry.file_type().is_dir() {
                    fs::create_dir_all(&target).unwrap();
                } else if entry.file_type().is_file() {
                    fs::copy(entry.path(), &target).unwrap();
                }
            }
            // Mark dest as a real git repo for the `.git` presence check.
            // Stash the URL inside `.git/origin-url` so `update` can
            // recover which URL last sourced this clone — that lets
            // `fail_update_for_url` selectively fail updates per origin.
            fs::create_dir_all(dest.join(".git")).unwrap();
            fs::write(dest.join(".git/origin-url"), url).unwrap();
            Ok(())
        }
        fn update(&self, dest: &Path, refname: &str) -> Result<(), GitError> {
            *self.update_calls.lock().unwrap() += 1;
            let origin = fs::read_to_string(dest.join(".git/origin-url")).unwrap_or_default();
            if self.update_fail_urls.lock().unwrap().contains(&origin) {
                return Err(GitError::RefNotFound {
                    url: origin,
                    refname: refname.to_string(),
                });
            }
            Ok(())
        }
        fn list_tags(&self, url: &str) -> Result<Vec<String>, GitError> {
            self.tags
                .lock()
                .unwrap()
                .get(url)
                .cloned()
                .ok_or_else(|| GitError::RepoNotFound {
                    url: url.to_string(),
                })
        }
        fn fetch_file_at_ref(
            &self,
            url: &str,
            refname: &str,
            path: &str,
        ) -> Result<Vec<u8>, GitError> {
            let key = (url.to_string(), refname.to_string(), path.to_string());
            self.files
                .lock()
                .unwrap()
                .get(&key)
                .cloned()
                .ok_or_else(|| GitError::FileNotFoundInRef {
                    url: url.to_string(),
                    refname: refname.to_string(),
                    path: path.to_string(),
                })
        }
    }

    pub(crate) fn manifest_text(name: &str, kind: &str, version: &str) -> String {
        format!(
            "[package]\ngroup = \"org.vibevm\"\nname = \"{name}\"\nkind = \"{kind}\"\nversion = \"{version}\"\n"
        )
    }

    /// The canonical group every fixture package in these tests belongs
    /// to. The registry is group-native (PROP-008): identity is
    /// `(group, name)`, `kind` plays no part in resolution.
    pub(crate) fn org() -> Group {
        Group::parse("org.vibevm").unwrap()
    }

    pub(crate) fn registry_with(
        cache: &Path,
        org_url: &str,
        naming: NamingConvention,
        backend: Arc<dyn GitBackend>,
    ) -> GitPerPackageRegistry {
        GitPerPackageRegistry::open_with(
            "vibespecs",
            org_url,
            "main",
            naming,
            cache,
            backend,
            DEFAULT_FRESHNESS_SECS,
        )
        .unwrap()
    }

    pub(crate) fn registry_with_mirrors(
        cache: &Path,
        org_url: &str,
        naming: NamingConvention,
        mirror_urls: Vec<String>,
        backend: Arc<dyn GitBackend>,
    ) -> GitPerPackageRegistry {
        GitPerPackageRegistry::open_with_mirrors(
            "vibespecs",
            org_url,
            "main",
            naming,
            mirror_urls,
            cache,
            backend,
            DEFAULT_FRESHNESS_SECS,
        )
        .unwrap()
    }
}
