//! Operation-owned registry constraints and exact managed destination policy.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY");

use crate::remap::endpoint::{EndpointObservation, normalized_host_key, observe_endpoint};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeSet;
use std::fmt;

mod catalog;
pub use catalog::{
    validate_locked_registry_pin, validate_new_registry_pin, validate_registry_aliases,
};
mod binding;
pub use binding::CanonicalSourceBinding;
mod error;
pub use error::SourcePolicyError;

/// A canonical registry alias on one requiring edge, never a new registry grant.
///
/// ```
/// use vibe_core::manifest::RegistryPin;
/// let pin = RegistryPin::new("team".into()).unwrap();
/// assert_eq!(pin.as_str(), "team");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RegistryPin(String);
impl RegistryPin {
    pub fn new(value: String) -> Result<Self, SourcePolicyError> {
        if value.is_empty() || value.trim() != value || value.chars().any(char::is_control) {
            return Err(SourcePolicyError::InvalidRegistryPin);
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for RegistryPin {
    type Error = SourcePolicyError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<RegistryPin> for String {
    fn from(value: RegistryPin) -> Self {
        value.0
    }
}
impl fmt::Display for RegistryPin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// One explicit hosting family or an exact normalized hostname selector.
/// Known families cover only verified managed service hosts, never suffixes.
///
/// ```
/// use vibe_core::manifest::HostingSelector;
/// let host = HostingSelector::try_from("host:EXAMPLE.invalid.".to_owned()).unwrap();
/// assert_eq!(host.to_string(), "host:example.invalid");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum HostingSelector {
    Github,
    Gitverse,
    Host(String),
}
impl TryFrom<String> for HostingSelector {
    type Error = SourcePolicyError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "github" => Ok(Self::Github),
            "gitverse" => Ok(Self::Gitverse),
            other => other
                .strip_prefix("host:")
                .and_then(normalized_host_key)
                .map(Self::Host)
                .ok_or(SourcePolicyError::InvalidHostingSelector),
        }
    }
}
impl fmt::Display for HostingSelector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Github => f.write_str("github"),
            Self::Gitverse => f.write_str("gitverse"),
            Self::Host(host) => write!(f, "host:{host}"),
        }
    }
}
impl From<HostingSelector> for String {
    fn from(value: HostingSelector) -> Self {
        value.to_string()
    }
}
impl HostingSelector {
    /// Exact destination keys consumed by transport admission, without DNS.
    pub fn host_keys(&self) -> Vec<&str> {
        match self {
            Self::Github => vec![
                "github.com",
                "ssh.github.com",
                "api.github.com",
                "raw.githubusercontent.com",
                "codeload.github.com",
                "objects.githubusercontent.com",
                "objects-origin.githubusercontent.com",
                "github-releases.githubusercontent.com",
                "github-registry-files.githubusercontent.com",
                "release-assets.githubusercontent.com",
            ],
            Self::Gitverse => vec!["gitverse.ru", "api.gitverse.ru"],
            Self::Host(host) => vec![host],
        }
    }
    pub fn matches_endpoint(&self, address: &str) -> bool {
        let EndpointObservation::Network(authority) = observe_endpoint(address) else {
            return false;
        };
        self.host_keys().contains(&authority.host.as_str())
    }
}

/// Authored operation policy; dependency manifests do not import this policy.
/// Absence and an explicit empty preference have different merge semantics.
///
/// ```
/// use vibe_core::manifest::SourcePolicyDecl;
/// let policy: SourcePolicyDecl = toml::from_str("prefer_hosting=[]\nexclude_hosting=['github']").unwrap();
/// assert_eq!(policy.prefer_hosting, Some(vec![]));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePolicyDecl {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_preference"
    )]
    pub prefer_hosting: Option<Vec<HostingSelector>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude_hosting: Vec<HostingSelector>,
}
fn present_preference<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Vec<HostingSelector>>, D::Error> {
    Vec::<HostingSelector>::deserialize(deserializer).map(Some)
}
impl SourcePolicyDecl {
    pub fn is_empty(&self) -> bool {
        self.prefer_hosting.is_none() && self.exclude_hosting.is_empty()
    }

