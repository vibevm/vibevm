//! Local generated-page coherence; opaque wire cursor strings are unverified.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-BOUND-PAGING");

use std::collections::BTreeSet;

use vibe_wire::generated::machine::e1::bound_page as wire;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageWireFault {
    UnsupportedEpoch,
    BlankIdentity,
    DuplicateStamp,
    DuplicateOccurrence,
    SelfParent,
    EndCursorMismatch,
    FalseCompleteChild,
    MixedPageDeclarations,
    TokenAdjacencyMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageWireFailure {
    pub original: wire::BoundPage,
    pub fault: PageWireFault,
}

/// A checked local page. This does not validate any owner-issued cursor or
/// authorize publication of a complete multi-page result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalPageReview {
    value: wire::BoundPage,
}
impl LocalPageReview {
    pub fn value(&self) -> &wire::BoundPage {
        &self.value
    }
    pub fn cursor_status(&self) -> OwnerCursorStatus {
        OwnerCursorStatus::Unverified
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerCursorStatus {
    Unverified,
}

fn nonblank(value: &str) -> bool {
    !value.trim().is_empty() && value.trim() == value && !value.chars().any(char::is_control)
}

/// Check only one page's local epoch, references, stamps, rows and end shape.
/// `page_cursor_id` remains a claim until a separate retained cursor owner
/// authenticates a full sequence against its projection and authorized scope.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-BOUND-PAGING",
    r = 1
)]
pub fn review_generated_page(
    original: wire::BoundPage,
) -> Result<LocalPageReview, PageWireFailure> {
    let check = || -> Result<(), PageWireFault> {
        if original.schema != 1 {
            return Err(PageWireFault::UnsupportedEpoch);
        }
        if !nonblank(&original.projection_id)
            || !nonblank(&original.filter_id)
            || !nonblank(&original.authorized_scope_id)
            || original
                .page_cursor_id
                .as_deref()
                .is_some_and(|v| !nonblank(v))
            || original
                .next_cursor_id
                .as_deref()
                .is_some_and(|v| !nonblank(v))
        {
            return Err(PageWireFault::BlankIdentity);
        }
        let mut axes = BTreeSet::new();
        for stamp in &original.axis_stamps {
            let axis = match stamp.axis {
                wire::Axis::Source => 0,
                wire::Axis::World => 1,
                wire::Axis::Target => 2,
                wire::Axis::Operation => 3,
            };
            if !axes.insert(axis) {
                return Err(PageWireFault::DuplicateStamp);
            }
            if !nonblank(&stamp.owner)
                || !nonblank(&stamp.capture)
                || !nonblank(&stamp.revision)
                || !nonblank(&stamp.observation)
            {
                return Err(PageWireFault::BlankIdentity);
            }
        }
        if axes.is_empty() {
            return Err(PageWireFault::BlankIdentity);
        }
        let mut occurrences = BTreeSet::new();
        let mut incomplete_child = false;
        for row in &original.rows {
            if !nonblank(&row.entity_reference_id)
                || !nonblank(&row.occurrence_id)
                || row
                    .parent_occurrence_id
                    .as_deref()
                    .is_some_and(|v| !nonblank(v))
            {
                return Err(PageWireFault::BlankIdentity);
            }
            if !occurrences.insert(row.occurrence_id.as_str()) {
                return Err(PageWireFault::DuplicateOccurrence);
            }
            if row.parent_occurrence_id.as_deref() == Some(row.occurrence_id.as_str()) {
                return Err(PageWireFault::SelfParent);
            }
            match &row.child_state {
                wire::ChildState::Complete(_) => {}
                wire::ChildState::Truncated(value) => {
                    incomplete_child = true;
                    if !nonblank(&value.reason) {
                        return Err(PageWireFault::BlankIdentity);
                    }
                }
                wire::ChildState::Unavailable(value) => {
                    incomplete_child = true;
                    if !nonblank(&value.reason) {
                        return Err(PageWireFault::BlankIdentity);
                    }
                }
                wire::ChildState::Redacted(value) => {
                    incomplete_child = true;
                    if !nonblank(&value.reason) {
                        return Err(PageWireFault::BlankIdentity);
                    }
                }
            }
        }
        match &original.end {
            wire::PageEnd::More(_) => {
                if original.next_cursor_id.is_none() || original.rows.is_empty() {
                    return Err(PageWireFault::EndCursorMismatch);
                }
            }
            wire::PageEnd::DeclaredComplete(_) => {
                if original.next_cursor_id.is_some() {
                    return Err(PageWireFault::EndCursorMismatch);
                }
                if incomplete_child {
                    return Err(PageWireFault::FalseCompleteChild);
                }
            }
            wire::PageEnd::Incomplete(value) => {
                if original.next_cursor_id.is_some() {
                    return Err(PageWireFault::EndCursorMismatch);
                }
                if let wire::IncompleteReason::DeclaredPartial(reason) = &value.reason {
                    if !nonblank(&reason.reason) {
                        return Err(PageWireFault::BlankIdentity);
                    }
                }
            }
        }
        Ok(())
    };
    check().map_err(|fault| PageWireFailure {
        original: original.clone(),
        fault,
    })?;
    Ok(LocalPageReview { value: original })
}

/// Check only stated two-page adjacency and the same declared projection,
/// filter, scope and stamps. Even success does not authenticate either token,
/// reconstruct omitted pages or confer a complete publication verdict.
pub fn compare_declared_adjacent_pages(
    first: &LocalPageReview,
    second: &LocalPageReview,
) -> Result<OwnerCursorStatus, PageWireFault> {
    let first = first.value();
    let second = second.value();
    if first.projection_id != second.projection_id
        || first.filter_id != second.filter_id
        || first.authorized_scope_id != second.authorized_scope_id
        || first.axis_stamps != second.axis_stamps
    {
        return Err(PageWireFault::MixedPageDeclarations);
    }
    if !matches!(first.end, wire::PageEnd::More(_))
        || first.next_cursor_id.as_deref() != second.page_cursor_id.as_deref()
        || first.next_cursor_id.is_none()
    {
        return Err(PageWireFault::TokenAdjacencyMismatch);
    }
    Ok(OwnerCursorStatus::Unverified)
}
