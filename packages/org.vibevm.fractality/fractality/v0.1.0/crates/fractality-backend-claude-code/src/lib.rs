//! The Claude Code worker backend.
//!
//! Phase 1 ships the provider-facing **facts** as constants — the exact
//! environment surface a headless Claude Code worker is configured
//! through, pinned by the Phase 0 spikes (plan F2/F3/F4). Phase 2 builds
//! the [`fractality_core::WorkerBackend`] implementation on top of them:
//! profile resolution (D6), the clean-slate environment constructor with
//! the poisoned-parent test (D5/I1), and the headless invocation.

pub mod env;

specmark::scope!("spec://fractality/PROP-001#architecture");

/// Marker type for the backend; gains its `WorkerBackend` impl in Phase 2.
///
/// ```
/// use fractality_backend_claude_code::ClaudeCodeBackend;
///
/// let backend = ClaudeCodeBackend;
/// assert_eq!(ClaudeCodeBackend::ID, "claude-code");
/// let _ = backend; // Phase 2 wires this into fractality_core::WorkerBackend
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct ClaudeCodeBackend;

impl ClaudeCodeBackend {
    /// Stable backend id, recorded per run.
    pub const ID: &'static str = "claude-code";
}
