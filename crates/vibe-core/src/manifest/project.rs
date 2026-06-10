//! Consumer-side sections of a `vibe.toml` manifest.
//!
//! Every node — a plain project, a workspace member, a published package —
//! may carry consumer-side configuration: `[project]` identity, registries,
//! mirrors, overrides, the active stack, and LLM settings. The types here are
//! the building blocks of that role; they are assembled into the unified
//! [`Manifest`](super::Manifest) document, which owns the file I/O.
//!
//! Registries are a priority-ordered `[[registry]]` array, with optional
//! `[[mirror]]` entries for transparent fallback and `[[override]]` entries
//! that bypass the registry layer for specific pkgrefs. Schema:
//! `VIBEVM-SPEC.md` §7.5, [PROP-002 §2.2 / §2.3 / §2.4](../../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md).

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#registry-model");

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// `[project]` — the identity of a non-publishable consumer node.
///
/// A `vibe.toml` carrying this table is a plain project; one carrying
/// `[package]` is a publishable artifact. The two are mutually exclusive —
/// see [`Manifest::validate`](super::Manifest::validate).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectSection {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub authors: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActiveSection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LlmSection {
    pub default_provider: String,
    pub default_model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
}

/// A single entry in `[[registry]]` — an organization-root URL plus the
/// naming convention that maps pkgrefs to per-package repos under it,
/// plus the authentication regime to use when fetching from it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistrySection {
    /// Local alias — used in lockfile `registry` field and to target
    /// `[[mirror]]` / `[[override]]` entries at this registry.
    pub name: String,

    /// Organization-root URL. Generic git URL — any scheme `git` accepts
    /// (`git@host:…`, `ssh://…`, `https://…`, `file://…`).
    pub url: String,

    /// Registry-level ref. Reserved for a future registry-level metadata
    /// branch (capability index, trust policy); not consumed by install
    /// today. Defaults to `main`.
    #[serde(default = "default_ref", skip_serializing_if = "is_default_ref")]
    pub r#ref: String,

    /// Convention mapping a `<kind>:<name>` pkgref to a package repo name
    /// under `url`.
    #[serde(default, skip_serializing_if = "NamingConvention::is_default")]
    pub naming: NamingConvention,

    /// Authentication regime for fetching from this registry. See
    /// [PROP-002 §2.2.1](../../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#registry-auth).
    /// Default `none`: public read, no credential prompts in scripted runs,
    /// 401 → walk to next registry.
    #[serde(default, skip_serializing_if = "AuthKind::is_default")]
    pub auth: AuthKind,

    /// Override env-var name for `auth = "token-env"`. Default
    /// (when omitted) is derived from the registry host —
    /// `VIBEVM_REGISTRY_TOKEN_<HOST_UPPER>` with dots and hyphens
    /// converted to underscores.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_env: Option<String>,
}

/// Authentication regime per `[[registry]]`. See PROP-002 §2.2.1 for
/// the full semantics matrix; this enum is the schema-level encoding.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthKind {
    /// Public read-only. No credentials sent. In non-TTY / `--unattended`
    /// runs, credential helpers and terminal prompts are silenced so a
    /// 401 / 403 response classifies as `UnknownPackage` and the walk
    /// continues to the next registry. Default.
    #[default]
    #[serde(rename = "none")]
    None,
    /// Token from env-var (default name derived from host, override via
    /// `token_env`). Token is injected into the URL as
    /// `https://x-access-token:<TOKEN>@host/...` for the duration of
    /// each git invocation; never logged, never written to lockfile.
    #[serde(rename = "token-env")]
    TokenEnv,
    /// Opt-in: respect the system git `credential.helper` / `core.askPass`.
    /// On an interactive TTY without `--unattended` a GUI prompt (GCM,
    /// keychain, etc.) may appear; in scripted runs this collapses to
    /// the same behaviour as `None`.
    #[serde(rename = "credential-helper")]
    CredentialHelper,
    /// SSH-based fetch — URL must be ssh-form (`git@host:org`,
    /// `ssh://...`). Authentication delegated to ssh-agent / system
    /// keys; vibe does not touch ssh config.
    #[serde(rename = "ssh")]
    Ssh,
}

