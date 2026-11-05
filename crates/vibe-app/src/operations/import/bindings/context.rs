//! One immutable scope framing owner; ephemeral delivery is never hashed.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-CONTEXT-IDENTITY");
use super::*;
use serde::Serialize;
use sha2::{Digest, Sha256};
impl<'a> ImportScope<'a> {
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-CONTEXT-IDENTITY"
    )]
    pub(crate) fn new(
        target: w::ApplicationImportTarget,
        roots: Vec<String>,
        bindings: BTreeMap<String, w::ApplicationPortableBinding>,
        secret_requirements: BTreeMap<String, Vec<String>>,
        secrets: &'a ImportSecrets,
    ) -> Result<Self, AppError> {
        Self::context(
            Some(target),
            roots,
            bindings,
            secret_requirements,
            secrets,
            None,
        )
    }
    pub(crate) fn preview(
        roots: Vec<String>,
        bindings: BTreeMap<String, w::ApplicationPortableBinding>,
        secret_requirements: BTreeMap<String, Vec<String>>,
        secrets: &'a ImportSecrets,
    ) -> Result<Self, AppError> {
        Self::context(None, roots, bindings, secret_requirements, secrets, None)
    }
    /// Only a verified exact application resume may carry its own ordinary
    /// target-only answers; whole-cohort input admission never uses this path.
    pub(crate) fn restored(
        target: w::ApplicationImportTarget,
        roots: Vec<String>,
        bindings: BTreeMap<String, w::ApplicationPortableBinding>,
        secret_requirements: BTreeMap<String, Vec<String>>,
        secrets: &'a ImportSecrets,
        current_key: &str,
    ) -> Result<Self, AppError> {
        Self::context(
            Some(target),
            roots,
            bindings,
            secret_requirements,
            secrets,
            Some(current_key),
        )
    }
    fn context(
        target: Option<w::ApplicationImportTarget>,
        roots: Vec<String>,
        bindings: BTreeMap<String, w::ApplicationPortableBinding>,
        mut secret_requirements: BTreeMap<String, Vec<String>>,
        secrets: &'a ImportSecrets,
        current_key: Option<&str>,
    ) -> Result<Self, AppError> {
        crate::portable::validate_rows(&roots, bindings.values().map(|b| &b.application))
            .map_err(|_| invalid("import", "invalid immutable imported binding set"))?;
        if target.as_ref().is_some_and(|target| {
            !pending::valid_operation(&target.operation)
                || !std::path::Path::new(&target.home).is_absolute()
        }) || bindings.iter().any(|(k, b)| k != &b.application.key)
            || secrets
                .keys()
                .any(|k| !bindings.contains_key(k) && current_key != Some(k))
            || secret_requirements.iter().any(|(k, ids)| {
                !bindings.contains_key(k) || ids.iter().any(|id| id.trim().is_empty())
            })
        {
            return Err(invalid(
                "import",
                "invalid target, binding key or scoped secret recipient",
            ));
        }
        #[derive(Serialize)]
        struct Meaning<'a> {
            target: &'a Option<w::ApplicationImportTarget>,
            roots: &'a [String],
            bindings: &'a BTreeMap<String, w::ApplicationPortableBinding>,
        }
        let bytes = serde_json::to_vec(&Meaning {
            target: &target,
            roots: &roots,
            bindings: &bindings,
        })
        .map_err(|_| invalid("import", "cannot frame immutable imported meaning"))?;
        let mut digest = Sha256::new();
        digest.update(b"vibevm.app.import-bindings/v1\0");
        digest.update(bytes);
        let fingerprint = format!("app-import-bindings/1:sha256:{:x}", digest.finalize());
        let active = crate::portable::active_keys(
            &roots,
            bindings.values().map(|binding| &binding.application),
        );
        for (key, binding) in &bindings {
            let ids = secret_requirements.entry(key.clone()).or_default();
            match &binding.application.configuration {
                w::PortableConfigurationState::Configured(c) => {
                    ids.extend(c.history.parameters.iter().filter_map(|p| match p {
                        w::PortableParameterHistory::Secret(p) if p.provided => Some(p.id.clone()),
                        _ => None,
                    }))
                }
                w::PortableConfigurationState::LegacyEffective(c) => ids.extend(
                    c.secret_origins
                        .iter()
                        .filter(|(_, origin)| {
                            matches!(origin, w::ConfigurationParameterOrigin::Explicit(_))
                        })
                        .map(|(id, _)| id.clone()),
                ),
            }
            ids.extend(secrets.for_key(key).iter().map(|(id, _)| id.clone()));
            ids.sort();
            ids.dedup();
        }
        Ok(Self {
            target,
            roots,
            bindings,
            active,
            secret_requirements,
            secrets,
            fingerprint,
        })
    }
}
