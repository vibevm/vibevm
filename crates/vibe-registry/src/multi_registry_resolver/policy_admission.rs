//! A registry the effective source policy disables is absent from the walk.
//!
//! `exclude_hosting` is a decision about *enumeration*, not only about
//! contact: a registry whose hosting the policy excludes is skipped as if it
//! were not declared at all, so resolution never reaches the HTTP-admission
//! refusal that would otherwise end the whole operation
//! (PROP-073 `#SOURCE-POLICY`, `#HTTP-ADMISSION-LAW`). The refusal in
//! [`crate::http_admission`] stays exactly where it is — it is the last line
//! of defence for anything that still gets that far, and this module's job is
//! to make sure resolution never does.
//!
//! The registry rows themselves are left untouched: the operator keeps the
//! default `github` / `gitverse` entries in `registry.toml` and the policy
//! genuinely switches them off.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY");

use vibe_core::manifest::{AuthKind, EffectiveSourcePolicy, RegistrySection};
use vibe_core::remap::endpoint::{EndpointObservation, observe_endpoint};

/// The mark a listing prints beside a registry the source policy disables.
/// The registry stays declared — nothing here suggests removing it.
pub const DISABLED_BY_POLICY_MARK: &str = "disabled by site policy (exclude_hosting) — not used";

/// One declared registry the effective source policy disables.
#[derive(Debug, Clone, PartialEq, Eq)]
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY-LAW")]
pub struct DisabledRegistry {
    /// The declared `[[registry]]` alias.
    pub name: String,
    /// The declared organization-root URL, as written.
    pub url: String,
    /// The contact host the URL spells, when it spells one.
    pub host: Option<String>,
    /// The `exclude_hosting` selector that disabled it.
    pub selector: String,
    /// The row's declared auth regime, for the walk report.
    pub auth: AuthKind,
}

impl DisabledRegistry {
    /// The skip note resolution says once per run, never once per attempt.
    ///
    /// ```
    /// use vibe_core::manifest::{EffectiveSourcePolicy, HostingSelector, SourcePolicyDecl};
    /// use vibe_registry::multi_registry_resolver::disabled_registry;
    ///
    /// let decl = SourcePolicyDecl {
    ///     prefer_hosting: None,
    ///     exclude_hosting: vec![HostingSelector::Github],
    /// };
    /// let empty = SourcePolicyDecl::default();
    /// let policy = EffectiveSourcePolicy::merge(&decl, &empty, &empty).unwrap();
    /// let row: vibe_core::manifest::RegistrySection =
    ///     toml::from_str("name='vibespecs'\nurl='https://github.com/vibespecs'").unwrap();
    /// let disabled = disabled_registry(&policy, &row).unwrap();
    /// assert_eq!(
    ///     disabled.skip_note(),
    ///     "registry vibespecs (github.com) skipped: excluded by site policy"
    /// );
    /// ```
    pub fn skip_note(&self) -> String {
        match &self.host {
            Some(host) => format!(
                "registry {} ({host}) skipped: excluded by site policy",
                self.name
            ),
            None => format!("registry {} skipped: excluded by site policy", self.name),
        }
    }

    /// The listing line: the mark plus the exclusion that produced it.
    pub fn listing_mark(&self) -> String {
        format!("{DISABLED_BY_POLICY_MARK} [{}]", self.selector)
    }
}

/// Whether this declared registry is disabled by the effective policy, and if
/// so by which exclusion. Pure observation of the declared URL — no catalog is
/// opened, no index probed, no host resolved.
pub fn disabled_registry(
    policy: &EffectiveSourcePolicy,
    row: &RegistrySection,
) -> Option<DisabledRegistry> {
    let selector = policy.excluded_by(&row.url)?;
    Some(DisabledRegistry {
        name: row.name.clone(),
        url: row.url.clone(),
        host: match observe_endpoint(&row.url) {
            EndpointObservation::Network(authority) => Some(authority.host),
            _ => None,
        },
        selector: selector.to_string(),
        auth: row.auth.unwrap_or(AuthKind::None),
    })
}

/// The effective primary (PROP-002 `#registry-layers`, PROP-019
/// `#site-profile`): declared precedence is unchanged, but a registry the
/// policy disables is absent from the walk, so it can never be primary in
/// practice. The effective primary is the first *allowed* registry.
pub fn effective_primary<'a>(
    policy: &EffectiveSourcePolicy,
    rows: impl IntoIterator<Item = &'a RegistrySection>,
) -> Option<&'a RegistrySection> {
    rows.into_iter()
        .find(|row| row.enabled && !policy.excludes_endpoint(&row.url))
}

#[cfg(test)]
#[path = "policy_admission/tests.rs"]
mod tests;
