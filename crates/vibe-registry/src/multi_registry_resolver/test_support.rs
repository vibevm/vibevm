//! Shared fixtures for the multi-registry resolver's submodule tests —
//! the canned [`GitBackend`] fake plus section / resolver builders.

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#registry-model");

pub(crate) use fixtures::*;

/// The fixtures live behind their own `#[cfg(test)]` marker: fact
/// extraction is per-file, and the no-unwrap rule scopes test code by
/// the enclosing `#[cfg(test)]` item — the marker keeps these fakes
/// reading as test code now that they live outside the parent module's
/// inline `mod test_support`.
#[cfg(test)]
mod fixtures {
    use std::fs;
    use std::sync::Mutex;

    use vibe_core::manifest::NamingConvention;

    use super::super::*;

    /// Test-only `GitBackend` shared across multi-registry tests. Same
    /// shape as the one in `git_package_registry::tests`; duplicated
    /// here rather than promoted to a shared `test_support` module to
    /// keep this commit narrow — that consolidation can land separately.
    #[derive(Default)]
    pub(crate) struct FakeBackend {
        pub(crate) tags: Mutex<HashMap<String, Vec<String>>>,
        pub(crate) files: Mutex<HashMap<(String, String, String), Vec<u8>>>,
        pub(crate) bootstrap_seeds: Mutex<HashMap<String, PathBuf>>,
        pub(crate) bootstrap_calls: Mutex<u32>,
        pub(crate) update_calls: Mutex<u32>,
        /// URLs whose `list_tags` should fail with `AuthFailed` —
        /// simulates a host that returned 401 / 403. Used to drive
        /// the per-`auth` walk-vs-halt rules in §2.3.1.
        pub(crate) auth_failure_urls: Mutex<std::collections::HashSet<String>>,
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
        pub(crate) fn seed_auth_failure(&self, url: impl Into<String>) {
            self.auth_failure_urls.lock().unwrap().insert(url.into());
        }
        pub(crate) fn bootstrap_count(&self) -> u32 {
            *self.bootstrap_calls.lock().unwrap()
        }
    }

    impl GitBackend for FakeBackend {
        fn bootstrap(&self, url: &str, _refname: &str, dest: &Path) -> Result<(), GitError> {
            *self.bootstrap_calls.lock().unwrap() += 1;
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
            fs::create_dir_all(dest.join(".git")).unwrap();
            Ok(())
        }
        fn update(&self, _dest: &Path, _refname: &str) -> Result<(), GitError> {
            *self.update_calls.lock().unwrap() += 1;
            Ok(())
        }
        fn list_tags(&self, url: &str) -> Result<Vec<String>, GitError> {
            if self.auth_failure_urls.lock().unwrap().contains(url) {
                return Err(GitError::AuthFailed {
                    url: url.to_string(),
                });
            }
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

    pub(crate) fn registry_section(name: &str, url: &str) -> RegistrySection {
        RegistrySection {
            name: name.to_string(),
            url: url.to_string(),
            r#ref: "main".to_string(),
            naming: NamingConvention::Fqdn,
            auth: vibe_core::manifest::AuthKind::None,
            token_env: None,
            enabled: true,
        }
    }

    pub(crate) fn registry_section_token_env(
        name: &str,
        url: &str,
        env_var: &str,
    ) -> RegistrySection {
        RegistrySection {
            name: name.to_string(),
            url: url.to_string(),
            r#ref: "main".to_string(),
            naming: NamingConvention::Fqdn,
            auth: vibe_core::manifest::AuthKind::TokenEnv,
            token_env: Some(env_var.to_string()),
            enabled: true,
        }
    }

    pub(crate) fn manifest_text(name: &str, kind: &str, version: &str) -> String {
        format!(
            "[package]\ngroup = \"org.vibevm\"\nname = \"{name}\"\nkind = \"{kind}\"\nversion = \"{version}\"\n"
        )
    }

    /// The canonical group every fixture package in these tests belongs
    /// to. The resolver is group-native (PROP-008): identity is
    /// `(group, name)`, `kind` plays no part in resolution.
    pub(crate) fn org() -> Group {
        Group::parse("org.vibevm").unwrap()
    }

    pub(crate) fn build_resolver(
        cache: &Path,
        registries: Vec<RegistrySection>,
        mirrors: Vec<MirrorSection>,
        overrides: Vec<OverrideSection>,
        backend: Arc<FakeBackend>,
    ) -> MultiRegistryResolver {
        MultiRegistryResolver::from_manifest(
            &registries,
            &mirrors,
            &overrides,
            cache.to_path_buf(),
            backend,
            DEFAULT_FRESHNESS_SECS,
        )
        .unwrap()
    }
}