impl AuthKind {
    pub fn is_default(&self) -> bool {
        matches!(self, AuthKind::None)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            AuthKind::None => "none",
            AuthKind::TokenEnv => "token-env",
            AuthKind::CredentialHelper => "credential-helper",
            AuthKind::Ssh => "ssh",
        }
    }
}

impl RegistrySection {
    /// Resolve the env-var name vibe should consult for the token under
    /// `auth = "token-env"`. Per PROP-002 §2.2.1: explicit `token_env`
    /// wins; otherwise the name is derived from the registry's host —
    /// `VIBEVM_REGISTRY_TOKEN_<HOST_UPPER>` with `.` and `-` mapped to
    /// `_`. Returns `None` when the URL has no parseable host.
    pub fn resolve_token_env_name(&self) -> Option<String> {
        if let Some(explicit) = &self.token_env {
            return Some(explicit.clone());
        }
        let host = registry_host(&self.url)?;
        let mut sanitised = String::with_capacity(host.len());
        for ch in host.chars() {
            match ch {
                '.' | '-' => sanitised.push('_'),
                other if other.is_ascii_alphanumeric() || other == '_' => {
                    sanitised.push(other.to_ascii_uppercase());
                }
                _ => return None,
            }
        }
        Some(format!("VIBEVM_REGISTRY_TOKEN_{sanitised}"))
    }
}

/// Best-effort host extraction from a registry URL. Handles both
/// `https://host/path` / `ssh://host/path` (URL-shape) and
/// `git@host:path` (scp-shape). Returns `None` for shapes we can't
/// extract a host from (e.g. `file://`).
fn registry_host(url: &str) -> Option<&str> {
    for prefix in ["https://", "http://", "ssh://", "git+ssh://"] {
        if let Some(rest) = url.strip_prefix(prefix) {
            return rest.split('/').next()?.split('@').next_back();
        }
    }
    // scp-shape: git@host:path
    if let Some(at_idx) = url.find('@')
        && let Some(colon_idx) = url[at_idx..].find(':')
    {
        let host_start = at_idx + 1;
        let host_end = at_idx + colon_idx;
        if host_end > host_start {
            return Some(&url[host_start..host_end]);
        }
    }
    None
}

/// Convention for mapping a pkgref to a package repository name under a
/// registry's organization URL. The convention is a property of the
/// registry, not a global rule.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum NamingConvention {
    /// `org.vibevm/wal` → `<org>/org.vibevm.wal`. The reverse-FQDN
    /// convention (PROP-008 §2.5): a flat `<group>.<name>` repo name,
    /// collision-free because `(group, name)` is unique. Default — the
    /// convention every group-aware registry uses.
    #[default]
    #[serde(rename = "fqdn")]
    Fqdn,
    /// `flow:wal` → `<org>/flow-wal`. A pre-`group` convention, kept for
    /// registries that have not adopted reverse-FQDN naming (PROP-008 §2.5).
    #[serde(rename = "kind-name")]
    KindName,
    /// `flow:wal` → `<org>/wal`. Legal only when names are globally unique
    /// across kinds within a registry.
    #[serde(rename = "name")]
    Name,
    /// `flow:wal` → `<org>/flow/wal`. Requires host support for nested
    /// repository paths (GitLab groups, Gitea orgs).
    #[serde(rename = "kind/name")]
    KindSlashName,
}

impl NamingConvention {
    pub fn is_default(&self) -> bool {
        matches!(self, NamingConvention::Fqdn)
    }

    /// Compute the repository name for a `(kind, group, name)` package
    /// under this convention.
    ///
    /// `Fqdn` uses `group` and is infallible — every group-native registry
    /// uses it. The legacy `kind-*` conventions use `kind`, which a kindless
    /// pkgref does not carry; calling them with `kind = None` is an error.
    pub fn repo_name(
        &self,
        kind: Option<crate::package_ref::PackageKind>,
        group: &crate::package_ref::Group,
        name: &str,
    ) -> Result<String> {
        match self {
            NamingConvention::Fqdn => Ok(format!("{group}.{name}")),
            NamingConvention::KindName => {
                let kind = kind.ok_or_else(|| Error::BadPackageRef {
                    input: format!("{group}/{name}"),
                    reason: "the `kind-name` naming convention needs a kind".into(),
                })?;
                Ok(format!("{}-{name}", kind.as_str()))
            }
            NamingConvention::Name => Ok(name.to_string()),
            NamingConvention::KindSlashName => {
                let kind = kind.ok_or_else(|| Error::BadPackageRef {
                    input: format!("{group}/{name}"),
                    reason: "the `kind/name` naming convention needs a kind".into(),
                })?;
                Ok(format!("{}/{name}", kind.as_str()))
            }
        }
    }
}

