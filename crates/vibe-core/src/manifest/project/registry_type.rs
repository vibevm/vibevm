//! `[[registry]].type` — which backend serves a registry and how publishing
//! reaches it (PROP-002 `#registry-type`).
//!
//! The type is optional on every declaration layer. When it is absent the
//! backend is inferred from the URL exactly as before the field existed, so
//! every existing `vibe.toml` and `registry.toml` keeps its meaning.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-type");

use serde::{Deserialize, Serialize};
use specmark::spec;

use super::{NamingConvention, RegistrySection, default_ref};

/// The backend family of one `[[registry]]` declaration.
///
/// Spelled case-insensitively on input (`GitVerse`, `gitverse` and `GITVERSE`
/// are one value) and always written lower-case.
///
/// ```
/// use vibe_core::manifest::RegistryType;
///
/// assert_eq!(RegistryType::parse("GitVerse"), Some(RegistryType::Gitverse));
/// assert_eq!(RegistryType::parse(" LocalDir "), Some(RegistryType::Localdir));
/// assert_eq!(RegistryType::parse("svn"), None);
/// assert_eq!(RegistryType::Monorepo.as_str(), "monorepo");
/// // Inference never yields `monorepo`: an org URL and a single-repository
/// // URL look alike over SSH.
/// assert_eq!(RegistryType::infer("ssh://git@git.corp/tools/all.git"), RegistryType::Git);
/// assert_eq!(RegistryType::infer("https://github.com/vibespecs"), RegistryType::Github);
/// assert_eq!(RegistryType::infer("file:///srv/vibe"), RegistryType::Localdir);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegistryType {
    /// Per-package repositories in a GitHub organisation; publishing may
    /// create repositories through the GitHub API.
    Github,
    /// Per-package repositories in a GitVerse organisation, on `gitverse.ru`
    /// or an on-premises GitVerse under any domain.
    Gitverse,
    /// Per-package repositories on any Git server; publishing pushes only
    /// into repositories that already exist and never calls an API.
    Git,
    /// One repository holds every package in the directory layout
    /// `<group>/<name>/v<version>/` on the registry's `ref`.
    Monorepo,
    /// A plain directory laid out `<group>/<name>/v<version>/`, read and
    /// written straight on the filesystem (a local disk or a network share).
    Localdir,
}

impl RegistryType {
    /// Every value, in documentation order.
    pub const ALL: [RegistryType; 5] = [
        RegistryType::Github,
        RegistryType::Gitverse,
        RegistryType::Git,
        RegistryType::Monorepo,
        RegistryType::Localdir,
    ];

    /// The stored (lower-case) spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            RegistryType::Github => "github",
            RegistryType::Gitverse => "gitverse",
            RegistryType::Git => "git",
            RegistryType::Monorepo => "monorepo",
            RegistryType::Localdir => "localdir",
        }
    }

    /// Case-insensitive parse; surrounding whitespace is ignored.
    pub fn parse(input: &str) -> Option<Self> {
        let wanted = input.trim();
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.as_str().eq_ignore_ascii_case(wanted))
    }

    /// The type a declaration without `type` gets: `file:` → `localdir`,
    /// host `github.com` → `github`, host `gitverse.ru` → `gitverse`, any
    /// other URL → `git`. `monorepo` is never inferred.
    pub fn infer(url: &str) -> Self {
        let trimmed = url.trim();
        if trimmed.to_ascii_lowercase().starts_with("file:") {
            return RegistryType::Localdir;
        }
        match super::registry_host(trimmed).map(str::to_ascii_lowercase) {
            Some(host) if host == "github.com" || host.ends_with(".github.com") => {
                RegistryType::Github
            }
            Some(host) if host == "gitverse.ru" || host.ends_with(".gitverse.ru") => {
                RegistryType::Gitverse
            }
            _ => RegistryType::Git,
        }
    }

    /// Whether the registry is read through Git (every type but `localdir`).
    pub const fn is_git(self) -> bool {
        !matches!(self, RegistryType::Localdir)
    }

    /// The accepted spellings, for diagnostics.
    pub fn expected_list() -> String {
        Self::ALL
            .iter()
            .map(|value| format!("`{}`", value.as_str()))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl std::fmt::Display for RegistryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for RegistryType {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::parse(input).ok_or_else(|| unknown_type_message(input))
    }
}

/// The refusal text for an unknown `type` value.
pub(crate) fn unknown_type_message(input: &str) -> String {
    format!(
        "unknown registry type `{input}` (violates \
         spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-type; \
         fix: use one of {} — spelling is case-insensitive — or omit `type` to infer it \
         from the URL)",
        RegistryType::expected_list()
    )
}

