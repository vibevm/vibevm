//! Public target decisions and nonserializable, exact-key ephemeral answers.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-SECRET-INPUT");
use crate::{AppError, configuration::ConfigurationError};
use std::collections::BTreeMap;
use vibe_wire::generated::application::e1::{
    import_input::ImportTargetInput, import_secrets::ImportSecretInput,
};

/// Values live only for this invocation; Debug names recipients, never answers.
/// There is no Serialize implementation or retained source filename.
#[derive(Default)]
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-SECRET-INPUT")]
pub struct ImportSecrets {
    answers: BTreeMap<String, Vec<(String, String)>>,
}
impl std::fmt::Debug for ImportSecrets {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImportSecrets")
            .field("recipients", &self.answers.keys().collect::<Vec<_>>())
            .finish()
    }
}
impl ImportSecrets {
    pub(crate) fn single(key: String, answers: Vec<(String, String)>) -> Self {
        Self {
            answers: BTreeMap::from([(key, answers)]),
        }
    }
    pub(crate) fn for_key(&self, key: &str) -> &[(String, String)] {
        self.answers.get(key).map(Vec::as_slice).unwrap_or(&[])
    }
    pub(crate) fn keys(&self) -> impl Iterator<Item = &str> {
        self.answers.keys().map(String::as_str)
    }
}
fn invalid(scope: &str, reason: &str) -> AppError {
    ConfigurationError::Invalid {
        option: scope.into(),
        reason: reason.into(),
    }
    .into()
}
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-SECRET-INPUT"
)]
/// Immediately move the generated transport into a redacted ephemeral owner.
pub fn parse_import_secrets(bytes: &[u8]) -> Result<ImportSecrets, AppError> {
    let input: ImportSecretInput = serde_json::from_slice(bytes).map_err(|_| {
        invalid(
            "import secrets",
            "invalid closed secret input JSON; inspect field names and value types",
        )
    })?;
    if input.protocol != "vibe-app-import-secrets/1" {
        return Err(invalid(
            "import secrets",
            "unsupported secret input protocol",
        ));
    }
    let mut answers = BTreeMap::new();
    for (key, values) in input.applications {
        super::document::exact_key(&key).map_err(|_| {
            invalid(
                "import secrets",
                "secret recipient must be an exact application key",
            )
        })?;
        answers.insert(key, values.into_iter().collect());
    }
    Ok(ImportSecrets { answers })
}
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-TARGET-INPUT"
)]
/// Decode target decisions independently of portable source history and secrets.
pub fn parse_import_target(bytes: &[u8]) -> Result<ImportTargetInput, AppError> {
    let input: ImportTargetInput = serde_json::from_slice(bytes).map_err(|_| {
        invalid(
            "import target",
            "invalid closed target input JSON; inspect field names and value types",
        )
    })?;
    if input.protocol != "vibe-app-import-input/1" {
        return Err(invalid(
            "import target",
            "unsupported target input protocol",
        ));
    }
    for (key, row) in &input.applications {
        super::document::exact_key(key).map_err(|_| {
            invalid(
                "import target",
                "target recipient must be an exact application key",
            )
        })?;
        if row
            .configuration
            .as_ref()
            .is_some_and(|v| v.protocol != "vibe-configuration-input/1")
        {
            return Err(invalid(
                "import target",
                "unsupported complete configuration protocol",
            ));
        }
    }
    Ok(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn secret_input_is_scoped_redacted_and_rejected_values_have_no_error_chain() {
        let parsed=parse_import_secrets(br#"{"protocol":"vibe-app-import-secrets/1","applications":{"org.example/a@1.0.0":{"token":"SYN_SECRET_TRANSPORT"},"org.example/b@1.0.0":{"token":"different"}}}"#).unwrap();
        assert_eq!(
            parsed.for_key("org.example/a@1.0.0")[0].1,
            "SYN_SECRET_TRANSPORT"
        );
        assert_eq!(parsed.for_key("org.example/b@1.0.0")[0].1, "different");
        assert!(parsed.for_key("org.example/c@1.0.0").is_empty());
        assert!(!format!("{parsed:?}").contains("SYN_SECRET_TRANSPORT"));
        let error=parse_import_secrets(br#"{"protocol":"vibe-app-import-secrets/1","applications":{},"SYN_SECRET_TRANSPORT":true}"#).unwrap_err();
        assert!(!error.to_string().contains("SYN_SECRET_TRANSPORT"));
        let mut source = std::error::Error::source(&error);
        while let Some(error) = source {
            assert!(!error.to_string().contains("SYN_SECRET_TRANSPORT"));
            source = error.source();
        }
    }
}