/// A `[[mirror]]` entry: transparent alternative URL for a specific
/// registry (or `*` for any).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MirrorSection {
    /// Target registry name (matches a `[[registry]].name`) or `"*"` for
    /// any registry.
    pub of: String,
    /// Mirror URL. Any git URL.
    pub url: String,
    /// Priority within the target registry's mirror chain — lower = tried
    /// first. Default 0.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub priority: i32,
}

/// A `[[override]]` entry: direct source pin for a specific pkgref.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverrideSection {
    /// `<kind>:<name>` — the override applies to whatever version the
    /// pinned source / ref resolves to. Version constraints belong on the
    /// source, not here.
    pub pkgref: String,
    /// Source URL (any git URL or `file://`).
    pub source_url: String,
    /// Optional explicit ref — tag, branch, or commit. Defaults to `HEAD`
    /// on the source's default branch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<String>,
    /// Human-readable reason — surfaces in `vibe list --overrides`. Empty
    /// is legal but discouraged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

// ---------------------------------------------------------------------------
// Defaults and helpers
// ---------------------------------------------------------------------------

/// Default registry URL written into every new project's `vibe.toml` by
/// `vibe init` unless the operator overrides it.
///
/// **Org root, not a per-package URL.** Per-package URLs are derived at
/// fetch time via the registry's `naming` convention (default
/// `kind-name` produces `<org>/<kind>-<name>`).
///
/// **Host: GitHub.** The `vibespecs` registry organization moved from
/// GitVerse to GitHub on 2026-04-29 because GitVerse's public REST API
/// does not expose org-scoped repo creation, blocking
/// `vibe registry publish` end-to-end automation. Migration rationale:
/// [PROP-000 §7](../../../spec/common/PROP-000.md#registry) and
/// [PROP-002 §2.10](../../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#publish).
/// The vibevm tool source itself stays on GitVerse.
pub const DEFAULT_REGISTRY_URL: &str = "https://github.com/vibespecs";

/// Default name for the primary registry written by `vibe init` into new
/// projects. Matches the `name` field callers see in `vibe.toml`.
pub const DEFAULT_REGISTRY_NAME: &str = "vibespecs";

/// Default ref on the registry URL — `main`.
pub const DEFAULT_REGISTRY_REF: &str = "main";

/// Secondary `[[registry]]` written by `vibe init` alongside the GitHub
/// primary. Different organisation, different package set: GitHub remains
/// canonical for `vibe registry publish` automation; GitVerse is queried
/// on resolve fall-through so consumers can install packages that only
/// live on GitVerse without manual setup.
pub const DEFAULT_REGISTRY_GITVERSE_URL: &str = "https://gitverse.ru/vibespecs";

/// Default name for the secondary GitVerse registry written by `vibe init`.
pub const DEFAULT_REGISTRY_GITVERSE_NAME: &str = "vibespecs-gitverse";

pub(crate) fn default_ref() -> String {
    DEFAULT_REGISTRY_REF.to_string()
}

pub(crate) fn is_default_ref(r: &String) -> bool {
    r == DEFAULT_REGISTRY_REF
}

