use std::path::{Path, PathBuf};
use std::sync::Arc;

use specmark::spec;
use vibe_core::manifest::LlmSection;
use vibe_core::user_config::LlmConfig;

use crate::{
    ApiKey, ApiKeyError, ChatTransport, Endpoint, EndpointError, OPENAI_COMPATIBLE_PROVIDER_ID,
    OpenAiCompatibleProvider,
};

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI");

/// Injectable access to the two credential sources. Implementations return
/// values only; errors deliberately carry no third-party message that could
/// echo a secret.
///
/// ```
/// use std::path::Path;
/// use vibe_llm::{CredentialReadError, CredentialReader};
///
/// struct EmptyCredentials;
/// impl CredentialReader for EmptyCredentials {
///     fn read_env(&self, _: &str) -> Result<Option<String>, CredentialReadError> {
///         Ok(None)
///     }
///     fn read_file(&self, _: &Path) -> Result<String, CredentialReadError> {
///         Err(CredentialReadError::Unavailable)
///     }
/// }
/// assert_eq!(EmptyCredentials.read_env("DEMO").unwrap(), None);
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
pub trait CredentialReader: Send + Sync {
    fn read_env(&self, name: &str) -> Result<Option<String>, CredentialReadError>;
    fn read_file(&self, path: &Path) -> Result<String, CredentialReadError>;
}

#[derive(Debug, Clone, Copy, Default)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
pub struct SystemCredentialReader;

impl CredentialReader for SystemCredentialReader {
    fn read_env(&self, name: &str) -> Result<Option<String>, CredentialReadError> {
        match std::env::var(name) {
            Ok(value) => Ok(Some(value)),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(std::env::VarError::NotUnicode(_)) => Err(CredentialReadError::Unavailable),
        }
    }

    fn read_file(&self, path: &Path) -> Result<String, CredentialReadError> {
        std::fs::read_to_string(path).map_err(|_| CredentialReadError::Unavailable)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
pub enum CredentialReadError {
    #[error(
        "credential source is unavailable \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI; \
         fix: populate or repair the configured credential source)"
    )]
    Unavailable,
}

/// Safe identity/provenance of the selected credential, never its value.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
pub enum CredentialSource {
    Environment(String),
    TokenFile(PathBuf),
}

impl std::fmt::Display for CredentialSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Environment(name) => write!(f, "environment variable `{name}`"),
            Self::TokenFile(path) => write!(f, "token file `{}`", path.display()),
        }
    }
}

/// Fully merged and policy-checked R7 provider configuration.
#[derive(Debug, Clone)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
pub struct EffectiveLlmConfig {
    provider: String,
    model: String,
    endpoint: Endpoint,
    api_key: Option<ApiKey>,
    credential_source: Option<CredentialSource>,
}

impl EffectiveLlmConfig {
    pub fn provider(&self) -> &str {
        &self.provider
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    pub fn api_key(&self) -> Option<&ApiKey> {
        self.api_key.as_ref()
    }

    pub fn credential_source(&self) -> Option<&CredentialSource> {
        self.credential_source.as_ref()
    }

    pub fn into_provider(
        self,
        transport: Arc<dyn ChatTransport>,
    ) -> Result<OpenAiCompatibleProvider, crate::ProviderError> {
        OpenAiCompatibleProvider::new(self.model, self.endpoint, self.api_key, transport)
    }
}

/// Resolve user/project layers by field, then load exactly one credential.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
pub fn resolve_effective_config(
    user: Option<&LlmConfig>,
    selected_user_config_path: &Path,
    project: Option<&LlmSection>,
    credentials: &dyn CredentialReader,
) -> Result<Option<EffectiveLlmConfig>, EffectiveLlmConfigError> {
    if user.is_none() && project.is_none() {
        return Ok(None);
    }

    let provider = project
        .map(|value| value.default_provider.as_str())
        .or_else(|| user.map(|value| value.provider.as_str()))
        .filter(|value| !value.trim().is_empty())
        .ok_or(EffectiveLlmConfigError::MissingField("provider"))?
        .to_owned();
    if provider != OPENAI_COMPATIBLE_PROVIDER_ID {
        return Err(EffectiveLlmConfigError::UnsupportedProvider(provider));
    }
    let model = project
        .map(|value| value.default_model.as_str())
        .or_else(|| user.map(|value| value.model.as_str()))
        .filter(|value| !value.trim().is_empty())
        .ok_or(EffectiveLlmConfigError::MissingField("model"))?
        .to_owned();
    let endpoint_raw = user
        .map(|value| value.endpoint.as_str())
        .filter(|value| !value.trim().is_empty())
        .ok_or(EffectiveLlmConfigError::MissingField("endpoint"))?;

    let credential_source = select_credential_source(user, selected_user_config_path, project);
    let endpoint = Endpoint::parse(endpoint_raw, credential_source.is_some())?;
    let api_key = credential_source
        .as_ref()
        .map(|source| read_api_key(source, credentials))
        .transpose()?;

    Ok(Some(EffectiveLlmConfig {
        provider,
        model,
        endpoint,
        api_key,
        credential_source,
    }))
}

fn select_credential_source(
    user: Option<&LlmConfig>,
    selected_user_config_path: &Path,
    project: Option<&LlmSection>,
) -> Option<CredentialSource> {
    if let Some(name) = project
        .and_then(|value| value.api_key_env.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Some(CredentialSource::Environment(name.to_owned()));
    }
    user.and_then(|value| value.token_file.as_ref())
        .map(|path| {
            let path = if path.is_absolute() {
                path.clone()
            } else {
                selected_user_config_path
                    .parent()
                    .unwrap_or_else(|| Path::new(""))
                    .join(path)
            };
            CredentialSource::TokenFile(path)
        })
}

fn read_api_key(
    source: &CredentialSource,
    credentials: &dyn CredentialReader,
) -> Result<ApiKey, EffectiveLlmConfigError> {
    let raw = match source {
        CredentialSource::Environment(name) => credentials
            .read_env(name)
            .map_err(|_| EffectiveLlmConfigError::CredentialUnavailable(source.clone()))?
            .ok_or_else(|| EffectiveLlmConfigError::CredentialUnavailable(source.clone()))?,
        CredentialSource::TokenFile(path) => credentials
            .read_file(path)
            .map_err(|_| EffectiveLlmConfigError::CredentialUnavailable(source.clone()))?,
    };
    ApiKey::new(raw).map_err(|reason| EffectiveLlmConfigError::InvalidCredential {
        credential_source: source.clone(),
        reason,
    })
}

#[derive(Debug, thiserror::Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI")]
pub enum EffectiveLlmConfigError {
    #[error(
        "effective LLM config is missing required field `{0}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI; \
         fix: complete the user/project LLM configuration)"
    )]
    MissingField(&'static str),
    #[error(
        "unsupported LLM provider `{0}`; expected exact id `openai-compatible` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI; \
         fix: set provider to `openai-compatible`)"
    )]
    UnsupportedProvider(String),
    #[error(transparent)]
    Endpoint(#[from] EndpointError),
    #[error(
        "configured {0} is unavailable \
         (violates spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-CLI; \
         fix: populate or repair that credential source)"
    )]
    CredentialUnavailable(CredentialSource),
    #[error(
        "configured {credential_source} is invalid: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-BOUNDARIES; \
         fix: replace the token at that source)"
    )]
    InvalidCredential {
        credential_source: CredentialSource,
        reason: ApiKeyError,
    },
}