impl Serialize for RegistryType {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for RegistryType {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Self::parse(&raw).ok_or_else(|| serde::de::Error::custom(unknown_type_message(&raw)))
    }
}

/// Why a declared `type` contradicts the declaration's URL.
///
/// ```
/// use vibe_core::manifest::{RegistrySection, RegistryTypeError};
///
/// let row: RegistrySection = toml::from_str(
///     "name='corp'\ntype='localdir'\nurl='ssh://git@git.corp/vibe.git'\n",
/// ).unwrap();
/// let err = row.checked_type().unwrap_err();
/// assert!(matches!(err, RegistryTypeError::LocaldirNeedsPath { .. }));
/// assert!(err.to_string().contains("#registry-type"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-type")]
pub enum RegistryTypeError {
    #[error(
        "registry `{name}` declares `type = \"localdir\"` but its url `{url}` is a remote \
         address (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-type; \
         fix: point `url` at the directory as `file:///…` or a plain filesystem path, or \
         choose `type = \"monorepo\"` / `\"git\"` for a Git server)"
    )]
    LocaldirNeedsPath { name: String, url: String },
}

impl RegistrySection {
    /// A declaration with only the two required fields; every other field
    /// takes its serde default.
    ///
    /// ```
    /// use vibe_core::manifest::{RegistrySection, RegistryType};
    ///
    /// let row = RegistrySection::new("corp", "file:///srv/vibe");
    /// assert_eq!(row.r#ref, "main");
    /// assert_eq!(row.effective_type(), RegistryType::Localdir);
    /// ```
    pub fn new(name: impl Into<String>, url: impl Into<String>) -> Self {
        RegistrySection {
            name: name.into(),
            r#type: None,
            url: url.into(),
            r#ref: default_ref(),
            naming: NamingConvention::default(),
            auth: None,
            token_env: None,
            enabled: true,
            index_url: None,
        }
    }

    /// The backend family serving this registry: the declared `type`, or
    /// the one [`RegistryType::infer`] reads off the URL.
    pub fn effective_type(&self) -> RegistryType {
        self.r#type
            .unwrap_or_else(|| RegistryType::infer(&self.url))
    }

    /// [`Self::effective_type`], refused when the declared type contradicts
    /// the URL (a `localdir` registry must name a directory).
    pub fn checked_type(&self) -> std::result::Result<RegistryType, RegistryTypeError> {
        let kind = self.effective_type();
        if kind == RegistryType::Localdir && !crate::url_is_local(&self.url) {
            return Err(RegistryTypeError::LocaldirNeedsPath {
                name: self.name.clone(),
                url: self.url.clone(),
            });
        }
        Ok(kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_value_round_trips_in_any_case() {
        for value in RegistryType::ALL {
            for spelling in [
                value.as_str().to_string(),
                value.as_str().to_ascii_uppercase(),
                format!(
                    "{}{}",
                    value.as_str()[..1].to_ascii_uppercase(),
                    &value.as_str()[1..]
                ),
            ] {
                assert_eq!(RegistryType::parse(&spelling), Some(value), "{spelling}");
                let parsed: RegistryType = spelling.parse().unwrap();
                assert_eq!(parsed, value);
            }
        }
    }

    #[test]
    fn inference_table() {
        for (url, want) in [
            ("https://github.com/vibespecs", RegistryType::Github),
            ("git@github.com:vibespecs", RegistryType::Github),
            ("ssh://git@github.com/vibespecs", RegistryType::Github),
            ("https://gitverse.ru/vibespecs", RegistryType::Gitverse),
            ("git@gitverse.ru:vibespecs", RegistryType::Gitverse),
            ("https://GitVerse.RU/vibespecs", RegistryType::Gitverse),
            ("file:///srv/vibe", RegistryType::Localdir),
            ("FILE:///C:/vibe", RegistryType::Localdir),
            (
                "ssh://git@git.corp.example/tools/all.git",
                RegistryType::Git,
            ),
            ("https://gitverse.corp.example/org", RegistryType::Git),
            ("/bare/path/is/a/git/repo", RegistryType::Git),
            ("git+file:///srv/bare.git", RegistryType::Git),
        ] {
            assert_eq!(RegistryType::infer(url), want, "url = {url}");
        }
    }

    #[test]
    fn unknown_value_names_the_choices() {
        let message = "svn".parse::<RegistryType>().unwrap_err();
        assert!(message.contains("`svn`"), "{message}");
        assert!(message.contains("`monorepo`"), "{message}");
        assert!(message.contains("case-insensitive"), "{message}");
    }
}