fn is_zero(x: &i32) -> bool {
    *x == 0
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_section_rejects_unknown_field() {
        let raw = r#"
name = "r"
url = "git@host:org"
bogus = 1
"#;
        assert!(toml::from_str::<RegistrySection>(raw).is_err());
    }

    #[test]
    fn registry_section_defaults() {
        let raw = r#"
name = "vibespecs"
url = "https://github.com/vibespecs"
"#;
        let r: RegistrySection = toml::from_str(raw).unwrap();
        assert_eq!(r.r#ref, DEFAULT_REGISTRY_REF);
        assert_eq!(r.naming, NamingConvention::Fqdn);
        assert_eq!(r.auth, AuthKind::None);
        assert!(r.token_env.is_none());
        // Defaults skip on serialize — no spurious diffs.
        let rendered = toml::to_string_pretty(&r).unwrap();
        assert!(!rendered.contains("auth ="));
        assert!(!rendered.contains("naming ="));
    }

    #[test]
    fn auth_kind_variants_roundtrip() {
        for (raw_value, expected) in [
            ("none", AuthKind::None),
            ("token-env", AuthKind::TokenEnv),
            ("credential-helper", AuthKind::CredentialHelper),
            ("ssh", AuthKind::Ssh),
        ] {
            let raw = format!("name = \"r\"\nurl = \"https://x/y\"\nauth = \"{raw_value}\"\n");
            let r: RegistrySection = toml::from_str(&raw).unwrap();
            assert_eq!(r.auth, expected);
            let back: RegistrySection =
                toml::from_str(&toml::to_string_pretty(&r).unwrap()).unwrap();
            assert_eq!(r, back);
        }
    }

    #[test]
    fn auth_kind_rejects_unknown_value() {
        let raw = "name = \"r\"\nurl = \"https://x/y\"\nauth = \"bogus\"\n";
        assert!(toml::from_str::<RegistrySection>(raw).is_err());
    }

    #[test]
    fn naming_convention_repo_name() {
        use crate::package_ref::{Group, PackageKind};
        let org = Group::parse("org.vibevm").unwrap();
        assert_eq!(
            NamingConvention::Fqdn.repo_name(None, &org, "wal").unwrap(),
            "org.vibevm.wal"
        );
        assert_eq!(
            NamingConvention::KindName
                .repo_name(Some(PackageKind::Flow), &org, "wal")
                .unwrap(),
            "flow-wal"
        );
        assert_eq!(
            NamingConvention::Name
                .repo_name(Some(PackageKind::Stack), &org, "rust-cli")
                .unwrap(),
            "rust-cli"
        );
        assert_eq!(
            NamingConvention::KindSlashName
                .repo_name(Some(PackageKind::Feat), &org, "welcome-page")
                .unwrap(),
            "feat/welcome-page"
        );
        // A legacy `kind-*` convention without a kind is an error.
        assert!(
            NamingConvention::KindName
                .repo_name(None, &org, "wal")
                .is_err()
        );
    }

    #[test]
    fn resolve_token_env_name_derives_from_host() {
        let r = RegistrySection {
            name: "r".into(),
            url: "https://gitlab.company.com/vibespecs".into(),
            r#ref: "main".into(),
            naming: NamingConvention::KindName,
            auth: AuthKind::TokenEnv,
            token_env: None,
        };
        assert_eq!(
            r.resolve_token_env_name().as_deref(),
            Some("VIBEVM_REGISTRY_TOKEN_GITLAB_COMPANY_COM")
        );
    }

    #[test]
    fn resolve_token_env_name_honours_explicit_override() {
        let r = RegistrySection {
            name: "r".into(),
            url: "https://gitlab.company.com/vibespecs".into(),
            r#ref: "main".into(),
            naming: NamingConvention::KindName,
            auth: AuthKind::TokenEnv,
            token_env: Some("MY_CUSTOM_TOKEN".to_string()),
        };
        assert_eq!(
            r.resolve_token_env_name().as_deref(),
            Some("MY_CUSTOM_TOKEN")
        );
    }

    #[test]
    fn resolve_token_env_name_handles_scp_form() {
        let r = RegistrySection {
            name: "r".into(),
            url: "git@gitlab.company.com:vibespecs".into(),
            r#ref: "main".into(),
            naming: NamingConvention::KindName,
            auth: AuthKind::Ssh,
            token_env: None,
        };
        assert_eq!(
            r.resolve_token_env_name().as_deref(),
            Some("VIBEVM_REGISTRY_TOKEN_GITLAB_COMPANY_COM")
        );
    }

    #[test]
    fn resolve_token_env_name_returns_none_for_file_url() {
        let r = RegistrySection {
            name: "r".into(),
            url: "file:///tmp/registry".into(),
            r#ref: "main".into(),
            naming: NamingConvention::KindName,
            auth: AuthKind::TokenEnv,
            token_env: None,
        };
        assert!(r.resolve_token_env_name().is_none());
    }
}
