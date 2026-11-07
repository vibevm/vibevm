//! Waiting policy for the HTTPS single-file read path.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-001#backend");
use std::{sync::Arc, time::Duration};

/// Where the HTTPS read fast path's pauses go.
///
/// A named type rather than a bare closure field for two reasons: the
/// backend keeps its derived [`Debug`] (a closure has nothing to print),
/// and `anonymized_for_public` can hand its copy the very same waiting
/// behaviour instead of quietly reverting to real time.
#[derive(Clone)]
pub(in crate::git_backend) struct Sleeper(pub(super) Arc<dyn Fn(Duration) + Send + Sync>);

impl Sleeper {
    /// Wait `how_long` — really, or however the test that installed this
    /// one chooses to.
    pub(in crate::git_backend) fn sleep(&self, how_long: Duration) {
        (self.0)(how_long)
    }
}

impl Default for Sleeper {
    fn default() -> Self {
        Sleeper(Arc::new(std::thread::sleep))
    }
}

impl std::fmt::Debug for Sleeper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Sleeper(..)")
    }
}