    /// This declaration layered over a lower one (`PROP-019#site-profile`):
    /// the higher explicit preference replaces the lower one, exclusions
    /// accumulate (each selector once, higher layer first) — the same law
    /// [`EffectiveSourcePolicy::merge`] applies between its layers.
    ///
    /// ```
    /// use vibe_core::manifest::{HostingSelector, SourcePolicyDecl};
    /// let site = SourcePolicyDecl {
    ///     prefer_hosting: Some(vec![HostingSelector::Gitverse]),
    ///     exclude_hosting: vec![HostingSelector::Github, HostingSelector::Gitverse],
    /// };
    /// let user = SourcePolicyDecl { prefer_hosting: None, exclude_hosting: vec![HostingSelector::Github] };
    /// let merged = user.over(&site);
    /// assert_eq!(merged.prefer_hosting, Some(vec![HostingSelector::Gitverse]));
    /// assert_eq!(merged.exclude_hosting, vec![HostingSelector::Github, HostingSelector::Gitverse]);
    /// ```
    pub fn over(&self, lower: &SourcePolicyDecl) -> SourcePolicyDecl {
        let mut exclude_hosting = self.exclude_hosting.clone();
        for selector in &lower.exclude_hosting {
            if !exclude_hosting.contains(selector) {
                exclude_hosting.push(selector.clone());
            }
        }
        SourcePolicyDecl {
            prefer_hosting: self
                .prefer_hosting
                .clone()
                .or_else(|| lower.prefer_hosting.clone()),
            exclude_hosting,
        }
    }
    fn validate(&self) -> Result<(), SourcePolicyError> {
        for values in [
            self.prefer_hosting.as_deref().unwrap_or(&[]),
            self.exclude_hosting.as_slice(),
        ] {
            let mut seen = BTreeSet::new();
            if values.iter().any(|value| !seen.insert(value)) {
                return Err(SourcePolicyError::DuplicateHostingSelector);
            }
            // Public enum construction cannot bypass host-key validation.
            for value in values {
                if let HostingSelector::Host(host) = value
                    && normalized_host_key(host).as_deref() != Some(host.as_str())
                {
                    return Err(SourcePolicyError::InvalidHostingSelector);
                }
            }
        }
        Ok(())
    }
}

/// Immutable normalized policy; source ranking is used only at a fixed version.
///
/// ```
/// use vibe_core::manifest::{EffectiveSourcePolicy, HostingSelector, SourcePolicyDecl};
///
/// let user = SourcePolicyDecl {
///     prefer_hosting: Some(vec![HostingSelector::Gitverse]),
///     exclude_hosting: vec![HostingSelector::Github],
/// };
/// let empty = SourcePolicyDecl::default();
/// let policy = EffectiveSourcePolicy::merge(&user, &empty, &empty).unwrap();
/// assert!(policy.has_preference());
/// assert!(policy.denied_host_keys().contains("github.com"));
/// assert_eq!(policy.preference_rank("https://gitverse.ru/org/pkg.git"), 0);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EffectiveSourcePolicy {
    preference: Vec<HostingSelector>,
    excluded: BTreeSet<HostingSelector>,
}
impl EffectiveSourcePolicy {
    /// Replace preference by the highest explicit layer; union exclusions.
    ///
    /// ```
    /// use vibe_core::manifest::{SourcePolicyDecl, EffectiveSourcePolicy};
    /// let empty = SourcePolicyDecl::default();
    /// let policy = EffectiveSourcePolicy::merge(&empty, &empty, &empty).unwrap();
    /// assert!(policy.denied_host_keys().is_empty());
    /// ```
    pub fn merge(
        user: &SourcePolicyDecl,
        project: &SourcePolicyDecl,
        invocation: &SourcePolicyDecl,
    ) -> Result<Self, SourcePolicyError> {
        for declaration in [user, project, invocation] {
            declaration.validate()?;
        }
        let preference = invocation
            .prefer_hosting
            .as_ref()
            .or(project.prefer_hosting.as_ref())
            .or(user.prefer_hosting.as_ref())
            .cloned()
            .unwrap_or_default();
        let excluded = user
            .exclude_hosting
            .iter()
            .chain(&project.exclude_hosting)
            .chain(&invocation.exclude_hosting)
            .cloned()
            .collect();
        Ok(Self {
            preference,
            excluded,
        })
    }
    pub fn denied_host_keys(&self) -> BTreeSet<String> {
        self.excluded
            .iter()
            .flat_map(HostingSelector::host_keys)
            .map(str::to_owned)
            .collect()
    }
    pub fn preference_rank(&self, actual_source: &str) -> usize {
        self.preference
            .iter()
            .position(|selector| selector.matches_endpoint(actual_source))
            .unwrap_or(self.preference.len())
    }
    pub fn has_preference(&self) -> bool {
        !self.preference.is_empty()
    }
}

#[cfg(test)]
#[path = "source_policy/tests.rs"]
mod tests;

impl std::str::FromStr for HostingSelector {
    type Err = SourcePolicyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::try_from(value.to_owned())
    }
}
