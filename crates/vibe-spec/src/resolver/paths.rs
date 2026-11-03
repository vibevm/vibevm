//! Lexical filesystem inputs; selected roots remain caller-owned authority.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#URI-LOOKUP-PATHS");

use super::ResolveError;
use std::path::Path;
use vibe_core::manifest::{declarant_path, declarant_path_component};

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#URI-LOOKUP-PATHS"
)]
pub(super) fn source(document: &str, formats: &[String]) -> Result<(), ResolveError> {
    declarant_path(Path::new(document)).map_err(|reason| ResolveError::InvalidLookupPath {
        field: "document path",
        value: document.into(),
        reason,
    })?;
    for format in formats {
        component("custom frontend filename", &format!("source.{format}"))?;
    }
    Ok(())
}

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#URI-LOOKUP-PATHS"
)]
pub(super) fn package_component(
    group: &str,
    name: &str,
    version: Option<&str>,
) -> Result<String, ResolveError> {
    let slot = format!("{group}.{name}");
    component("package slot", &slot)?;
    if let Some(version) = version {
        component("slot version", version)?;
    }
    Ok(slot)
}

fn component(field: &'static str, value: &str) -> Result<(), ResolveError> {
    declarant_path_component(value).map_err(|reason| ResolveError::InvalidLookupPath {
        field,
        value: value.into(),
        reason,
    })
}
