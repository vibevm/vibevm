//! The private transform-behavior family (R4-TRANSFORM-PLAN-ABI §6.1): one
//! stage-declared, level-preserving behavior contract over the four compiler
//! carriers, and its typed wrong-stage refusal.
//!
//! T5 landed the family with NO production builtin, and the four identity
//! behaviors that exercise this trait still live only in the test cell as
//! `test-identity-*` vehicles. R4.2 registered the first REAL one —
//! [`super::xml_minify_binding::XmlMinify`] — so the family's refusal enum
//! now has a second arm carrying that binding's typed refusal. Behavior
//! objects stay inside the transform cells — nothing here crosses the crate
//! boundary, and the plan, digest, refusal and config cells remain free of
//! `Arc`/`dyn` by the syntax fence.

// Four of the trait's methods have only cfg-test referents (a behavior
// declares ONE stage, so three of them are the typed wrong-stage default for
// every implementation); the wrapper cell and the tests are their referents.
#![allow(dead_code)]

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY");

use crate::compiler::ir::{DocumentIr, LaneIr, SourceIr};

use super::plan::{TransformConfig, TransformStage};
use super::plan_validate::BoundedPreview;
use super::xml_minify_binding::XmlMinifyBindingError;

/// One transform behavior: a catalog name, a nonzero behavior epoch, one
/// declared stage, and four level-preserving invocations.
///
/// A behavior declares exactly one stage; invoking any other stage's method
/// yields the typed [`TransformBehaviorError::WrongStage`] default. Every
/// method receives the entry's exact effective configuration — `None` when
/// none was authored, `Some(empty)` when an authored activation cleared the
/// value — even when the behavior ignores it, so the delivery seam is fixed
/// by the family rather than re-cut per behavior.
pub(crate) trait TransformBehavior: Send + Sync {
    /// The builtin's exact catalog name; the one backend-id grammar
    /// `[a-z0-9][a-z0-9._-]{0,63}` is checked at registration.
    fn name(&self) -> &str;

    /// The registry-owned behavior epoch: nonzero, and moving whenever
    /// observable behavior moves.
    fn epoch(&self) -> u32;

    /// The one staged tier this behavior is declared for.
    fn stage(&self) -> TransformStage;

    /// Source-level invocation: one document's raw text before parsing.
    fn run_source(
        &self,
        _config: Option<&TransformConfig>,
        _input: SourceIr,
    ) -> Result<SourceIr, TransformBehaviorError> {
        Err(self.wrong_stage(TransformStage::Source))
    }

    /// Document-level invocation: one document's parsed tree before it enters
    /// the closure.
    fn run_document(
        &self,
        _config: Option<&TransformConfig>,
        _input: DocumentIr,
    ) -> Result<DocumentIr, TransformBehaviorError> {
        Err(self.wrong_stage(TransformStage::Document))
    }

    /// Lane-level invocation: the assembled lane after assemble, still
    /// structured.
    fn run_lane(
        &self,
        _config: Option<&TransformConfig>,
        _input: LaneIr,
    ) -> Result<LaneIr, TransformBehaviorError> {
        Err(self.wrong_stage(TransformStage::Lane))
    }

    /// Emitted-level invocation: owned artifact bytes in, new bytes out —
    /// never a mutable artifact reference.
    fn run_emitted(
        &self,
        _config: Option<&TransformConfig>,
        _input: Vec<u8>,
    ) -> Result<Vec<u8>, TransformBehaviorError> {
        Err(self.wrong_stage(TransformStage::Emitted))
    }

    /// The shared wrong-stage refusal: the bounded name, the declared stage
    /// and the stage actually invoked, so the exact fault is identifiable
    /// without echoing a possibly attacker-sized name.
    fn wrong_stage(&self, called: TransformStage) -> TransformBehaviorError {
        TransformBehaviorError::WrongStage {
            preview: super::plan_validate::bounded(self.name()),
            declared: self.stage(),
            called,
        }
    }
}

/// Why one behavior invocation refused.
///
/// Two arms, and they are different claims. [`TransformBehaviorError::WrongStage`]
/// is the family's own law — a behavior was invoked at a stage it does not
/// declare — and every implementation shares it. [`TransformBehaviorError::EmittedTape`]
/// is a behavior refusing the CARRIER it was handed; it names the one
/// production binding's typed refusal directly rather than a rendered string,
/// because a typed source is what a caller downcasting this family needs, and
/// the `Box<dyn …>` an open family would take is banned in these cells.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum TransformBehaviorError {
    #[error("transform behavior {preview} declares {declared:?}, refusing a {called:?} invocation")]
    WrongStage {
        preview: BoundedPreview,
        declared: TransformStage,
        called: TransformStage,
    },
    #[error("transform behavior {preview} refused the emitted artifact it was handed: {source}")]
    EmittedTape {
        preview: BoundedPreview,
        #[source]
        source: XmlMinifyBindingError,
    },
}
