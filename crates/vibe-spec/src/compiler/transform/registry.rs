//! The private transform behavior registry (R4-TRANSFORM-PLAN-ABI §6.1): the
//! deterministic name → {epoch, stage, behavior} catalog, its bounded
//! registration refusals and its resolution refusals.
//!
//! The registry is a `vibe-spec` sibling of `BackendRegistry` and nothing
//! like a second declaration collector: it reads no manifest, no registry
//! row and no filesystem — callers hand it already-typed values only. T5
//! shipped an EMPTY production catalog; R4.2 registers the first real
//! behavior, `xml-minify` at epoch 1. The four `test-identity-*` vehicles
//! still register only in the test cell, so no test scaffolding silently
//! becomes public manifest vocabulary. `TransformPlan::build` never consults
//! this registry: the plan stays grammar-only, and off-catalog T2 test
//! candidates remain legal plan values resolved — or refused — here.

// `catalog()` is the golden view the registry tests read; production resolves
// and never enumerates.
#![allow(dead_code)]

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY");

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::compiler::ir::BackendId;

use super::behavior::TransformBehavior;
use super::plan::{TransformImplementation, TransformStage};
use super::plan_validate::BoundedPreview;
use super::plan_validate::bounded;
use super::xml_minify_binding::XmlMinify;

/// One registered catalog row: the behavior epoch, its declared stage and the
/// behavior object itself, stored name-keyed and lent only as a clone.
struct RegisteredBehavior {
    epoch: u32,
    stage: TransformStage,
    behavior: Arc<dyn TransformBehavior>,
}

/// The closed transform-builtin catalog of one build, keyed by exact name in
/// deterministic (byte-sorted) order.
#[derive(Default)]
pub(crate) struct TransformRegistry {
    implementations: BTreeMap<String, RegisteredBehavior>,
}

impl TransformRegistry {
    /// The production catalog: exactly the behaviors that really ship.
    ///
    /// One row since R4.2 — `xml-minify` at epoch 1, declared for the emitted
    /// stage (R4 architecture §8). Registration refuses an invalid name, a
    /// zero epoch or a collision; none can hold for a const-spelled name at a
    /// nonzero epoch entering an empty catalog, so a refusal here would be a
    /// defect in THIS file rather than a runtime condition — the same reading
    /// `BackendRegistry::builtins` already applies to its two built-in
    /// backends. The exact `(name, epoch, stage)` golden is a test.
    pub(crate) fn builtins() -> Self {
        let mut registry = Self::default();
        registry
            .register(Arc::new(XmlMinify))
            .expect("xml-minify is the first valid production transform builtin");
        registry
    }

    /// Register one behavior under its own name, or refuse it.
    ///
    /// The name obeys the one frozen backend-id grammar — checked borrowed,
    /// never cloned into the refusal — the epoch is nonzero, and the name is
    /// not already cataloged. Epoch and stage are read from the behavior
    /// itself, so a stored row can never disagree with the object it holds.
    pub(crate) fn register(
        &mut self,
        behavior: Arc<dyn TransformBehavior>,
    ) -> Result<(), TransformRegistryError> {
        let name = behavior.name();
        if !BackendId::is_valid_spelling(name) {
            return Err(TransformRegistryError::InvalidName {
                preview: bounded(name),
            });
        }
        if behavior.epoch() == 0 {
            return Err(TransformRegistryError::EpochZero {
                preview: bounded(name),
            });
        }
        if self.implementations.contains_key(name) {
            return Err(TransformRegistryError::Collision {
                preview: bounded(name),
            });
        }
        let epoch = behavior.epoch();
        let stage = behavior.stage();
        self.implementations.insert(
            name.to_owned(),
            RegisteredBehavior {
                epoch,
                stage,
                behavior,
            },
        );
        Ok(())
    }

    /// The catalog's behavior epoch for one declared builtin name.
    ///
    /// The registry-owned half of implementation identity (ABI §2.1: a
    /// caller supplies a name, never an epoch). T10B's lowering calls it so
    /// an off-catalog name refuses AT LOWERING, through the same bounded
    /// `UnknownBuiltin` arm resolution already raises — one refusal, one
    /// spelling, two moments.
    ///
    /// Only the name is judged here. Stage agreement stays
    /// [`TransformRegistry::resolve`]'s, where the behavior it would return
    /// is the thing that disagrees; duplicating the check would be a second
    /// home for one law.
    pub(crate) fn epoch_of(&self, name: &str) -> Result<u32, TransformRegistryError> {
        self.implementations
            .get(name)
            .map(|registered| registered.epoch)
            .ok_or_else(|| TransformRegistryError::UnknownBuiltin {
                preview: bounded(name),
            })
    }

    /// Resolve one T2 implementation identity at one stage to its registered
    /// behavior, refusing — bounded, before any clone — an unknown name, a
    /// stale epoch or the wrong stage.
    pub(crate) fn resolve(
        &self,
        implementation: &TransformImplementation,
        stage: &TransformStage,
    ) -> Result<Arc<dyn TransformBehavior>, TransformRegistryError> {
        let name = implementation.builtin_name();
        let Some(registered) = self.implementations.get(name) else {
            return Err(TransformRegistryError::UnknownBuiltin {
                preview: bounded(name),
            });
        };
        let requested = implementation.builtin_epoch();
        if requested != registered.epoch {
            return Err(TransformRegistryError::EpochMismatch {
                preview: bounded(name),
                requested,
                catalog: registered.epoch,
            });
        }
        if stage != &registered.stage {
            return Err(TransformRegistryError::StageMismatch {
                preview: bounded(name),
                requested: stage.clone(),
                catalog: registered.stage.clone(),
            });
        }
        Ok(registered.behavior.clone())
    }

    /// The exact catalog rows `(name, epoch, stage)` in deterministic
    /// byte-sorted name order — the golden view.
    pub(crate) fn catalog(&self) -> Vec<(&str, u32, &TransformStage)> {
        self.implementations
            .iter()
            .map(|(name, row)| (name.as_str(), row.epoch, &row.stage))
            .collect()
    }
}

/// Why one registration or resolution refused: typed by fault, never echoing
/// a payload — a builtin name can be attacker-sized, so every error carries
/// at most the fixed-size preview plus the true length.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum TransformRegistryError {
    #[error(
        "transform builtin name violates the frozen backend-id grammar [a-z0-9][a-z0-9._-]{{0,63}} {preview}"
    )]
    InvalidName { preview: BoundedPreview },
    #[error("transform builtin {preview} declares epoch 0, which is not a behavior epoch")]
    EpochZero { preview: BoundedPreview },
    #[error("transform builtin {preview} is already registered")]
    Collision { preview: BoundedPreview },
    #[error("unknown transform builtin {preview}")]
    UnknownBuiltin { preview: BoundedPreview },
    #[error(
        "transform builtin {preview} requested at epoch {requested}, but this registry catalogs epoch {catalog}"
    )]
    EpochMismatch {
        preview: BoundedPreview,
        requested: u32,
        catalog: u32,
    },
    #[error(
        "transform builtin {preview} is cataloged for {catalog:?}, refusing the {requested:?} stage"
    )]
    StageMismatch {
        preview: BoundedPreview,
        requested: TransformStage,
        catalog: TransformStage,
    },
}
