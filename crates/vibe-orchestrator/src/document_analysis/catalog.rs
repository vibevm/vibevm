//! Existing solver's provider interface over retained manifests only.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#DOCUMENT-ANALYSIS-CAPTURE-LAW");
use semver::Version;
use std::collections::BTreeMap;
use vibe_core::{Group, PackageRef, manifest::Manifest};
use vibe_resolver::{DepProvider, DepProviderError, VersionEnumerator};

impl super::AnalysisCapture {
    /// Project cached choices without invoking the preview solver or expanding
    /// defaults. Reads only the immutable resources already in this capture.
    pub fn configuration_discovery(
        &self,
    ) -> vibe_workspace::configuration_discovery::ConfigurationDiscovery {
        let mut result = vibe_workspace::configuration_discovery::discover(
            self.resources.values().filter_map(|resource| {
                if resource.declaration.role != super::ResourceRole::Manifest {
                    return None;
                }
                resource.raw().map(|bytes| (resource.path(), bytes))
            }),
        );
        for resource in self.resources.values() {
            if resource.declaration.role == super::ResourceRole::Manifest
                && resource.raw().is_none()
            {
                result
                    .unavailable
                    .push(format!("{}: absent from frozen capture", resource.path()));
            }
        }
        result
    }
}

/// Explicit owner admission joins a normalized B2 definition and the C1
/// metadata read to its exact retained source bytes. Paths never mint receipts.
#[derive(Debug, Clone)]
pub struct ConfigurationAdmission {
    pub resource: super::ResourceId,
    pub manifest_bytes: Vec<u8>,
    pub metadata: vibe_registry::CandidateMetadata,
    pub candidate: vibe_resolver::resolvo_engine::configuration::CandidateConfiguration,
}

