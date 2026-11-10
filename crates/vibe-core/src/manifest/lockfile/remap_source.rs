//! Complete remap source authority is separate from report-only provenance.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE");
use super::*;

/// Canonical source facts for a held remap, never credentials or acquisition scratch.
/// ```
/// let source: vibe_core::manifest::LockedRemapSource = toml::from_str(
///     "source_url='https://example.invalid/remap.git'\nsource_ref='v1.0.0'\nsource_kind='registry'"
/// ).unwrap();
/// assert_eq!(source.source_ref.as_deref(), Some("v1.0.0"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockedRemapSource {
    pub source_url: SourceUrl,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_commit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_kind: Option<SourceKind>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub overridden: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub via_redirect: Option<String>,
}

/// Structured replay refusal shared by installation and read-only preview.
/// ```
/// let error = vibe_core::manifest::LockedRemapSourceError::MissingSource { package: "org.example/policy".into() };
/// assert!(error.to_string().contains("missing-locked-source"));
/// ```
#[derive(Debug, thiserror::Error)]
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW")]
pub enum LockedRemapSourceError {
    #[error(
        "missing-locked-source: remap `{package}` has no authoritative source binding (violates spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW; fix: explicitly supply its original source binding; current registries and home offers cannot recover it)"
    )]
    MissingSource { package: String },
    #[error(
        "invalid-locked-source: remap `{package}`: {reason} (violates spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW; fix: correct the explicit remap source binding)"
    )]
    InvalidSource {
        package: String,
        reason: &'static str,
    },
}
impl LockedRemap {
    /// Convert the authoritative remap facts into the existing held-source owner.
    /// ```
    /// let row: vibe_core::manifest::LockedRemap = toml::from_str("package='org.example/policy'\nversion='1.0.0'\ncontent_hash='sha256:ab'").unwrap();
    /// assert!(matches!(row.locked_package(), Err(vibe_core::manifest::LockedRemapSourceError::MissingSource { .. })));
    /// ```
    pub fn locked_package(&self) -> std::result::Result<LockedPackage, LockedRemapSourceError> {
        let invalid = |reason| LockedRemapSourceError::InvalidSource {
            package: self.package.clone(),
            reason,
        };
        let source = self
            .source
            .as_ref()
            .ok_or_else(|| LockedRemapSourceError::MissingSource {
                package: self.package.clone(),
            })?;
        let pkg = PackageRef::parse(&self.package).map_err(|_| invalid("invalid coordinate"))?;
        if pkg.qualified_name() != self.package {
            return Err(invalid("coordinate must be an unversioned group/name"));
        }
        let group = pkg
            .group
            .ok_or_else(|| invalid("coordinate has no group"))?;
        let version =
            semver::Version::parse(&self.version).map_err(|_| invalid("invalid version"))?;
        let content_hash =
            ContentHash::parse(&self.content_hash).map_err(|_| invalid("invalid content hash"))?;
        let address = source.source_url.as_str();
        if address.is_empty() {
            return Err(invalid("empty canonical source"));
        }
        for plain in std::iter::once(address).chain(source.via_redirect.as_deref()) {
            let transport = plain.strip_prefix("git+").unwrap_or(plain);
            match url::Url::parse(transport) {
                Ok(url)
                    if url.password().is_some()
                        || (url.scheme() != "ssh" && !url.username().is_empty()) =>
                {
                    return Err(invalid(
                        "credentials cannot be persisted in source metadata",
                    ));
                }
                Err(_) if transport.contains("://") => {
                    return Err(invalid("malformed authority-bearing source URL"));
                }
                _ => {}
            }
        }
        let directory = source.source_kind == Some(SourceKind::Path)
            || (address.starts_with("file:") && source.source_ref.is_none());
        if !directory && source.source_ref.is_none() && source.resolved_commit.is_none() {
            return Err(invalid("Git source needs a held ref or commit"));
        }
        Ok(LockedPackage {
            kind: PackageKind::Remap,
            name: pkg.name,
            group,
            version,
            bridge: false,
            authors: vec![],
            registry: self.registry.clone(),
            source_url: source.source_url.clone(),
            source_ref: source.source_ref.clone(),
            resolved_commit: source.resolved_commit.clone(),
            content_hash,
            embedded_sources: vec![],
            boot_snippet: None,
            files_written: vec![],
            dependencies: vec![],
            required_by: vec![],
            admitted_by: None,
            via_override: None,
            overridden: source.overridden,
            source_kind: source.source_kind,
            via_redirect: source.via_redirect.clone(),
            features: vec![],
            subskills_active: vec![],
            describes: None,
            language: None,
            materialization: Materialization::default(),
            template: None,
            via_channel: None,
        })
    }
    /// Explicit caller input repairs missing source facts; acquisition must still
    /// verify the held content hash before publishing this candidate.
    /// ```
    /// let row: vibe_core::manifest::LockedRemap = toml::from_str("package='org.example/policy'\nversion='1.0.0'\ncontent_hash='sha256:ab'").unwrap();
    /// let source = toml::from_str("source_url='file:///explicit/policy'").unwrap();
    /// assert!(row.bind_source(source).unwrap().source.is_some());
    /// ```
    pub fn bind_source(
        mut self,
        source: LockedRemapSource,
    ) -> std::result::Result<Self, LockedRemapSourceError> {
        self.source = Some(source);
        self.locked_package()?;
        Ok(self)
    }
}
impl Lockfile {
    pub(crate) fn validate_remap_sources(&self) -> Result<()> {
        for row in &self.remaps {
            if row.source.is_none() {
                continue;
            }
            if self.meta.schema_version < REMAP_SOURCE_SCHEMA_VERSION {
                return Err(crate::Error::InvalidLockfile {
                    reason: "remap source bindings require schema9 (reader/writer 1.0.12)".into(),
                });
            }
            row.locked_package()
                .map_err(|error| crate::Error::InvalidLockfile {
                    reason: error.to_string(),
                })?;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests;
