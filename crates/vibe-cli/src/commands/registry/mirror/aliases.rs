//! The alias table a mirror run carries from its source into its target
//! (PROP-008 `#short-name`, PROP-002 `#registry-mirror`).
//!
//! The merge RULE itself lives beside the maintainer's own edits in
//! `registry::alias` — it is a statement about one table with two
//! authors, not about the mirror pipeline. What lives here is the
//! pipeline's half of it: which packages this target will actually
//! hold, and what the previous run recorded carrying.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror");

use std::path::Path;

use anyhow::Result;

use super::plan::{self, Decision};
use super::record;
use super::source;

/// The target's alias table after this run — the source's rows merged
/// in by the rule in [`crate::commands::registry::alias::mirrored`], with the rows this
/// mirror cannot honour taken back out.
///
/// A row is carried only when the target will actually hold the package
/// it names: the versions this run copies plus the ones it already had.
/// A row that survives that and still would not stand against the
/// target's own index — a spelling the target publishes itself — is
/// dropped here rather than failing the run: a mirror must not become
/// un-mirrorable because its source offers a spelling this registry
/// already uses for something real.
pub(super) fn alias_merge(
    listing: &source::SourceListing,
    planned: &[plan::Planned],
    target_index: &vibe_index::index::Index,
    target_root: &Path,
    recorded: &Option<record::MirrorRecord>,
) -> Result<crate::commands::registry::alias::MirrorMerge> {
    let target_table =
        vibe_index::aliases::AliasTable::read(&vibe_core::layout_registry::index_dir(target_root))?;
    let carried = recorded
        .as_ref()
        .map(record::MirrorRecord::carried_aliases)
        .unwrap_or_default();
    let holds = |target: &vibe_index::types::AliasTarget| {
        target_index.get(&target.group, &target.name).is_some()
            || planned.iter().any(|p| {
                matches!(p.decision, Decision::Copy | Decision::Update)
                    && p.version.group == target.group
                    && p.version.name == target.name
            })
    };
    let mut merge = crate::commands::registry::alias::mirrored(
        &listing.aliases,
        &target_table,
        &carried,
        holds,
    );
    for fault in vibe_index::aliases::check(target_index, &merge.table) {
        merge.drop_row(fault.alias());
    }
    Ok(merge)
}