/// One immutable query basis. Construction consumes admission; queries cannot
/// acquire authority, reopen disk, select defaults or prepare another capture.
#[derive(Debug)]
pub struct PreparedConfigurationDiscovery {
    capture: super::AnalysisCapture,
    view: vibe_workspace::configuration_discovery::ConfigurationDiscovery,
}
impl PreparedConfigurationDiscovery {
    pub fn admit(capture: super::AnalysisCapture, admissions: Vec<ConfigurationAdmission>) -> Self {
        let mut candidates = Vec::new();
        let mut unavailable = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for admission in &admissions {
            let checked = (|| -> Result<_, String> {
                if !seen.insert(admission.resource) {
                    return Err("duplicate resource admission".into());
                }
                let resource = capture
                    .resource(admission.resource)
                    .ok_or("foreign resource")?;
                if resource.declaration.role != super::ResourceRole::Manifest {
                    return Err("resource is not a manifest".into());
                }
                if resource.raw() != Some(admission.manifest_bytes.as_slice()) {
                    return Err("admitted source bytes differ from frozen resource".into());
                }
                let d = &admission.candidate;
                let metadata = &admission.metadata;
                if d.identity != metadata.identity
                    || d.identity.source.is_empty()
                    || d.identity.content.is_empty()
                    || d.identity.definition.is_empty()
                {
                    return Err("missing or substituted C1 receipt".into());
                }
                let parsed = Manifest::parse_str(
                    std::str::from_utf8(&admission.manifest_bytes).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                if serde_json::to_value(&parsed).map_err(|e| e.to_string())?
                    != serde_json::to_value(&metadata.manifest).map_err(|e| e.to_string())?
                {
                    return Err("metadata differs from admitted source bytes".into());
                }
                let package = parsed.require_package().map_err(|e| e.to_string())?;
                if d.package.group.as_ref() != Some(&package.group)
                    || d.package.name != package.name
                    || d.version != package.version
                    || !d.package.version.matches(&d.version)
                    || !matches!(&d.definition.instance.package,
                        vibe_core::manifest::feature_constraints::PackageIdentity::Vibe(p)
                        if p.group == d.package.group && p.name == d.package.name && p.version.matches(&d.version))
                {
                    return Err("candidate coordinate or configuration instance differs".into());
                }
                d.definition
                    .constraints
                    .order(&d.definition.program, &d.definition.declarations)
                    .map_err(|e| e.to_string())?;
                let definition = serde_json::from_str(
                    &vibe_install::configuration_definition(&d.definition)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                Ok(vibe_workspace::configuration_discovery::CachedDefinition {
                    resource: resource.path().into(),
                    manifest_digest: resource.content_digest().ok_or("missing frozen bytes")?,
                    receipt: serde_json::json!({"source": d.identity.source,
                        "content": d.identity.content, "definition": d.identity.definition}),
                    coordinate: d.package.qualified_name(),
                    version: d.version.to_string(),
                    definition,
                })
            })();
            match checked {
                Ok(candidate) => candidates.push(candidate),
                Err(e) => unavailable.push(e),
            }
        }
        // Duplicate bindings are ambiguous; never expose the first as admitted.
        if admissions.len() != seen.len() {
            candidates.clear();
        }
        for (id, resource) in capture.resources() {
            if resource.declaration.role == super::ResourceRole::Manifest && !seen.contains(&id) {
                match resource.raw() {
                    None => {
                        unavailable.push(format!("{}: absent from frozen capture", resource.path()))
                    }
                    Some(bytes) => match std::str::from_utf8(bytes)
                        .map_err(|e| e.to_string())
                        .and_then(|text| Manifest::parse_str(text).map_err(|e| e.to_string()))
                    {
                        Ok(manifest) if manifest.package.is_some() => unavailable.push(format!(
                            "{}: normalized configuration authority unavailable",
                            resource.path()
                        )),
                        Err(error) => unavailable.push(format!("{}: {error}", resource.path())),
                        _ => {}
                    },
                }
            }
        }
        let view = vibe_workspace::configuration_discovery::project_admitted(
            capture.id().into(),
            candidates,
            unavailable,
        );
        Self { capture, view }
    }
    pub fn capture_id(&self) -> &str {
        self.capture.id()
    }
    pub fn root(&self) -> &std::path::Path {
        &self.capture.root
    }
    pub fn query(&self) -> &vibe_workspace::configuration_discovery::ConfigurationDiscovery {
        &self.view
    }

    /// One presentation-only CLI exchange on a connection supplied by the
    /// trusted embedding host. The caller owns endpoint access and lifecycle;
    /// this method neither listens nor admits client-supplied definitions.
    /// The client ends its write half after a bounded JSON capture/root query.
    pub fn serve_cli_query(
        &self,
        connection: &mut (impl std::io::Read + std::io::Write),
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Query {
            capture: String,
            root: std::path::PathBuf,
        }
        let query: Query =
            serde_json::from_reader(std::io::Read::take(&mut *connection, 64 * 1024))?;
        if query.capture != self.capture_id() || query.root != self.root() {
            return Err("prepared discovery capture or root differs".into());
        }
        serde_json::to_writer(&mut *connection, self.query())?;
        std::io::Write::flush(connection)?;
        Ok(())
    }
}

/// Explicit read-only preparation, separate from querying the returned capture.
/// Paths name caller-prepared cached manifests relative to this admitted root;
/// no provider, cache hydration, candidate choice or write is available here.
pub fn prepare_configuration_discovery(
    root: &std::path::Path,
    cached_manifests: &[String],
) -> Result<super::AnalysisCapture, super::DocumentAnalysisError> {
    use super::{AnalysisLimits, DocumentAnalysisHost, ResourceDeclaration, ResourceRole};
    let mut paths = std::collections::BTreeSet::from(["vibe.toml".to_owned()]);
    paths.extend(cached_manifests.iter().cloned());
    let declarations = paths
        .into_iter()
        .map(|path| ResourceDeclaration {
            path,
            role: ResourceRole::Manifest,
        })
        .collect();
    let host = DocumentAnalysisHost::new(
        root,
        "vibe.toml",
        declarations,
        AnalysisLimits {
            resource_bytes: 8 * 1024 * 1024,
            capture_bytes: 64 * 1024 * 1024,
            open_buffers: 1,
        },
    )?;
    host.client()?.freeze()
}
#[derive(Debug, Clone, Default)]
pub(super) struct CapturedCatalog {
    manifests: BTreeMap<(Group, String, Version), Manifest>,
}
impl CapturedCatalog {
    pub fn insert(&mut self, manifest: Manifest) -> Result<(), String> {
        let Some(package) = manifest.package.as_ref() else {
            return Ok(());
        };
        let key = (
            package.group.clone(),
            package.name.clone(),
            package.version.clone(),
        );
        if self.manifests.insert(key, manifest).is_some() {
            return Err("ambiguous captured package version".into());
        }
        Ok(())
    }
}
impl DepProvider for CapturedCatalog {
    fn resolve_version(&self, reference: &PackageRef) -> Result<Version, DepProviderError> {
        let group = reference.group.as_ref().ok_or_else(|| {
            DepProviderError::Other("capture requires qualified package roots".into())
        })?;
        self.list_versions(group, reference.name.as_str())?
            .into_iter()
            .filter(|v| reference.version.matches(v))
            .max()
            .ok_or_else(|| DepProviderError::NoMatchingVersion {
                group: group.clone(),
                name: reference.name.to_string(),
                constraint: reference.version.to_string(),
            })
    }
    fn fetch_manifest(
        &self,
        group: &Group,
        name: &str,
        version: &Version,
    ) -> Result<Manifest, DepProviderError> {
        self.manifests
            .get(&(group.clone(), name.into(), version.clone()))
            .cloned()
            .ok_or_else(|| {
                DepProviderError::Other(
                    "manifest unavailable in immutable capture; no fetch is permitted".into(),
                )
            })
    }
}
impl VersionEnumerator for CapturedCatalog {
    fn list_versions(&self, group: &Group, name: &str) -> Result<Vec<Version>, DepProviderError> {
        let versions: Vec<_> = self
            .manifests
            .keys()
            .filter(|(g, n, _)| g == group && n == name)
            .map(|(_, _, v)| v.clone())
            .collect();
        if versions.is_empty() {
            Err(DepProviderError::UnknownPackage {
                group: group.clone(),
                name: name.into(),
            })
        } else {
            Ok(versions)
        }
    }
}

#[cfg(test)]
use crate::document_analysis as discovery_api;
#[cfg(test)]
#[path = "configuration_discovery_fixture.rs"]
mod discovery_fixture;
#[cfg(test)]
mod m_05_e_1_cases {
    use super::discovery_fixture::*;
    use super::*;
    #[test]
    fn captured_choices_effects_defaults_no_selection() {
        let (dir, capture, admission) = fixture();
        let expected = serde_json::from_str::<serde_json::Value>(
            &vibe_install::configuration_definition(&admission.candidate.definition).unwrap(),
        )
        .unwrap();
        let basis = PreparedConfigurationDiscovery::admit(capture, vec![admission.clone()]);
        let before = basis.query().clone();
        assert_eq!(before.status, "needs_selection");
        assert_eq!(before.candidates[0].definition, expected);
        assert_eq!(
            before.candidates[0].receipt["content"],
            admission.metadata.identity.content
        );
        assert_eq!(
            expected["constraints"]["groups"]["mode"]["defaults"][0],
            "red"
        );
        assert_eq!(
            expected["constraints"]["groups"]["detail"]["guards"][0]["member"],
            "blue"
        );
        assert_eq!(
            expected["program"]["features"]["red"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
        std::fs::write(dir.path().join("cache/vibe.toml"), "changed after freeze").unwrap();
        assert_eq!(basis.query(), &before);
    }
    #[test]
    fn empty_capture_unavailable_no_provider() {
        let (dir, _, _) = fixture();
        let capture = prepare_configuration_discovery(dir.path(), &[]).unwrap();
        let basis = PreparedConfigurationDiscovery::admit(capture, vec![]);
        assert_eq!(basis.query().status, "unavailable");
        assert_eq!(basis.query().counters.provider_calls, 0);
    }
    #[test]
    fn repeated_unresolved_is_stable_needs_selection() {
        let (_, capture, admission) = fixture();
        let basis = PreparedConfigurationDiscovery::admit(capture, vec![admission]);
        assert_eq!(basis.query().status, "needs_selection");
        assert_eq!(basis.query(), basis.query());
        assert_eq!(basis.query().counters.solves, 0);
    }
    #[cfg(unix)]
    #[test]
    fn cli_host_rejects_foreign_capture_root_and_dto_authority() {
        use std::io::{Read, Write};
        let (_, capture, admission) = fixture();
        let basis = PreparedConfigurationDiscovery::admit(capture, vec![admission]);
        for request in [
            serde_json::json!({"capture":"foreign", "root":basis.root()}),
            serde_json::json!({"capture":basis.capture_id(), "root":"/foreign"}),
            serde_json::json!({"capture":basis.capture_id(), "root":basis.root(), "admitted":true}),
        ] {
            let (mut client, mut host) = std::os::unix::net::UnixStream::pair().unwrap();
            serde_json::to_writer(&mut client, &request).unwrap();
            client.flush().unwrap();
            client.shutdown(std::net::Shutdown::Write).unwrap();
            assert!(basis.serve_cli_query(&mut host).is_err());
            drop(host);
            let mut reply = Vec::new();
            if let Err(error) = client.read_to_end(&mut reply) {
                // Rejecting unknown fields may leave unread request bytes;
                // closing that socket reports reset rather than clean EOF.
                assert_eq!(error.kind(), std::io::ErrorKind::ConnectionReset);
            }
            assert!(reply.is_empty());
        }
    }
    #[test]
    fn no_cache_or_authored_mutations() {
        let (dir, capture, admission) = fixture();
        let read = || {
            ["vibe.toml", "vibe.lock", "cache/vibe.toml"]
                .map(|p| std::fs::read(dir.path().join(p)).unwrap())
        };
        let before = read();
        let basis = PreparedConfigurationDiscovery::admit(capture, vec![admission]);
        for _ in 0..2 {
            assert_eq!(basis.query().counters.authored_writes, 0);
        }
        assert_eq!(read(), before);
        assert_eq!(
            std::fs::read_dir(dir.path().join("cache")).unwrap().count(),
            1
        );
    }
    #[test]
    fn missing_or_substituted_authority_is_unavailable() {
        let (_, capture, mut admission) = fixture();
        admission
            .candidate
            .identity
            .content
            .push_str("-substituted");
        assert_eq!(
            PreparedConfigurationDiscovery::admit(capture, vec![admission])
                .query()
                .status,
            "unavailable"
        );
        let (_, capture, mut admission) = fixture();
        admission.manifest_bytes.push(b' ');
        assert_eq!(
            PreparedConfigurationDiscovery::admit(capture, vec![admission])
                .query()
                .status,
            "unavailable"
        );
        let (_, capture, _) = fixture();
        assert_eq!(
            PreparedConfigurationDiscovery::admit(capture, vec![])
                .query()
                .status,
            "unavailable"
        );
    }
    #[test]
    fn same_version_definition_changes_discovery() {
        let (_, capture, admission) = fixture();
        let first = PreparedConfigurationDiscovery::admit(capture, vec![admission]);
        let (_, capture, mut admission) = fixture();
        admission
            .candidate
            .definition
            .constraints
            .groups
            .get_mut(&local("mode"))
            .unwrap()
            .defaults = Some([local("blue")].into());
        let second = PreparedConfigurationDiscovery::admit(capture, vec![admission]);
        assert_eq!(
            first.query().candidates[0].version,
            second.query().candidates[0].version
        );
        assert_ne!(
            first.query().candidates[0].definition,
            second.query().candidates[0].definition
        );
        let (dir, _, mut admission) = fixture();
        let path = dir.path().join("cache/vibe.toml");
        let mut bytes = std::fs::read(&path).unwrap();
        bytes.extend_from_slice(b"# changed same-version source\n");
        std::fs::write(&path, &bytes).unwrap();
        let metadata = vibe_registry::CandidateMetadata::read_local(
            &dir.path().join("cache"),
            "fixture:admitted-source".into(),
        )
        .unwrap();
        let capture =
            prepare_configuration_discovery(dir.path(), &["cache/vibe.toml".into()]).unwrap();
        admission.resource = capture
            .resources()
            .find(|(_, r)| r.path() == "cache/vibe.toml")
            .unwrap()
            .0;
        admission.manifest_bytes = bytes;
        admission.candidate.identity = metadata.identity.clone();
        admission.metadata = metadata;
        let changed = PreparedConfigurationDiscovery::admit(capture, vec![admission]);
        assert_eq!(
            first.query().candidates[0].version,
            changed.query().candidates[0].version
        );
        assert_ne!(
            first.query().candidates[0].receipt,
            changed.query().candidates[0].receipt
        );
        assert_ne!(
            first.query().candidates[0].manifest_digest,
            changed.query().candidates[0].manifest_digest
        );
    }
}
