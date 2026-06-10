//! `vibe-redirect.toml` — registry stub marker file.
//!
//! Schema: PROP-002 §2.4.2. A stub repo carries `vibe-redirect.toml` at
//! its root *instead of* `vibe.toml`; the file points at an
//! external git URL where the package's actual content lives. The
//! resolver, when fetching a manifest at `<stub_url>@<ref>`, falls
//! through from `vibe.toml` to `vibe-redirect.toml`, parses
//! the marker, and re-resolves against `target_url` at the
//! pass-through-tag (`<ref>`) or pinned ref.
//!
//! The marker file is mutually exclusive with `vibe.toml` in
//! the same repo at the same ref — both present is rejected as
//! `AmbiguousStub`.

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#redirect");

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

use super::project::AuthKind;
use super::{read_toml, write_toml};

/// `vibe-redirect.toml` — top-level shape.
///
/// `[redirect]` section is required; everything else is reserved for
/// future expansion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RedirectFile {
    pub redirect: RedirectSection,
}

impl RedirectFile {
    /// Canonical filename.
    pub const FILENAME: &'static str = "vibe-redirect.toml";

    pub fn read<P: AsRef<Path>>(path: P) -> Result<Self> {
        read_toml::<Self, _>(path)
    }

    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        write_toml(path, self)
    }

    /// Build a pass-through-tag stub pointing at `target_url`.
    pub fn pass_through_tag(target_url: impl Into<String>) -> Self {
        RedirectFile {
            redirect: RedirectSection {
                target_url: target_url.into(),
                ref_policy: RefPolicy::PassThroughTag,
                pinned_ref: None,
                auth: AuthKind::None,
                token_env: None,
                description: None,
            },
        }
    }
}

/// `[redirect]` body — the operator-facing schema.
///
/// Validation rules (enforced via `try_from = "RedirectSectionWire"`
/// at deserialise time):
///
/// - `target_url` is required and non-empty.
/// - `pinned_ref` must be `Some` iff `ref_policy = "pinned"`; absent
///   for `pass-through-tag`. Mismatch rejected at parse.
/// - `token_env` is meaningful only when `auth = "token-env"`; with
///   any other auth regime it's a parse error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "RedirectSectionWire", try_from = "RedirectSectionWire")]
pub struct RedirectSection {
    pub target_url: String,
    pub ref_policy: RefPolicy,
    pub pinned_ref: Option<String>,
    pub auth: AuthKind,
    pub token_env: Option<String>,
    pub description: Option<String>,
}

/// How stub-side tags map onto target-side refs.
///
/// - `PassThroughTag` (default) — stub tag `T` resolves to
///   `target_url@T`. The org owner gates which versions appear in
///   their namespace by managing stub tags; `vibe registry
///   redirect-sync` mirrors target tags into the stub for
///   ergonomic version management.
/// - `Pinned` — every consumer resolves to `target_url@pinned_ref`
///   regardless of which stub tag they probed. Stub tags are
///   informational metadata only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum RefPolicy {
    #[default]
    PassThroughTag,
    Pinned,
}

