//! Opaque row identity and effective-order index projections.

use std::fmt;
use std::sync::Arc;

use specmark::spec;
use vibe_core::lifecycle::ExtensionPoint;
use vibe_core::manifest::ExtensionHandler;

use super::{ExtensionRegistry, ExtensionRegistryRow};

/// Private allocation token shared by one registry and every index it mints.
#[derive(Debug)]
pub(crate) struct RegistryIdentity;

pub(crate) fn new_registry_identity() -> Arc<RegistryIdentity> {
    Arc::new(RegistryIdentity)
}

/// Opaque identity of one row in one retained [`ExtensionRegistry`].
///
/// Cloneable without a self-reference: private storage position is not
/// compiler dense order. The token retains no rows and is useful only while
/// co-owned with its immutable origin: moving that registry stays valid, a
/// clone is foreign, and after origin drop the token is inert but ABA-proof.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ORDER-LAW")]
#[derive(Clone)]
pub struct RegistryRowIndex {
    storage_position: usize,
    identity: Arc<RegistryIdentity>,
}

impl fmt::Debug for RegistryRowIndex {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RegistryRowIndex(..)")
    }
}

impl ExtensionRegistry {
    /// Project an opaque index through the exact registry that minted it.
    ///
    /// Another registry, including a clone or a later allocation after the
    /// origin is dropped, returns `None`; mismatched epochs never index or
    /// panic publicly.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#ORDER-LAW")]
    #[must_use]
    pub fn row_at(&self, index: &RegistryRowIndex) -> Option<&ExtensionRegistryRow> {
        if !Arc::ptr_eq(&self.identity, &index.identity) {
            return None;
        }
        self.rows.get(index.storage_position)
    }

    /// The ONE effective-order authority for borrowed and retained subsets.
    fn enabled_positions_where<'registry>(
        &'registry self,
        accepts: impl Fn(&ExtensionRegistryRow) -> bool + 'registry,
    ) -> impl Iterator<Item = usize> + 'registry {
        self.effective_order
            .iter()
            .copied()
            .filter(move |position| {
                let row = &self.rows[*position];
                row.is_enabled() && accepts(row)
            })
    }

    fn mint_index(&self, storage_position: usize) -> RegistryRowIndex {
        RegistryRowIndex {
            storage_position,
            identity: Arc::clone(&self.identity),
        }
    }

    /// Borrow enabled rows directly through the same private positions the
    /// index views use. Minted same-registry positions never need fallible
    /// public reprojection.
    fn enabled_rows_where<'registry>(
        &'registry self,
        accepts: impl Fn(&ExtensionRegistryRow) -> bool + 'registry,
    ) -> impl Iterator<Item = &'registry ExtensionRegistryRow> + 'registry {
        self.enabled_positions_where(accepts)
            .map(|position| &self.rows[position])
    }

    pub(super) fn enabled_rows_at<'registry>(
        &'registry self,
        point: ExtensionPoint,
    ) -> impl Iterator<Item = &'registry ExtensionRegistryRow> + 'registry {
        self.enabled_rows_where(move |row| row.declaration.point == point)
    }

    /// Return every enabled `compile:*` row in ONE global effective order.
    ///
    /// This compatibility view and [`Self::enabled_compile_indices`] share the
    /// same position authority. Selectors stay unevaluated; disabled and
    /// inactive rows stay absent; `compile:pass` remains in the family.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION")]
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
    #[must_use]
    pub fn enabled_compile_rows(&self) -> Vec<&ExtensionRegistryRow> {
        self.enabled_rows_where(|row| matches!(row.declaration.point, ExtensionPoint::Compile(_)))
            .collect()
    }

    /// Return indices for enabled compile-family rows in effective order.
    ///
    /// A later manager assigns dense order by enumerating this complete
    /// sequence; neither registry storage nor a native subset supplies it.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION")]
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
    #[must_use]
    pub fn enabled_compile_indices(&self) -> Vec<RegistryRowIndex> {
        self.enabled_positions_where(|row| {
            matches!(row.declaration.point, ExtensionPoint::Compile(_))
        })
        .map(|position| self.mint_index(position))
        .collect()
    }

    /// Return enabled native-handler indices across every extension family.
    ///
    /// Phase/slot native rows retain their position relative to compiler-native
    /// rows, and selectors remain unevaluated for the pre-subject build epoch.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#BUILD-PHASE-OWNS-IT")]
    #[must_use]
    pub fn enabled_native_indices(&self) -> Vec<RegistryRowIndex> {
        self.enabled_positions_where(|row| {
            matches!(row.declaration.handler, ExtensionHandler::Native { .. })
        })
        .map(|position| self.mint_index(position))
        .collect()
    }
}
