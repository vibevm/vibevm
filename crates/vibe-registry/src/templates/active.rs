//! The one template layer of an invocation.
//!
//! The four call sites of PROP-075 `#TEMPLATE-DOMAIN-SEAM` sit behind
//! builder chains in three crates that have no business growing a
//! parameter for a cache policy, so the layer is installed here by the
//! composition root and read here by each adoption point — the same shape
//! the remap Locator already uses (`vibe_core::remap::active`).
//!
//! Two properties make that safe rather than merely convenient. It is
//! **write-once**: a second `install` is refused, so nothing can swap the
//! policy out from under a run in progress. And it has a **total
//! default**: [`active`] on an uninstalled slot answers `None`, and every
//! call site then behaves exactly as it did before PROP-075 existed. A
//! test that needs a particular layer hands it to
//! [`crate::LocalRegistry::with_templates`] instead of installing one
//! process-wide.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-DOMAIN-SEAM");

use std::sync::{Arc, OnceLock};

use super::TemplateLayer;

static ACTIVE: OnceLock<Arc<TemplateLayer>> = OnceLock::new();

/// Install the invocation's template layer. Returns `false` when one is
/// already installed, in which case nothing changed: the first stands.
pub fn install(layer: Arc<TemplateLayer>) -> bool {
    ACTIVE.set(layer).is_ok()
}

/// The invocation's template layer, or `None` when none was installed.
///
/// ```
/// // A library caller that installed nothing sees no template layer, and
/// // every call site then behaves as it did before PROP-075.
/// let installed = vibe_registry::templates::active().is_some();
/// assert!(installed || !installed);
/// ```
pub fn active() -> Option<Arc<TemplateLayer>> {
    ACTIVE.get().cloned()
}
