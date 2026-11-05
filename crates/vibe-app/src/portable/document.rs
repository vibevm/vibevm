//! Versioned typed portable documents; source history is never an executable plan.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-HISTORY");
use crate::{AppError, error_lifecycle::LifecycleError};
use std::collections::BTreeSet;
use vibe_wire::generated::{application::e2::apps as full, apps_manifest as old, shared as w};
pub const FULL_APPS_PROTOCOL: &str = "vibe-apps/2";
#[derive(Debug, Clone, PartialEq, Eq)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-HISTORY")]
/// Version-specific generated documents, without an extra serialized enum wrapper.
pub enum PortableDocument {
    Legacy(old::AppsManifest),
    Full(full::AppsManifest),
}
impl PortableDocument {
    #[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-HISTORY")]
    /// Number of carried records, including nonroot dependency data.
    pub fn application_count(&self) -> usize {
        match self {
            Self::Legacy(v) => v.application.len(),
            Self::Full(v) => v.application.len(),
        }
    }
    #[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-HISTORY")]
    /// Serialize the exact generated JSON world, not this domain enum.
    pub fn json(&self) -> Result<String, AppError> {
        if let Self::Full(document) = self {
            validate_full(document).map_err(|reason| invalid("export", &reason))?;
        }
        let body = match self {
            Self::Legacy(v) => serde_json::to_string_pretty(v),
            Self::Full(v) => serde_json::to_string_pretty(v),
        }
        .map_err(|_| invalid("export", "cannot encode portable JSON"))?;
        Ok(format!("{body}\n"))
    }
    #[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-HISTORY")]
    /// TOML preserves explicit empty values and omits only declared None fields.
    pub fn toml(&self) -> Result<String, AppError> {
        if let Self::Full(document) = self {
            validate_full(document).map_err(|reason| invalid("export", &reason))?;
        }
        match self {Self::Legacy(v)=>super::portable_toml(v),Self::Full(v)=>toml::to_string(v).map(|body|format!("# vibe-apps/2 — full configuration; target paths and secrets are separate inputs.\n\n{body}")).map_err(|_|invalid("export","cannot encode portable TOML"))}
    }
}
pub(crate) fn invalid(file: &str, reason: &str) -> AppError {
    LifecycleError::PortableInvalid {
        file: file.into(),
        reason: reason.into(),
    }
    .into()
}
pub(crate) fn exact_key(key: &str) -> Result<(), String> {
    let coordinate = crate::operations::pending::coordinate(key)
        .map_err(|_| "expected an exact application key".to_owned())?;
    if coordinate.key() != key {
        return Err("application key is not in its canonical exact spelling".into());
    }
    Ok(())
}
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-HISTORY")]
/// Dispatch by protocol, then use its closed generated reader. Decoder failures
/// discard rejected values and their serde source chain.
pub fn parse_portable_document(file: &str, text: &str) -> Result<PortableDocument, AppError> {
    #[derive(serde::Deserialize)]
    struct Header {
        protocol: String,
    }
    let json = text.trim_start().starts_with('{');
    let header: Header = if json {
        serde_json::from_str(text).map_err(|_| invalid(file, "invalid portable JSON header"))?
    } else {
        toml::from_str(text).map_err(|_| invalid(file, "invalid portable TOML header"))?
    };
    match header.protocol.as_str() {
        super::APPS_PROTOCOL => {
            let value = if json {
                serde_json::from_str(text)
                    .map_err(|_| invalid(file, "invalid legacy portable JSON"))?
            } else {
                toml::from_str(text).map_err(|_| invalid(file, "invalid legacy portable TOML"))?
            };
            Ok(PortableDocument::Legacy(value))
        }
        FULL_APPS_PROTOCOL => {
            let value: full::AppsManifest = if json {
                serde_json::from_str(text)
                    .map_err(|_| invalid(file, "invalid closed full portable JSON"))?
            } else {
                toml::from_str(text)
                    .map_err(|_| invalid(file, "invalid closed full portable TOML"))?
            };
            validate_full(&value).map_err(|reason| invalid(file, &reason))?;
            Ok(PortableDocument::Full(value))
        }
        _ => Err(invalid(
            file,
            "unsupported portable protocol; use vibe-apps/1 or vibe-apps/2",
        )),
    }
}
pub(crate) fn validate_full(value: &full::AppsManifest) -> Result<(), String> {
    if value.protocol != FULL_APPS_PROTOCOL {
        return Err("unsupported full portable protocol".into());
    }
    validate_rows(&value.roots, &value.application)
}

