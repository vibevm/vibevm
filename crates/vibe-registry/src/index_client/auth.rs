//! Index-side authentication: resolve the registry's auth regime
//! into an HTTP plan for `IndexClient`, hold the bearer token in a
//! Debug-redacting newtype, and produce the guidance text that tells
//! a refused probe/lookup apart from a missing index
//! (PROP-002 §2.2.1, mirrored from the git side's
//! `git_package_registry/auth.rs`).
//!
//! **One access mode, not a second.** The index is THIS registry's
//! index, so it shares the registry's credentials — the same
//! `token-env` token the git side reads. No new config key, no second
//! token file. The token travels as an `Authorization: Bearer` header
//! (never in the URL, where it would leak into logs / proxies /
//! referer), and only over `https://` (never `http://`, for the same
//! reason [`inject_token`](crate::git_package_registry::inject_token)
//! skips it). `ssh` / `credential-helper` authorise git transport,
//! not HTTP — under them the client carries no HTTP credential and,
//! when the index answers 401/403, must say so rather than look like
//! "no index here".

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-auth");

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use vibe_core::manifest::{AuthKind, RegistrySection};

/// A bearer token for the index, held so its [`Debug`](std::fmt::Debug)
/// representation never prints the secret. Stored inside
/// [`IndexAuth::Bearer`], which is stored inside `IndexClient` — so the
/// client's derived `Debug` (and every error / log that flows through
/// it) leaks nothing. The redaction is structural: the only accessor
/// for the raw secret is private to this module and sinks it straight
/// into an HTTP [`HeaderValue`]; there is no `Display` and no public
/// reader a caller could format into user-facing text.
#[derive(Clone)]
pub struct BearerToken(String);

impl BearerToken {
    /// Wrap a resolved secret. Public so tests / direct constructors
    /// can build a [`IndexAuth::Bearer`] plan; the secret itself is
    /// not otherwise reachable.
    pub fn new(secret: String) -> Self {
        Self(secret)
    }

    /// Raw secret — private; only [`IndexAuth::header_map`] reads it,
    /// sinking it into an HTTP `HeaderValue`.
    fn secret(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for BearerToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BearerToken(<redacted>)")
    }
}

/// How [`super::IndexClient`] authenticates to THIS registry's index.
/// Computed once from the registry section and the resolved index URL
/// via [`IndexAuth::for_registry`], then carried on the client so the
/// single `build_client` chokepoint can attach it to every request.
///
/// The plan is the scheme gate for Р3: a [`IndexAuth::Bearer`] plan is
/// produced only for an `https://` base, so a token never travels over
/// plaintext. The other variants send no `Authorization` header.
#[derive(Debug, Clone, Default)]
pub enum IndexAuth {
    /// Send no `Authorization` header. Covers `auth = "none"`, any
    /// `http://` base (a token never travels over plaintext), and
    /// `token-env` whose env-var resolved empty. A 401/403 surfaces as
    /// the generic [`super::IndexError::Status`].
    #[default]
    None,
    /// Attach `Authorization: Bearer <token>` to every request.
    /// Constructed only for an `https://` base whose registry carries a
    /// resolved `token-env` token.
    Bearer(BearerToken),
    /// The registry's regime authorises git transport, not HTTP. There
    /// is no HTTP credential this client can ever send, so a 401/403
    /// must surface [`super::IndexError::AuthIncapable`] (naming the
    /// regime and the fix) rather than silent fall-through. Holds the
    /// regime's kebab name (`"ssh"` / `"credential-helper"`).
    HttpIncapable(&'static str),
}

impl IndexAuth {
    /// Resolve the index-auth plan for a registry from its manifest
    /// section and the resolved index base URL. Mirrors the git side's
    /// token resolution (PROP-002 §2.2.1): under `token-env` the token
    /// is read from the explicit-or-host-derived env var, trimmed, and
    /// used only over `https://`. The decision matrix itself lives in
    /// [`IndexAuth::plan`] so it is unit-testable without mutating
    /// process env (forbidden under `#![forbid(unsafe_code)]` on
    /// edition 2024+, where `std::env::set_var` is `unsafe`).
    pub fn for_registry(reg: &RegistrySection, base_url: &str) -> IndexAuth {
        let token = if matches!(reg.auth, AuthKind::TokenEnv) {
            resolve_env_token(reg)
        } else {
            None
        };
        Self::plan(token, reg.auth, base_url.starts_with("https://"))
    }

