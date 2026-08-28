//! This surface's half of the compile-trace funnel: the registered report
//! families, and everything that turns a finished join into printed bytes.
//!
//! The OWNER and the funnel itself are no longer here. The session state, the
//! injected clock, the two funnel entry points, the supersession pass, the
//! bounded diagnostic, the member construction, [`CommandExit`],
//! [`PlanDisposition`], [`FinalizedCommand`] and the consuming [`finalize`] are
//! surface-neutral values and live in [`vibe_orchestrator::trace`] — a hosted
//! MCP surface runs exactly the same funnel over exactly the same recorder, and
//! a second copy of "is this workspace being traced right now" is the one thing
//! a cooperative lock cannot survive.
//!
//! What is genuinely OURS stays here, and the split is a dependency fact rather
//! than a taste:
//!
//! * [`draft::RegisteredReportDraft`] — the closed four-family sum of registered
//!   roots a CLI command boundary may return. It closes over four CLI render
//!   adapters and four clap-visible command identities, so it cannot cross the
//!   boundary without dragging them along;
//! * [`adapter`] — the one post-finalize order: attach, flush-or-discard, emit,
//!   route notices, return the original error;
//! * [`present`] — the human table and the quiet suffix;
//! * [`quiet`] — how a FAILED quiet command gets its suffix onto `main`'s one
//!   line without ever reformatting the error object.
//!
//! The two carriers this surface used to own are gone with the funnel: a
//! measured failure travels on the ONE generic
//! [`vibe_orchestrator::failure::Carried`], instantiated here with this
//! surface's own draft sum. Same owned evidence, same untouched error, same
//! site-frozen emission bit — one implementation, two evidence types.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE");

mod adapter;
mod draft;
mod present;
mod quiet;

pub(crate) use adapter::render_finalized;
pub(crate) use draft::{RegisteredReportDraft, carry, carry_measured, classify};
pub(crate) use quiet::detach as detach_quiet_suffix;

// The two funnel ENTRY points — open-under-a-root and stand-down-honestly —
// are deliberately absent from this list. A surface never chooses between them
// itself: the pairing with "which root may hold a trace" is ONE join, and the
// prelude epoch owns it (`RunPrelude::prepare_trace`). Re-exporting the arms
// here would put the fork back within reach of every command in the tree, and
// the red in `tests.rs` refuses either arm by name in production source.
pub(crate) use vibe_orchestrator::trace::{
    CommandExit, FinalizedCommand, PlanDisposition, TracePreparation, finalize,
};

#[cfg(test)]
mod tests;
