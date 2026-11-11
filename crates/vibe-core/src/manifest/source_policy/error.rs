//! Typed policy failures; never infer excluded endpoint availability.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY");

/// Typed source-policy refusal; every message cites its REQ and fix surface.
///
/// ```
/// use vibe_core::manifest::{RegistryPin, SourcePolicyError};
///
/// let err = RegistryPin::new(" team ".into()).unwrap_err();
/// assert_eq!(err, SourcePolicyError::InvalidRegistryPin);
/// assert!(err.to_string().contains("PROP-073#SOURCE-POLICY-LAW"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY-LAW")]
pub enum SourcePolicyError {
    #[error(
        "source_policy_duplicate_registry: duplicate alias {registry} in {scope} catalog (violates spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY-LAW; fix: give each registry one unambiguous alias within its scope)"
    )]
    DuplicateRegistry {
        registry: String,
        scope: &'static str,
    },
    #[error(
        "source_policy_source_conflict: {package} has a registry pin and a {kind:?} source declaration (violates spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY-LAW; fix: choose one source authority on that requirement)"
    )]
    DirectSourceConflict {
        package: String,
        kind: crate::manifest::SourceKind,
    },
    #[error(
        "source_policy_source_conflict: {package} has a named registry pin under an explicit local-registry override (violates spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY-LAW; fix: remove --registry or the incompatible named pin)"
    )]
    ExplicitLocalConflict { package: String },
    #[error(
        "source_policy_resource_limit: candidate count {actual} exceeds {limit} (violates spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY-LAW; fix: narrow the requested source domain or use a separately qualified operation bound)"
    )]
    ResourceLimit { actual: usize, limit: usize },
    #[error(
        "invalid registry pin (violates spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY-LAW; fix: use a nonempty exact registry alias without surrounding whitespace)"
    )]
    InvalidRegistryPin,
    #[error(
        "invalid hosting selector (violates spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY-LAW; fix: use github, gitverse or host:<hostname-or-IP>)"
    )]
    InvalidHostingSelector,
    #[error(
        "duplicate hosting selector in one policy declaration (violates spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY-LAW; fix: name each selector once in each list)"
    )]
    DuplicateHostingSelector,
    #[error(
        "source_policy_unknown_registry: {package} pins unknown registry {registry} (violates spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY-LAW; fix: declare the registry in the operation catalog or correct the pin)"
    )]
    UnknownRegistry { package: String, registry: String },
    #[error(
        "source_policy_disabled_registry: {package} pins disabled registry {registry} (violates spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY-LAW; fix: enable the pinned registry or explicitly change the requirement)"
    )]
    DisabledRegistry { package: String, registry: String },
    #[error(
        "source_policy_conflicting_pins: {package} requires both {first} and {second} (violates spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY-LAW; fix: reconcile the active requiring edges)"
    )]
    ConflictingPins {
        package: String,
        first: String,
        second: String,
    },
    #[error(
        "source_policy_locked_conflict: {package} pins {requested}, held registry is {held:?} (violates spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW; fix: retain the locked source or use a separately authorized relock operation)"
    )]
    LockedConflict {
        package: String,
        requested: String,
        held: Option<String>,
    },
    /// The admitted registries are named beside the excluded endpoints: a
    /// refusal has to say what WAS asked, not only what was not, or the
    /// operator cannot tell an empty allowed set from a genuine absence
    /// (PROP-073 `#EXCLUDED-REGISTRY-ENUMERATION`). The excluded registries
    /// stay declared; nothing here proposes removing them.
    #[error(
        "source_host_excluded: no admitted registry offers {package}; allowed registries: {}; its remaining candidate sources are on excluded hosting ({}) and were not contacted (violates spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY-LAW; fix: admit that hosting for this operation, or declare a registry outside the excluded hosting that serves {package})",
        if .admitted.is_empty() { "(none — the source policy disables every configured registry)".to_owned() } else { .admitted.join(", ") },
        .endpoints.join(", ")
    )]
    ExcludedOnly {
        package: String,
        endpoints: Vec<String>,
        /// The registries that were in play, by name; empty when the policy
        /// disables every configured registry.
        admitted: Vec<String>,
    },
}

impl SourcePolicyError {
    /// Stable code for command/host diagnostics; consumers never parse Display.
    pub fn code(&self) -> &'static str {
        match self {
            Self::DuplicateRegistry { .. } => "source_policy_duplicate_registry",
            Self::DirectSourceConflict { .. } | Self::ExplicitLocalConflict { .. } => {
                "source_policy_source_conflict"
            }
            Self::ResourceLimit { .. } => "source_policy_resource_limit",
            Self::InvalidRegistryPin => "source_policy_invalid_pin",
            Self::InvalidHostingSelector => "source_policy_invalid_hosting",
            Self::DuplicateHostingSelector => "source_policy_duplicate_hosting",
            Self::UnknownRegistry { .. } => "source_policy_unknown_registry",
            Self::DisabledRegistry { .. } => "source_policy_disabled_registry",
            Self::ConflictingPins { .. } => "source_policy_conflicting_pins",
            Self::LockedConflict { .. } => "source_policy_locked_conflict",
            Self::ExcludedOnly { .. } => "source_host_excluded",
        }
    }
}