/// Shared admission for full documents and retained exact-key bindings.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-HISTORY")]
pub(crate) fn validate_rows<'a>(
    roots: &[String],
    applications: impl IntoIterator<Item = &'a w::PortableApplication>,
) -> Result<(), String> {
    let applications: Vec<_> = applications.into_iter().collect();
    let mut keys = BTreeSet::new();
    for row in &applications {
        exact_key(&row.key)?;
        if !keys.insert(row.key.as_str()) {
            return Err(format!("duplicate portable application `{}`", row.key));
        }
        match &row.configuration {
            w::PortableConfigurationState::Configured(v) => {
                crate::configuration::portable::decode(&v.history).map_err(|_| {
                    format!(
                        "application `{}` has invalid historical configuration",
                        row.key
                    )
                })?;
            }
            w::PortableConfigurationState::LegacyEffective(v) => {
                crate::configuration::record::validate_origins(&v.secret_origins)?;
                if let Some(id) = v
                    .parameters
                    .keys()
                    .find(|id| v.secret_origins.contains_key(*id))
                {
                    return Err(format!(
                        "application `{}` contains a public value for secret parameter `{id}`",
                        row.key
                    ));
                }
            }
        }
        for edge in &row.requires {
            exact_key(&edge.key)?;
            let reference = crate::qualified_ref(&edge.package).map_err(|_| {
                format!(
                    "application `{}` has an invalid recorded requirement",
                    row.key
                )
            })?;
            let target = crate::operations::pending::coordinate(&edge.key)
                .map_err(|_| "invalid requirement target".to_string())?;
            let group = reference.group.as_ref().map(ToString::to_string);
            if group.as_deref() != Some(target.group.as_str())
                || reference.name.as_str() != target.name
                || !reference.version.matches(
                    &semver::Version::parse(&target.version)
                        .map_err(|_| "invalid exact dependency version".to_string())?,
                )
            {
                return Err(format!(
                    "recorded requirement of `{}` does not admit `{}`",
                    row.key, edge.key
                ));
            }
        }
    }
    let mut seen_roots = BTreeSet::new();
    for key in roots {
        if !seen_roots.insert(key)
            || !applications
                .iter()
                .any(|r| &r.key == key && r.mark == w::PortableApplicationMark::Manual)
        {
            return Err("portable roots must be unique, present Manual applications".into());
        }
    }
    for row in &applications {
        for edge in &row.requires {
            if !keys.contains(edge.key.as_str()) {
                return Err(format!(
                    "recorded dependency `{}` is absent from full set",
                    edge.key
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;

/// Exact source dependency closure of the selected roots. Orphan rows are data,
/// not additional jobs or permission to resolve host sources.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-PREFLIGHT")]
pub fn selected_import_keys(document: &full::AppsManifest) -> Result<BTreeSet<String>, AppError> {
    validate_full(document).map_err(|reason| invalid("full import", &reason))?;
    Ok(active_keys(&document.roots, document.application.iter()))
}
pub(crate) fn active_keys<'a>(
    roots: &[String],
    applications: impl IntoIterator<Item = &'a w::PortableApplication>,
) -> BTreeSet<String> {
    let rows = applications
        .into_iter()
        .map(|row| (row.key.as_str(), row))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut active = BTreeSet::new();
    let mut queue = roots.to_vec();
    while let Some(key) = queue.pop() {
        if active.insert(key.clone())
            && let Some(row) = rows.get(key.as_str())
        {
            queue.extend(row.requires.iter().map(|edge| edge.key.clone()));
        }
    }
    active
}
