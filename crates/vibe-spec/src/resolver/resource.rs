//! Selected-world canonicalization for strict Vibe and named export values.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-070#SELECTED-RESOURCE-AUTHORITY");

use std::path::Path;

use vibe_core::manifest::PackageMeta;

use crate::address::resource::{validate_authority, validate_path, validate_version};
use crate::address::{
    Authority, ResourceAddress, ResourceAddressError, SpecAddress, SpecAddressError,
};

use super::{FileResolver, ResolveError, lookup, portable};

impl FileResolver {
    /// Construct a classified Vibe resource from strict spelling and an exact
    /// selected world. This returns an identity, not source bytes or a grant.
    pub fn canonical_vibe_resource(
        &self,
        raw: &str,
    ) -> Result<ResourceAddress, ResourceAddressError> {
        if !raw.starts_with("spec://") {
            return Err(ResourceAddressError::InvalidScheme);
        }
        if raw.contains('%') {
            return Err(ResourceAddressError::PercentEncoding);
        }
        let parsed = SpecAddress::parse(raw).map_err(map_syntax)?;
        self.canonical_vibe(&parsed, true, false)
    }

    /// The only compatibility bridge from incumbent syntactic SpecAddress.
    /// Its old parser may carry a `=version` spelling, but no host guess or
    /// revision pin is silently dropped. Legacy source classification is
    /// preserved for this explicitly named internal conversion only.
    pub fn convert_legacy_vibe_address(
        &self,
        address: &SpecAddress,
    ) -> Result<ResourceAddress, ResourceAddressError> {
        self.canonical_vibe(address, false, true)
    }

    fn canonical_vibe(
        &self,
        address: &SpecAddress,
        require_descriptor: bool,
        legacy_equals: bool,
    ) -> Result<ResourceAddress, ResourceAddressError> {
        if address.raw.contains('%') {
            return Err(ResourceAddressError::PercentEncoding);
        }
        if address.pinned_r.is_some() {
            return Err(ResourceAddressError::UnsupportedRevision);
        }
        validate_path(&address.doc_path)?;
        if address.doc_path.ends_with(".md") || address.doc_path.ends_with(".xml") {
            return Err(ResourceAddressError::PhysicalSerialization);
        }
        let (group, name, selected_version) =
            self.selected_authority(&address.authority, legacy_equals)?;
        let base = self.spec_root(&address.authority)?;
        if require_descriptor
            && !self
                .descriptor_maps
                .get(&base)
                .is_some_and(|map| map.is_some())
        {
            return Err(ResourceAddressError::VibeSourceRequired);
        }
        let path = match self.resolve_source_file(address, &[]) {
            Ok(file) => file.path,
            Err(ResolveError::DocNotFound { .. }) if legacy_equals => {
                let Some(Some(map)) = self.descriptor_maps.get(&base) else {
                    return Err(ResourceAddressError::VibeSourceRequired);
                };
                let found = lookup::resolve_doc_with_formats(&base, &address.doc_path, &[])?;
                if !map.owns_vibe_output(&found.path) {
                    return Err(ResourceAddressError::VibeSourceRequired);
                }
                found.path
            }
            Err(error) => return Err(error.into()),
        };
        if !portable::checked_file(&base, &path)? {
            return Err(ResolveError::DocNotFound {
                doc_path: address.doc_path.clone(),
                base: base.display().to_string(),
            }
            .into());
        }
        let relative = path
            .strip_prefix(&base)
            .map_err(|_| ResourceAddressError::VibeSourceRequired)?;
        let logical = lookup::canonical_doc_path(&slash(relative));
        ResourceAddress::vibe(
            &group,
            &name,
            selected_version.as_deref(),
            &logical,
            &address.anchor,
        )
    }

