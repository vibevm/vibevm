//! The one note a plan puts on a step that WILL run
//! (`PROP-066#not-covered`).
//!
//! What a `recipe` script downloads on its own — `curl`, `git clone`,
//! `pip install` inside the script — is outside vibe's fetches and
//! outside the rows, so no remap can redirect it. Under a `deny` policy
//! that is the difference between a build that works and a build that
//! stops halfway, and the reader has to see it in the plan rather than
//! discover it in the output.
//!
//! Its own cell because it is the one place the planner consults the
//! invocation's remap policy, and because the file it came out of sits at
//! its length budget.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#not-covered");

use specmark::spec;
use vibe_core::manifest::{Need, StepDecl, StepKind};

/// The marking of `PROP-066#not-covered`: a running `recipe` step that
/// declares `needs = "network"` under a `deny` policy downloads on its own,
/// and nothing in the rows can redirect it.
///
/// The policy is read from the invocation's one Locator, the same point
/// every fetch path consults — the plan must not reach a second answer to
/// "is this host an allowlist".
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#not-covered")]
pub(super) fn unremappable_network(run: bool, step: &StepDecl) -> Option<String> {
    let declares_network = step.needs.iter().any(|need| matches!(need, Need::Network));
    let denied = vibe_core::remap::active().policy() == vibe_core::remap::UnmatchedPolicy::Deny;
    (run && denied && declares_network && matches!(step.kind, StepKind::Recipe { .. }))
        .then(|| "needs network, not remappable".to_owned())
}
