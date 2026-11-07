//! Builtin and admitted custom input consume the same held source buffer.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#URI-OBSERVED-SOURCE-READ"
);
use super::*;

/// Resolve native captured bytes without granting any filesystem or source
/// authority. The caller owns the metadata/byte observation and read role.
pub fn resolve_captured_declared_source(
    address: &SpecAddress,
    metadata: ResolvedSourceMetadata,
    bytes: &[u8],
    media: &str,
    interpretation: vibe_core::manifest::SourceInterpretation,
) -> Result<ResolvedSource, String> {
    if metadata.format() != "markdown" {
        return Err("violates REQ spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-INSTALLED-READ-LAW: captured native projection requires builtin markdown metadata; fix surface: root source metadata format".into());
    }
    let projected = crate::project_declared_native(bytes, media, interpretation)?;
    resolve_section(&projected, metadata.origin(), address)
        .map(|text| ResolvedSource::markdown(text, metadata.physical_stem))
}

impl FsSectionSource {
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#URI-OBSERVED-SOURCE-READ"
    )]
    pub(super) fn read_resolved(
        &self,
        address: &SpecAddress,
        metadata: ResolvedSourceMetadata,
    ) -> Result<ResolvedSource, String> {
        let formats = if metadata.active_formats.is_empty() && metadata.format() != "markdown" {
            vec![metadata.format().to_owned()]
        } else {
            metadata.active_formats.clone()
        };
        let location = self
            .resolver
            .locate_source(address, &formats)
            .map_err(|error| error.to_string())?;
        if location.resolved.path != metadata.origin
            || location.resolved.format != metadata.format()
            || location.resolved.physical_stem != metadata.physical_stem
        {
            return Err(format!(
                "violates REQ spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#URI-OBSERVED-SOURCE-READ: source locator `{}` differs from the named selected source; fix surface: resolve metadata again before reading",
                metadata.origin.display()
            ));
        }
        self.read_location(address, location)
    }

    pub(super) fn read_location(
        &self,
        address: &SpecAddress,
        location: crate::resolver::SourceLocation,
    ) -> Result<ResolvedSource, String> {
        let metadata = ResolvedSourceMetadata::new(
            location.resolved.path.clone(),
            location.resolved.format.clone(),
            location.resolved.physical_stem.clone(),
        )?;
        let overlay = self.overlay.get(metadata.origin());
        let observed = location
            .read_with_overlay(overlay.map(|bytes| bytes.as_ref()))
            .map_err(|error| error.to_string())?;
        let bytes = observed.as_ref();
        if metadata.format() == "markdown" {
            let projected = if let Some((interpretation, media)) = location.declared_media() {
                crate::project_declared_native(bytes, media, interpretation)?
            } else {
                project_bytes(
                    metadata.origin(),
                    bytes,
                    if overlay.is_some() {
                        "overlay"
                    } else {
                        "filesystem"
                    },
                )?
            };
            return resolve_section(&projected, metadata.origin(), address)
                .map(|text| ResolvedSource::markdown(text, metadata.physical_stem));
        }
        let text = std::str::from_utf8(bytes).map_err(|error| {
            format!(
                "custom source `{}` is not UTF-8: {error}",
                metadata.origin().display()
            )
        })?;
        ResolvedSource::custom(
            text.to_owned(),
            metadata.format().to_owned(),
            metadata.physical_stem,
        )
    }
}
