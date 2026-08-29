//! The T10B lowering: borrowed kernel compile rows → one owner-scoped
//! [`TransformPlan`] (R4 architecture §5.3, R4-TRANSFORM-PLAN-ABI §§1–2).
//!
//! **The split this cell realises.** `vibe-workspace` owns the WORLD — the
//! lock-ordered snapshot, the owner-scoped view per lane owner, and the
//! choice of epoch authority. `vibe-spec` owns the LOWERING, and this is the
//! whole of it: one public entry that consumes rows the caller has ALREADY
//! filtered to the compile family in effective order
//! (`ExtensionRegistry::enabled_compile_rows`), and that maps, per row,
//! stage from `CompilePoint`, provider through the one
//! `From<&ExtensionProvider>` conversion, config through the one effective
//! configuration lowering, selector by clone, and implementation by
//! resolving the declared builtin name against the behavior registry. No
//! manifest object, resolver, display string or `Arc<dyn …>` crosses the
//! seam, and the caller never authors an order, a digest or an epoch.
//!
//! **The caller filters; this cell refuses.** A row at a non-compile point
//! is a typed CALLER ERROR, never a skip: skipping would let a wrong input
//! contract produce a plausible plan, and a plan digest would then bless a
//! membership no manifest declared. `compile:pass` is inside the compile
//! family by construction (`enabled_compile_rows` is the whole family), and
//! it refuses on its own arm until R6 owns the pass tier — a separate arm
//! rather than a shared one, because R6 splits it into routing, not into a
//! different error.
//!
//! **The epoch is registry-owned.** The workspace hands a handler name; the
//! epoch comes from [`TransformRegistry`] and from nowhere else, so no
//! caller can author an identity the ABI §2.1 forbids it to author. An
//! off-catalog name is the existing bounded `UnknownBuiltin` refusal, raised
//! HERE — at lowering — rather than deferred to schedule resolution, so a
//! plan that exists is a plan whose every implementation was cataloged when
//! it was built.
//!
//! **First fault wins, in row order.** Each fault carries the row's bounded
//! key preview and its zero-based row index, so a refusal names the exact
//! declaration without echoing an attacker-sized key.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY");

use vibe_core::lifecycle::{CompilePoint, ExtensionPoint};
use vibe_core::manifest::ExtensionHandler;
use vibe_extension_registry::{CompiledSelector, ExtensionRegistryRow};

use super::config_lowering::lower_effective_config;
use super::fault::{LoweringFault, TransformLoweringError, lowering_fault as fault};
use super::plan::{
    TransformImplementation, TransformPlan, TransformProvider, TransformSeed, TransformStage,
    is_behaviorally_unscoped,
};
use super::plan_validate::{BoundedPreview, bounded};
use super::registry::TransformRegistry;

impl TransformPlan {
    /// Lower one lane owner's effective compile rows into a plan.
    ///
    /// The rows are borrowed, already filtered to the compile family and
    /// already in the registry's ONE effective order — exactly what
    /// `ExtensionRegistry::enabled_compile_rows()` returns. An empty slice
    /// lowers to [`TransformPlan::empty`], which is the shared plan of every
    /// owner that declares no compile-point extension.
    ///
    /// The behavior epoch of each builtin comes from the production catalog;
    /// the caller supplies a name and never an epoch.
    pub fn from_effective_rows(
        rows: &[&ExtensionRegistryRow],
    ) -> Result<Self, TransformLoweringError> {
        Self::lower_rows(rows, &TransformRegistry::builtins())
    }

    /// [`TransformPlan::from_effective_rows`] against an injected behavior
    /// catalog — the crate-internal seam the transform tests drive, mirroring
    /// `compile_artifact_with_registries`.
    ///
    /// The production catalog is empty until R4.2 registers the first real
    /// behavior, so a test that lowers a REAL collected registry needs the
    /// same cfg-test identity catalog the execution tests already use. The
    /// seam is `#[cfg(test)]`: the workspace still cannot reach a registry,
    /// and therefore still cannot supply an epoch.
    #[cfg(test)]
    pub(crate) fn from_effective_rows_with(
        rows: &[&ExtensionRegistryRow],
        registry: &TransformRegistry,
    ) -> Result<Self, TransformLoweringError> {
        Self::lower_rows(rows, registry)
    }