    /// Pure decision: given a resolved token (already read from env),
    /// the auth regime, and whether the base is `https://`, pick the
    /// plan. Split out of [`IndexAuth::for_registry`] so the full
    /// matrix — including the `http://` suppression (Р3) and the
    /// ssh / credential-helper routing (Р4) — is unit-testable.
    fn plan(token: Option<String>, auth: AuthKind, is_https: bool) -> IndexAuth {
        match (auth, token, is_https) {
            (AuthKind::TokenEnv, Some(token), true) => IndexAuth::Bearer(BearerToken::new(token)),
            // `http://` base: never send the token over plaintext,
            // even though one resolved. `token-env` with no resolved
            // token: nothing to send — a 401/403 surfaces as the
            // generic Status, which is the operator's "set the token
            // env var" signal (the git side's MissingToken is the
            // parallel cue on its path).
            (AuthKind::TokenEnv, _, _) => IndexAuth::None,
            (AuthKind::Ssh, _, _) => IndexAuth::HttpIncapable("ssh"),
            (AuthKind::CredentialHelper, _, _) => IndexAuth::HttpIncapable("credential-helper"),
            (AuthKind::None, _, _) => IndexAuth::None,
        }
    }

    /// The `Authorization` default-header map this plan attaches to the
    /// HTTP client, or `None` when the plan sends no header
    /// ([`IndexAuth::None`] / [`IndexAuth::HttpIncapable`]). A bearer
    /// token whose bytes are invalid for an HTTP header value (control
    /// chars, newlines) cannot be sent safely and is refused here with
    /// a `warn` rather than failing the whole request build — such a
    /// token is malformed and would be rejected by any server anyway.
    pub(super) fn header_map(&self) -> Option<HeaderMap> {
        let IndexAuth::Bearer(token) = self else {
            return None;
        };
        let mut map = HeaderMap::new();
        match HeaderValue::from_str(&format!("Bearer {}", token.secret())) {
            Ok(value) => {
                map.insert(HeaderName::from_static("authorization"), value);
                Some(map)
            }
            Err(_) => {
                tracing::warn!(
                    target: "vibe_registry::index_client",
                    "index token contains bytes invalid for an Authorization header — not attaching"
                );
                None
            }
        }
    }
}

/// Human-facing reason for a probe the index refused (401/403),
/// carrying regime-specific guidance. Surfaced as
/// `UnreachableRegistry.reason` by the CLI so a private index is
/// distinguishable from a missing one (Р5). `base` is the
/// operator-supplied index URL.
pub(super) fn refusal_reason(base: &str, status: u16, auth: &IndexAuth) -> String {
    match auth {
        IndexAuth::HttpIncapable(regime) => format!(
            "index at `{base}` refused the probe (HTTP {status}): the registry's \
             `auth = \"{regime}\"` cannot supply HTTP credentials — declare \
             `auth = \"token-env\"` with a token the index accepts"
        ),
        IndexAuth::Bearer(_) => format!(
            "index at `{base}` refused the probe (HTTP {status}): the supplied \
             token was rejected — check the token env var for this registry"
        ),
        IndexAuth::None => format!(
            "index at `{base}` refused the probe (HTTP {status}): it requires \
             authentication this registry does not provide — declare \
             `auth = \"token-env\"` with a token the index accepts"
        ),
    }
}

/// Read and trim the `token-env` token for a registry, following the
/// same algorithm as the git side's `open_with_auth`: the explicit
/// `token_env` override wins, otherwise the host-derived default from
/// [`RegistrySection::resolve_token_env_name`]. Returns `None` for an
/// absent or whitespace-only value.
fn resolve_env_token(reg: &RegistrySection) -> Option<String> {
    reg.resolve_token_env_name()
        .and_then(|var| std::env::var(&var).ok())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reg(auth: AuthKind) -> RegistrySection {
        toml::from_str(&format!(
            r#"name = "r"
               url = "https://git.example.com/org"
               auth = "{}"
            "#,
            auth.as_str()
        ))
        .unwrap()
    }

    // ---- plan() decision matrix (no env mutation) ----

    #[test]
    fn plan_token_env_https_with_token_is_bearer() {
        let plan = IndexAuth::plan(Some("tok".into()), AuthKind::TokenEnv, true);
        assert!(matches!(plan, IndexAuth::Bearer(_)), "got {plan:?}");
    }

    #[test]
    fn plan_token_env_http_with_token_is_none() {
        // Р3: a resolved token is never sent over plaintext.
        let plan = IndexAuth::plan(Some("tok".into()), AuthKind::TokenEnv, false);
        assert!(matches!(plan, IndexAuth::None));
    }

    #[test]
    fn plan_token_env_https_missing_token_is_none() {
        let plan = IndexAuth::plan(None, AuthKind::TokenEnv, true);
        assert!(matches!(plan, IndexAuth::None));
    }

    #[test]
    fn plan_ssh_is_http_incapable() {
        let plan = IndexAuth::plan(None, AuthKind::Ssh, true);
        assert!(matches!(plan, IndexAuth::HttpIncapable("ssh")));
    }

    #[test]
    fn plan_credential_helper_is_http_incapable() {
        let plan = IndexAuth::plan(None, AuthKind::CredentialHelper, true);
        assert!(matches!(
            plan,
            IndexAuth::HttpIncapable("credential-helper")
        ));
    }

    #[test]
    fn plan_none_auth_is_none() {
        let plan = IndexAuth::plan(Some("tok".into()), AuthKind::None, true);
        assert!(matches!(plan, IndexAuth::None));
    }

    // ---- Debug redaction (Р6 / acceptance 6) ----

    #[test]
    fn bearer_token_debug_redacts_secret() {
        let token = BearerToken::new("hunter2-supersecret".into());
        let rendered = format!("{token:?}");
        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains("hunter2"));
    }

