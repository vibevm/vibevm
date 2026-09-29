//! Raw and projected positions retain their separate bases and resources.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SOURCE-LOCATION");

use super::{MachineViewError, OpaqueText, invalid};

/// Arbitrary exact resource bytes. These are not an OS path or URI grant.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OpaqueResourceId(Vec<u8>);
impl OpaqueResourceId {
    pub fn new(bytes: Vec<u8>) -> Result<Self, MachineViewError> {
        if bytes.is_empty() {
            return Err(invalid("resource", "resource identity must not be empty"));
        }
        Ok(Self(bytes))
    }
    pub fn bytes(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteSpan {
    start: u64,
    end: u64,
}
impl ByteSpan {
    pub fn new(start: u64, end: u64) -> Result<Self, MachineViewError> {
        if start > end {
            return Err(invalid("byte_span", "start exceeds end"));
        }
        Ok(Self { start, end })
    }
    pub fn start(&self) -> u64 {
        self.start
    }
    pub fn end(&self) -> u64 {
        self.end
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TextPosition {
    line: u64,
    column: u64,
}
impl TextPosition {
    pub fn new(line: u64, column: u64) -> Result<Self, MachineViewError> {
        if line == 0 || column == 0 {
            return Err(invalid("text_position", "line and column are one-based"));
        }
        Ok(Self { line, column })
    }
    pub fn line(&self) -> u64 {
        self.line
    }
    pub fn column(&self) -> u64 {
        self.column
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSpan {
    start: TextPosition,
    end: TextPosition,
}
impl TextSpan {
    pub fn new(start: TextPosition, end: TextPosition) -> Result<Self, MachineViewError> {
        if start > end {
            return Err(invalid("text_span", "start exceeds end"));
        }
        Ok(Self { start, end })
    }
    pub fn start(&self) -> TextPosition {
        self.start
    }
    pub fn end(&self) -> TextPosition {
        self.end
    }
}

/// Position basis is tagged: a projected line can never be mistaken for an
/// original byte offset by a caller matching this value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PositionBasis {
    OriginalBytes {
        encoding: OpaqueText,
        span: ByteSpan,
    },
    ProjectedText {
        projection: OpaqueText,
        encoding: OpaqueText,
        span: TextSpan,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    resource: OpaqueResourceId,
    captured_revision: OpaqueText,
    basis: PositionBasis,
}
impl SourceLocation {
    pub fn original(
        resource: OpaqueResourceId,
        captured_revision: OpaqueText,
        encoding: Option<OpaqueText>,
        span: ByteSpan,
    ) -> LocationObservation {
        let Some(encoding) = encoding else {
            return LocationObservation::Unavailable {
                resource,
                captured_revision,
                reason: LocationUnavailable::UnknownEncoding,
            };
        };
        LocationObservation::Available(Self {
            resource,
            captured_revision,
            basis: PositionBasis::OriginalBytes { encoding, span },
        })
    }

    pub fn projected(
        resource: OpaqueResourceId,
        captured_revision: OpaqueText,
        projection: OpaqueText,
        encoding: Option<OpaqueText>,
        span: TextSpan,
    ) -> LocationObservation {
        let Some(encoding) = encoding else {
            return LocationObservation::Unavailable {
                resource,
                captured_revision,
                reason: LocationUnavailable::UnknownEncoding,
            };
        };
        LocationObservation::Available(Self {
            resource,
            captured_revision,
            basis: PositionBasis::ProjectedText {
                projection,
                encoding,
                span,
            },
        })
    }

    pub fn resource(&self) -> &OpaqueResourceId {
        &self.resource
    }
    pub fn captured_revision(&self) -> &OpaqueText {
        &self.captured_revision
    }
    pub fn basis(&self) -> &PositionBasis {
        &self.basis
    }

    /// No retained reversible owner map is bound in this first atom. Projected
    /// text cannot be reported as original bytes from matching coordinates.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SOURCE-LOCATION",
        r = 1
    )]
    pub fn original_from_projected(&self) -> LocationObservation {
        match &self.basis {
            PositionBasis::OriginalBytes { .. } => LocationObservation::Available(self.clone()),
            PositionBasis::ProjectedText { .. } => LocationObservation::Unavailable {
                resource: self.resource.clone(),
                captured_revision: self.captured_revision.clone(),
                reason: LocationUnavailable::MissingReverseMap,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocationUnavailable {
    UnknownEncoding,
    RawBytesUnavailable,
    MissingReverseMap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocationObservation {
    Available(SourceLocation),
    Unavailable {
        resource: OpaqueResourceId,
        captured_revision: OpaqueText,
        reason: LocationUnavailable,
    },
}
impl LocationObservation {
    pub fn raw_bytes_unavailable(
        resource: OpaqueResourceId,
        captured_revision: OpaqueText,
    ) -> Self {
        Self::Unavailable {
            resource,
            captured_revision,
            reason: LocationUnavailable::RawBytesUnavailable,
        }
    }
}
