//! The flattened, rendered tree-row model (PROP-036 §2.11–§2.12). `VisibleRow`
//! is one drawn row of the scrollable list `App` derives from the model;
//! `RowNode` tags what a row points at; `load_label` maps a package's effective
//! load to its column label. Split out of `state` along the model-vs-state seam
//! so each file stays within the surface-form budget (module-grain cells).

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-036#tui");

use super::super::model::LoadType;

/// What a visible row points at. `Copy` so it can be read out from behind a
/// shared borrow of `App::rows` without moving the row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowNode {
    /// A resolved package — an index into the model's package list.
    Package(usize),
    /// A dependency edge whose target is not in the lockfile.
    Missing,
    /// The "not reached from a declared root" divider (§2.12 orphan pass).
    Separator,
    /// A SubTables section subheader (`static dependencies`, …).
    Subheader,
}

/// One flattened, rendered tree row. Owns its drawn strings so the derived
/// list outlives any borrow of the model during a render pass.
#[derive(Debug, Clone)]
pub struct VisibleRow {
    /// What this row is.
    pub node: RowNode,
    /// The bare identity (`group/name`, or the edge target for a missing node;
    /// empty for the separator) — used by the detail modal.
    pub id: String,
    /// The drawn name cell: prefix + connector + `+`/`-` indicator + id +
    /// `(*)` re-occurrence marker.
    pub name: String,
    /// The effective-load column label (meaningful for `Package` rows).
    pub load: &'static str,
    /// `T` — transitive-static flag.
    pub transitive: bool,
    /// `C` — `when`-condition flag.
    pub condition: bool,
    /// `S` — physically in `STATIC.md`.
    pub in_static: bool,
}

/// The effective-load column label (PROP-036 §2.3).
pub(super) fn load_label(load: LoadType) -> &'static str {
    match load {
        LoadType::Static => "static",
        LoadType::Dynamic => "dynamic",
        LoadType::None => "none",
    }
}
