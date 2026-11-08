//! The lifecycle scratch an install or update run owns until it ends.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ENVELOPE-LAW");

use vibe_lifecycle::{LifecycleLease, RunMetadata};

/// The lifecycle scratch of one fresh run, discarded when the operation that
/// allocated it fails or stops: the execution records stay in
/// `.vibe/lifecycle.toml` (PROP-054 `#OBS-RUN-RECORD`), the captured output
/// was already read, and the scratch files are garbage. An adopted (resumed)
/// run keeps its scratch for the resume, as does a parked one, whose outbox
/// [`vibe_lifecycle::process::discard_run_scratch`] refuses to touch
/// (`#REF-AGENT-RESUME`).
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#ENVELOPE-LAW")]
pub struct RunScratch(Option<(std::sync::Arc<LifecycleLease>, String)>);

impl RunScratch {
    pub fn fresh(
        lease: &std::sync::Arc<LifecycleLease>,
        metadata: &RunMetadata,
        adopted: bool,
    ) -> Self {
        Self((!adopted).then(|| (lease.clone(), metadata.run_id.clone())))
    }

    /// Discard the scratch when the operation `failed`.
    pub fn settle(self, failed: bool) {
        if failed && let Some((lease, run_id)) = self.0 {
            let _ = vibe_lifecycle::process::discard_run_scratch(lease.root(), &run_id);
        }
    }
}
