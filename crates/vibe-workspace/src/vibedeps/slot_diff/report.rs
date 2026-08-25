//! Outcome accounting for one slot reconciliation.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#SELF-HEAL");

use std::path::PathBuf;

use super::super::SlotRecord;

/// Internal reconciliation facts used by install orchestration.
///
/// Paths are slot-relative. `written` and `removed` describe only filesystem
/// mutations that actually happened; `footprint` is the complete next owned
/// payload regardless of whether each row needed placement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MaterialiseReport {
    pub(crate) footprint: Vec<PathBuf>,
    pub(crate) written: Vec<PathBuf>,
    pub(crate) removed: Vec<PathBuf>,
    pub(crate) migrated: bool,
    pub(crate) identity_changed: bool,
}

impl MaterialiseReport {
    pub(super) fn new(
        footprint: Vec<PathBuf>,
        written: Vec<PathBuf>,
        removed: Vec<PathBuf>,
        migrated: bool,
        old: Option<&SlotRecord>,
        next: &SlotRecord,
    ) -> Self {
        let identity_changed = old.is_none_or(|old| {
            old.source_hash != next.source_hash
                || old.spec_format != next.spec_format
                || old.converter_recipe != next.converter_recipe
                || old.overlay_hash != next.overlay_hash
        });
        Self {
            footprint,
            written,
            removed,
            migrated,
            identity_changed,
        }
    }

    pub(crate) fn into_footprint(self) -> Vec<PathBuf> {
        self.footprint
    }
}