    /// The one lowering body both entries share.
    fn lower_rows(
        rows: &[&ExtensionRegistryRow],
        registry: &TransformRegistry,
    ) -> Result<Self, TransformLoweringError> {
        let mut seeds = Vec::with_capacity(rows.len());
        for (row, source) in rows.iter().enumerate() {
            seeds.push(seed(row, source, registry)?);
        }
        // Order is the caller's effective order, preserved exactly: `build`
        // assigns dense zero-based order from THIS sequence. Sorting or
        // re-tiering here would fabricate an order no manifest authored.
        Self::build(seeds).map_err(|source| fault(LoweringFault::Plan { source }))
    }
}

/// Lower exactly one row into one seed.
fn seed(
    row: usize,
    source: &ExtensionRegistryRow,
    registry: &TransformRegistry,
) -> Result<TransformSeed, TransformLoweringError> {
    let preview = || bounded(source.key().as_str());
    let stage = stage(row, preview, source.declaration().point)?;
    let implementation = implementation(row, preview, source, registry)?;
    let config = lower_effective_config(source.effective_config()).map_err(|gap| {
        fault(LoweringFault::Config {
            row,
            preview: preview(),
            source: gap,
        })
    })?;
    Ok(TransformSeed::new(
        source.key().clone(),
        TransformProvider::from(source.provider()),
        stage,
        implementation,
        config,
        selector(source),
    ))
}

/// The compiled selector one row SUPPLIES to its seed.
///
/// Every collected row carries a compiled selector, present or not, so
/// "supplied" cannot be read off the field: it is read off whether any
/// dimension was authored, through the one canonicalization predicate. That
/// matters at lane/emitted, where manifest presence itself is illegal — the
/// manifest grammar already refuses `applies_to` there
/// (`ExtensionDecl::validate`), so an unscoped value is dropped rather than
/// handed on, while a dimension that somehow reached this point is supplied
/// and refused by the plan's own stage law instead of being silently lost.
///
/// The kernel is the single glob compiler; this is a clone of the row's
/// already-compiled value and never a second compilation.
fn selector(source: &ExtensionRegistryRow) -> Option<CompiledSelector> {
    let selector = source.compiled_selector();
    (!is_behaviorally_unscoped(selector)).then(|| selector.clone())
}

/// The staged tier one compile point names.
///
/// Two refusals, deliberately separate. A NON-compile point means the caller
/// broke the input contract — `enabled_compile_rows()` cannot produce one —
/// so it names the point it actually saw. `compile:pass` is a lawful member
/// of the compile family whose tier R6 owns; it refuses on its own arm so
/// that R6's routing split replaces one arm rather than reinterpreting a
/// shared one.
fn stage(
    row: usize,
    preview: impl Fn() -> BoundedPreview,
    point: ExtensionPoint,
) -> Result<TransformStage, TransformLoweringError> {
    let ExtensionPoint::Compile(point) = point else {
        return Err(fault(LoweringFault::NonCompilePoint {
            row,
            preview: preview(),
            point: point.to_string(),
        }));
    };
    Ok(match point {
        CompilePoint::Source => TransformStage::Source,
        CompilePoint::Document => TransformStage::Document,
        CompilePoint::Lane => TransformStage::Lane,
        CompilePoint::Emitted => TransformStage::Emitted,
        CompilePoint::Pass => {
            return Err(fault(LoweringFault::PassTier {
                row,
                preview: preview(),
            }));
        }
    })
}

/// The implementation identity one declared handler names.
///
/// `Builtin` resolves its name against the behavior catalog and takes the
/// catalog's epoch — the registry-owned identity, never a caller's. Every
/// other handler kind refuses: `native` joins here in R5 as the second arm
/// under this same registry authority (ABI §2 reserves the discriminant),
/// and `script` / `binary` / `agent` are not compiler-tier handlers at all.
fn implementation(
    row: usize,
    preview: impl Fn() -> BoundedPreview,
    source: &ExtensionRegistryRow,
    registry: &TransformRegistry,
) -> Result<TransformImplementation, TransformLoweringError> {
    let handler = &source.declaration().handler;
    let ExtensionHandler::Builtin { name } = handler else {
        return Err(fault(LoweringFault::UnsupportedHandler {
            row,
            preview: preview(),
            kind: handler.kind(),
        }));
    };
    registry
        .epoch_of(name)
        .map(|epoch| TransformImplementation::builtin_candidate(name, epoch))
        .map_err(|error| {
            fault(LoweringFault::Implementation {
                row,
                preview: preview(),
                source: error,
            })
        })
}
