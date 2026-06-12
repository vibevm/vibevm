//! `vibe install <kind>:<name>[@version] …` — plan → confirm → apply.
//!
//! Spec: `VIBEVM-SPEC.md` §5.6, §9.1, §11.1. The pipeline itself lives
//! in [`pipeline`]; the submodules carry the resolver construction,
//! plan-side helpers, and recording / reporting halves.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#install-workflow-in-detail");

mod pipeline;
mod planning;
mod recording;
mod resolver;

#[cfg(test)]
mod tests;

pub use pipeline::run;
pub(crate) use planning::exact_pinned_pkgref;
pub(crate) use resolver::{InstallResolver, build_install_resolver};
