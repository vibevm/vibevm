use reqwest::header::HeaderValue;
use specmark::spec;

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-000#token-secrecy");

/// A provider credential whose raw bytes never participate in formatting.
#[derive(Clone)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-000#TS-NEVER-PRINTED")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-000#TS-NEVER-PRINTED")]
pub struct ApiKey(String);

impl ApiKey {
    pub fn new(value: impl Into<String>) -> Result<Self, ApiKeyError> {
        let value = value.into();
        let value = value.trim();
        if value.is_empty() {
            return Err(ApiKeyError::Empty);
        }
        let bearer = format!("Bearer {value}");
        HeaderValue::from_str(&bearer).map_err(|_| ApiKeyError::InvalidHeaderValue)?;
        Ok(Self(value.to_owned()))
    }

    /// Construct the header at the transport chokepoint. The raw string never
    /// leaves this module and never appears in a diagnostic.
    pub(crate) fn bearer_header(&self) -> Result<HeaderValue, ApiKeyError> {
        let mut header = HeaderValue::from_str(&format!("Bearer {}", self.0))
            .map_err(|_| ApiKeyError::InvalidHeaderValue)?;
        header.set_sensitive(true);
        Ok(header)
    }
}

impl std::fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ApiKey([REDACTED])")
    }
}

impl std::fmt::Display for ApiKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[REDACTED]")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-000#TS-NEVER-PRINTED")]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-000#TS-NEVER-PRINTED")]
pub enum ApiKeyError {
    #[error(
        "the selected credential is empty \
         (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-BOUNDARIES; \
         fix: provide a non-empty token at the configured source)"
    )]
    Empty,
    #[error(
        "the selected credential cannot be represented as a bearer header \
         (violates spec://org.vibevm.core/vibevm/common/PROP-000#TS-BOUNDARIES; \
         fix: replace the configured token with valid header bytes)"
    )]
    InvalidHeaderValue,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bearer_header_is_structurally_sensitive_and_formatting_is_redacted() {
        let key = ApiKey::new("structural-secret-canary").unwrap();
        let header = key.bearer_header().unwrap();
        assert!(header.is_sensitive());
        assert!(!format!("{key:?}").contains("structural-secret-canary"));
        assert!(!format!("{key}").contains("structural-secret-canary"));
    }
}
