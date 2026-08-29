//! Selected whole-artifact Lane -> Emitted lowering.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR");

use std::sync::Arc;

use super::assemble::{LaneValidationError, validate_lane};
use super::backend::{BackendError, BackendId, EmitBackend};
use super::ir::{
    EmissionContributionWitness, EmissionProvenance, EmittedArtifact, LaneChunk, LaneContribution,
    LaneIr, LaneNode, PreEmissionWitness, PreparedEmissionTarget,
};
use super::pass::{Pass, PassName};

mod digest;
pub(crate) use digest::bytes_digest as emitted_bytes_digest;
pub(crate) mod framing;
#[cfg(feature = "test-support")]
pub(crate) mod opaque_test_vehicle;
pub(crate) mod static_md;
pub(crate) mod static_xml;
mod validate;

pub(crate) struct EmitPass {
    backend: Arc<dyn EmitBackend>,
    /// The artifact's active-transforms header payload (R4 architecture
    /// §7.1), computed once by the schedule builder from the owner plan the
    /// artifact carries. `None` is the empty plan — and the exact historical
    /// bytes.
    transforms_header: Option<String>,
}

impl EmitPass {
    pub(crate) fn new(backend: Arc<dyn EmitBackend>, transforms_header: Option<String>) -> Self {
        Self {
            backend,
            transforms_header,
        }
    }
}

impl Pass for EmitPass {
    type Input = LaneIr;
    type Output = EmittedArtifact;
    type Error = EmitPassError;

    fn name(&self) -> &PassName {
        self.backend.pass_name()
    }

