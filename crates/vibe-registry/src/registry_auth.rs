//! Pure lowering of auth declarations for an already selected source address.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-auth");
use crate::RegistryError;
use vibe_core::manifest::{AuthKind, RegistrySection};

/// The caller chooses source authority before requesting credential policy.
/// Locked-source inputs are independent local choices, never current-catalog matching.
pub enum RegistryAuthInput<'a> {
    SelectedRegistry(&'a RegistrySection),
    /// Root/operator-selected source; never an untrusted dependency or redirect grant.
    OperatorSource {
        source_url: &'a str,
        auth: Option<AuthKind>,
        token_env: Option<&'a str>,
    },
    LockedSource {
        source_url: &'a str,
        auth: Option<AuthKind>,
        token_env: Option<&'a str>,
    },
}
/// Credential mechanism/name only; no secret lookup, source selection or profile capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRegistryAuth {
    pub kind: AuthKind,
    pub token_env_name: Option<String>,
    ssh_source: bool,
}
impl ResolvedRegistryAuth {
    pub fn is_ssh_source(&self) -> bool {
        self.ssh_source
    }
}

/// Omission follows the selected transport; explicit mechanisms remain unchanged.
/// Anonymous HTTPS remains usable under an explicit Ssh credential ceiling.
/// ```
/// use vibe_registry::{resolve_registry_auth, RegistryAuthInput};
/// use vibe_core::manifest::AuthKind;
/// let value = resolve_registry_auth(RegistryAuthInput::LockedSource {
///     source_url: "git@alias:org/repo", auth: None, token_env: None,
/// }).unwrap();
/// assert_eq!(value.kind, AuthKind::Ssh);
/// ```
pub fn resolve_registry_auth(
    input: RegistryAuthInput<'_>,
) -> Result<ResolvedRegistryAuth, RegistryError> {
    let (label, source, declared, token_env) = match input {
        RegistryAuthInput::SelectedRegistry(row) => (
            row.name.as_str(),
            row.url.as_str(),
            row.auth,
            row.token_env.as_deref(),
        ),
        RegistryAuthInput::OperatorSource {
            source_url,
            auth,
            token_env,
        } => ("operator source", source_url, auth, token_env),
        RegistryAuthInput::LockedSource {
            source_url,
            auth,
            token_env,
        } => ("locked source", source_url, auth, token_env),
    };
    let ssh_source =
        ssh_source(source).map_err(|reason| declaration_error(label, source, reason))?;
    let kind = match declared {
        Some(kind) => kind,
        None if ssh_source => AuthKind::Ssh,
        None => AuthKind::None,
    };
    if token_env.is_some() && kind != AuthKind::TokenEnv {
        return Err(declaration_error(
            label,
            source,
            "token_env requires an explicit token-env declaration",
        ));
    }
    let token_env_name = if kind == AuthKind::TokenEnv {
        vibe_core::manifest::registry_token_env_name(source, token_env)
    } else {
        None
    };
    Ok(ResolvedRegistryAuth {
        kind,
        token_env_name,
        ssh_source,
    })
}

pub(crate) fn declaration_error(label: &str, source: &str, reason: &str) -> RegistryError {
    let raw = source.strip_prefix("git+").unwrap_or(source);
    let endpoint = if raw.contains("://") {
        vibe_core::progress::transfer::safe_url(raw)
    } else if ssh_source(raw).unwrap_or(false) {
        "SSH source".into()
    } else {
        "selected source".into()
    };
    RegistryError::AuthDeclaration {
        registry: label.into(),
        endpoint,
        reason: reason.into(),
    }
}

/// Syntax classification only: SSH aliases are allowed here, not resolved or trusted.
pub(crate) fn ssh_source(source: &str) -> Result<bool, &'static str> {
    let raw = source.strip_prefix("git+").unwrap_or(source);
    if raw.is_empty() || raw.chars().any(char::is_control) {
        return Err("invalid selected source spelling");
    }
    if raw.contains("://") || raw.starts_with("file:") {
        let url = reqwest::Url::parse(raw).map_err(|_| "invalid selected source URL")?;
        if url.scheme() == "ssh" && url.host_str().is_none() {
            return Err("SSH source has no host");
        }
        return Ok(url.scheme() == "ssh");
    }
    if raw.starts_with(['/', '\\'])
        || raw.starts_with("./")
        || raw.starts_with("../")
        || raw.starts_with("~/")
        || (raw.as_bytes().get(1) == Some(&b':') && raw.as_bytes()[0].is_ascii_alphabetic())
    {
        return Ok(false);
    }
    let remote = raw.rsplit_once('@').map_or(raw, |(_, remote)| remote);
    let parts = if remote.starts_with('[') {
        remote.find("]:").map(|i| (&remote[..=i], &remote[i + 2..]))
    } else {
        remote.split_once(':')
    };
    match parts {
        Some((host, path))
            if !host.is_empty()
                && !path.is_empty()
                && !path.starts_with(':')
                && !host.contains(['/', '\\', '@', '?', '#'])
                && !host.chars().any(char::is_whitespace) =>
        {
            Ok(true)
        }
        _ => Ok(false),
    }
}

#[cfg(test)]
mod tests;
