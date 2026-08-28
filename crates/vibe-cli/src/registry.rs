//! The CLI's cell-selection registry — R-001 ("flag at the seam, never in
//! the veins", GUIDE-RUST §3): the **only** module in the binary allowed
//! to construct seam cells. An explicit `match` is chosen over distributed
//! registration deliberately — one `match` is the system's table of
//! contents.
//!
//! Two tiers, never confused: cargo features answer "is the code in the
//! binary"; the runtime flags answer "is the cell selected".
//!
//! Since R7.4 A15a the install/package-source half of this registry — the
//! selection flags and the solver/provider cell constructors behind
//! `vibe install` — lives in the lower `vibe-package-source` crate
//! (`cells.rs` there, fenced by that crate's own exact-set RED), because
//! the ONE production package-source composition is shared with the later
//! hosted MCP adapter. What remains HERE is the CLI's own publish seam;
//! `conform.toml`'s R-001 pin still names this file, and the conform
//! engine's one-registry limitation is recorded as debt at the root.
//!
//! Enforced by `cargo xtask conform check` (R-001): constructing any
//! `#[cell]`-manifested type anywhere else in `vibe-cli` is a finding.

specmark::scope!(
    "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#configuration-sources-in-precedence-order"
);

use vibe_publish::DirectRepoCreator;

/// Construct the `RepoCreator/direct` cell for `vibe registry publish
/// --repo-url <url>` — the publish-seam construction site (R-001). The
/// host adapters (`github` / `gitverse`) are selected inside vibe-publish
/// by `creator_for_url`; the direct adapter is the one the CLI builds
/// from an explicit flag, so its construction lives here with the other
/// cell-selection sites and the publish command threads the instance in.
pub fn direct_git_creator(repo_url: String) -> DirectRepoCreator {
    DirectRepoCreator::new(repo_url)
}
