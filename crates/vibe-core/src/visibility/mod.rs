specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-050#model");

mod closure;
mod diagnostics;
mod effective;
mod graph;

pub use closure::analyze;
pub use diagnostics::Diagnostic;
pub use effective::{Analysis, Provenance, ProvenanceRule};
pub use graph::{EdgeDecl, NodeDecl, NodeId, VisibilityGraph};

#[cfg(test)]
mod tests;
