//! The T6c lane vehicles: one behavior per property the manager-side lane
//! admission gate is supposed to decide.
//!
//! Every vehicle here returns a CHANGED `LaneIr` — that is the point of T6c —
//! and each one changes exactly one thing, so the refusal (or acceptance) it
//! provokes cannot be produced by any other rule:
//!
//! * [`ReorderLane`] reorders contributions and leaves the carried
//!   `contribution` indices stale — the intrinsic contract refuses it;
//! * [`RenumberedReorderLane`] does the same reordering AND renumbers — the
//!   lawful change, which must be accepted and must move the bytes;
//! * the six provenance vehicles each rewrite exactly one immutable field.
//!
//! They extend a CLONE of the shared identity registry, never `builtins()`,
//! and never alter the T5 golden.

use crate::compiler::ir::{
    ArtifactContext, ArtifactFrame, LaneChunk, LaneContribution, LaneFrame, LaneIr, LaneNode,
    LinkInputDigest, OriginRename, StaticCompileMode,
};

use super::behavior::{TransformBehavior, TransformBehaviorError};
use super::plan::{TransformConfig, TransformStage};

// One shared per-thread counter for every lane vehicle: a lane position runs
// once per artifact, so a test that installs one vehicle reads its own count.
std::thread_local! {
    pub(super) static LANE_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

pub(super) fn reset_lane_vehicle_count() {
    LANE_COUNT.with(|count| count.set(0));
}

pub(super) fn lane_vehicle_invocations() -> usize {
    LANE_COUNT.with(std::cell::Cell::get)
}

/// Declare one lane-stage vehicle around a pure `LaneIr -> LaneIr` rewrite.
macro_rules! lane_vehicle {
    ($type:ident, $name:literal, $rewrite:path) => {
        pub(super) struct $type;

        impl TransformBehavior for $type {
            fn name(&self) -> &str {
                $name
            }
            fn epoch(&self) -> u32 {
                1
            }
            fn stage(&self) -> TransformStage {
                TransformStage::Lane
            }
            fn run_lane(
                &self,
                _config: Option<&TransformConfig>,
                input: LaneIr,
            ) -> Result<LaneIr, TransformBehaviorError> {
                LANE_COUNT.with(|count| count.set(count.get() + 1));
                Ok($rewrite(input))
            }
        }
    };
}

lane_vehicle!(ReorderLane, "test-lane-reorder", reordered);
lane_vehicle!(
    RenumberedReorderLane,
    "test-lane-renumber",
    renumbered_reorder
);
lane_vehicle!(RewriteContextLane, "test-lane-context", rewritten_context);
lane_vehicle!(BumpNodeCountLane, "test-lane-node-count", bumped_node_count);
lane_vehicle!(RewriteDigestLane, "test-lane-digest", rewritten_digest);
lane_vehicle!(
    RewriteGeneratedPathLane,
    "test-lane-generated-path",
    rewritten_generated_path
);
lane_vehicle!(
    RewriteSourceRootLane,
    "test-lane-source-root",
    rewritten_source_root
);
lane_vehicle!(RewriteRenamesLane, "test-lane-renames", rewritten_renames);

/// The forged spellings, kept in one place so a test can assert on the exact
/// value that reached the refusal.
pub(super) const FORGED_GENERATED_PATH: &str = "vibevm/vibespecs/boot/FORGED.xml";
pub(super) const FORGED_SOURCE_ROOT: &str = "vibevm/vibeforged";
pub(super) const FORGED_DIGEST: LinkInputDigest = LinkInputDigest([0x5a; 32]);

/// Reverse the contributions and leave every carried `contribution` index
/// pointing at the position it used to occupy.
fn reordered(input: LaneIr) -> LaneIr {
    let (context, source_node_count, digest, frame, contributions) = input.parts_for_test();
    let mut contributions = contributions.to_vec();
    contributions.reverse();
    LaneIr::assembled(
        context.clone(),
        source_node_count,
        digest.clone(),
        frame.clone(),
        contributions,
    )
}

/// The same reordering, renumbered: the lawful working-surface rewrite.
fn renumbered_reorder(input: LaneIr) -> LaneIr {
    let (context, source_node_count, digest, frame, contributions) = input.parts_for_test();
    let mut contributions = contributions.to_vec();
    contributions.reverse();
    renumber(&mut contributions);
    LaneIr::assembled(
        context.clone(),
        source_node_count,
        digest.clone(),
        frame.clone(),
        contributions,
    )
}

/// Rewrite every carried `contribution` index to the position the enclosing
/// contribution now occupies — what the intrinsic contract demands of any
/// reordering.
fn renumber(contributions: &mut [LaneContribution]) {
    for (index, entry) in contributions.iter_mut().enumerate() {
        let chunks = match entry {
            LaneContribution::Normal { chunks, .. } | LaneContribution::Simple { chunks, .. } => {
                chunks
            }
            LaneContribution::Elided { .. } | LaneContribution::Hoisted { .. } => continue,
        };
        for chunk in chunks {
            match chunk {
                LaneChunk::NormalOpen { contribution, .. }
                | LaneChunk::ForcedNewline { contribution, .. }
                | LaneChunk::NormalClose { contribution, .. } => *contribution = index,
                LaneChunk::Node(node) => match node.as_mut() {
                    LaneNode::Normal { contribution, .. }
                    | LaneNode::Simple { contribution, .. } => *contribution = index,
                },
            }
        }
    }
}

/// Flip the compile mode only: the frame halves still agree, so the intrinsic
/// contract accepts it and only the transition can catch it.
fn rewritten_context(input: LaneIr) -> LaneIr {
    let (context, source_node_count, digest, frame, contributions) = input.parts_for_test();
    let forged = ArtifactContext::testing(
        context.artifact().clone(),
        context.target(),
        context.frame().clone(),
        StaticCompileMode::Plain,
    );
    LaneIr::assembled(
        forged,
        source_node_count,
        digest.clone(),
        frame.clone(),
        contributions.to_vec(),
    )
}

/// Widen the node bound: the intrinsic contract only checks that carried node
/// ids stay BELOW it, so a larger count passes there.
fn bumped_node_count(input: LaneIr) -> LaneIr {
    let (context, source_node_count, digest, frame, contributions) = input.parts_for_test();
    LaneIr::assembled(
        context.clone(),
        source_node_count + 1,
        digest.clone(),
        frame.clone(),
        contributions.to_vec(),
    )
}

/// Replace the link-input digest, which no intrinsic rule inspects at all.
fn rewritten_digest(input: LaneIr) -> LaneIr {
    let (context, source_node_count, _, frame, contributions) = input.parts_for_test();
    LaneIr::assembled(
        context.clone(),
        source_node_count,
        FORGED_DIGEST,
        frame.clone(),
        contributions.to_vec(),
    )
}

/// Rewrite the generated path in BOTH halves — the only shape in which such a
/// rewrite survives the intrinsic frame/context agreement check.
fn rewritten_generated_path(input: LaneIr) -> LaneIr {
    reframed(input, Some(FORGED_GENERATED_PATH), None)
}

/// The same, for the source root.
fn rewritten_source_root(input: LaneIr) -> LaneIr {
    reframed(input, None, Some(FORGED_SOURCE_ROOT))
}

fn reframed(input: LaneIr, generated_path: Option<&str>, source_root: Option<&str>) -> LaneIr {
    let (context, source_node_count, digest, frame, contributions) = input.parts_for_test();
    let ArtifactFrame::StaticLane {
        generated_path: current_path,
        source_root: current_root,
    } = context.frame()
    else {
        unreachable!("the shared fixture compiles a static lane")
    };
    let path = generated_path.unwrap_or(current_path.as_str()).to_string();
    let root = source_root.unwrap_or(current_root.as_str()).to_string();
    let forged = ArtifactContext::testing(
        context.artifact().clone(),
        context.target(),
        ArtifactFrame::StaticLane {
            generated_path: path.clone(),
            source_root: root.clone(),
        },
        context.mode(),
    );
    LaneIr::assembled(
        forged,
        source_node_count,
        digest.clone(),
        LaneFrame {
            generated_path: Some(path),
            source_root: Some(root),
            renames: frame.renames.clone(),
        },
        contributions.to_vec(),
    )
}

/// Append one rename: `frame.renames` flows onward into
/// `EmissionProvenance.renames`, so this is the forgery the boundary exists
/// to stop, and no intrinsic rule looks at it.
fn rewritten_renames(input: LaneIr) -> LaneIr {
    let (context, source_node_count, digest, frame, contributions) = input.parts_for_test();
    let mut renames = frame.renames.clone();
    renames.push(OriginRename {
        origin: "org.demo/forged".to_string(),
        rename: crate::RenameEntry {
            original: "root".to_string(),
            qualified: "org-demo-forged--root".to_string(),
        },
    });
    LaneIr::assembled(
        context.clone(),
        source_node_count,
        digest.clone(),
        LaneFrame {
            generated_path: frame.generated_path.clone(),
            source_root: frame.source_root.clone(),
            renames,
        },
        contributions.to_vec(),
    )
}
