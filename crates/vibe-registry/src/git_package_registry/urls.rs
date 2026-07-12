//! Per-package repo-URL composition — the registry's
//! [`NamingConvention`]-driven `<org>/<group>.<name>.git` shape, the
//! primary-plus-mirrors URL chain, and the credentialed form handed
//! to git invocations (PROP-008 §2.5).

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-008#repo-naming");

use super::*;

impl GitPackageRegistry {
    /// Compose the per-package repo URL — `<org_url>/<naming(group, name)>.git`
    /// for normal multi-package registries, or the verbatim single-package
    /// URL for git-source registries (PROP-002 §2.4.1). Trailing slashes
    /// on `org_url` are tolerated. **No credentials are embedded** —
    /// this URL is safe to record in the lockfile, log, or surface to
    /// humans. For URLs that drive git invocations (`ls-remote` /
    /// `clone` / `fetch`) under `auth = AuthKind::TokenEnv`, callers
    /// reach for [`Self::credentialed_url`] instead.
    ///
    /// The registry is group-native (PROP-008): `kind` plays no part in
    /// URL composition. `repo_name` is always called with `kind = None` —
    /// the default `Fqdn` convention is infallible there; a registry
    /// configured with a legacy `kind-*` convention surfaces a
    /// [`RegistryError::Core`] instead.
    pub fn package_repo_url(&self, group: &Group, name: &str) -> Result<String, RegistryError> {
        if let Some(url) = &self.single_package_url {
            return Ok(url.clone());
        }
        let repo_name = self
            .naming
            .repo_name(None, group, name)
            .map_err(RegistryError::Core)?;
        let trimmed = self.org_url.trim_end_matches('/');
        Ok(format!("{trimmed}/{repo_name}.git"))
    }

    /// Return the URL to actually pass to a git invocation. For
    /// `auth = AuthKind::TokenEnv` with a loaded token this injects
    /// `https://x-access-token:<TOKEN>@host/...` (the same shape
    /// `vibe-publish` already uses on the push side); for any other
    /// regime — or for a non-https URL — this is the plain
    /// `package_repo_url` value. The token never escapes this method;
    /// modern git redacts it from any error stderr it prints, and
    /// vibe never logs the resulting URL outside the spawned-process
    /// boundary.
    pub fn credentialed_url(&self, plain_url: &str) -> String {
        match (&self.effective_token, plain_url) {
            (Some(token), url)
                if url.starts_with("https://") && !url.contains("x-access-token:") =>
            {
                let body = &url["https://".len()..];
                format!("https://x-access-token:{token}@{body}")
            }
            // Never inject for ssh, http (rare and would expose token
            // in cleartext), file, or already-credentialed URLs.
            _ => plain_url.to_string(),
        }
    }

