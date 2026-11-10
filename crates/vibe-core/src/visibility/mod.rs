specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-050#model");

mod closure;
mod diagnostics;
mod effective;
mod graph;
pub mod query;
mod weak;

pub use closure::analyze;
pub use diagnostics::Diagnostic;
pub use effective::{Analysis, Provenance, ProvenanceRule};
pub use graph::{EdgeDecl, NodeDecl, NodeId, VisibilityGraph};
pub use query::{
    AllowFriendsState, BlockReason, BlockedEdge, FriendsReport, InstalledWorld, WhyVerdict,
    friends, load_installed_world, why,
};
pub use weak::append_recommended_edges;

#[cfg(test)]
mod tests;
