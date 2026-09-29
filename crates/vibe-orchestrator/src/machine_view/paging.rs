//! One fixed authorized row projection; cursors never reread domain owners.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-BOUND-PAGING");

use std::sync::Arc;
use vibe_core::machine_view::OpaqueText;

use super::{AxisStamp, CompositeCompleteness, CompositeSnapshot};

#[derive(Debug, Clone)]
struct ProjectionKey {
    instance: Arc<()>,
    projection: OpaqueText,
    filter: OpaqueText,
    authorized_scope: OpaqueText,
    axes: Vec<AxisStamp>,
}
impl PartialEq for ProjectionKey {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.instance, &other.instance)
            && self.projection == other.projection
            && self.filter == other.filter
            && self.authorized_scope == other.authorized_scope
            && self.axes == other.axes
    }
}
impl Eq for ProjectionKey {}

/// Opaque process-local cursor. Its identity claims are supplied by the
/// projection owner; it is neither a serialized wire token nor a file grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageCursor {
    key: ProjectionKey,
    offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectionExtent {
    DeclaredComplete,
    Partial { reason: OpaqueText },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IncompleteReason {
    DeclaredPartial(OpaqueText),
    CompositePartial,
    CrossOwnerUnverified,
    MissingCompatibilityClaim,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageEnd {
    More,
    DeclaredComplete,
    Incomplete(IncompleteReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page<T> {
    pub rows: Vec<T>,
    pub next: Option<PageCursor>,
    pub end: PageEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageFault {
    ZeroLimit,
    MixedCursor,
    OffsetBeyondProjection,
}

#[derive(Debug, Clone)]
pub struct AuthorizedProjection<T> {
    key: ProjectionKey,
    rows: Vec<T>,
    extent: ProjectionExtent,
    composite: CompositeCompleteness,
}
impl<T: Clone> AuthorizedProjection<T> {
    /// Capture fixed in-memory rows supplied by the authorizing caller. This
    /// constructor does not authenticate their completeness or read a path.
    pub fn supplied(
        snapshot: &CompositeSnapshot,
        projection: OpaqueText,
        filter: OpaqueText,
        authorized_scope: OpaqueText,
        rows: Vec<T>,
        extent: ProjectionExtent,
    ) -> Self {
        Self {
            key: ProjectionKey {
                instance: Arc::new(()),
                projection,
                filter,
                authorized_scope,
                axes: snapshot.stamps().to_vec(),
            },
            rows,
            extent,
            composite: snapshot.completeness(),
        }
    }

    /// A page/child/chunk cursor is bound to the exact supplied projection,
    /// filter, authorized scope and participating axis identities/revisions.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-BOUND-PAGING",
        r = 1
    )]
    pub fn page(&self, cursor: Option<&PageCursor>, limit: usize) -> Result<Page<T>, PageFault> {
        if limit == 0 {
            return Err(PageFault::ZeroLimit);
        }
        let offset = match cursor {
            Some(cursor) if cursor.key != self.key => return Err(PageFault::MixedCursor),
            Some(cursor) => cursor.offset,
            None => 0,
        };
        if offset > self.rows.len() {
            return Err(PageFault::OffsetBeyondProjection);
        }
        let end = offset.saturating_add(limit).min(self.rows.len());
        let rows = self.rows[offset..end].to_vec();
        if end < self.rows.len() {
            return Ok(Page {
                rows,
                next: Some(PageCursor {
                    key: self.key.clone(),
                    offset: end,
                }),
                end: PageEnd::More,
            });
        }
        let status = match (&self.extent, self.composite) {
            (ProjectionExtent::Partial { reason }, _) => {
                PageEnd::Incomplete(IncompleteReason::DeclaredPartial(reason.clone()))
            }
            (_, CompositeCompleteness::Partial) => {
                PageEnd::Incomplete(IncompleteReason::CompositePartial)
            }
            (_, CompositeCompleteness::UnverifiedCrossOwner) => {
                PageEnd::Incomplete(IncompleteReason::CrossOwnerUnverified)
            }
            (_, CompositeCompleteness::UnverifiedMissingClaim) => {
                PageEnd::Incomplete(IncompleteReason::MissingCompatibilityClaim)
            }
            (
                ProjectionExtent::DeclaredComplete,
                CompositeCompleteness::DeclaredCompleteSingleOwner,
            ) => PageEnd::DeclaredComplete,
        };
        Ok(Page {
            rows,
            next: None,
            end: status,
        })
    }
}