    /// The URLs to try for a `(group, name)` lookup — the primary, then
    /// the mirrors in priority order. The split is structural so callers
    /// never index into a "primary is element zero" convention: the
    /// primary always exists by type. Mirrors are composed using the
    /// same naming convention as the primary, since the mirror is meant
    /// to be a transparent alternative to the primary's content.
    /// Single-package registries (PROP-002 §2.4.1) return the verbatim
    /// URL with no mirrors — mirrors do not apply.
    ///
    /// Group-native (PROP-008): `repo_name` is called with `kind = None`
    /// — the registry resolves by `(group, name)`. A legacy `kind-*`
    /// naming convention surfaces a [`RegistryError::Core`].
    pub(super) fn package_urls(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<(String, Vec<String>), RegistryError> {
        if let Some(url) = &self.single_package_url {
            return Ok((url.clone(), Vec::new()));
        }
        let repo_name = self
            .naming
            .repo_name(None, group, name)
            .map_err(RegistryError::Core)?;
        let primary = format!("{}/{}.git", self.org_url.trim_end_matches('/'), repo_name);
        let mirrors = self
            .mirror_urls
            .iter()
            .map(|mirror| format!("{}/{}.git", mirror.trim_end_matches('/'), repo_name))
            .collect();
        Ok((primary, mirrors))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    use crate::git_package_registry::test_support::*;

    #[test]
    fn package_repo_url_default_naming() {
        // Group-native default (PROP-008): `Fqdn` composes the repo name
        // as `<group>.<name>`, collision-free because `(group, name)` is
        // unique.
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        let r = registry_with(
            cache.path(),
            "git@gitverse.ru:vibespecs",
            NamingConvention::Fqdn,
            fake,
        );
        assert_eq!(
            r.package_repo_url(&org(), "wal").unwrap(),
            "git@gitverse.ru:vibespecs/org.vibevm_wal.git"
        );
        assert_eq!(
            r.package_repo_url(&org(), "rust-cli").unwrap(),
            "git@gitverse.ru:vibespecs/org.vibevm_rust-cli.git"
        );
    }

    #[test]
    fn package_repo_url_strips_trailing_slash() {
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        let r = registry_with(
            cache.path(),
            "https://gitverse.ru/vibespecs/",
            NamingConvention::Fqdn,
            fake,
        );
        assert_eq!(
            r.package_repo_url(&org(), "rust-cli").unwrap(),
            "https://gitverse.ru/vibespecs/org.vibevm_rust-cli.git"
        );
    }

    #[test]
    fn single_package_url_returned_verbatim() {
        // M1.15 git-source: a single-package registry has its full URL
        // declared up front, naming is bypassed.
        let cache = tempdir().unwrap();
        let fake: Arc<dyn GitBackend> = Arc::new(FakeBackend::default());
        let r = GitPackageRegistry::open_single_package(
            "git-source-flow-internal",
            "https://github.com/me/flow-internal",
            "v0.1.0",
            cache.path(),
            fake,
            DEFAULT_FRESHNESS_SECS,
            vibe_core::manifest::AuthKind::None,
            None,
        )
        .unwrap();
        assert!(r.is_single_package());
        // The repo URL is the URL we passed, regardless of (group, name).
        assert_eq!(
            r.package_repo_url(&org(), "internal").unwrap(),
            "https://github.com/me/flow-internal"
        );
        // (group, name) does not even matter — naming is skipped.
        assert_eq!(
            r.package_repo_url(&org(), "totally-different").unwrap(),
            "https://github.com/me/flow-internal"
        );
    }

    #[test]
    fn single_package_skips_mirror_chain() {
        let cache = tempdir().unwrap();
        let fake: Arc<dyn GitBackend> = Arc::new(FakeBackend::default());
        let r = GitPackageRegistry::open_single_package(
            "git-source",
            "https://github.com/me/flow-internal",
            "v1.0",
            cache.path(),
            fake,
            DEFAULT_FRESHNESS_SECS,
            vibe_core::manifest::AuthKind::None,
            None,
        )
        .unwrap();
        let (primary, mirrors) = r.package_urls(&org(), "internal").unwrap();
        assert_eq!(primary, "https://github.com/me/flow-internal");
        assert!(
            mirrors.is_empty(),
            "a single-package registry has no mirror chain"
        );
    }

    #[test]
    fn package_repo_url_name_only_naming() {
        // The `Name` convention is kind-free, so it composes fine even
        // though the registry is group-native — `repo_name` is called
        // with `kind = None` and yields the bare `name`.
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        let r = registry_with(cache.path(), "git@host:org", NamingConvention::Name, fake);
        assert_eq!(
            r.package_repo_url(&org(), "wal").unwrap(),
            "git@host:org/wal.git"
        );
    }

    #[test]
    fn package_repo_url_legacy_kind_convention_errors_on_group_native_registry() {
        // PROP-008: the registry resolves by `(group, name)` and always
        // composes URLs with `kind = None`. A registry still configured
        // with a legacy `kind/name` convention therefore cannot compose
        // a URL — `package_repo_url` surfaces a `RegistryError::Core`
        // rather than silently producing a kind-shaped name.
        let cache = tempdir().unwrap();
        let fake = Arc::new(FakeBackend::default());
        let r = registry_with(
            cache.path(),
            "git@host:org",
            NamingConvention::KindSlashName,
            fake,
        );
        let err = r.package_repo_url(&org(), "welcome-page").unwrap_err();
        assert!(
            matches!(err, RegistryError::Core(_)),
            "legacy kind-* naming on a group-native registry must error: {err:?}"
        );
    }
}
