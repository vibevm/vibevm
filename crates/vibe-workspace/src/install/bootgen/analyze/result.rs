//! The lane analyzer's result types: what one in-memory analysis returns.
//!
//! Split out of `analyze.rs` along the result/behaviour seam per the
//! file-length budget. Each carries the compile's output beside its
//! attribution side; the analysis itself stays in the parent cell.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#install");

use vibe_spec::{DocumentProvider, EmittedArtifact};

use crate::boot_artifacts;

/// One analyzed lane: the compile's result beside its attribution side.
#[derive(Debug, Clone)]
pub struct AnalyzedLane {
    /// The emitted artifact — byte-equal to what regeneration would
    /// write, held in memory; nothing was written.
    pub artifact: EmittedArtifact,
    /// The typed provider that declared each plan input, in input order.
    /// `None` marks an elided/hoisted seat whose provenance does not type
    /// through the one grammar — an unattributable seat the analyzer
    /// refuses rather than half-reports.
    pub providers: Vec<Option<DocumentProvider>>,
    /// Each input's display provenance `(origin, path)`, in input order,
    /// for the alignment fence between the authored plan and the
    /// compiler's returned evidence.
    pub identities: Vec<(String, String)>,
}

impl AnalyzedLane {
    /// The admitted lane bytes, as one opaque segment.
    ///
    /// This is the form the system-prompt prefix takes the lane in: the
    /// bytes regeneration would publish, reused rather than recompiled, and
    /// never re-entered into the compiler as a fresh normal source
    /// (PROP-076 `#SYS-LANE-VERBATIM`). The accessor exists so a capture
    /// caller never reaches through `artifact` and acquires the habit of
    /// looking inside the segment — nothing may read the lane's content.
    pub fn lane_bytes(&self) -> &[u8] {
        self.artifact.bytes()
    }
}

pub struct AnalyzedBoundLane {
    pub artifact: EmittedArtifact,
    pub native: Option<boot_artifacts::OwnerNativeCompileContinuation>,
    pub providers: Vec<Option<DocumentProvider>>,
    pub identities: Vec<(String, String)>,
}
