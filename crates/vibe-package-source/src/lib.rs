//! The ONE production package-source composition — the [`InstallResolver`]
//! local / multi-registry / embedded dispatch, the R-001 cell-selection
//! registry behind it ([`cells`]), and the surface-neutral construction entry
//! [`build_install_resolver`] every surface builds its package source through
//! (the CLI today; the hosted MCP adapter in A15c later).
//!
//! Extracted verbatim from `vibe-cli` (R7.4 A15a) so both surfaces construct
//! the same algorithmic resolver instead of re-deriving it below the seam.
//! A surface's argument grammar never crosses this boundary: construction
//! reads the neutral [`PackageSourceOptions`], whose `Default` IS the hosted
//! posture — no flags, the resolvo default, the public auth walk, and
//! local / project / embedded discovery enabled.
//!
//! What deliberately stays UP in a surface:
//!
//! - **PROP-008 §2.6 short-name qualification** — the CLI's own input
//!   boundary with its exit-7 ambiguity refusal, injected here as a
//!   [`PackageQualifier`] and reached through [`RegistryPackageSource`];
//! - **the publish seam** (`direct_git_creator`) — a CLI composition-root
//!   cell that has nothing to do with resolving an install;
//! - argument parsing, confirmation, and rendering.
//!
//! The R-001 law itself is unchanged: the only constructors of the solver /
//! provider cells live in one named module — `cells.rs` here, fenced by the
//! crate's own exact-set RED (the conform engine's single-registry pin stays
//! with `vibe-cli` for now; recorded as conform-engine debt at the root).

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

mod builder;
pub(crate) mod cells;
mod options;
mod project_local;
mod qualified;
mod source;

pub use builder::build_install_resolver;
pub use options::PackageSourceOptions;
pub use qualified::{PackageQualifier, RefusesQualification, RegistryPackageSource};
pub use source::InstallResolver;

#[cfg(test)]
#[path = "fence_tests.rs"]
mod fence_tests;
