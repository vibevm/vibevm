//! Process-local owner-issued pages over one fixed authorized projection.
//! The tokens are one-use lookup keys, never durable cursor authority.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-BOUND-PAGING");

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};

use vibe_core::machine_view::OpaqueText;
use vibe_wire::generated::machine::e1::{bound_page as wire, entity_reference::EntityReference};

use super::{
    AuthorizedProjection, AxisStamp, CompositeSnapshot, IncompleteReason, PageCursor,
    PageEnd as CorePageEnd, PageFault, PageWireFailure, ProjectionExtent, review_generated_page,
};

static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageSessionError {
    SessionIdsExhausted,
    InvalidEntity {
        id: String,
        field: &'static str,
        reason: &'static str,
    },
    DuplicateEntity(String),
    UnknownEntity(String),
    EntityProjectionMismatch(String),
    EntitySnapshotMismatch(String),
    DuplicateSourceCaptureBinding(String),
    ConflictingSourceCaptureBinding(String),
    BindingSnapshotMismatch(String),
    OccurrenceMismatch(String),
    ReusedEntityReference(String),
    DuplicateOccurrence(String),
    ParentNotPrior(String),
    MixedProjection,
    CursorMissing,
    CursorForgedOrReplayed,
    AlreadyTerminal,
    Page(PageFault),
    GeneratedLocal(PageWireFailure),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageSessionRequest {
    pub projection: OpaqueText,
    pub filter: OpaqueText,
    pub authorized_scope: OpaqueText,
    pub axis_stamps: Vec<AxisStamp>,
    pub cursor_token: Option<String>,
    pub limit: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionCursorStatus {
    OwnerIssuedProcessLocal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionPage {
    pub value: wire::BoundPage,
    pub cursor_status: SessionCursorStatus,
    pub source_bytes_authenticated: bool,
}

/// An owner-declared join from captured source identity to one selected axis.
/// Matching these opaque values does not authenticate the source bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceCaptureBinding {
    pub content_or_source: OpaqueText,
    pub revision: OpaqueText,
    pub observation: OpaqueText,
    pub selected_capture: OpaqueText,
}

/// One immutable row set and finite validated entity catalogue. The parent
/// hierarchy is checked over retained rows, not discovered from a filesystem.
pub struct PageSession {
    projection: OpaqueText,
    filter: OpaqueText,
    authorized_scope: OpaqueText,
    stamps: Vec<AxisStamp>,
    rows: AuthorizedProjection<wire::Row>,
    entities: BTreeMap<String, EntityReference>,
    session_id: u64,
    next_number: u64,
    pending: Option<(String, PageCursor)>,
    started: bool,
    terminal: bool,
}

fn wire_axis(axis: super::SnapshotAxis) -> wire::Axis {
    match axis {
        super::SnapshotAxis::Source => wire::Axis::Source,
        super::SnapshotAxis::World => wire::Axis::World,
        super::SnapshotAxis::Target => wire::Axis::Target,
        super::SnapshotAxis::Operation => wire::Axis::Operation,
    }
}
fn wire_stamp(stamp: &AxisStamp) -> wire::Stamp {
    wire::Stamp {
        axis: wire_axis(stamp.axis),
        owner: stamp.owner.as_str().to_owned(),
        capture: stamp.capture.as_str().to_owned(),
        revision: stamp.revision.as_str().to_owned(),
        observation: stamp.observation.as_str().to_owned(),
    }
}
fn wire_end(end: CorePageEnd) -> wire::PageEnd {
    match end {
        CorePageEnd::More => wire::PageEnd::More(Box::new(wire::PageEndMore {})),
        CorePageEnd::DeclaredComplete => {
            wire::PageEnd::DeclaredComplete(Box::new(wire::PageEndDeclaredComplete {}))
        }
        CorePageEnd::Incomplete(reason) => {
            let reason = match reason {
                IncompleteReason::DeclaredPartial(reason) => {
                    wire::IncompleteReason::DeclaredPartial(Box::new(
                        wire::IncompleteReasonDeclaredPartial {
                            reason: reason.as_str().to_owned(),
                        },
                    ))
                }
                IncompleteReason::CompositePartial => wire::IncompleteReason::CompositePartial(
                    Box::new(wire::IncompleteReasonCompositePartial {}),
                ),
                IncompleteReason::CrossOwnerUnverified => {
                    wire::IncompleteReason::CrossOwnerUnverified(Box::new(
                        wire::IncompleteReasonCrossOwnerUnverified {},
                    ))
                }
                IncompleteReason::MissingCompatibilityClaim => {
                    wire::IncompleteReason::MissingCompatibilityClaim(Box::new(
                        wire::IncompleteReasonMissingCompatibilityClaim {},
                    ))
                }
            };
            wire::PageEnd::Incomplete(Box::new(wire::PageEndIncomplete { reason }))
        }
    }
}

impl PageSession {
    /// Retain only validated generated entity rows and a fixed projection.
    /// The caller still owns authorization and source-byte authentication.
    #[allow(
        clippy::too_many_arguments,
        reason = "one finite owner-issued session binding"
    )]
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-BOUND-PAGING",
        r = 1
    )]
    pub fn new(
        snapshot: &CompositeSnapshot,
        projection: OpaqueText,
        filter: OpaqueText,
        authorized_scope: OpaqueText,
        rows: Vec<wire::Row>,
        catalogue: Vec<EntityReference>,
        source_bindings: Vec<SourceCaptureBinding>,
        extent: ProjectionExtent,
    ) -> Result<Self, PageSessionError> {
        let session_id = NEXT_SESSION
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .map_err(|_| PageSessionError::SessionIdsExhausted)?;
        let mut bindings = BTreeMap::new();
        for binding in source_bindings {
            if !snapshot.stamps().iter().any(|stamp| {
                stamp.capture == binding.selected_capture
                    && stamp.revision == binding.revision
                    && stamp.observation == binding.observation
            }) {
                return Err(PageSessionError::BindingSnapshotMismatch(
                    binding.content_or_source.as_str().to_owned(),
                ));
            }
            let key = (
                binding.content_or_source.as_str().to_owned(),
                binding.revision.as_str().to_owned(),
                binding.observation.as_str().to_owned(),
            );
            if let Some(prior) = bindings.insert(key, binding.selected_capture.clone()) {
                let source = binding.content_or_source.as_str().to_owned();
                return Err(if prior == binding.selected_capture {
                    PageSessionError::DuplicateSourceCaptureBinding(source)
                } else {
                    PageSessionError::ConflictingSourceCaptureBinding(source)
                });
            }
        }
        let mut entities = BTreeMap::new();
        for entity in catalogue {
            entity
                .validate_machine_entity_reference()
                .map_err(|error| PageSessionError::InvalidEntity {
                    id: entity.reference_id.clone(),
                    field: error.field,
                    reason: error.reason,
                })?;
            if entity.observed.projection != projection.as_str() {
                return Err(PageSessionError::EntityProjectionMismatch(
                    entity.reference_id,
                ));
            }
            if !bindings.contains_key(&(
                entity.captured.content_or_source.clone(),
                entity.captured.revision.clone(),
                entity.observed.observation.clone(),
            )) {
                return Err(PageSessionError::EntitySnapshotMismatch(
                    entity.reference_id,
                ));
            }
            let id = entity.reference_id.clone();
            if entities.insert(id.clone(), entity).is_some() {
                return Err(PageSessionError::DuplicateEntity(id));
            }
        }
        let mut seen = BTreeSet::new();
        let mut used_entities = BTreeSet::new();
        let mut incomplete_children = false;
        for row in &rows {
            if seen.contains(row.occurrence_id.as_str()) {
                return Err(PageSessionError::DuplicateOccurrence(
                    row.occurrence_id.clone(),
                ));
            }
            let entity = entities
                .get(&row.entity_reference_id)
                .ok_or_else(|| PageSessionError::UnknownEntity(row.entity_reference_id.clone()))?;
            if !used_entities.insert(row.entity_reference_id.as_str()) {
                return Err(PageSessionError::ReusedEntityReference(
                    row.entity_reference_id.clone(),
                ));
            }
            if row.occurrence_id != entity.occurrence.occurrence_id {
                return Err(PageSessionError::OccurrenceMismatch(
                    row.entity_reference_id.clone(),
                ));
            }
            if row
                .parent_occurrence_id
                .as_deref()
                .is_some_and(|id| !seen.contains(id))
            {
                return Err(PageSessionError::ParentNotPrior(row.occurrence_id.clone()));
            }
            seen.insert(row.occurrence_id.as_str());
            incomplete_children |= !matches!(&row.child_state, wire::ChildState::Complete(_));
        }
        let extent = if incomplete_children && extent == ProjectionExtent::DeclaredComplete {
            ProjectionExtent::Partial {
                reason: OpaqueText::new("retained-child-incomplete")
                    .expect("static nonblank reason"),
            }
        } else {
            extent
        };
        let stamps = snapshot.stamps().to_vec();
        // Check all retained row-local fields before page one can escape.
        // The synthetic incomplete end says nothing about global fullness.
        let whole_rows = wire::BoundPage {
            schema: 1,
            projection_id: projection.as_str().to_owned(),
            filter_id: filter.as_str().to_owned(),
            authorized_scope_id: authorized_scope.as_str().to_owned(),
            axis_stamps: stamps.iter().map(wire_stamp).collect(),
            page_cursor_id: None,
            rows: rows.clone(),
            next_cursor_id: None,
            end: wire::PageEnd::Incomplete(Box::new(wire::PageEndIncomplete {
                reason: wire::IncompleteReason::DeclaredPartial(Box::new(
                    wire::IncompleteReasonDeclaredPartial {
                        reason: "retained-row-preflight".to_owned(),
                    },
                )),
            })),
        };
        review_generated_page(whole_rows).map_err(PageSessionError::GeneratedLocal)?;
        let projection_rows = AuthorizedProjection::supplied(
            snapshot,
            projection.clone(),
            filter.clone(),
            authorized_scope.clone(),
            rows,
            extent,
        );
        Ok(Self {
            projection,
            filter,
            authorized_scope,
            stamps,
            rows: projection_rows,
            entities,
            session_id,
            next_number: 1,
            pending: None,
            started: false,
            terminal: false,
        })
    }

    /// The exact retained catalogue row, for navigation only.
    pub fn entity(&self, reference_id: &str) -> Option<&EntityReference> {
        self.entities.get(reference_id)
    }

    /// Consume a process-local token once and issue the next generated page.
    /// A request must restate the selected scope/filter/revisions exactly.
    pub fn next_page(
        &mut self,
        request: PageSessionRequest,
    ) -> Result<SessionPage, PageSessionError> {
        if self.terminal {
            return Err(PageSessionError::AlreadyTerminal);
        }
        if request.projection != self.projection
            || request.filter != self.filter
            || request.authorized_scope != self.authorized_scope
            || request.axis_stamps != self.stamps
        {
            return Err(PageSessionError::MixedProjection);
        }
        let cursor = match (&request.cursor_token, &self.pending, self.started) {
            (None, None, false) => None,
            (None, _, _) => return Err(PageSessionError::CursorMissing),
            (Some(_), None, _) => return Err(PageSessionError::CursorForgedOrReplayed),
            (Some(supplied), Some((issued, cursor)), _) if supplied == issued => Some(cursor),
            (Some(_), Some(_), _) => return Err(PageSessionError::CursorForgedOrReplayed),
        };
        let page = self
            .rows
            .page(cursor, request.limit)
            .map_err(PageSessionError::Page)?;
        let next = page
            .next
            .as_ref()
            .map(|_| format!("session-{}-page-{}", self.session_id, self.next_number));
        let value = wire::BoundPage {
            schema: 1,
            projection_id: self.projection.as_str().to_owned(),
            filter_id: self.filter.as_str().to_owned(),
            authorized_scope_id: self.authorized_scope.as_str().to_owned(),
            axis_stamps: self.stamps.iter().map(wire_stamp).collect(),
            page_cursor_id: request.cursor_token,
            rows: page.rows,
            next_cursor_id: next.clone(),
            end: wire_end(page.end),
        };
        review_generated_page(value.clone()).map_err(PageSessionError::GeneratedLocal)?;
        self.started = true;
        self.pending = next.zip(page.next);
        if self.pending.is_some() {
            self.next_number += 1;
        } else {
            self.terminal = true;
        }
        Ok(SessionPage {
            value,
            cursor_status: SessionCursorStatus::OwnerIssuedProcessLocal,
            source_bytes_authenticated: false,
        })
    }
}

#[cfg(test)]
#[path = "page_session/tests.rs"]
mod tests;
