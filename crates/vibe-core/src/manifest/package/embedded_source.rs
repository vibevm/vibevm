//! `[[embedded_source]]` — immutable external source inputs carried by a
//! package manifest without vendoring those bytes into the package tree.

use serde::{Deserialize, Serialize};

use crate::content_hash::ContentHash;
use crate::manifest::declarant_path::{declarant_path, is_windows_device_name};

/// The closed source transport vocabulary for the first reference-bridge
/// wire. Later transports require an explicit schema addition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddedSourceKind {
    Git,
}

/// Authentication posture for an embedded source. Reference bridges are
/// public and credential-free in v1; this enum makes any future expansion an
/// explicit wire change rather than accepting arbitrary strings today.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddedSourceAuth {
    #[default]
    None,
}

/// One immutable upstream source referenced by a package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbeddedSourceDecl {
    /// Portable manifest-local identity used by `[[skill]]` declarations.
    pub name: String,
    pub kind: EmbeddedSourceKind,
    /// Original public upstream repository URL.
    pub url: String,
    /// Exact full Git commit object id (SHA-1 or SHA-256 repository).
    pub commit: String,
    /// Independent SHA-256 identity of the referenced source tree.
    pub content_hash: ContentHash,
    /// Optional advertised ref used only to make the exact commit fetchable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ref_hint: Option<String>,
    /// Public anonymous access is the only v1 posture.
    #[serde(default, skip_serializing_if = "is_default_auth")]
    pub auth: EmbeddedSourceAuth,
    /// SPDX expression describing the upstream bytes, distinct from the
    /// bridge package's own `[package].license`.
    pub upstream_license: String,
    /// License file inside the authenticated upstream tree.
    pub license_path: std::path::PathBuf,
    /// Immutable public HTTPS link to the upstream license at the pin.
    pub license_url: String,
}

fn is_default_auth(value: &EmbeddedSourceAuth) -> bool {
    *value == EmbeddedSourceAuth::None
}

impl EmbeddedSourceDecl {
    pub fn validate(&self) -> Result<(), String> {
        if !valid_embedded_source_name(&self.name) {
            return Err(format!(
                "[[embedded_source]] name `{}` must be portable lowercase kebab-case, 1..64 bytes, exactly one normal path component, and not a Windows device name",
                self.name
            ));
        }
        if !is_public_credential_free_https(&self.url) {
            return Err(format!(
                "[[embedded_source]] `{}` url must be a public credential-free HTTPS URL with no query or fragment",
                self.name
            ));
        }
        if !is_full_lowercase_git_oid(&self.commit) {
            return Err(format!(
                "[[embedded_source]] `{}` commit must be a full 40- or 64-character lowercase hexadecimal Git object id",
                self.name
            ));
        }
        if !is_portable_tree_hash(self.content_hash.as_str()) {
            return Err(format!(
                "[[embedded_source]] `{}` content_hash must be a full lowercase `sha256-tree/1:<64-hex>` digest",
                self.name
            ));
        }
        if let Some(reference) = &self.ref_hint
            && !is_full_git_ref(reference)
        {
            return Err(format!(
                "[[embedded_source]] `{}` ref_hint must be a full safe `refs/...` name",
                self.name
            ));
        }
        if !is_spdx_expression(&self.upstream_license) {
            return Err(format!(
                "[[embedded_source]] `{}` upstream_license must be a non-empty SPDX expression",
                self.name
            ));
        }
        if declarant_path(&self.license_path).is_err() {
            return Err(format!(
                "[[embedded_source]] `{}` license_path must be a portable source-relative file path",
                self.name
            ));
        }
        if !is_public_credential_free_https(&self.license_url)
            || !self
                .license_url
                .split('/')
                .any(|segment| segment == self.commit)
        {
            return Err(format!(
                "[[embedded_source]] `{}` license_url must be a credential-free HTTPS URL pinned by an exact `{}` path segment",
                self.name, self.commit
            ));
        }
        Ok(())
    }
}

pub(super) fn valid_embedded_source_name(name: &str) -> bool {
    if !name.is_ascii() || name.is_empty() || name.len() > 64 || is_windows_device_name(name) {
        return false;
    }
    let mut segment_len = 0usize;
    for byte in name.bytes() {
        if byte.is_ascii_lowercase() || byte.is_ascii_digit() {
            segment_len += 1;
        } else if byte == b'-' && segment_len > 0 {
            segment_len = 0;
        } else {
            return false;
        }
    }
    segment_len > 0
}

fn is_public_credential_free_https(value: &str) -> bool {
    if value.trim() != value
        || value.bytes().any(|byte| byte.is_ascii_whitespace())
        || value.chars().any(|ch| matches!(ch, '\\' | '?' | '#'))
    {
        return false;
    }
    let Some(rest) = value.strip_prefix("https://") else {
        return false;
    };
    let authority_end = rest.find('/').unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    !authority.is_empty() && !authority.contains('@')
}

fn is_full_lowercase_git_oid(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_portable_tree_hash(value: &str) -> bool {
    value.strip_prefix("sha256-tree/1:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn is_full_git_ref(value: &str) -> bool {
    let Some(tail) = value.strip_prefix("refs/") else {
        return false;
    };
    !tail.is_empty()
        && !value.ends_with('/')
        && !value.ends_with('.')
        && !value.contains("..")
        && !value.contains("@{")
        && !value.contains("//")
        && !value
            .chars()
            .any(|ch| ch.is_control() || ch.is_whitespace() || "~^:?*[\\".contains(ch))
}

fn is_spdx_expression(value: &str) -> bool {
    value.trim() == value
        && !value.is_empty()
        && value.is_ascii()
        && value.chars().all(|ch| {
            ch.is_ascii_alphanumeric() || matches!(ch, '-' | '.' | '+' | ':' | '(' | ')' | ' ')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declaration() -> EmbeddedSourceDecl {
        EmbeddedSourceDecl {
            name: "upstream".into(),
            kind: EmbeddedSourceKind::Git,
            url: "https://github.com/example/upstream.git".into(),
            commit: "0123456789abcdef0123456789abcdef01234567".into(),
            content_hash: ContentHash::from_validated(format!("sha256-tree/1:{}", "a".repeat(64))),
            ref_hint: Some("refs/tags/v1.2.3".into()),
            auth: EmbeddedSourceAuth::None,
            upstream_license: "MIT OR Apache-2.0".into(),
            license_path: "LICENSE".into(),
            license_url: format!(
                "https://github.com/example/upstream/blob/{}/LICENSE",
                "0123456789abcdef0123456789abcdef01234567"
            ),
        }
    }

    #[test]
    fn valid_reference_source_passes_and_round_trips() {
        let source = declaration();
        source.validate().unwrap();
        let text = toml::to_string(&source).unwrap();
        let back: EmbeddedSourceDecl = toml::from_str(&text).unwrap();
        assert_eq!(source, back);
    }

    #[test]
    fn reference_source_boundaries_are_closed() {
        let mut source = declaration();
        source.url = "https://token@example.com/repo.git".into();
        assert!(source.validate().unwrap_err().contains("credential-free"));

        source = declaration();
        source.commit = "ABCDEF0123456789ABCDEF0123456789ABCDEF01".into();
        assert!(source.validate().unwrap_err().contains("lowercase"));

        source = declaration();
        source.ref_hint = Some("main".into());
        assert!(source.validate().unwrap_err().contains("refs/..."));

        source = declaration();
        source.upstream_license.clear();
        assert!(source.validate().unwrap_err().contains("SPDX"));
    }
}
