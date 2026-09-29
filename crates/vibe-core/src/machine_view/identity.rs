//! Distinct logical, captured, observed and occurrence reference axes.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-ENTITY-REFERENCES");

use super::{MachineViewError, invalid};

/// Caller-supplied opaque text. Validation does not normalize or mint it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OpaqueText(String);
impl OpaqueText {
    pub fn new(value: impl Into<String>) -> Result<Self, MachineViewError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(invalid("reference", "reference must be nonblank"));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An address minted by the domain owner, not derived from a line or path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalOwnerAddress {
    owner: OpaqueText,
    address: OpaqueText,
}
impl LogicalOwnerAddress {
    pub fn supplied(owner: OpaqueText, address: OpaqueText) -> Self {
        Self { owner, address }
    }
    pub fn owner(&self) -> &OpaqueText {
        &self.owner
    }
    pub fn address(&self) -> &OpaqueText {
        &self.address
    }
}

/// Absence of an owner address is explicit, never replaced with a path/hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogicalIdentity {
    OwnerMinted(LogicalOwnerAddress),
    ExtractionGap { reason: OpaqueText },
}

/// Captured source/content identity and its own revision are distinct from a
/// selected world or observation. Both values come from the owning capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedSourceRef {
    content_or_source: OpaqueText,
    revision: OpaqueText,
}
impl CapturedSourceRef {
    pub fn supplied(content_or_source: OpaqueText, revision: OpaqueText) -> Self {
        Self {
            content_or_source,
            revision,
        }
    }
    pub fn content_or_source(&self) -> &OpaqueText {
        &self.content_or_source
    }
    pub fn revision(&self) -> &OpaqueText {
        &self.revision
    }
}

/// One owner-provided observation/projection, with no wall-clock inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationRef {
    projection: OpaqueText,
    observation: OpaqueText,
}
impl ObservationRef {
    pub fn supplied(projection: OpaqueText, observation: OpaqueText) -> Self {
        Self {
            projection,
            observation,
        }
    }
    pub fn projection(&self) -> &OpaqueText {
        &self.projection
    }
    pub fn observation(&self) -> &OpaqueText {
        &self.observation
    }
}

/// One presentation/contribution occurrence. A repeated inclusion receives
/// another occurrence even when its logical address and captured bytes agree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContributionOccurrenceRef {
    observed: ObservationRef,
    occurrence_id: OpaqueText,
    contribution: OpaqueText,
    ordinal: u64,
}
impl ContributionOccurrenceRef {
    pub fn supplied(
        observed: ObservationRef,
        occurrence_id: OpaqueText,
        contribution: OpaqueText,
        ordinal: u64,
    ) -> Self {
        Self {
            observed,
            occurrence_id,
            contribution,
            ordinal,
        }
    }
    pub fn observed(&self) -> &ObservationRef {
        &self.observed
    }
    pub fn occurrence_id(&self) -> &OpaqueText {
        &self.occurrence_id
    }
    pub fn contribution(&self) -> &OpaqueText {
        &self.contribution
    }
    pub fn ordinal(&self) -> u64 {
        self.ordinal
    }
}

/// A navigation value over four independently supplied identity axes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityReference {
    logical: LogicalIdentity,
    captured: CapturedSourceRef,
    observed: ObservationRef,
    occurrence: ContributionOccurrenceRef,
}
impl EntityReference {
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-ENTITY-REFERENCES",
        r = 1
    )]
    pub fn new(
        logical: LogicalIdentity,
        captured: CapturedSourceRef,
        observed: ObservationRef,
        occurrence: ContributionOccurrenceRef,
    ) -> Result<Self, MachineViewError> {
        if occurrence.observed() != &observed {
            return Err(invalid(
                "occurrence.observation",
                "occurrence and projection observation differ",
            ));
        }
        Ok(Self {
            logical,
            captured,
            observed,
            occurrence,
        })
    }
    pub fn logical(&self) -> &LogicalIdentity {
        &self.logical
    }
    pub fn captured(&self) -> &CapturedSourceRef {
        &self.captured
    }
    pub fn observed(&self) -> &ObservationRef {
        &self.observed
    }
    pub fn occurrence(&self) -> &ContributionOccurrenceRef {
        &self.occurrence
    }
}
