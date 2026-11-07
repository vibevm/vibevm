//! Shared projection of observed acquisition facts; resolver identity is unchanged.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-EVIDENCE");
use crate::CachedPackage;
use vibe_core::manifest::SourceKind;
use vibe_core::progress::transfer::safe_remote;
use vibe_wire::generated::shared::{SourceEvidence, SourceSelectionKind};

/// Report-only hosting labels derived independently from the two held URL facts.
/// Missing evidence and unknown providers stay absent; store reads do not invent
/// a differing acquisition route. This is URL spelling, not a contact record.
///
/// ```
/// let evidence = serde_json::from_str(r#"{"source_url":"https://github.com/a","fetched_from":"git@gitverse.ru:a"}"#).unwrap();
/// let view = vibe_registry::source_hosting(Some(&evidence)).unwrap();
/// assert_eq!(view.selected_origin.as_deref(), Some("github"));
/// assert_eq!(view.differing_acquisition_route.as_deref(), Some("gitverse"));
/// assert!(vibe_registry::source_hosting(None).is_none());
/// ```
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#HOSTING-OBSERVATION")]
pub fn source_hosting(
    evidence: Option<&SourceEvidence>,
) -> Option<vibe_wire::generated::shared::HostingView> {
    let evidence = evidence?;
    let selected_origin = hosting_label(evidence.source_url.as_deref());
    let differing_acquisition_route = hosting_label(evidence.fetched_from.as_deref());
    (selected_origin.is_some() || differing_acquisition_route.is_some()).then_some(
        vibe_wire::generated::shared::HostingView {
            selected_origin,
            differing_acquisition_route,
        },
    )
}

fn hosting_label(url: Option<&str>) -> Option<String> {
    use vibe_core::remap::endpoint::{EndpointObservation, observe_endpoint};
    match observe_endpoint(url?) {
        EndpointObservation::Network(authority) => authority.hosting.map(str::to_owned),
        _ => None,
    }
}

fn selection(kind: SourceKind) -> SourceSelectionKind {
    match kind {
        SourceKind::Registry => SourceSelectionKind::Registry,
        SourceKind::Git => SourceSelectionKind::Git,
        SourceKind::Override => SourceSelectionKind::Override,
        SourceKind::Path => SourceSelectionKind::Path,
        SourceKind::Embedded => SourceSelectionKind::Embedded,
        SourceKind::Local => SourceSelectionKind::Local,
    }
}

/// Display the facts already retained by a project lock; absent kind stays unknown.
///
/// ```
/// let row: vibe_core::manifest::LockedPackage = toml::from_str(
///     "kind='tool'\nname='demo'\ngroup='org.example'\nversion='1.0.0'\nsource_url='https://u:secret@example.test/demo'\ncontent_hash='sha256:abc'\n"
/// ).unwrap();
/// let evidence = vibe_registry::locked_source_evidence(&row);
/// assert_eq!(evidence.source_url.as_deref(), Some("https://example.test/demo"));
/// assert_eq!(evidence.source_kind, None);
/// ```
pub fn locked_source_evidence(record: &vibe_core::manifest::LockedPackage) -> SourceEvidence {
    SourceEvidence {
        registry: record.registry.clone(),
        source_url: Some(safe_remote(record.source_url.as_str())),
        source_ref: record.source_ref.clone(),
        resolved_commit: record.resolved_commit.clone(),
        source_kind: record.source_kind.map(selection),
        via_redirect: record.via_redirect.as_deref().map(safe_remote),
        fetched_from: None,
        acquisition: None,
        store_reused: None,
    }
}

impl CachedPackage {
    /// The existing lock selection vocabulary, independent of transport.
    ///
    /// ```
    /// let temp = tempfile::tempdir().unwrap();
    /// let source = temp.path().join("registry/org.example/demo/v1.0.0");
    /// std::fs::create_dir_all(&source).unwrap();
    /// std::fs::write(source.join("vibe.toml"), "[package]\ngroup='org.example'\nname='demo'\nversion='1.0.0'\nkind='tool'\n").unwrap();
    /// let registry = vibe_registry::LocalRegistry::new(temp.path().join("registry")).unwrap();
    /// let selected = registry.resolve(&vibe_core::PackageRef::parse("org.example/demo").unwrap()).unwrap();
    /// let cached = registry.fetch(&selected, &temp.path().join("store")).unwrap();
    /// let kind = cached.source_kind();
    /// assert_eq!(kind, Some(vibe_core::manifest::SourceKind::Registry));
    /// ```
    pub fn source_kind(&self) -> Option<SourceKind> {
        if self.acquisition == Some(vibe_wire::generated::shared::SourceAcquisition::Store) {
            return self.recorded_source_kind;
        }
        Some(if self.overridden {
            SourceKind::Override
        } else if self.is_path_source {
            SourceKind::Path
        } else if self.is_git_source {
            SourceKind::Git
        } else if self.is_local {
            SourceKind::Local
        } else if self.is_embedded {
            SourceKind::Embedded
        } else {
            SourceKind::Registry
        })
    }

