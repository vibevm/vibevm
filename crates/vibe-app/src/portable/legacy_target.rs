//! One public legacy-value admission path serves preparation, preview and execution.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-TARGET-INPUT");
use crate::{AppError, configuration::ConfigurationError};
use std::collections::BTreeMap;
use vibe_core::manifest::{InstallerOptionDecl, ParameterType};
use vibe_wire::generated::shared as w;
fn invalid(key: &str, reason: impl Into<String>) -> AppError {
    ConfigurationError::Invalid {
        option: key.into(),
        reason: reason.into(),
    }
    .into()
}
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-TARGET-INPUT"
)]
pub(crate) fn public(
    key: &str,
    source: &w::PortableConfigurationStateLegacyEffective,
    target: &w::PortableTargetInput,
    option: &InstallerOptionDecl,
) -> Result<BTreeMap<String, String>, AppError> {
    if target.configuration.is_some() {
        return Err(invalid(key, "legacy/common replacement mismatch"));
    }
    let mut values = source.parameters.clone();
    for (id, path) in &target.path_bindings {
        if !source.parameters.contains_key(id)
            || !option
                .parameters
                .iter()
                .any(|p| &p.id == id && !p.secret && p.kind == ParameterType::Path)
        {
            return Err(invalid(
                key,
                "path binding does not name a present public source Path",
            ));
        }
        if !std::path::Path::new(path).is_absolute() {
            return Err(invalid(key, "target path binding must be absolute"));
        }
        values.insert(id.clone(), path.clone());
    }
    for (id, value) in &values {
        let p = option
            .parameters
            .iter()
            .find(|p| &p.id == id && !p.secret)
            .ok_or_else(|| {
                invalid(
                    key,
                    "legacy public input now names an unknown or secret parameter",
                )
            })?;
        if p.kind == ParameterType::Path && !target.path_bindings.contains_key(id) {
            return Err(invalid(
                key,
                "present legacy Path requires explicit target binding",
            ));
        }
        let canonical = p
            .canonical(value)
            .map_err(|_| invalid(key, "legacy public input violates target validation"))?;
        if canonical != *value
            || p.kind == ParameterType::Path
                && p.must_exist
                && !std::path::Path::new(value).exists()
        {
            return Err(invalid(
                key,
                "noncanonical input or unavailable target path",
            ));
        }
    }
    if source
        .features
        .iter()
        .any(|id| !option.features.iter().any(|f| &f.id == id))
    {
        return Err(invalid(key, "unknown target legacy feature"));
    }
    Ok(values)
}