    #[test]
    fn index_auth_debug_redacts_bearer() {
        let auth = IndexAuth::Bearer(BearerToken::new("hunter2-supersecret".into()));
        let rendered = format!("{auth:?}");
        assert!(rendered.contains("<redacted>"));
        assert!(!rendered.contains("hunter2"));
    }

    // ---- header_map (the single attachment chokepoint) ----

    #[test]
    fn header_map_attaches_bearer_authorization() {
        let map = IndexAuth::Bearer(BearerToken::new("tok123".into()))
            .header_map()
            .expect("bearer plan yields a header map");
        assert_eq!(
            map.get("authorization").unwrap().to_str().unwrap(),
            "Bearer tok123"
        );
    }

    #[test]
    fn header_map_is_none_for_non_bearer_plans() {
        assert!(IndexAuth::None.header_map().is_none());
        assert!(IndexAuth::HttpIncapable("ssh").header_map().is_none());
    }

    // ---- for_registry routing (regime → plan, no token) ----

    #[test]
    fn for_registry_ssh_is_http_incapable() {
        assert!(matches!(
            IndexAuth::for_registry(&reg(AuthKind::Ssh), "https://idx.example.com"),
            IndexAuth::HttpIncapable("ssh")
        ));
    }

    #[test]
    fn for_registry_none_is_none() {
        assert!(matches!(
            IndexAuth::for_registry(&reg(AuthKind::None), "https://idx.example.com"),
            IndexAuth::None
        ));
    }
}
