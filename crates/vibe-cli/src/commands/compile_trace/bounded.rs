//! The one clamp, and a type that carries the proof of it.
//!
//! The law this cell exists for is *clamp exactly once*, and a plain `String`
//! cannot express half of it. A `String` in the trace owner could be a raw
//! `TraceWarning` `Display` that still has to be clamped, or a startup notice
//! that already was; code holding both then either under-clamps the first or
//! re-clamps the second. Re-clamping is not merely wasteful — it is what makes
//! the single real clamp untestable, because deleting it changes no observable
//! behaviour while a second pass still runs.
//!
//! So the proof travels in the type. [`BoundedDiagnostic::new`] is the only way
//! to make one, and it is the only call to the writer's bounded formatter in
//! this crate: delete the clamp inside it and an over-cap message reaches the
//! wire, which the hostile reds see immediately.
//!
//! It is its OWN module for exactly that reason. A newtype beside its users has
//! a private field those users can still fill in — privacy reaches descendants,
//! not ancestors — so `BoundedDiagnostic("anything".into())` would be legal one
//! module up and the invariant would be a convention. Here the field is
//! reachable from nowhere else in the tree, and bypassing the constructor is a
//! compile error rather than a review comment.

use std::fmt;

use vibe_workspace::compile_trace::bounded_diagnostic;

/// Text that has already passed the writer's bounded formatter.
///
/// Cloning is free of the "has this been clamped?" question by construction:
/// the invariant travels with the value, so a bounded notice folded into a
/// report is copied, never re-measured.
#[derive(Debug, Clone)]
pub(super) struct BoundedDiagnostic(String);

impl BoundedDiagnostic {
    /// The one clamp. Callers pass `format_args!`, so the unbounded
    /// intermediate never exists — the writer's sink streams into its own cap.
    pub(super) fn new(args: fmt::Arguments<'_>) -> Self {
        Self(bounded_diagnostic(args))
    }

    pub(super) fn as_str(&self) -> &str {
        &self.0
    }

    /// Drop the proof at the crate boundary, where the value becomes ordinary
    /// presentation text.
    pub(super) fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for BoundedDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
