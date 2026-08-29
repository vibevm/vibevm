//! The whole-artifact invocation plan: ordered inputs in effective-boot
//! order plus the owner-scoped transform plan carriage (R4.1 T4,
//! `R4-TRANSFORM-PLAN-ABI-v0.1.md` §7.1).

use std::collections::BTreeMap;

use crate::SpecAddress;
use crate::compiler::ir::{ArtifactTarget, SourceIr};
use crate::compiler::transform::plan::TransformPlan;

use super::{
    ArtifactContext, ArtifactFrame, ArtifactId, ArtifactInput, ArtifactInputKind,
    ArtifactInputType, ArtifactInputWitness, ArtifactPlanError, ContributionMeta,
    StaticCompileMode,
};

/// Immutable invocation input for compiling one artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactPlan {
    context: ArtifactContext,
    contributions: Vec<ArtifactInput>,
    transforms: TransformPlan,
}

impl ArtifactPlan {
    pub(crate) fn new(
        context: ArtifactContext,
        contributions: Vec<ArtifactInput>,
    ) -> Result<Self, ArtifactPlanError> {
        // The identity is the typed (origin, path) pair, never a joined string:
        // a delimiter spelling cannot separate ("a", "b\0c") from ("a\0b", "c"),
        // so a plan holding both would drop one conflict check on the floor.
        let mut simple_identities: BTreeMap<(&str, &str), (usize, &SourceIr)> = BTreeMap::new();
        for (index, contribution) in contributions.iter().enumerate() {
            contribution
                .validate()
                .map_err(|source| ArtifactPlanError::Input {
                    index,
                    source: Box::new(source),
                })?;
            if let ArtifactInputKind::Simple { meta, source } = contribution.kind() {
                if super::simple_alias_machinery(source.text()) {
                    return Err(ArtifactPlanError::SimpleAlias {
                        index,
                        origin: meta.origin.clone(),
                    });
                }
                let key = (meta.origin.as_str(), meta.path.as_str());
                if let Some((first, prior)) = simple_identities.get(&key) {
                    if *prior != source {
                        return Err(ArtifactPlanError::ConflictingSimpleIdentity {
                            first: *first,
                            second: index,
                            origin: meta.origin.clone(),
                            path: meta.path.clone(),
                        });
                    }
                } else {
                    simple_identities.insert(key, (index, source));
                }
            }
        }
        Ok(Self {
            context,
            contributions,
            transforms: TransformPlan::empty(),
        })
    }

    pub(crate) fn compatibility(seed: SpecAddress, mode: StaticCompileMode) -> Self {
        let meta = ContributionMeta {
            origin: crate::compiler::close::document_origin(&seed),
            path: seed.doc_path.clone(),
        };
        Self {
            context: ArtifactContext::compatibility(mode),
            contributions: vec![ArtifactInput::from_kind(ArtifactInputKind::Normal {
                meta,
                seed,
            })],
            transforms: TransformPlan::empty(),
        }
    }

    /// Build one validated final STATIC artifact in exact input order.
    pub fn static_lane(
        target: ArtifactTarget,
        generated_path: impl Into<String>,
        source_root: impl Into<String>,
        contributions: Vec<ArtifactInput>,
    ) -> Result<Self, ArtifactPlanError> {
        let artifact = target.backend_id();
        if target.is_custom() {
            return Err(ArtifactPlanError::InvalidContextTuple {
                artifact: artifact.to_string(),
                target,
                frame: "StaticLane".to_string(),
                mode: format!("{:?}", StaticCompileMode::QualifyPerNode),
            });
        }
        let context = ArtifactContext::new(
            ArtifactId::new(artifact)?,
            target,
            ArtifactFrame::StaticLane {
                generated_path: generated_path.into(),
                source_root: source_root.into(),
            },
            StaticCompileMode::QualifyPerNode,
        )?;
        Self::new(context, contributions)
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn custom_for_test(
        backend: &'static str,
        contributions: Vec<ArtifactInput>,
    ) -> Result<Self, ArtifactPlanError> {
        let target = ArtifactTarget::custom(backend)
            .expect("the test-support backend id is a valid BackendId");
        let context = ArtifactContext::new(
            ArtifactId::new(backend)?,
            target,
            ArtifactFrame::CompatibilityFragment,
            StaticCompileMode::Plain,
        )?;
        Self::new(context, contributions)
    }

    /// Attach an already-built owner-scoped transform plan to this artifact.
    ///
    /// Whole-value replacement with no mutable entry/order surface: the only
    /// route to a nonempty plan is [`TransformPlan::build`], which owns the
    /// refusal law and assigns the dense effective order itself, so a
    /// caller-authored or sparse order cannot cross this seam. Every
    /// constructor pins [`TransformPlan::empty`]; the compatibility wrappers
    /// stay permanently empty-plan.
    ///
    /// `pub` since T10B, as T4 promised ("T10 widens only the minimum needed
    /// by the workspace adapter"): the adapter lowers one lane owner's rows
    /// and attaches the result here. It is still whole-value replacement of
    /// a value only [`TransformPlan::build`] can have produced, so widening
    /// it grants no entry, order or digest authority.
    pub fn with_transforms(self, transforms: TransformPlan) -> Self {
        Self { transforms, ..self }
    }

    /// The owner-scoped transform plan this artifact carries.
    pub(crate) fn transforms(&self) -> &TransformPlan {
        &self.transforms
    }

    pub(crate) fn context(&self) -> &ArtifactContext {
        &self.context
    }

    pub(crate) fn contributions(&self) -> &[ArtifactInput] {
        &self.contributions
    }

    pub(crate) fn input_witness(&self, index: usize) -> Option<ArtifactInputWitness> {
        let input = self.contributions.get(index)?;
        Some(ArtifactInputWitness {
            index,
            origin: input.meta().origin.clone(),
            path: input.meta().path.clone(),
            kind: match input.kind() {
                ArtifactInputKind::Normal { .. } => ArtifactInputType::Normal,
                ArtifactInputKind::Simple { .. } => ArtifactInputType::Simple,
                ArtifactInputKind::Elided { .. } => ArtifactInputType::Elided,
                ArtifactInputKind::Hoisted { .. } => ArtifactInputType::Hoisted,
            },
        })
    }
}

#[cfg(test)]
mod carriage_tests;
