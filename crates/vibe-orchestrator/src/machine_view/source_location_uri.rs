//! Match generated source-location URI declarations to one pure owner map.
//! This publishes navigation metadata only, not a resource read capability.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SOURCE-LOCATION");

use vibe_core::machine_view::{OpaqueResourceId, OpaqueText};
use vibe_wire::generated::machine::e1::source_location::{
    SourceLocation, UriMapping, UriMappingOwnerDeclaredUnverified, UriMappingUnavailable,
    UriUnavailableReason,
};

use super::resource_uri::safe_token;
use super::{OwnerResourceUriMap, ResourceUriLookupError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceUriFault {
    InvalidScope,
    InvalidSource {
        field: &'static str,
        reason: &'static str,
    },
    PreclaimedMapping,
    ForgedPublication,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceUriFailure {
    pub original: SourceLocation,
    pub fault: SourceUriFault,
}

fn neutral_input(value: &SourceLocation) -> bool {
    value.client_uri.is_none()
        && matches!(
            &value.uri_mapping,
            UriMapping::Unavailable(row) if row.reason == UriUnavailableReason::NoOwnerMapping
        )
}

/// Publish only a neutral, unclaimed generated source location through an
/// exact owner-declared resource/revision/scope map. The domain position stays
/// unchanged; a missing lookup becomes null plus a typed unavailable reason.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SOURCE-LOCATION",
    r = 1
)]
pub fn publish_source_location_uri(
    map: Option<&OwnerResourceUriMap>,
    scope: &OpaqueText,
    original: SourceLocation,
) -> Result<SourceLocation, SourceUriFailure> {
    let checked = || -> Result<SourceLocation, SourceUriFault> {
        if !safe_token(scope.as_str()) {
            return Err(SourceUriFault::InvalidScope);
        }
        original
            .validate_machine_source_location()
            .map_err(|error| SourceUriFault::InvalidSource {
                field: error.field,
                reason: error.reason,
            })?;
        if !neutral_input(&original) {
            return Err(SourceUriFault::PreclaimedMapping);
        }
        let resource =
            OpaqueResourceId::new(original.resource_id_octets.clone()).map_err(|_| {
                SourceUriFault::InvalidSource {
                    field: "resource_id_octets",
                    reason: "resource identity is empty",
                }
            })?;
        let revision = OpaqueText::new(original.captured_revision.clone()).map_err(|_| {
            SourceUriFault::InvalidSource {
                field: "captured_revision",
                reason: "revision identity is blank",
            }
        })?;
        let mut published = original.clone();
        let outcome = map.map(|owner| owner.lookup(scope, &resource, &revision));
        match outcome {
            Some(Ok(binding)) => {
                published.client_uri = Some(binding.uri().to_owned());
                published.uri_mapping = UriMapping::OwnerDeclaredUnverified(Box::new(
                    UriMappingOwnerDeclaredUnverified {
                        authorized_scope_id: scope.as_str().to_owned(),
                    },
                ));
            }
            other => {
                let reason = match other {
                    None => UriUnavailableReason::NoOwnerMapping,
                    Some(Err(ResourceUriLookupError::WrongScope)) => {
                        UriUnavailableReason::WrongScope
                    }
                    Some(Err(ResourceUriLookupError::WrongRevision)) => {
                        UriUnavailableReason::WrongRevision
                    }
                    Some(Err(ResourceUriLookupError::AbsentResource)) => {
                        UriUnavailableReason::AbsentResource
                    }
                    Some(Ok(_)) => unreachable!("handled mapped result above"),
                };
                published.client_uri = None;
                published.uri_mapping =
                    UriMapping::Unavailable(Box::new(UriMappingUnavailable { reason }));
            }
        }
        published
            .validate_machine_source_location()
            .map_err(|error| SourceUriFault::InvalidSource {
                field: error.field,
                reason: error.reason,
            })?;
        Ok(published)
    };
    checked().map_err(|fault| SourceUriFailure { original, fault })
}

/// Recompute the exact expected URI/status against the supplied owner map.
/// A shape-valid forged URI, status, scope, resource or revision is rejected
/// while retaining the original DTO for diagnostic-only treatment.
pub fn check_published_source_location_uri(
    map: Option<&OwnerResourceUriMap>,
    scope: &OpaqueText,
    original: SourceLocation,
) -> Result<SourceLocation, SourceUriFailure> {
    original
        .validate_machine_source_location()
        .map_err(|error| SourceUriFailure {
            original: original.clone(),
            fault: SourceUriFault::InvalidSource {
                field: error.field,
                reason: error.reason,
            },
        })?;
    let mut neutral = original.clone();
    neutral.client_uri = None;
    neutral.uri_mapping = UriMapping::Unavailable(Box::new(UriMappingUnavailable {
        reason: UriUnavailableReason::NoOwnerMapping,
    }));
    let expected =
        publish_source_location_uri(map, scope, neutral).map_err(|failure| SourceUriFailure {
            original: original.clone(),
            fault: failure.fault,
        })?;
    if original != expected {
        return Err(SourceUriFailure {
            original,
            fault: SourceUriFault::ForgedPublication,
        });
    }
    Ok(original)
}

#[cfg(test)]
#[path = "source_location_uri/tests.rs"]
mod tests;
