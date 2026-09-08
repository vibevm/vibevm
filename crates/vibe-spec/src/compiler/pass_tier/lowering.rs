//! One traversal partitions effective compile rows into the two plan tiers.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#PASS-TIER-LAW");

use vibe_core::lifecycle::{CompilePoint, ExtensionPoint};
use vibe_core::manifest::{ExtensionHandler, ExtensionPass};
use vibe_extension_registry::{ExtensionProvider, ExtensionRegistryRow};

use super::fault::{CompilePlanLoweringError, PassTierFault};
use super::plan::{PassEntry, PassPlacement, PassPlan};
use crate::compiler::transform::plan::{TransformPlan, TransformProvider};

/// The two immutable owner-scoped compiler plans produced from one row walk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilePlans {
    transforms: TransformPlan,
    passes: PassPlan,
}

impl CompilePlans {
    #[must_use]
    pub fn transforms(&self) -> &TransformPlan {
        &self.transforms
    }

    #[must_use]
    pub fn passes(&self) -> &PassPlan {
        &self.passes
    }
}

/// Partition one existing enabled-compile-index view without recollection.
pub fn lower_effective_compile_rows(
    rows: &[&ExtensionRegistryRow],
) -> Result<CompilePlans, CompilePlanLoweringError> {
    let mut staged = Vec::new();
    let mut passes = Vec::new();
    let mut effective_ordinal = 0_usize;
    for (row_index, row) in rows.iter().enumerate() {
        // Defensive and harmless for a caller that supplies an all-row view:
        // disable/inactivity can never be turned back on by this lowering.
        if !row.is_enabled() {
            continue;
        }
        let ordinal = u32::try_from(effective_ordinal).map_err(|_| PassTierFault::OrderOverflow)?;
        effective_ordinal += 1;
        match row.declaration().point {
            ExtensionPoint::Compile(CompilePoint::Pass) => {
                passes.push(lower_pass(row_index, ordinal, row)?);
            }
            ExtensionPoint::Compile(_) => staged.push(*row),
            point => {
                return Err(PassTierFault::NonCompilePoint {
                    row: row_index,
                    key: bounded_key(row),
                    point: point.to_string(),
                }
                .into());
            }
        }
    }
    let transforms = TransformPlan::from_effective_rows(&staged)
        .map_err(|source| PassTierFault::Transform { source })?;
    let passes = PassPlan::build(passes)?;
    Ok(CompilePlans { transforms, passes })
}

fn lower_pass(
    row_index: usize,
    ordinal: u32,
    row: &ExtensionRegistryRow,
) -> Result<PassEntry, CompilePlanLoweringError> {
    if matches!(row.provider(), ExtensionProvider::Dependency(_))
        && row.activation_ordinal().is_none()
    {
        return Err(PassTierFault::DependencyNotActivated {
            row: row_index,
            key: bounded_key(row),
        }
        .into());
    }
    let declaration = row.declaration();
    let pass = declaration
        .pass
        .as_ref()
        .ok_or_else(|| PassTierFault::MissingPass {
            row: row_index,
            key: bounded_key(row),
        })?;
    match declaration.handler {
        ExtensionHandler::Builtin { .. } | ExtensionHandler::Native { .. } => {}
        ref handler => {
            return Err(PassTierFault::UnsupportedHandler {
                row: row_index,
                key: bounded_key(row),
                kind: handler.kind(),
            }
            .into());
        }
    }
    Ok(PassEntry::new(
        ordinal,
        row.key().clone(),
        TransformProvider::from(row.provider()),
        row.effective_config().cloned(),
        declaration.handler.clone(),
        pass.clone(),
        placement(pass),
    ))
}

fn placement(pass: &ExtensionPass) -> PassPlacement {
    if let Some(anchor) = &pass.before {
        PassPlacement::Before(anchor.clone())
    } else if let Some(anchor) = &pass.after {
        PassPlacement::After(anchor.clone())
    } else if let Some(anchor) = &pass.replace {
        PassPlacement::Replace(anchor.clone())
    } else {
        PassPlacement::Intrinsic
    }
}

fn bounded_key(row: &ExtensionRegistryRow) -> String {
    let key = row.key().as_str();
    let mut preview = key.chars().take(120).collect::<String>();
    if key.chars().count() > 120 {
        preview.push('…');
    }
    preview
}