    /// Credential-redacted observations for plans and installed history.
    /// Canonical resolver identifiers themselves remain unchanged.
    ///
    /// ```
    /// let temp = tempfile::tempdir().unwrap();
    /// let source = temp.path().join("registry/org.example/demo/v1.0.0");
    /// std::fs::create_dir_all(&source).unwrap();
    /// std::fs::write(source.join("vibe.toml"), "[package]\ngroup='org.example'\nname='demo'\nversion='1.0.0'\nkind='tool'\n").unwrap();
    /// let registry = vibe_registry::LocalRegistry::new(temp.path().join("registry")).unwrap();
    /// let selected = registry.resolve(&vibe_core::PackageRef::parse("org.example/demo").unwrap()).unwrap();
    /// let cached = registry.fetch(&selected, &temp.path().join("store")).unwrap();
    /// let evidence = cached.source_evidence();
    /// assert_eq!(evidence.resolved_commit, cached.resolved_commit);
    /// assert_eq!(evidence.source_url, Some(vibe_core::progress::transfer::safe_remote(&cached.source_uri)));
    /// ```
    pub fn source_evidence(&self) -> SourceEvidence {
        SourceEvidence {
            registry: self.registry_name.clone(),
            source_url: Some(safe_remote(&self.source_uri)),
            source_ref: self.source_ref.clone(),
            resolved_commit: self.resolved_commit.clone(),
            source_kind: self.source_kind().map(selection),
            via_redirect: self.via_redirect.as_deref().map(safe_remote),
            fetched_from: self.fetched_from.as_deref().map(safe_remote),
            store_reused: self.store_reused,
            acquisition: self.acquisition.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn acquisition_keeps_canonical_origin_redacts_routes_and_observes_store_reuse() {
        let root = tempfile::tempdir().unwrap();
        let package = root.path().join("registry/org.example/demo/v1.0.0");
        std::fs::create_dir_all(&package).unwrap();
        std::fs::write(
            package.join("vibe.toml"),
            "[package]\ngroup='org.example'\nname='demo'\nversion='1.0.0'\nkind='tool'\n",
        )
        .unwrap();
        let registry = crate::LocalRegistry::new(root.path().join("registry")).unwrap();
        let resolved = registry
            .resolve(&vibe_core::PackageRef::parse("org.example/demo").unwrap())
            .unwrap();
        let store = root.path().join("store");
        let mut first = registry.fetch(&resolved, &store).unwrap();
        assert_eq!(first.source_evidence().store_reused, Some(false));
        assert_eq!(
            registry
                .fetch(&resolved, &store)
                .unwrap()
                .source_evidence()
                .store_reused,
            Some(true)
        );
        first.source_uri =
            "https://user:SYN_SECRET@canonical.invalid/demo?token=SYN_QUERY#private".into();
        first.fetched_from = Some("https://mirror.invalid/demo?token=SYN_SECRET".into());
        first.via_redirect = Some("git@stub.invalid:org/demo.git".into());
        first.resolved_commit = Some("observed-commit".into());
        let evidence = first.source_evidence();
        assert_eq!(
            evidence.source_url.as_deref(),
            Some("https://canonical.invalid/demo")
        );
        assert_eq!(
            evidence.fetched_from.as_deref(),
            Some("https://mirror.invalid/demo")
        );
        assert_eq!(
            evidence.via_redirect.as_deref(),
            Some("stub.invalid:org/demo.git")
        );
        assert_eq!(evidence.resolved_commit.as_deref(), Some("observed-commit"));
        assert!(!serde_json::to_string(&evidence).unwrap().contains("SYN_"));
        assert!(
            first.source_uri.contains("SYN_SECRET"),
            "display projection must not rewrite resolver identity"
        );
        first.acquisition = Some(vibe_wire::generated::shared::SourceAcquisition::Store);
        first.resolved_commit = None;
        assert_eq!(first.source_evidence().source_kind, None);
        assert_eq!(first.source_evidence().resolved_commit, None);
        first.recorded_source_kind = Some(SourceKind::Override);
        assert_eq!(
            first.source_evidence().source_kind,
            Some(SourceSelectionKind::Override)
        );
    }
}

#[cfg(test)]
mod hosting_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn hosting_uses_only_independent_held_urls_and_does_not_mutate_evidence() {
        let facts = json!({"source_url":"https://github.com/a", "fetched_from":"https://gitverse.ru/a", "via_redirect":"https://elsewhere.test/stub", "acquisition":"git_repository", "store_reused":false});
        let evidence = serde_json::from_value(facts.clone()).unwrap();
        let view = source_hosting(Some(&evidence)).unwrap();
        assert_eq!(view.selected_origin.as_deref(), Some("github"));
        assert_eq!(
            view.differing_acquisition_route.as_deref(),
            Some("gitverse")
        );
        assert_eq!(serde_json::to_value(&evidence).unwrap(), facts);
        for facts in [
            json!({}),
            json!({"source_url":"/local/repo"}),
            json!({"source_url":"https://unknown.test/a", "via_redirect":"https://github.com/stub"}),
        ] {
            assert!(source_hosting(Some(&serde_json::from_value(facts).unwrap())).is_none());
        }
        assert!(source_hosting(None).is_none());
        let stored = serde_json::from_value(json!({"source_url":"https://github.com/a", "acquisition":"store", "store_reused":true})).unwrap();
        let view = source_hosting(Some(&stored)).unwrap();
        assert_eq!(view.selected_origin.as_deref(), Some("github"));
        assert_eq!(view.differing_acquisition_route, None);
        let route_only = serde_json::from_value(
            json!({"source_url":"/local", "fetched_from":"git@gitverse.ru:a"}),
        )
        .unwrap();
        let view = source_hosting(Some(&route_only)).unwrap();
        assert_eq!(view.selected_origin, None);
        assert_eq!(
            view.differing_acquisition_route.as_deref(),
            Some("gitverse")
        );
    }
}
