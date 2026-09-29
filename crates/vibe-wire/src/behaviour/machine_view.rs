//! Semantic admission of generated entity/location roots through core owners.
//! Generated serde retains wire shape; this cell neither mints identities nor
//! maps opaque resource bytes to a path or URI.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-ENTITY-REFERENCES");

use std::fmt;

use vibe_core::machine_view::{
    ByteSpan, CapturedSourceRef, ContributionOccurrenceRef, EntityReference as CoreEntityReference,
    LocationObservation, LocationUnavailable, LogicalIdentity, LogicalOwnerAddress, ObservationRef,
    OpaqueResourceId, OpaqueText, SourceLocation as CoreSourceLocation, TextPosition, TextSpan,
};

use crate::behaviour::scalars::{has_control_bytes, is_canonical_decimal};
use crate::generated::machine::e1::{
    entity_reference::{EntityReference, Logical},
    source_location::{Location, Reason, SourceLocation, UriMapping},
};

/// One bounded semantic refusal. The original generated DTO stays with the
/// caller; this type neither serializes it nor fabricates a replacement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MachineViewWireError {
    pub field: &'static str,
    pub reason: &'static str,
}
impl fmt::Display for MachineViewWireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "machine view field {}: {}", self.field, self.reason)
    }
}
impl std::error::Error for MachineViewWireError {}

type Validated<T> = Result<T, MachineViewWireError>;

fn invalid(field: &'static str, reason: &'static str) -> MachineViewWireError {
    MachineViewWireError { field, reason }
}
fn opaque(field: &'static str, text: &str) -> Validated<OpaqueText> {
    if has_control_bytes(text) {
        return Err(invalid(field, "textual identity carries CR, LF or NUL"));
    }
    OpaqueText::new(text.to_owned()).map_err(|_| invalid(field, "identity is blank"))
}
fn decimal(field: &'static str, text: &str) -> Validated<u64> {
    if !is_canonical_decimal(text) {
        return Err(invalid(field, "not canonical unsigned decimal"));
    }
    text.parse::<u64>()
        .map_err(|_| invalid(field, "unsigned decimal exceeds u64"))
}

/// The URI owner's declared portable token spelling. Matching this spelling
/// is syntax admission only; it never proves that a resource is authorized.
fn uri_token(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.first().is_some_and(u8::is_ascii_alphanumeric)
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'-' || *byte == b'_')
}

fn validate_client_uri(value: &SourceLocation) -> Validated<()> {
    match (&value.client_uri, &value.uri_mapping) {
        (Some(uri), UriMapping::OwnerDeclaredUnverified(mapping)) => {
            let token = uri.strip_prefix("vibevm-resource:").ok_or_else(|| {
                invalid("client_uri", "URI is not an owner-declared resource token")
            })?;
            if !uri_token(token) {
                return Err(invalid("client_uri", "resource token is not portable"));
            }
            if !uri_token(&mapping.authorized_scope_id) {
                return Err(invalid(
                    "uri_mapping.authorized_scope_id",
                    "scope token is not portable",
                ));
            }
            Ok(())
        }
        (None, UriMapping::Unavailable(_)) => Ok(()),
        (None, UriMapping::OwnerDeclaredUnverified(_)) => Err(invalid(
            "client_uri",
            "declared URI mapping requires a non-null client URI",
        )),
        (Some(_), UriMapping::Unavailable(_)) => Err(invalid(
            "uri_mapping",
            "unavailable URI mapping cannot carry a client URI",
        )),
    }
}