// ---------------------------------------------------------------------------
// Wire form for `RedirectSection`. Deserialise validates; Serialise
// elides defaults so a minimal stub round-trips minimally.
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RedirectSectionWire {
    target_url: String,
    #[serde(default, skip_serializing_if = "is_default_ref_policy")]
    ref_policy: RefPolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pinned_ref: Option<String>,
    #[serde(default, skip_serializing_if = "is_default_auth_kind")]
    auth: AuthKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    token_env: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

fn is_default_ref_policy(p: &RefPolicy) -> bool {
    matches!(p, RefPolicy::PassThroughTag)
}

fn is_default_auth_kind(a: &AuthKind) -> bool {
    matches!(a, AuthKind::None)
}

impl From<RedirectSection> for RedirectSectionWire {
    fn from(s: RedirectSection) -> Self {
        RedirectSectionWire {
            target_url: s.target_url,
            ref_policy: s.ref_policy,
            pinned_ref: s.pinned_ref,
            auth: s.auth,
            token_env: s.token_env,
            description: s.description,
        }
    }
}

impl TryFrom<RedirectSectionWire> for RedirectSection {
    type Error = String;

    fn try_from(w: RedirectSectionWire) -> std::result::Result<Self, Self::Error> {
        if w.target_url.trim().is_empty() {
            return Err("`[redirect].target_url` must be a non-empty git URL".to_string());
        }
        match (w.ref_policy, w.pinned_ref.as_deref()) {
            (RefPolicy::PassThroughTag, Some(_)) => {
                return Err(
                    "`pinned_ref` is set but `ref_policy = \"pass-through-tag\"`; either drop \
                     `pinned_ref` or change `ref_policy = \"pinned\"`"
                        .to_string(),
                );
            }
            (RefPolicy::Pinned, None) => {
                return Err(
                    "`ref_policy = \"pinned\"` requires `pinned_ref = \"<tag/branch/rev>\"`"
                        .to_string(),
                );
            }
            _ => {}
        }
        if w.token_env.is_some() && !matches!(w.auth, AuthKind::TokenEnv) {
            return Err(
                "`token_env` is only meaningful with `auth = \"token-env\"`; drop `token_env` or \
                 set `auth = \"token-env\"`"
                    .to_string(),
            );
        }
        Ok(RedirectSection {
            target_url: w.target_url,
            ref_policy: w.ref_policy,
            pinned_ref: w.pinned_ref,
            auth: w.auth,
            token_env: w.token_env,
            description: w.description,
        })
    }
}

/// Try to interpret a byte slice as a stub marker. Used by the
/// registry resolver after a `git archive` of `vibe.toml` came up
/// empty: the same archive call is re-run with `vibe-redirect.toml`
/// and the result fed here.
pub fn parse_redirect_bytes(bytes: &[u8]) -> Result<RedirectFile> {
    let text = std::str::from_utf8(bytes).map_err(|e| Error::BadDependencyDecl {
        input: RedirectFile::FILENAME.to_string(),
        reason: format!("invalid UTF-8: {e}"),
    })?;
    toml::from_str::<RedirectFile>(text).map_err(|e| Error::BadDependencyDecl {
        input: RedirectFile::FILENAME.to_string(),
        reason: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(raw: &str) -> Result<RedirectFile> {
        toml::from_str::<RedirectFile>(raw).map_err(|e| Error::BadDependencyDecl {
            input: "test".to_string(),
            reason: e.to_string(),
        })
    }

    #[test]
    fn pass_through_tag_minimal_parses() {
        let raw = r#"
[redirect]
target_url = "https://github.com/external/flow-internal"
"#;
        let r = parse(raw).unwrap();
        assert_eq!(
            r.redirect.target_url,
            "https://github.com/external/flow-internal"
        );
        assert_eq!(r.redirect.ref_policy, RefPolicy::PassThroughTag);
        assert_eq!(r.redirect.pinned_ref, None);
        assert_eq!(r.redirect.auth, AuthKind::None);
    }

    #[test]
    fn pinned_with_ref_parses() {
        let raw = r#"
[redirect]
target_url = "git@gitlab:specs/internal"
ref_policy = "pinned"
pinned_ref = "v0.3.0"
"#;
        let r = parse(raw).unwrap();
        assert_eq!(r.redirect.ref_policy, RefPolicy::Pinned);
        assert_eq!(r.redirect.pinned_ref.as_deref(), Some("v0.3.0"));
    }

    #[test]
    fn auth_token_env_parses() {
        let raw = r#"
[redirect]
target_url = "https://gitlab.acme.example/x/y"
auth = "token-env"
token_env = "VIBEVM_TARGET_TOKEN"
description = "Delegated to acme-corp"
"#;
        let r = parse(raw).unwrap();
        assert_eq!(r.redirect.auth, AuthKind::TokenEnv);
        assert_eq!(r.redirect.token_env.as_deref(), Some("VIBEVM_TARGET_TOKEN"));
        assert_eq!(
            r.redirect.description.as_deref(),
            Some("Delegated to acme-corp")
        );
    }

    #[test]
    fn pinned_without_ref_rejected() {
        let raw = r#"
[redirect]
target_url = "https://x/y"
ref_policy = "pinned"
"#;
        let err = toml::from_str::<RedirectFile>(raw).unwrap_err();
        assert!(
            err.to_string().contains("pinned_ref"),
            "expected pinned_ref-required message, got: {err}"
        );
    }

    #[test]
    fn pass_through_with_pinned_ref_rejected() {
        let raw = r#"
[redirect]
target_url = "https://x/y"
pinned_ref = "v1.0"
"#;
        let err = toml::from_str::<RedirectFile>(raw).unwrap_err();
        assert!(
            err.to_string().contains("pinned_ref"),
            "expected pinned-without-policy rejection, got: {err}"
        );
    }

    #[test]
    fn token_env_without_token_auth_rejected() {
        let raw = r#"
[redirect]
target_url = "https://x/y"
token_env = "FOO"
"#;
        let err = toml::from_str::<RedirectFile>(raw).unwrap_err();
        assert!(
            err.to_string().contains("token_env"),
            "expected token_env-without-auth rejection, got: {err}"
        );
    }

    #[test]
    fn empty_target_url_rejected() {
        let raw = r#"
[redirect]
target_url = ""
"#;
        let err = toml::from_str::<RedirectFile>(raw).unwrap_err();
        assert!(
            err.to_string().contains("target_url"),
            "expected empty-target-url rejection, got: {err}"
        );
    }

    #[test]
    fn unknown_field_rejected() {
        let raw = r#"
[redirect]
target_url = "https://x/y"
secret_field = "..."
"#;
        let err = toml::from_str::<RedirectFile>(raw).unwrap_err();
        assert!(
            err.to_string().to_lowercase().contains("unknown"),
            "expected unknown-field rejection, got: {err}"
        );
    }

    #[test]
    fn round_trip_preserves_minimal_shape() {
        let r = RedirectFile::pass_through_tag("https://github.com/x/y");
        let rendered = toml::to_string_pretty(&r).unwrap();
        // Defaults elide: only `target_url` is present.
        assert!(rendered.contains("target_url = \"https://github.com/x/y\""));
        assert!(
            !rendered.contains("ref_policy"),
            "default ref_policy should elide"
        );
        assert!(!rendered.contains("auth"), "default auth should elide");
        let back: RedirectFile = toml::from_str(&rendered).unwrap();
        assert_eq!(r, back);
    }

    #[test]
    fn parse_redirect_bytes_passes_through() {
        let raw = b"[redirect]\ntarget_url = \"https://x/y\"\n";
        let r = parse_redirect_bytes(raw).unwrap();
        assert_eq!(r.redirect.target_url, "https://x/y");
    }

    #[test]
    fn parse_redirect_bytes_rejects_non_utf8() {
        let bytes = vec![0xFF, 0xFE, 0xFD];
        let err = parse_redirect_bytes(&bytes).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("utf-8"));
    }
}
