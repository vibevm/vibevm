//! Selected-request credential capture controls.
use super::*;
use crate::git_package_registry::test_support::*;
use tempfile::tempdir;
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-auth"
)]
fn deferred_token_checks_selected_route_before_capture_and_keeps_missing_token() {
    let cache = tempdir().unwrap();
    let backend = Arc::new(FakeBackend::default());
    let registry = GitPerPackageRegistry::open_with_deferred_auth(
        "internal",
        "https://host/org",
        "main",
        NamingConvention::Fqdn,
        vec![],
        cache.path(),
        backend.clone(),
        DEFAULT_FRESHNESS_SECS,
        vibe_core::manifest::AuthKind::TokenEnv,
        Some("VIBEVM_TEST_UNSET_AUTH_DEMAND_7B53E1"),
    )
    .unwrap();
    assert!(registry.deferred_token.as_ref().unwrap().get().is_none());
    assert!(
        registry
            .selected_credentialed_url("ssh://host/org/pkg")
            .is_err()
    );
    assert!(
        registry.deferred_token.as_ref().unwrap().get().is_none(),
        "SSH refusal must precede token lookup"
    );
    assert!(matches!(
        registry.selected_credentialed_url("https://host/org/pkg"),
        Err(RegistryError::MissingToken { .. })
    ));
    assert!(registry.deferred_token.as_ref().unwrap().get().is_some());
    assert_eq!(backend.bootstrap_count(), 0);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-auth"
)]
fn ssh_ceiling_still_permits_anonymous_https_and_none_refuses_ssh() {
    let cache = tempdir().unwrap();
    for kind in [
        vibe_core::manifest::AuthKind::None,
        vibe_core::manifest::AuthKind::Ssh,
    ] {
        let registry = GitPerPackageRegistry::open_with_deferred_auth(
            "internal",
            "https://host/org",
            "main",
            NamingConvention::Fqdn,
            vec![],
            cache.path(),
            Arc::new(FakeBackend::default()),
            DEFAULT_FRESHNESS_SECS,
            kind,
            None,
        )
        .unwrap();
        assert_eq!(
            registry
                .selected_credentialed_url("https://host/org/pkg")
                .unwrap(),
            "https://host/org/pkg"
        );
        assert_eq!(
            registry
                .selected_credentialed_url("ssh://host/org/pkg")
                .is_ok(),
            kind == vibe_core::manifest::AuthKind::Ssh
        );
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-auth"
)]
fn excluded_primary_precedes_auth_and_allows_an_independent_mirror() {
    struct ExcludedPrimary(Arc<FakeBackend>);
    impl GitBackend for ExcludedPrimary {
        fn uses_native_materialisation(&self) -> bool {
            true
        }
        // A native backend binds its credential mechanism before any effect
        // (`bind_registry_backend`); this fake binds to itself.
        fn bound_for_registry_auth(
            &self,
            _auth: vibe_core::manifest::AuthKind,
        ) -> Result<Option<Arc<dyn GitBackend>>, GitError> {
            Ok(Some(Arc::new(ExcludedPrimary(self.0.clone()))))
        }
        fn preflight_selected_address(&self, address: &str) -> Result<(), GitError> {
            if address.starts_with("ssh://excluded/") {
                return Err(GitError::ContactRefused(
                    crate::ContactRefusal::ExcludedHost {
                        endpoint: "ssh://excluded".into(),
                    },
                ));
            }
            Ok(())
        }
        fn bootstrap(&self, url: &str, reference: &str, dest: &Path) -> Result<(), GitError> {
            self.0.bootstrap(url, reference, dest)
        }
        fn update(&self, dest: &Path, reference: &str) -> Result<(), GitError> {
            self.0.update(dest, reference)
        }
        fn list_tags(&self, url: &str) -> Result<Vec<String>, GitError> {
            self.0.list_tags(url)
        }
        fn fetch_file_at_ref(
            &self,
            url: &str,
            reference: &str,
            path: &str,
        ) -> Result<Vec<u8>, GitError> {
            self.0.fetch_file_at_ref(url, reference, path)
        }
    }
    let cache = tempdir().unwrap();
    let fake = Arc::new(FakeBackend::default());
    fake.seed_tags(
        "https://allowed/org/org.vibevm.wal.git",
        vec!["v1.0.0".into()],
    );
    let registry = GitPerPackageRegistry::open_with_deferred_auth(
        "internal",
        "ssh://excluded/org",
        "main",
        NamingConvention::Fqdn,
        vec!["https://allowed/org".into()],
        cache.path(),
        Arc::new(ExcludedPrimary(fake)),
        DEFAULT_FRESHNESS_SECS,
        vibe_core::manifest::AuthKind::None,
        None,
    )
    .unwrap();
    let versions = registry.list_versions(&org(), "wal").unwrap();
    assert_eq!(
        versions.len(),
        1,
        "explicit none cannot authenticate primary SSH, but independently admitted HTTPS is anonymous"
    );
    assert!(registry.deferred_token.as_ref().unwrap().get().is_none());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-auth"
)]
fn configured_route_shares_the_same_captured_token() {
    let cache = tempdir().unwrap();
    let registry = GitPerPackageRegistry::open_with_deferred_auth(
        "internal",
        "https://host/org",
        "main",
        NamingConvention::Fqdn,
        vec![],
        cache.path(),
        Arc::new(FakeBackend::default()),
        DEFAULT_FRESHNESS_SECS,
        vibe_core::manifest::AuthKind::TokenEnv,
        Some("UNCHANGED_NAME"),
    )
    .unwrap();
    let target = registry
        .configured_redirect(
            &org(),
            "wal",
            "https://host/org/org.vibevm.wal.git",
            "v1.0.0",
        )
        .unwrap();
    assert!(Arc::ptr_eq(
        registry.deferred_token.as_ref().unwrap(),
        target.deferred_token.as_ref().unwrap()
    ));
    registry
        .deferred_token
        .as_ref()
        .unwrap()
        .set(Some("synthetic-token".into()))
        .unwrap();
    assert_eq!(target.effective_token_value(), Some("synthetic-token"));
    assert_eq!(target.token_env_name.as_deref(), Some("UNCHANGED_NAME"));
}
