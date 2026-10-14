//! Explicit three-way review and selection. No merge and no writer.
use super::*;
use vibe_core::manifest::derivation::{BindingError, RetainedSource};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebaseDisposition {
    UnchangedBase,
    RebaseRequired,
}

#[derive(Debug)]
pub struct RebaseReview {
    pub disposition: RebaseDisposition,
    previous: BoundDerivative,
    selected_base: SourceInput,
    saved_local: SourceInput,
    pub upstream_changes: Vec<Change>,
    pub local_changes: Vec<Change>,
}
#[derive(Debug)]
pub struct RebaseHandoff {
    previous: BoundDerivative,
    selected: BoundDerivative,
}
impl RebaseHandoff {
    pub fn previous(&self) -> &BoundDerivative {
        &self.previous
    }
    pub fn selected(&self) -> &BoundDerivative {
        &self.selected
    }
}
impl RebaseReview {
    /// Old capture must be independently retained; a version spelling alone
    /// cannot authorize movement onto different bytes of that same version.
    pub fn compare(
        previous: &str,
        old: Option<&SourceInput>,
        new: &SourceInput,
        local: &SourceInput,
    ) -> Result<Outcome<Self>, BindingError> {
        let previous = prepare_binding(previous, old)?.value;
        let old = old.ok_or(BindingError("old capture absent"))?;
        let b = previous.binding();
        if new.coordinate().group != old.coordinate().group
            || new.coordinate().name != old.coordinate().name
            || new.coordinate().kind != old.coordinate().kind
            || new.descriptor() != old.descriptor()
            || local.coordinate() != &b.local().owner
            || local.descriptor() != &b.local().descriptor
        {
            return Err(BindingError("rebase selection or saved local mismatch"));
        }
        let changes = |base: &SourceInput, source: &SourceInput| {
            DerivativeService::diff(&DerivedOutput { base, source }).value
        };
        let upstream_changes = changes(old, new);
        let local_changes = changes(old, local);
        let disposition = if old.same_basis(new) {
            RebaseDisposition::UnchangedBase
        } else {
            RebaseDisposition::RebaseRequired
        };
        Ok(passive(Self {
            disposition,
            previous,
            selected_base: new.clone(),
            saved_local: local.clone(),
            upstream_changes,
            local_changes,
        }))
    }
    pub fn old_base(&self) -> &RetainedSource {
        self.previous.binding().base()
    }
    pub fn new_base(&self) -> &SourceInput {
        &self.selected_base
    }
    pub fn saved_local(&self) -> &SourceInput {
        &self.saved_local
    }
    /// Caller supplies a complete explicitly authored selection. Partial patches
    /// and duplicate candidate selections have no implicit winning order.
    pub fn select(
        self,
        candidates: &[&str],
        current_base: &SourceInput,
        revised_local: &SourceInput,
    ) -> Result<Outcome<RebaseHandoff>, BindingError> {
        if candidates.len() != 1 || !self.selected_base.same_basis(current_base) {
            return Err(BindingError("ambiguous selection or changed reviewed base"));
        }
        let selected = prepare_binding(candidates[0], Some(current_base))?.value;
        let old = self.previous.binding();
        let new = selected.binding();
        if new.base().resource != old.base().resource
            || new.local().owner != old.local().owner
            || new.local().resource != old.local().resource
            || revised_local.retained_source(&new.local().resource) != *new.local()
        {
            return Err(BindingError(
                "explicit replacement ownership or saved bytes mismatch",
            ));
        }
        Ok(passive(RebaseHandoff {
            previous: self.previous,
            selected,
        }))
    }
}
