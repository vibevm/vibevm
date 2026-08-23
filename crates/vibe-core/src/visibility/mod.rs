specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-050#model");

mod closure;
mod diagnostics;
mod effective;
mod graph;
pub mod query;

pub use closure::analyze;
pub use diagnostics::Diagnostic;
pub use effective::{Analysis, Provenance, ProvenanceRule};
pub use graph::{EdgeDecl, NodeDecl, NodeId, VisibilityGraph};
pub use query::{
    AllowFriendsState, BlockReason, BlockedEdge, FriendsReport, InstalledWorld, WhyVerdict,
    friends, load_installed_world, why,
};

#[cfg(test)]
mod tests;