    /// Construct a named CommonMark export identity from an exact validated
    /// export row. Heading/byte resolution stays with the source owner.
    pub fn canonical_export_resource(
        &self,
        raw: &str,
        package_meta: &PackageMeta,
    ) -> Result<ResourceAddress, ResourceAddressError> {
        let body = raw
            .strip_prefix("resource://")
            .ok_or(ResourceAddressError::InvalidScheme)?;
        if body.contains('%') {
            return Err(ResourceAddressError::PercentEncoding);
        }
        let parts = body.split('/').collect::<Vec<_>>();
        if parts.len() != 3 {
            return Err(ResourceAddressError::EmptyComponent);
        }
        let (group, package, export_name) = (parts[0], parts[1], parts[2]);
        if group.is_empty() || package.is_empty() || export_name.is_empty() {
            return Err(ResourceAddressError::EmptyComponent);
        }
        let (name, version) = match package.split_once('@') {
            Some((name, version)) => (name, Some(version)),
            None => (package, None),
        };
        validate_authority(group, name)?;
        validate_path(export_name)?;
        if self.is_self(group, name) {
            return Err(ResourceAddressError::InvalidAuthority);
        }
        let selected = self
            .selected
            .as_ref()
            .ok_or(ResourceAddressError::SelectedWorldRequired)?
            .get(&(group.to_owned(), name.to_owned()))
            .ok_or_else(|| ResourceAddressError::UnselectedPackage(format!("{group}/{name}")))?;
        validate_version(&selected.version)?;
        if package_meta.group.as_str() != group
            || package_meta.name != name
            || package_meta.version.to_string() != selected.version
        {
            return Err(ResourceAddressError::ExportPackageMismatch);
        }
        if !package_meta
            .source_exports
            .iter()
            .any(|export| export.name() == export_name)
        {
            return Err(ResourceAddressError::UndeclaredExport);
        }
        if let Some(requested) = version {
            validate_version(requested)?;
            if requested != selected.version {
                return Err(ResourceAddressError::WrongSelectedVersion {
                    requested: requested.to_owned(),
                    selected: selected.version.clone(),
                });
            }
        }
        ResourceAddress::export(group, name, &selected.version, export_name)
    }

    fn selected_authority(
        &self,
        authority: &Authority,
        legacy_equals: bool,
    ) -> Result<(String, String, Option<String>), ResourceAddressError> {
        let Authority::Package {
            group,
            name,
            version,
        } = authority
        else {
            return Err(ResourceAddressError::LegacyHostAuthority);
        };
        validate_authority(group, name)?;
        if self.is_self(group, name) {
            if version.is_some() {
                return Err(ResourceAddressError::InvalidVersion);
            }
            return Ok((group.clone(), name.clone(), None));
        }
        let selected = self
            .selected
            .as_ref()
            .ok_or(ResourceAddressError::SelectedWorldRequired)?
            .get(&(group.clone(), name.clone()))
            .ok_or_else(|| ResourceAddressError::UnselectedPackage(format!("{group}/{name}")))?;
        validate_version(&selected.version)?;
        if let Some(requested) = version.as_deref() {
            let requested = if legacy_equals {
                requested.strip_prefix('=').unwrap_or(requested)
            } else {
                requested
            };
            validate_version(requested)?;
            if requested != selected.version {
                return Err(ResourceAddressError::WrongSelectedVersion {
                    requested: requested.to_owned(),
                    selected: selected.version.clone(),
                });
            }
        }
        Ok((group.clone(), name.clone(), Some(selected.version.clone())))
    }
}

fn map_syntax(error: SpecAddressError) -> ResourceAddressError {
    match error {
        SpecAddressError::MissingScheme => ResourceAddressError::InvalidScheme,
        SpecAddressError::EmptyAuthority
        | SpecAddressError::EmptyName
        | SpecAddressError::MissingDocPath
        | SpecAddressError::EmptyPathSegment
        | SpecAddressError::EmptyAnchor => ResourceAddressError::EmptyComponent,
        SpecAddressError::MissingName => ResourceAddressError::InvalidAuthority,
        SpecAddressError::ContainsWhitespace
        | SpecAddressError::InvalidAnchorSegment(_)
        | SpecAddressError::InvalidRevision(_) => ResourceAddressError::InvalidCharacter,
    }
}

fn slash(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