    fn run(&self, lane: LaneIr) -> Result<EmittedArtifact, EmitPassError> {
        validate_lane(&lane).map_err(|error| EmitPassError::InvalidLane(Box::new(error)))?;
        let target = lane.context().target();
        let expected_backend = target.backend_id();
        if self.backend.id().as_str() != expected_backend {
            return Err(EmitPassError::TargetMismatch {
                backend: self.backend.id().as_str().to_string(),
                expected: expected_backend.to_string(),
                actual: target,
            });
        }
        let witness = capture_witness(&lane, self.backend.id(), self.transforms_header.clone())
            .map_err(EmitPassError::Backend)?;
        #[cfg(test)]
        record_invocation(self.backend.id());
        let bytes = self
            .backend
            .emit(&lane, &witness)
            .map_err(EmitPassError::Backend)?;
        let provenance = build_provenance(
            &witness,
            self.backend.id().clone(),
            self.backend.pass_name().clone(),
            &bytes,
        );
        let emitted = EmittedArtifact { provenance, bytes };
        validate::transition(
            self.backend.id(),
            self.backend.pass_name(),
            &witness,
            &lane,
            &emitted,
        )
        .map_err(EmitPassError::Backend)?;
        validate::current(
            self.backend.id(),
            self.backend.pass_name(),
            &witness,
            emitted.bytes(),
            emitted.provenance(),
        )
        .map_err(EmitPassError::Backend)?;
        Ok(emitted)
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum EmitPassError {
    #[error("emit requires a valid Lane: {0}")]
    InvalidLane(#[source] Box<LaneValidationError>),
    #[error("backend `{backend}` cannot serve target `{expected}` ({actual:?})")]
    TargetMismatch {
        backend: String,
        expected: String,
        actual: super::ir::ArtifactTarget,
    },
    #[error(transparent)]
    Backend(#[from] BackendError),
}

fn build_provenance(
    witness: &PreEmissionWitness,
    backend: BackendId,
    producer: PassName,
    bytes: &[u8],
) -> EmissionProvenance {
    EmissionProvenance {
        context: witness.context.clone(),
        backend,
        producer,
        source_lane_digest: witness.lane_digest.clone(),
        renames: witness.frame.renames.clone(),
        contributions: witness.emission_witnesses.clone(),
        // Empty at emission: the backend produced these bytes and no
        // post-backend transform has rewritten them yet. Only
        // `transform::emitted_reconstruction` ever appends, and only when a
        // behavior really returned different bytes — so an artifact nothing
        // changed carries exactly the value this member had before it existed
        // (R4-TRANSFORM-PLAN-ABI §6.5).
        emitted_transforms: Vec::new(),
        bytes_digest: digest::bytes_digest(bytes),
    }
}

/// The pass's own witness capture, for tests that drive a backend directly
/// instead of building a whole schedule. Production always reaches it through
/// [`EmitPass::run`].
#[cfg(test)]
pub(crate) fn capture_witness_for_test(
    lane: &LaneIr,
    backend: &BackendId,
) -> Result<PreEmissionWitness, BackendError> {
    capture_witness(lane, backend, None)
}

fn capture_witness(
    lane: &LaneIr,
    backend: &BackendId,
    transforms_header: Option<String>,
) -> Result<PreEmissionWitness, BackendError> {
    Ok(PreEmissionWitness {
        context: lane.context().clone(),
        source_node_count: lane.source_node_count,
        source_link_digest: lane.source_link_digest.clone(),
        frame: lane.frame.clone(),
        contributions: lane.contributions.clone(),
        lane_digest: digest::lane_digest(lane),
        emission_witnesses: contribution_witnesses(lane),
        prepared_target: prepare_target(lane, backend)?,
        transforms_header,
    })
}

fn prepare_target(
    lane: &LaneIr,
    backend: &BackendId,
) -> Result<PreparedEmissionTarget, BackendError> {
    let target = lane.context().target();
    if target.is_static_markdown() {
        return Ok(PreparedEmissionTarget::Markdown);
    }
    if target.is_static_xml() {
        let mut documents = Vec::with_capacity(lane.contributions.len());
        for contribution in &lane.contributions {
            let document = match contribution {
                LaneContribution::Normal { meta, chunks, .. }
                | LaneContribution::Simple { meta, chunks, .. } => {
                    #[cfg(test)]
                    static_xml::record_pivot_call();
                    let markdown = xml_markdown(chunks);
                    Some(vibe_specdoc::from_markdown(&markdown).map_err(|error| {
                        BackendError::Emit {
                            backend: backend.as_str().to_string(),
                            reason: format!(
                                "converting static contribution `{}` to XML: {error}",
                                meta.origin
                            ),
                        }
                    })?)
                }
                LaneContribution::Elided { .. } | LaneContribution::Hoisted { .. } => None,
            };
            documents.push(document);
        }
        return Ok(PreparedEmissionTarget::Xml { documents });
    }
    #[cfg(any(test, feature = "test-support"))]
    if target.is_custom() {
        return Ok(PreparedEmissionTarget::Custom);
    }
    Err(BackendError::Emit {
        backend: backend.as_str().to_string(),
        reason: "unsupported artifact target".to_string(),
    })
}

fn xml_markdown(chunks: &[LaneChunk]) -> String {
    let mut output = String::new();
    for chunk in chunks {
        match chunk {
            LaneChunk::NormalOpen { .. } | LaneChunk::NormalClose { .. } => {}
            LaneChunk::Node(node) => match node.as_ref() {
                LaneNode::Normal { body, .. } | LaneNode::Simple { body, .. } => {
                    output.push_str(body)
                }
            },
            LaneChunk::ForcedNewline { .. } => output.push('\n'),
        }
    }
    output
}

fn contribution_witnesses(lane: &LaneIr) -> Vec<EmissionContributionWitness> {
    lane.contributions
        .iter()
        .map(|contribution| match contribution {
            LaneContribution::Normal {
                meta,
                seed,
                seed_address,
                chunks,
            } => EmissionContributionWitness::Normal {
                meta: meta.clone(),
                seed: *seed,
                seed_address: seed_address.clone(),
                chunk_digest: digest::chunks_digest(chunks),
            },
            LaneContribution::Simple {
                meta,
                address,
                chunks,
            } => EmissionContributionWitness::Simple {
                meta: meta.clone(),
                address: address.clone(),
                chunk_digest: digest::chunks_digest(chunks),
            },
            LaneContribution::Elided { meta } => {
                EmissionContributionWitness::Elided { meta: meta.clone() }
            }
            LaneContribution::Hoisted { meta, target } => EmissionContributionWitness::Hoisted {
                meta: meta.clone(),
                target: target.clone(),
            },
        })
        .collect()
}

#[cfg(test)]
std::thread_local! {
    static INVOCATIONS: std::cell::RefCell<std::collections::BTreeMap<String, usize>> =
        const { std::cell::RefCell::new(std::collections::BTreeMap::new()) };
}

#[cfg(test)]
fn record_invocation(backend: &BackendId) {
    INVOCATIONS.with(|counts| {
        *counts
            .borrow_mut()
            .entry(backend.as_str().to_string())
            .or_default() += 1;
    });
}

#[cfg(test)]
pub(crate) fn reset_emit_invocations() {
    INVOCATIONS.with(|counts| counts.borrow_mut().clear());
}

#[cfg(test)]
pub(crate) fn emit_invocations(backend: &str) -> usize {
    INVOCATIONS.with(|counts| counts.borrow().get(backend).copied().unwrap_or(0))
}

#[cfg(test)]
#[path = "emit/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "emit/mutant_tests.rs"]
mod mutant_tests;

#[cfg(test)]
#[path = "emit/digest_tests.rs"]
mod digest_tests;