impl EntityReference {
    /// Validate the wire and return the existing core four-axis identity.
    /// `reference_id` remains a projection-local lookup key on this DTO.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-ENTITY-REFERENCES",
        r = 1
    )]
    pub fn validate_machine_entity_reference(&self) -> Validated<CoreEntityReference> {
        if self.schema != 1 {
            return Err(invalid("schema", "unsupported entity-reference epoch"));
        }
        opaque("reference_id", &self.reference_id)?;
        let logical = match &self.logical {
            Logical::OwnerMinted(value) => {
                LogicalIdentity::OwnerMinted(LogicalOwnerAddress::supplied(
                    opaque("logical.owner", &value.owner)?,
                    opaque("logical.address", &value.address)?,
                ))
            }
            Logical::ExtractionGap(value) => LogicalIdentity::ExtractionGap {
                reason: opaque("logical.reason", &value.reason)?,
            },
        };
        let captured = CapturedSourceRef::supplied(
            opaque(
                "captured.content_or_source",
                &self.captured.content_or_source,
            )?,
            opaque("captured.revision", &self.captured.revision)?,
        );
        let observed = ObservationRef::supplied(
            opaque("observed.projection", &self.observed.projection)?,
            opaque("observed.observation", &self.observed.observation)?,
        );
        let occurrence_observed = ObservationRef::supplied(
            opaque(
                "occurrence.observed.projection",
                &self.occurrence.observed.projection,
            )?,
            opaque(
                "occurrence.observed.observation",
                &self.occurrence.observed.observation,
            )?,
        );
        let occurrence = ContributionOccurrenceRef::supplied(
            occurrence_observed,
            opaque("occurrence.contribution", &self.occurrence.contribution)?,
            decimal("occurrence.ordinal", &self.occurrence.ordinal)?,
        );
        CoreEntityReference::new(logical, captured, observed, occurrence).map_err(|_| {
            invalid(
                "occurrence.observed",
                "occurrence and root observation differ",
            )
        })
    }
}

impl SourceLocation {
    /// Keep opaque octets intact and ask the core position constructors to
    /// judge bounds and basis. This grants no resource-to-URI conversion.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SOURCE-LOCATION",
        r = 1
    )]
    pub fn validate_machine_source_location(&self) -> Validated<LocationObservation> {
        if self.schema != 1 {
            return Err(invalid("schema", "unsupported source-location epoch"));
        }
        validate_client_uri(self)?;
        let resource = OpaqueResourceId::new(self.resource_id_octets.clone())
            .map_err(|_| invalid("resource_id_octets", "resource identity is empty"))?;
        let revision = opaque("captured_revision", &self.captured_revision)?;
        match &self.location {
            Location::OriginalBytes(value) => {
                let span = ByteSpan::new(
                    decimal("location.start", &value.start)?,
                    decimal("location.end", &value.end)?,
                )
                .map_err(|_| invalid("location.span", "start exceeds end"))?;
                Ok(CoreSourceLocation::original(
                    resource,
                    revision,
                    Some(opaque("location.encoding", &value.encoding)?),
                    span,
                ))
            }
            Location::ProjectedText(value) => {
                let start = TextPosition::new(
                    decimal("location.start_line", &value.start_line)?,
                    decimal("location.start_column", &value.start_column)?,
                )
                .map_err(|_| invalid("location.start", "text coordinates are one-based"))?;
                let end = TextPosition::new(
                    decimal("location.end_line", &value.end_line)?,
                    decimal("location.end_column", &value.end_column)?,
                )
                .map_err(|_| invalid("location.end", "text coordinates are one-based"))?;
                let span = TextSpan::new(start, end)
                    .map_err(|_| invalid("location.span", "start exceeds end"))?;
                Ok(CoreSourceLocation::projected(
                    resource,
                    revision,
                    opaque("location.projection", &value.projection)?,
                    Some(opaque("location.encoding", &value.encoding)?),
                    span,
                ))
            }
            Location::Unavailable(value) => {
                let reason = match &value.reason {
                    Reason::UnknownEncoding => LocationUnavailable::UnknownEncoding,
                    Reason::RawBytesUnavailable => LocationUnavailable::RawBytesUnavailable,
                    Reason::MissingReverseMap => LocationUnavailable::MissingReverseMap,
                };
                Ok(LocationObservation::Unavailable {
                    resource,
                    captured_revision: revision,
                    reason,
                })
            }
        }
    }
}

#[cfg(test)]
#[path = "machine_view/tests.rs"]
mod tests;
