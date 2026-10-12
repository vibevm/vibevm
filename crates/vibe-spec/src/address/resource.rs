//! Strict selected-resource URI values; legacy `SpecAddress` stays separate.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-070#RESOURCE-SCHEME-LAW");

use std::fmt;

use vibe_core::{Group, PackageName, VersionSpec};

use crate::resolver::ResolveError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceAddressKind {
    VibeDocument,
    SourceExport,
}

/// A canonical logical address made only after syntax and selected authority
/// checks. It is an identity, never a file handle or captured source witness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceAddress {
    uri: String,
    kind: ResourceAddressKind,
}

impl ResourceAddress {
    pub fn as_str(&self) -> &str {
        &self.uri
    }

    pub fn kind(&self) -> ResourceAddressKind {
        self.kind
    }

    pub(crate) fn vibe(
        group: &str,
        name: &str,
        version: Option<&str>,
        doc_path: &str,
        anchor: &[String],
    ) -> Result<Self, ResourceAddressError> {
        validate_authority(group, name)?;
        validate_path(doc_path)?;
        for segment in anchor {
            if segment.is_empty() {
                return Err(ResourceAddressError::EmptyComponent);
            }
            if !super::is_valid_anchor_segment(segment) {
                return Err(ResourceAddressError::InvalidCharacter);
            }
        }
        if doc_path.ends_with(".md") || doc_path.ends_with(".xml") {
            return Err(ResourceAddressError::PhysicalSerialization);
        }
        if let Some(version) = version {
            validate_version(version)?;
        }
        let mut uri = format!("spec://{group}/{name}");
        if let Some(version) = version {
            uri.push('@');
            uri.push_str(version);
        }
        uri.push('/');
        uri.push_str(doc_path);
        if !anchor.is_empty() {
            uri.push('#');
            uri.push_str(&anchor.join("."));
        }
        Ok(Self {
            uri,
            kind: ResourceAddressKind::VibeDocument,
        })
    }

    pub(crate) fn export(
        group: &str,
        name: &str,
        version: &str,
        export_name: &str,
    ) -> Result<Self, ResourceAddressError> {
        validate_authority(group, name)?;
        validate_version(version)?;
        validate_path(export_name)?;
        if export_name.contains('/') {
            return Err(ResourceAddressError::InvalidExportName);
        }
        Ok(Self {
            uri: format!("resource://{group}/{name}@{version}/{export_name}"),
            kind: ResourceAddressKind::SourceExport,
        })
    }
}

impl fmt::Display for ResourceAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.uri)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ResourceAddressError {
    #[error("resource URI has an unsupported scheme")]
    InvalidScheme,
    #[error("percent encoding is not admitted in a resource URI")]
    PercentEncoding,
    #[error("resource URI contains an empty or dot path component")]
    EmptyComponent,
    #[error("resource URI has an invalid qualified group/name authority")]
    InvalidAuthority,
    #[error("resource URI has an invalid exact version")]
    InvalidVersion,
    #[error("Vibe resource path must not contain a physical .md or .xml serialization suffix")]
    PhysicalSerialization,
    #[error("resource URI pins `{requested}`, but selection holds `{selected}`")]
    WrongSelectedVersion { requested: String, selected: String },
    #[error("a selected package world is required for dependency resources")]
    SelectedWorldRequired,
    #[error("package `{0}` is absent from the selected world")]
    UnselectedPackage(String),
    #[error("a legacy undotted host authority cannot be inferred")]
    LegacyHostAuthority,
    #[error("revision pins are unsupported for resource reads")]
    UnsupportedRevision,
    #[error("resource URI contains an invalid character or anchor")]
    InvalidCharacter,
    #[error("source export name is invalid")]
    InvalidExportName,
    #[error("source export is not the selected validated declaration")]
    UndeclaredExport,
    #[error("source export manifest does not match the selected package identity")]
    ExportPackageMismatch,
    #[error("a selected Vibe source descriptor is required")]
    VibeSourceRequired,
    #[error(transparent)]
    Lookup(#[from] ResolveError),
}

pub(crate) fn validate_authority(group: &str, name: &str) -> Result<(), ResourceAddressError> {
    Group::parse(group).map_err(|_| ResourceAddressError::InvalidAuthority)?;
    PackageName::parse(name).map_err(|_| ResourceAddressError::InvalidAuthority)?;
    Ok(())
}

pub(crate) fn validate_version(version: &str) -> Result<(), ResourceAddressError> {
    if version.trim() != version
        || !VersionSpec::parse(&format!("={version}")).is_ok_and(|pin| pin.is_exact_pin())
    {
        return Err(ResourceAddressError::InvalidVersion);
    }
    Ok(())
}

pub(crate) fn validate_path(path: &str) -> Result<(), ResourceAddressError> {
    if path.contains('%') {
        return Err(ResourceAddressError::PercentEncoding);
    }
    if path.is_empty()
        || path
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
    {
        return Err(ResourceAddressError::EmptyComponent);
    }
    if path.chars().any(|ch| {
        ch.is_control() || ch.is_whitespace() || matches!(ch, '\\' | '?' | '#' | '@' | '~')
    }) {
        return Err(ResourceAddressError::InvalidCharacter);
    }
    Ok(())
}
